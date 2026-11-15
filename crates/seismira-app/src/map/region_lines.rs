//! Bölge sınırı vurgusu (Bölüm 9): etkilenen bölgelerin sınırları bir kez
//! GPU'ya yüklenir; her karede yalnızca hangi bölgenin hangi renk, kalınlık
//! ve opaklıkla çizileceği (küçük uniform dilimleri) güncellenir.

use eframe::wgpu::{self, util::DeviceExt as _};
use seismira_core::region::RegionId;
use std::collections::HashMap;
use std::ops::Range;

/// WGSL `VsIn` ile aynı.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub side: f32,
    pub end: f32,
}

/// WGSL `LineUniforms` ile aynı.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineUniforms {
    pub scale: [f32; 2],
    pub center: [f32; 2],
    pub viewport: [f32; 2],
    pub half_width: f32,
    pub first_copy: f32,
    pub color: [f32; 4],
}

/// WGSL `map.wgsl` `Uniforms` ile aynı (dolgu için harita shader'ı).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FillUniforms {
    pub scale: [f32; 2],
    pub center: [f32; 2],
    pub color: [f32; 4],
    pub first_copy: f32,
    pub _pad: [f32; 3],
}

/// CPU'da hazırlanmış sınır ve dolgu geometrisi (arka planda üretilir).
#[derive(Default)]
pub struct RegionLines {
    pub vertices: Vec<LineVertex>,
    pub ranges: HashMap<RegionId, Range<u32>>,
    pub fill_vertices: Vec<[f32; 2]>,
    pub fill_indices: Vec<u32>,
    /// Bölge → indeks aralığı.
    pub fill_ranges: HashMap<RegionId, Range<u32>>,
}

impl RegionLines {
    /// Verilen bölgelerin halkalarından parça dörtgenleri üretir.
    pub fn build(
        outlines: &seismira_core::region::Outlines,
        ids: impl IntoIterator<Item = RegionId>,
    ) -> Self {
        let mut out = Self::default();
        for id in ids {
            let Some(rings) = outlines.get(usize::from(id).wrapping_sub(1)) else {
                continue;
            };
            let start = out.vertices.len() as u32;
            for ring in rings {
                for seg in ring.points.windows(2) {
                    let (a, b) = (seg[0], seg[1]);
                    if a == b {
                        continue;
                    }
                    let v = |side: f32, end: f32| LineVertex { a, b, side, end };
                    out.vertices.extend_from_slice(&[
                        v(-1.0, 0.0),
                        v(1.0, 0.0),
                        v(-1.0, 1.0),
                        v(1.0, 0.0),
                        v(1.0, 1.0),
                        v(-1.0, 1.0),
                    ]);
                }
            }
            out.ranges.insert(id, start..out.vertices.len() as u32);
            let (fv, fi) = seismira_core::region::fill_mesh(rings);
            let base = out.fill_vertices.len() as u32;
            let istart = out.fill_indices.len() as u32;
            out.fill_vertices.extend(fv);
            out.fill_indices.extend(fi.into_iter().map(|i| base + i));
            out.fill_ranges
                .insert(id, istart..out.fill_indices.len() as u32);
        }
        out
    }
}

/// Bir karede çizilecek bölge sınırı.
#[derive(Clone, Debug)]
pub struct RegionDraw {
    pub range: Range<u32>,
    pub uniforms: LineUniforms,
}

/// Bir karede çizilecek bölge dolgusu.
#[derive(Clone, Debug)]
pub struct FillDraw {
    pub range: Range<u32>,
    pub uniforms: FillUniforms,
}

const SLOT: u64 = 256;
/// Bir karede vurgulanabilecek en fazla bölge. Natural Earth'te bazı
/// ülkelerin birinci düzey birimleri çok küçüktür (Slovenya 193, Birleşik
/// Krallık 232): Ljubljana yakınında Mw 8,0 için 392 bölge ölçüldü
/// (2026-11-15). Aşılırsa en zayıf bölgeler atlanır (bkz. `MapWidget`).
pub const MAX_REGIONS: usize = 1024;
/// Sınır çizimleri: bölge başına koyu kontur + renkli çizgi.
const MAX_LINE_DRAWS: usize = 2 * MAX_REGIONS;

