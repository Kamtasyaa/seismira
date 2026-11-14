//! Harita geometrisinin GPU'da tutulması ve çizimi (egui-wgpu paint callback).
//!
//! Geometri bir kez yüklenir; her karede yalnızca küçük bir uniform tamponu
//! (merkez, ölçek, renk) güncellenir. Kaydırma ve yakınlaştırma geometriyi
//! yeniden üretmez (CLAUDE.md Bölüm 11).

use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu::{self, util::DeviceExt as _};
use seismira_core::mapfile::{Layer, LayerKind, MapFile};
use std::ops::Range;

/// Ayrıntı düzeyleri (Natural Earth ölçekleri), kabadan inceye.
pub const LOD_FILES: [&str; 3] = ["ne_110m", "ne_50m", "ne_10m"];

/// Çizilen katmanlar; [`LayerId::name`] dosyadaki katman adıdır.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerId {
    Land,
    Lakes,
    Admin0,
    Admin1,
}

impl LayerId {
    const ALL: [LayerId; 4] = [
        LayerId::Land,
        LayerId::Lakes,
        LayerId::Admin0,
        LayerId::Admin1,
    ];

    fn name(self) -> &'static str {
        match self {
            LayerId::Land => "land",
            LayerId::Lakes => "lakes",
            LayerId::Admin0 => "admin0",
            LayerId::Admin1 => "admin1",
        }
    }
}

/// Bir çizim komutunun uniform verisi. WGSL `Uniforms` ile birebir aynı.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub scale: [f32; 2],
    pub center: [f32; 2],
    pub color: [f32; 4],
    pub first_copy: f32,
    pub _pad: [f32; 3],
}

/// Her çizim komutu uniform tamponunda ayrı bir dilim kullanır. 256 bayt,
/// tüm arka uçlarda `min_uniform_buffer_offset_alignment` üst sınırıdır.
const SLOT: u64 = 256;
const MAX_DRAWS: usize = 16;

struct GpuLayer {
    kind: LayerKind,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

/// GPU kaynakları; egui-wgpu'nun `CallbackResources` içinde yaşar.
pub struct MapGpu {
    fill: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    lods: [Vec<(LayerId, GpuLayer)>; 3],
}

impl MapGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, msaa: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("harita"),
            source: wgpu::ShaderSource::Wgsl(include_str!("map.wgsl").into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("harita"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(size_of::<Uniforms>() as u64),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("harita"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = |topology, strip_index_format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("harita"),
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
                primitive: wgpu::PrimitiveState {
                    topology,
                    strip_index_format,
                    ..Default::default()
                },
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
            })
        };
        let fill = pipeline(wgpu::PrimitiveTopology::TriangleList, None);
        let line = pipeline(
            wgpu::PrimitiveTopology::LineStrip,
            Some(wgpu::IndexFormat::Uint32),
        );
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("harita uniform"),
            size: SLOT * MAX_DRAWS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("harita"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniforms,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<Uniforms>() as u64),
                }),
            }],
        });
        Self {
            fill,
            line,
            uniforms,
            bind_group,
            lods: Default::default(),
        }
    }

    /// Bir ayrıntı düzeyinin katmanlarını GPU'ya yükler.
    pub fn upload(&mut self, device: &wgpu::Device, lod: usize, map: &MapFile) {
        let mut layers = Vec::new();
        for id in LayerId::ALL {
            let Some(layer) = map.layer(id.name()) else {
                log::warn!("{}: '{}' katmanı yok", LOD_FILES[lod], id.name());
                continue;
            };
            if let Some(gpu) = upload_layer(device, layer) {
                layers.push((id, gpu));
            }
        }
        self.lods[lod] = layers;
    }

    pub fn has_lod(&self, lod: usize) -> bool {
        !self.lods[lod].is_empty()
    }
}

fn upload_layer(device: &wgpu::Device, layer: &Layer) -> Option<GpuLayer> {
    if layer.indices.is_empty() {
        return None;
    }
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&layer.name),
        contents: bytemuck::cast_slice(&layer.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&layer.name),
        contents: bytemuck::cast_slice(&layer.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    Some(GpuLayer {
        kind: layer.kind,
        vertices,
        indices,
        index_count: layer.indices.len() as u32,
    })
}

/// Tek bir katman çizimi.
#[derive(Clone, Debug)]
pub struct Draw {
    pub layer: LayerId,
    pub uniforms: Uniforms,
    /// Dünya kopyaları (instance aralığı; `first_copy` ile birlikte).
    pub copies: Range<u32>,
}

/// Bir karede haritayı çizen callback.
pub struct MapPaint {
    pub lod: usize,
    pub draws: Vec<Draw>,
}

impl CallbackTrait for MapPaint {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(gpu) = resources.get::<MapGpu>() {
            for (i, draw) in self.draws.iter().take(MAX_DRAWS).enumerate() {
                queue.write_buffer(
                    &gpu.uniforms,
                    i as u64 * SLOT,
                    bytemuck::bytes_of(&draw.uniforms),
                );
            }
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(gpu) = resources.get::<MapGpu>() else {
            return;
        };
        let layers = &gpu.lods[self.lod];
        for (i, draw) in self.draws.iter().take(MAX_DRAWS).enumerate() {
            let Some((_, layer)) = layers.iter().find(|(id, _)| *id == draw.layer) else {
                continue;
            };
            pass.set_pipeline(match layer.kind {
                LayerKind::Triangles => &gpu.fill,
                LayerKind::LineStrips => &gpu.line,
            });
            pass.set_bind_group(0, &gpu.bind_group, &[(i as u64 * SLOT) as u32]);
            pass.set_vertex_buffer(0, layer.vertices.slice(..));
            pass.set_index_buffer(layer.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..layer.index_count, 0, draw.copies.clone());
        }
    }
}

/// `egui_wgpu::Callback` kısayolu.
pub fn paint_callback(rect: egui::Rect, paint: MapPaint) -> egui::PaintCallback {
    egui_wgpu::Callback::new_paint_callback(rect, paint)
}
