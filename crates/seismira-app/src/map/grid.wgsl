// Sarsıntı ızgarası katmanı. Doku enlem/boylam ızgarasındadır (r: son
// Shindo sınıfı, veri yoksa < 0; g: S varışı, s; b: bir sınıf yükselme
// süresi, s). Parça gölgelendiricisi ekran noktasını Mercator'dan
// enlem/boylama çevirip hücreyi okur; yeniden örnekleme yok. Görünen sınıf
// zamanın saf fonksiyonu: S ulaştıktan sonra 1'den başlayıp her `b`
// saniyede bir artar (shaking::SiteResult::shindo_at ile aynı kural).

struct GridUniforms {
    scale: vec2<f32>,
    center: vec2<f32>,
    // Izgaranın Mercator sınır kutusu (x0, y0 kuzey, x1, y1 güney).
    bbox: vec4<f32>,
    north: f32,
    cell: f32,
    first_copy: f32,
    t: f32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    colors: array<vec4<f32>, 10>,
};

@group(0) @binding(0) var<uniform> u: GridUniforms;
@group(0) @binding(1) var cells: texture_2d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) inst: u32) -> VsOut {
    // İki üçgenlik dörtgen.
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

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(cells));
    let dlon = (in.world.x - u.bbox.x) * 360.0;
    let lat = degrees(atan(sinh(PI * (1.0 - 2.0 * in.world.y))));
    let col = i32(floor(dlon / u.cell));
    let row = i32(floor((u.north - lat) / u.cell));
    if (col < 0 || row < 0 || col >= dims.x || row >= dims.y) {
        discard;
    }
    let v = textureLoad(cells, vec2<i32>(col, row), 0);
    let final_class = v.r;
    let t_s = v.g;
    if (final_class < 1.0 || u.t < t_s) {
        discard;
    }
    let shown = min(final_class, 1.0 + floor((u.t - t_s) / v.b + 1e-6));
    let c = u.colors[u32(shown)];
    return vec4<f32>(c.rgb * u.opacity, u.opacity);
}
