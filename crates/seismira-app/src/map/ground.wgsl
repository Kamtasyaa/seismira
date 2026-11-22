// Sıvılaşma katmanı (kırmızı tonlu kareler). Doku sarsıntı ızgarasıyla aynı
// enlem/boylam ızgarasında, blok piramidi olarak (mip düzeyi k: 2^k × 2^k
// hücrenin ortalaması; ground_style::pyramid): r = renk sınıfı (0–5;
// çizilmeyecekse < 0), g = bloğun belirme anı (s). Ekranda bir kare en az
// MIN_PX olacak düzey seçilir; her karenin 1 piksellik koyu kenarı vardır
// (Shindo renklerinden ayrılsın). Kare belirme anından sonra `fade`
// saniyede belirir (fade = 0: anında, "Hareketi azalt"). Görünen durum
// zamanın saf fonksiyonu.

struct GroundUniforms {
    scale: vec2<f32>,
    center: vec2<f32>,
    bbox: vec4<f32>,
    north: f32,
    cell: f32,
    first_copy: f32,
    t: f32,
    fade: f32,
    cell_px: f32,
    _pad0: f32,
    _pad1: f32,
    colors: array<vec4<f32>, 6>,
};

@group(0) @binding(0) var<uniform> u: GroundUniforms;
@group(0) @binding(1) var cells: texture_2d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) inst: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let c = corners[vi];
    let world = vec2<f32>(mix(u.bbox.x, u.bbox.z, c.x), mix(u.bbox.y, u.bbox.w, c.y));
    let copy = u.first_copy + f32(inst);
    let p = (world + vec2<f32>(copy, 0.0) - u.center) * u.scale;
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    out.world = world;
    return out;
}

const PI: f32 = 3.14159265358979;

const MIN_PX: f32 = 8.0;
const EDGE: vec3<f32> = vec3<f32>(0.08, 0.0, 0.02);

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let levels = i32(textureNumLevels(cells));
    let dlon = (in.world.x - u.bbox.x) * 360.0;
    let lat = degrees(atan(sinh(PI * (1.0 - 2.0 * in.world.y))));
    var k = 0;
    if (u.cell_px < MIN_PX) {
        k = min(i32(ceil(log2(MIN_PX / max(u.cell_px, 1e-6)))), levels - 1);
    }
    let block = u.cell * exp2(f32(k));
    let fx = dlon / block;
    let fy = (u.north - lat) / block;
    let dims = vec2<i32>(textureDimensions(cells, k));
    let col = i32(floor(fx));
    let row = i32(floor(fy));
    if (col < 0 || row < 0 || col >= dims.x || row >= dims.y) {
        discard;
    }
    let v = textureLoad(cells, vec2<i32>(col, row), k);
    if (v.r < 0.0 || u.t < v.g) {
        discard;
    }
    var a = 1.0;
    if (u.fade > 0.0) {
        a = clamp((u.t - v.g) / u.fade, 0.0, 1.0);
    }
    var rgb = u.colors[u32(v.r)].rgb;
    // 1 piksellik koyu kenar (kare 4 pikselden büyükse).
    let px = u.cell_px * exp2(f32(k));
    if (px >= 4.0) {
        let e = 1.0 / px;
        if (fract(fx) < e || fract(fy) < e) {
            rgb = EDGE;
        }
    }
    return vec4<f32>(rgb * a, a);
}
