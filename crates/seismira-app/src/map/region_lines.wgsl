// Bölge sınırı vurgusu: ekran ölçeğinde kalın çizgi. Her sınır parçası
// iki üçgenlik bir dörtgendir; köşeler parçanın iki ucunu (a, b), dik yönde
// tarafı (side = ±1) ve hangi uçta olduğunu (end = 0/1) taşır. Genişlik ve
// kare uç uzatması piksel cinsinden vertex shader'da uygulanır; böylece
// yakınlaştırmada çizgi kalınlığı sabit kalır ve geometri yeniden
// üretilmez. Renk ve opaklık (yanıp sönme) uniform'dan gelir.

struct LineUniforms {
    scale: vec2<f32>,      // dünya birimi → NDC
    center: vec2<f32>,
    viewport: vec2<f32>,   // fiziksel piksel
    half_width: f32,       // piksel
    first_copy: f32,
    color: vec4<f32>,      // önceden çarpılmış alfa
};

@group(0) @binding(0) var<uniform> u: LineUniforms;

struct VsIn {
    @location(0) a: vec2<f32>,
    @location(1) b: vec2<f32>,
    @location(2) side: f32,
    @location(3) end: f32,
};

@vertex
fn vs(v: VsIn, @builtin(instance_index) inst: u32) -> @builtin(position) vec4<f32> {
    let copy = vec2<f32>(u.first_copy + f32(inst), 0.0);
    let pa = (v.a + copy - u.center) * u.scale;
    let pb = (v.b + copy - u.center) * u.scale;
    let half_vp = u.viewport * 0.5;
    let d = (pb - pa) * half_vp;
    let len = length(d);
    var dir = vec2<f32>(1.0, 0.0);
    if (len > 1e-6) {
        dir = d / len;
    }
    let n = vec2<f32>(-dir.y, dir.x);
    let base = mix(pa, pb, v.end);
    let off_px = n * v.side * u.half_width + dir * (v.end * 2.0 - 1.0) * u.half_width;
    return vec4<f32>(base + off_px / half_vp, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return u.color;
}
