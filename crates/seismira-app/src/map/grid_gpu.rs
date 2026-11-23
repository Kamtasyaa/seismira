//! Sarsıntı ızgarası katmanının GPU kaynakları.
//!
//! Izgara bir kez dokuya yüklenir (Rgba32Float: son Shindo sınıfı, S varışı,
//! bir sınıf yükselme süresi, boş);
//! her karede yalnızca küçük bir uniform (görünüm, zaman) güncellenir.
//! Görünen renk shader'da zamanın saf fonksiyonu olarak hesaplanır.

use eframe::wgpu;
use seismira_core::grid::{GridSpec, NO_DATA, ShakeGrid};

/// WGSL `GridUniforms` ile birebir aynı.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridUniforms {
    pub scale: [f32; 2],
    pub center: [f32; 2],
    pub bbox: [f32; 4],
    pub north: f32,
    pub cell: f32,
    pub first_copy: f32,
    pub t: f32,
    pub opacity: f32,
    pub _pad: [f32; 3],
    pub colors: [[f32; 4]; 10],
}

pub struct GridGpu {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    pub uniforms: wgpu::Buffer,
    /// Yüklü ızgara: bağlama grubu ve geometrisi.
    pub loaded: Option<(wgpu::BindGroup, GridSpec)>,
}

impl GridGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, msaa: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sarsıntı ızgarası"),
            source: wgpu::ShaderSource::Wgsl(include_str!("grid.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sarsıntı ızgarası"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<GridUniforms>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sarsıntı ızgarası"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sarsıntı ızgarası"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: msaa,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sarsıntı ızgarası uniform"),
            size: size_of::<GridUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            layout,
            uniforms,
            loaded: None,
        }
    }

    /// Izgarayı dokuya yükler (öncekinin yerine).
    /// `scale`: Shindo ya da MMI (+ Mw). MMI'da doku sınıfı MMI − 1
    /// (Worden vd. 2012, hücrenin büyük bileşen PGV'sinden; mesafe katsayısı
    /// sıfır olduğundan Rrup gerekmez) ve bir sınıf yükselme süresi, son
    /// sınıfa Shindo ile aynı anda varacak şekilde ölçeklenir.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        grid: &ShakeGrid,
        (scale, mw): (crate::mmi_style::Scale, f64),
    ) {
        let (w, h) = (grid.spec.width, grid.spec.height);
        let max = device.limits().max_texture_dimension_2d;
        if w > max || h > max {
            log::error!("sarsıntı ızgarası {w}×{h} doku sınırını ({max}) aşıyor");
            self.loaded = None;
            return;
        }
        let texels: Vec<[f32; 4]> = grid
            .class
            .iter()
            .zip(&grid.t_s)
            .zip(&grid.rise)
            .zip(&grid.pgv)
            .map(|(((&c, &t), &rise), &pgv)| {
                if c == NO_DATA {
                    return [-1.0, f32::INFINITY, 1.0, 0.0];
                }
                match scale {
                    crate::mmi_style::Scale::Shindo => [f32::from(c), t, rise, 0.0],
                    crate::mmi_style::Scale::Mmi => {
                        if !pgv.is_finite() || pgv <= 0.0 {
                            return [-1.0, f32::INFINITY, 1.0, 0.0];
                        }
                        // Rrup: C6 = 0 (bkz. seismira_core::mmi), değer etkisiz.
                        let mmi = seismira_core::mmi::mmi_from_pgv_larger(f64::from(pgv), mw, 10.0);
                        let idx = f32::from(crate::mmi_style::class(mmi) - 1);
                        // Shindo c sınıfa c adımda (1'den başlayıp) varır;
                        // MMI idx'e aynı sürede varsın.
                        let rise = if idx >= 1.0 && c >= 1 {
                            rise * f32::from(c) / idx
                        } else {
                            rise
                        };
                        [idx, t, rise, 0.0]
                    }
                }
            })
            .collect();
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sarsıntı ızgarası"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&texels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 16),
                rows_per_image: Some(h),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sarsıntı ızgarası"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        self.loaded = Some((bind_group, grid.spec));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'static>, copies: std::ops::Range<u32>) {
        if let Some((bg, _)) = &self.loaded {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..6, copies);
        }
    }
}