/// Çizim başına bir dilim olacak şekilde uniform verisini tek seferde yazar.
fn write_slots<T: bytemuck::Pod>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    items: impl ExactSizeIterator<Item = T>,
) {
    let mut bytes = vec![0u8; items.len() * SLOT as usize];
    for (slot, item) in bytes
        .as_chunks_mut::<{ SLOT as usize }>()
        .0
        .iter_mut()
        .zip(items)
    {
        let src = bytemuck::bytes_of(&item);
        slot[..src.len()].copy_from_slice(src);
    }
    if !bytes.is_empty() {
        queue.write_buffer(buffer, 0, &bytes);
    }
}

pub struct RegionLinesGpu {
    pipeline: wgpu::RenderPipeline,
    pub uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: Option<wgpu::Buffer>,
    fill_pipeline: wgpu::RenderPipeline,
    fill_uniforms: wgpu::Buffer,
    fill_bind_group: wgpu::BindGroup,
    fill: Option<(wgpu::Buffer, wgpu::Buffer)>,
}

impl RegionLinesGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, msaa: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bölge sınırı"),
            source: wgpu::ShaderSource::Wgsl(include_str!("region_lines.wgsl").into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bölge sınırı"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(size_of::<LineUniforms>() as u64),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bölge sınırı"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bölge sınırı"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32
                    ],
                })],
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
            label: Some("bölge sınırı uniform"),
            size: SLOT * MAX_LINE_DRAWS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bölge sınırı"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniforms,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<LineUniforms>() as u64),
                }),
            }],
        });
        let (fill_pipeline, fill_uniforms, fill_bind_group) = fill_pipeline(device, format, msaa);
        Self {
            pipeline,
            uniforms,
            bind_group,
            vertices: None,
            fill_pipeline,
            fill_uniforms,
            fill_bind_group,
            fill: None,
        }
    }

    pub fn upload(&mut self, device: &wgpu::Device, lines: Option<&RegionLines>) {
        self.fill = lines.filter(|l| !l.fill_indices.is_empty()).map(|l| {
            (
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bölge dolgusu"),
                    contents: bytemuck::cast_slice(&l.fill_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bölge dolgusu"),
                    contents: bytemuck::cast_slice(&l.fill_indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            )
        });
        self.vertices = lines.filter(|l| !l.vertices.is_empty()).map(|l| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bölge sınırı"),
                contents: bytemuck::cast_slice(&l.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
    }

    pub fn write_fill_uniforms(&self, queue: &wgpu::Queue, draws: &[FillDraw]) {
        write_slots(
            queue,
            &self.fill_uniforms,
            draws.iter().take(MAX_REGIONS).map(|d| d.uniforms),
        );
    }

    pub fn draw_fills(
        &self,
        pass: &mut wgpu::RenderPass<'static>,
        draws: &[FillDraw],
        copies: Range<u32>,
    ) {
        let Some((vb, ib)) = &self.fill else { return };
        pass.set_pipeline(&self.fill_pipeline);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        for (i, d) in draws.iter().take(MAX_REGIONS).enumerate() {
            pass.set_bind_group(0, &self.fill_bind_group, &[(i as u64 * SLOT) as u32]);
            pass.draw_indexed(d.range.clone(), 0, copies.clone());
        }
    }

    pub fn write_uniforms(&self, queue: &wgpu::Queue, draws: &[RegionDraw]) {
        write_slots(
            queue,
            &self.uniforms,
            draws.iter().take(MAX_LINE_DRAWS).map(|d| d.uniforms),
        );
    }

    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'static>,
        draws: &[RegionDraw],
        copies: Range<u32>,
    ) {
        let Some(vb) = &self.vertices else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vb.slice(..));
        for (i, d) in draws.iter().take(MAX_LINE_DRAWS).enumerate() {
            pass.set_bind_group(0, &self.bind_group, &[(i as u64 * SLOT) as u32]);
            pass.draw(d.range.clone(), copies.clone());
        }
    }
}

/// Dolgu hattı: harita shader'ı (`map.wgsl`), üçgen listesi.
fn fill_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    msaa: u32,
) -> (wgpu::RenderPipeline, wgpu::Buffer, wgpu::BindGroup) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bölge dolgusu"),
        source: wgpu::ShaderSource::Wgsl(include_str!("map.wgsl").into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bölge dolgusu"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size_of::<FillUniforms>() as u64),
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bölge dolgusu"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bölge dolgusu"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            })],
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
        label: Some("bölge dolgusu uniform"),
        size: SLOT * MAX_REGIONS as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bölge dolgusu"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &uniforms,
                offset: 0,
                size: wgpu::BufferSize::new(size_of::<FillUniforms>() as u64),
            }),
        }],
    });
    (pipeline, uniforms, bind_group)
}
