//! Sıvılaşma katmanının GPU kaynakları (grid_gpu.rs ile aynı yapı).
//!
//! Kaplama bir kez dokuya yüklenir (Rgba32Float: renk sınıfı, hücrede
//! sarsıntının bittiği an); her karede yalnızca uniform güncellenir.

use eframe::wgpu;
use seismira_core::grid::{GridSpec, ShakeGrid};
use seismira_core::groundfailure::liquefaction_grid::LiquefactionGrid;

/// WGSL `GroundUniforms` ile birebir aynı.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GroundUniforms {
    pub scale: [f32; 2],
    pub center: [f32; 2],
    pub bbox: [f32; 4],
    pub north: f32,
    pub cell: f32,
    pub first_copy: f32,
    pub t: f32,
    pub fade: f32,
    /// Düzey 0 hücresinin ekrandaki kenarı (piksel).
    pub cell_px: f32,
    pub _pad: [f32; 2],
    pub colors: [[f32; 4]; 6],
}

pub struct GroundGpu {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    pub uniforms: wgpu::Buffer,
    /// Yüklü ızgara: bağlama grubu ve geometrisi.
    pub loaded: Option<(wgpu::BindGroup, GridSpec)>,
}

impl GroundGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, msaa: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sıvılaşma"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ground.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sıvılaşma"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<GroundUniforms>() as u64),
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
            label: Some("sıvılaşma"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sıvılaşma"),
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
            label: Some("sıvılaşma uniform"),
            size: size_of::<GroundUniforms>() as u64,
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

    /// Kaplamayı dokuya yükler (öncekinin yerine). `shake`: aynı ızgaranın
    /// sarsıntı sonucu (hücrede sarsıntının bittiği an).
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        liq: &LiquefactionGrid,
        shake: &ShakeGrid,
    ) {
        let (w, h) = (liq.spec.width, liq.spec.height);
        let max = device.limits().max_texture_dimension_2d;
        if w > max || h > max || shake.spec != liq.spec {
            log::error!("sıvılaşma ızgarası {w}×{h} yüklenemedi (doku sınırı ya da ızgara farkı)");
            self.loaded = None;
            return;
        }
        let (pw, ph, levels) = crate::ground_style::pyramid(&liq.coverage, &shake.t_end, w, h);
        if pw > max || ph > max {
            log::error!("sıvılaşma dokusu {pw}×{ph} sınırı ({max}) aşıyor");
            self.loaded = None;
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sıvılaşma"),
            size: wgpu::Extent3d {
                width: pw,
                height: ph,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (k, texels) in levels.iter().enumerate() {
            let (lw, lh) = (pw >> k, ph >> k);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: k as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(texels),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(lw * 16),
                    rows_per_image: Some(lh),
                },
                wgpu::Extent3d {
                    width: lw,
                    height: lh,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&Default::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sıvılaşma"),
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
        self.loaded = Some((bind_group, liq.spec));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'static>, copies: std::ops::Range<u32>) {
        if let Some((bg, _)) = &self.loaded {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..6, copies);
        }
    }
}
