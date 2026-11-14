// Harita geometrisi: Web Mercator birim karesindeki köşeleri, görünümün
// merkez ve ölçeğine göre NDC'ye taşır. Yatay tekrar instance_index ile:
// her örnek bir dünya kopyasıdır (x + kopya).

struct Uniforms {
    scale: vec2<f32>,   // dünya birimi → NDC (y ters)
    center: vec2<f32>,  // görünüm merkezi (dünya birimi)
    color: vec4<f32>,   // önceden çarpılmış alfa, egui ile aynı renk uzayı
    first_copy: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs(@location(0) pos: vec2<f32>, @builtin(instance_index) inst: u32) -> @builtin(position) vec4<f32> {
    let copy = u.first_copy + f32(inst);
    let p = (pos + vec2<f32>(copy, 0.0) - u.center) * u.scale;
    return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return u.color;
}
