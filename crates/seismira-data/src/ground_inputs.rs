//! Faz 7 girdi katmanları (sıvılaşma: Zhu vd. 2017; heyelan: Nowicki
//! Jessee vd. 2018), Vs30 ızgarasında (30″, 180°B–180°D, 84°K–56°G):
//!
//! - `slope30`: eğim (derece), GMTED2010 md30 (30″ medyan yükseklik),
//!   Horn (1981) 3 × 3 farkları, enleme göre metrik hücre boyu.
//! - `cti`: birleşik topoğrafik indis, HYDRO1k tanımı (USGS HYDRO1k
//!   Documentation ReadMe, Bölüm 3.2.3 ve 3.2.5; Moore vd. 1991; Jessee 2018
//!   bu ürünü kullanmış): CTI = ln(FA / tan β), FA = hücrenin kendisi hariç
//!   yukarı havza alanı, km² cinsinden sayı (HYDRO1k'da 1 km hücre sayısı;
//!   tepelerde 0). Düz hücrede tan β yerine 0,001. HYDRO1k FA = 0'da ne
//!   yaptığını yazmıyor (ln 0 tanımsız); VARSAYIM: yaygın kural, FA'ya
//!   hücrenin kendisi eklenir (ln((FA + 1 hücre) / tan β), km²). Fark:
//!   HYDRO1k eğimi tam sayı derece en büyük komşu eğimi, burada Horn eğimi.
//!   Jessee CTI'yi 0–19'a kırpıyor; katman 0–25 saklar.
//! - `dr_km`: en yakın nehre uzaklık (km). Nehir = yukarı havza alanı
//!   ≥ [`RIVER_KM2`] olan hücre (VARSAYIM: HydroRIVERS'ın 10 km² ölçütü;
//!   Zhu 2017 HydroSHEDS kullanmış, eşiği doğrulanmadı), iç su hücresi
//!   (Vs30 göl kodu) ya da Natural Earth 10m nehir orta hattı (kamu malı;
//!   düz deltalarda D8 hattının kaymasını büyük nehirlerde düzeltir).
//! - `dc_km`: en yakın kıyıya (okyanus hücresine) uzaklık (km).
//! - `precip_mm`: yıllık yağış (CHELSA V2.1 bio12, 1981–2010; Zhu 2017
//!   WorldClim 1950–2000 kullanmış, fark docs/veri.md'de).
//!
//! Akış yönleri: öncelikli taşma (priority-flood; Barnes vd. 2014,
//! Computers & Geosciences 62, 117–127) ile, hücre keşfedildiği komşuya
//! akar (D8); çukur ve düzlükler bu sırayla çözülür. Çıkışlar: okyanus,
//! göl (Vs30 göl kodu: iç havzalarda — Hazar, Aral — sahte taşma nehri
//! oluşmasın diye göller havuz sayılır) ve ızgaranın kuzey/güney kenarı.
//! Uzaklıklar: 5 × 5 pahlı (chamfer) uzaklık dönüşümü, her satırda gerçek
//! metrik adım boylarıyla (en büyük yön hatası ~%2).

use crate::aig::{self, Aig};
use crate::geotiff::GeoTiff;
use crate::vs30::{CODE_LAKE, CODE_NODATA, CODE_OCEAN};
use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::{self, Encoded, Grid, LayerInfo, Raster, Scale};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Ortalama Dünya yarıçapı (km), IUGG R1.
const R_KM: f64 = 6371.0088;
/// Nehir eşiği: yukarı havza alanı (km²). VARSAYIM (bkz. modül belgesi).
pub const RIVER_KM2: f32 = 10.0;
/// Uzaklık katmanlarının üst sınırı (km). Zhu 2017'de −0,2054/km:
/// 50 km'de terim −10,3, olasılık ~0; ötesi kırpılır.
pub const DIST_MAX_KM: f32 = 50.0;

const SINK: u8 = 9;
const OUTLET: u8 = 8;
const UNSEEN: u8 = 255;
/// D8 komşuları: 0 D, 1 GD, 2 G, 3 GB, 4 B, 5 KB, 6 K, 7 KD (satır güneye artar).
const DX: [i32; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
const DY: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];

pub fn layers() -> Vec<LayerInfo> {
    let lin = |name: &str, hi: f32| LayerInfo {
        name: name.into(),
        scale: Scale::Linear,
        n_valid: 251,
        lo: 0.0,
        hi,
    };
    vec![
        lin("slope30", 75.0),
        lin("cti", 25.0),
        lin("dr_km", DIST_MAX_KM),
        lin("dc_km", DIST_MAX_KM),
        lin("precip_mm", 2500.0),
    ]
}

/// Hücre türü: 0 kara, 1 okyanus, 2 göl.
const LAND: u8 = 0;
const OCEAN: u8 = 1;
const LAKE: u8 = 2;

struct Ctx {
    grid: Grid,
    w: usize,
    h: usize,
    water: Vec<u8>,
}

impl Ctx {
    /// Komşu hücre dizini (boylamda sarar); satır dışındaysa `None`.
    #[inline]
    fn nb(&self, i: usize, k: usize) -> Option<usize> {
        let (x, y) = ((i % self.w) as i64, (i / self.w) as i64);
        let ny = y + i64::from(DY[k]);
        if ny < 0 || ny >= self.h as i64 {
            return None;
        }
        let nx = (x + i64::from(DX[k])).rem_euclid(self.w as i64);
        Some(ny as usize * self.w + nx as usize)
    }

    /// Satır `y` hücre merkezinin enlemi (derece).
    fn lat(&self, y: usize) -> f64 {
        self.grid.north - (y as f64 + 0.5) * self.grid.cell
    }

    /// Hücre alanı (km²).
    fn area_km2(&self, y: usize) -> f32 {
        let c = self.grid.cell.to_radians();
        (R_KM * R_KM * c * c * self.lat(y).to_radians().cos()) as f32
    }
}

/// Horn (1981) eğimi, tan β. `e` tüm ızgara (m).
fn tan_slope(ctx: &Ctx, e: &[i16], x: usize, y: usize) -> f32 {
    let (w, h) = (ctx.w, ctx.h);
    let c = ctx.grid.cell.to_radians();
    let dy_m = R_KM * 1000.0 * c;
    let dx_m = (dy_m * ctx.lat(y).to_radians().cos()).max(1.0);
    let ym = y.saturating_sub(1);
    let yp = (y + 1).min(h - 1);
    let (xm, xp) = ((x + w - 1) % w, (x + 1) % w);
    let z = |xx: usize, yy: usize| f64::from(e[yy * w + xx]);
    let (a, b, cc) = (z(xm, ym), z(x, ym), z(xp, ym));
    let (d, f) = (z(xm, y), z(xp, y));
    let (g, hh, i) = (z(xm, yp), z(x, yp), z(xp, yp));
    let dzdx = ((cc + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * dx_m);
    // Satırlar arası gerçek uzaklık: kenar satırında tek yönlü fark.
    let span = (yp - ym) as f64 / 2.0;
    let dzdy = ((g + 2.0 * hh + i) - (a + 2.0 * b + cc)) / (8.0 * dy_m * span.max(0.5));
    (dzdx * dzdx + dzdy * dzdy).sqrt() as f32
}

/// Tüm ızgaralık kod dizisini döşemelere böler ve kodlar.
fn encode_full(codes: &[u8], grid: &Grid) -> Result<Vec<Encoded>> {
    let (tx_n, ty_n) = (grid.tiles_x() as usize, grid.tiles_y() as usize);
    let t = grid.tile as usize;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let v: std::io::Result<Vec<Encoded>> = (0..tx_n * ty_n)
        .into_par_iter()
        .map(|k| {
            let (tx, ty) = (k % tx_n, k / tx_n);
            let mut tile = vec![CODE_NODATA; t * t];
            for y in 0..t {
                let gy = ty * t + y;
                if gy >= h {
                    break;
                }
                for x in 0..t {
                    let gx = tx * t + x;
                    if gx < w {
                        tile[y * t + x] = codes[gy * w + gx];
                    }
                }
            }
            raster::encode(&tile, |b| zstd::bulk::compress(b, 19))
        })
        .collect();
    Ok(v?)
}

/// Su hücrelerine özel kod verir (okyanus, göl); geri kalan `f(i)`.
fn codes_with_water(ctx: &Ctx, f: impl Fn(usize) -> u8 + Sync) -> Vec<u8> {
    (0..ctx.w * ctx.h)
        .into_par_iter()
        .map(|i| match ctx.water[i] {
            OCEAN => CODE_OCEAN,
            LAKE => CODE_LAKE,
            _ => f(i),
        })
        .collect()
}

/// 5 × 5 pahlı uzaklık dönüşümü (km). `dist`: kaynaklar 0, diğerleri ∞.
fn chamfer(ctx: &Ctx, dist: &mut [f32]) {
    let (w, h) = (ctx.w, ctx.h);
    // İleri geçiş komşuları (önceki satırlar ya da aynı satırda soldakiler).
    let mut fwd: Vec<(i32, i32)> = Vec::new();
    for dy in -2..=0i32 {
        for dx in -2..=2i32 {
            let prior = dy < 0 || dx < 0;
            // 5 × 5'te yalnızca asal yönler (gcd = 1) gerekir.
            let (a, b) = (dx.unsigned_abs(), dy.unsigned_abs());
            let prime = (a, b) == (1, 0) || (a, b) == (0, 1) || gcd(a, b) == 1;
            if prior && prime {
                fwd.push((dx, dy));
            }
        }
    }
    let c = ctx.grid.cell.to_radians() * R_KM;
    // Satır y ile y+dy arası adım boyu (orta enlem).
    let step = |y: usize, dx: i32, dy: i32| -> f32 {
        let ym = y as f64 + f64::from(dy) / 2.0;
        let lat = ctx.grid.north - (ym + 0.5) * ctx.grid.cell;
        let ex = f64::from(dx) * c * lat.to_radians().cos();
        let ey = f64::from(dy) * c;
        (ex * ex + ey * ey).sqrt() as f32
    };
    for _ in 0..2 {
        for (pass, offs) in [
            (0, fwd.clone()),
            (
                1,
                fwd.iter().map(|&(dx, dy)| (-dx, -dy)).collect::<Vec<_>>(),
            ),
        ] {
            let steps: Vec<Vec<f32>> = (0..h)
                .map(|y| offs.iter().map(|&(dx, dy)| step(y, dx, dy)).collect())
                .collect();
            let ys: Box<dyn Iterator<Item = usize>> = if pass == 0 {
                Box::new(0..h)
            } else {
                Box::new((0..h).rev())
            };
            for y in ys {
                let row_steps = &steps[y];
                for xi in 0..w {
                    let x = if pass == 0 { xi } else { w - 1 - xi };
                    let i = y * w + x;
                    let mut best = dist[i];
                    if best == 0.0 {
                        continue;
                    }
                    for (k, &(dx, dy)) in offs.iter().enumerate() {
                        let ny = y as i64 + i64::from(dy);
                        if ny < 0 || ny >= h as i64 {
                            continue;
                        }
                        let nx = (x as i64 + i64::from(dx)).rem_euclid(w as i64) as usize;
                        let d = dist[ny as usize * w + nx] + row_steps[k];
                        if d < best {
                            best = d;
                        }
                    }
                    dist[i] = best;
                }
            }
        }
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Çizgileri ızgaraya işler: her parça çeyrek hücre adımla örneklenir.
fn rasterize_lines(shp: &Path, grid: &Grid) -> Result<Vec<u32>> {
    let mut cells = Vec::new();
    for s in crate::read_shapes(shp)? {
        let parts: Vec<Vec<(f64, f64)>> = match s {
            shapefile::Shape::Polyline(p) => p
                .parts()
                .iter()
                .map(|q| q.iter().map(|pt| (pt.x, pt.y)).collect())
                .collect(),
            shapefile::Shape::PolylineZ(p) => p
                .parts()
                .iter()
                .map(|q| q.iter().map(|pt| (pt.x, pt.y)).collect())
                .collect(),
            shapefile::Shape::NullShape => continue,
            other => anyhow::bail!("nehir: beklenmeyen şekil {}", other.shapetype()),
        };
        for part in parts {
            for seg in part.windows(2) {
                let ((x0, y0), (x1, y1)) = (seg[0], seg[1]);
                let len = (x1 - x0).hypot(y1 - y0);
                let k = (len / (grid.cell / 4.0)).ceil().max(1.0) as usize;
                for j in 0..=k {
                    let t = j as f64 / k as f64;
                    if let Some((c, r)) = grid.cell_of(x0 + t * (x1 - x0), y0 + t * (y1 - y0)) {
                        cells.push(r * grid.width + c);
                    }
                }
            }
        }
    }
    cells.sort_unstable();
    cells.dedup();
    Ok(cells)
}

pub fn run(
    gmted: &Path,
    vs30: &Path,
    chelsa: &Path,
    rivers_shp: &Path,
    output: &Path,
) -> Result<()> {
    let t0 = std::time::Instant::now();
    let vs = Raster::open(std::fs::read(vs30)?)?;
    let grid = vs.grid;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let n = w * h;

    // 1. Su maskesi (Vs30 özel kodları).
    let mut water = vec![LAND; n];
    let t = grid.tile as usize;
    for ty in 0..grid.tiles_y() {
        for tx in 0..grid.tiles_x() {
            let tile = vs.tile(0, tx, ty)?;
            for y in 0..t {
                let gy = ty as usize * t + y;
                if gy >= h {
                    break;
                }
                for x in 0..t {
                    let gx = tx as usize * t + x;
                    if gx >= w {
                        break;
                    }
                    water[gy * w + gx] = match tile.get(grid.tile, x as u32, y as u32) {
                        CODE_OCEAN => OCEAN,
                        CODE_LAKE => LAKE,
                        _ => LAND,
                    };
                }
            }
        }
    }
    drop(vs);
    let ctx = Ctx { grid, w, h, water };
    let land = ctx.water.iter().filter(|&&c| c == LAND).count();
    println!(
        "su maskesi: kara {land} ({:.1}%), {:.0} s",
        100.0 * land as f64 / n as f64,
        t0.elapsed().as_secs_f64()
    );

    // 2. Yükseklik (GMTED md30). Izgara Vs30 ile aynı olmalı; GMTED'in
    // köşesi 1/7200° (0,5″, ~15 m) kaymış, ihmal edilir.
    let mut dem = Aig::open(gmted, crate::gmted_tile_name)?;
    ensure!(
        dem.width == grid.width
            && dem.height == grid.height
            && (dem.west - grid.west).abs() < grid.cell / 10.0
            && (dem.north - grid.north).abs() < grid.cell / 10.0,
        "GMTED ızgarası Vs30 ızgarasıyla aynı değil"
    );
    let mut elev = vec![0i16; n];
    let band = dem.block_h() * 64;
    for y0 in (0..grid.height).step_by(band as usize) {
        let m = band.min(grid.height - y0);
        let s = y0 as usize * w;
        dem.rows(y0, m, &mut elev[s..s + m as usize * w])?;
    }
    let nodata = elev.iter().filter(|&&v| v == aig::NODATA).count();
    elev.iter_mut()
        .filter(|v| **v == aig::NODATA)
        .for_each(|v| *v = 0);
    println!(
        "GMTED: {} CCITT bloğu RMin ile dolduruldu, nodata {nodata} (0 alındı), {:.0} s",
        dem.ccitt_blocks,
        t0.elapsed().as_secs_f64()
    );
    drop(dem);

    let infos = layers();
    let mut encoded: Vec<Vec<Encoded>> = Vec::new();

    // 3. Eğim.
    let codes = codes_with_water(&ctx, |i| {
        let ts = tan_slope(&ctx, &elev, i % w, i / w);
        infos[0].quantize(ts.atan().to_degrees())
    });
    encoded.push(encode_full(&codes, &grid)?);
    drop(codes);
    println!("eğim: {:.0} s", t0.elapsed().as_secs_f64());

    // 4. Öncelikli taşma → D8 yönleri.
    let mut dir = vec![UNSEEN; n];
    let mut buckets: Vec<Vec<u32>> = (0..65536).map(|_| Vec::new()).collect();
    let key = |e: i16| (i32::from(e) + 32768) as usize;
    for i in 0..n {
        if ctx.water[i] != LAND {
            dir[i] = SINK;
        }
    }
    let mut seeds = 0u64;
    for i in 0..n {
        if ctx.water[i] != LAND {
            continue;
        }
        let mut d = UNSEEN;
        let mut edge = false;
        for k in 0..8 {
            match ctx.nb(i, k) {
                Some(j) if ctx.water[j] != LAND => {
                    d = k as u8;
                    // Asal yön tercih edilir (çapraz yerine).
                    if k % 2 == 0 {
                        break;
                    }
                }
                None => edge = true,
                _ => {}
            }
        }
        if d == UNSEEN && edge {
            d = OUTLET;
        }
        if d != UNSEEN {
            dir[i] = d;
            buckets[key(elev[i])].push(i as u32);
            seeds += 1;
        }
    }
    let mut level = 0usize;
    let mut popped = 0u64;
    while level < buckets.len() {
        let mut q = 0;
        while q < buckets[level].len() {
            let c = buckets[level][q] as usize;
            q += 1;
            popped += 1;
            for k in 0..8 {
                let Some(j) = ctx.nb(c, k) else { continue };
                if dir[j] == UNSEEN {
                    dir[j] = ((k + 4) % 8) as u8;
                    buckets[key(elev[j]).max(level)].push(j as u32);
                }
            }
        }
        buckets[level] = Vec::new();
        level += 1;
    }
    drop(buckets);
    let unseen = dir.iter().filter(|&&d| d == UNSEEN).count();
    println!(
        "taşma: tohum {seeds}, işlenen {popped}, ulaşılmayan {unseen}, {:.0} s",
        t0.elapsed().as_secs_f64()
    );

    // 5. Akış birikimi (km²), zincir izleyen Kahn sıralaması.
    let down = |i: usize| -> Option<usize> {
        let d = dir[i];
        if d < 8 { ctx.nb(i, d as usize) } else { None }
    };
    let mut indeg = vec![0u8; n];
    for i in 0..n {
        if dir[i] < 8
            && let Some(j) = down(i)
            && dir[j] != SINK
        {
            indeg[j] += 1;
        }
    }
    let mut acc: Vec<f32> = (0..n)
        .map(|i| {
            if ctx.water[i] == LAND {
                ctx.area_km2(i / w)
            } else {
                0.0
            }
        })
        .collect();
    const DONE: u8 = 200;
    let mut done = 0u64;
    for i in 0..n {
        if ctx.water[i] != LAND || indeg[i] != 0 {
            continue;
        }
        let mut c = i;
        loop {
            indeg[c] = DONE;
            done += 1;
            let Some(d) = down(c) else { break };
            if dir[d] == SINK {
                break;
            }
            acc[d] += acc[c];
            indeg[d] -= 1;
            if indeg[d] == 0 {
                c = d;
            } else {
                break;
            }
        }
    }
    let max_acc = acc.iter().cloned().fold(0.0f32, f32::max);
    println!(
        "birikim: işlenen kara {done} / {land}, en büyük havza {max_acc:.0} km², {:.0} s",
        t0.elapsed().as_secs_f64()
    );
    ensure!(done == land as u64, "akış ağında döngü var");
    drop(dir);

    // 6. CTI.
    let codes = codes_with_water(&ctx, |i| {
        let ts = tan_slope(&ctx, &elev, i % w, i / w).max(1e-3);
        // FA + kendi hücresi (= acc, km²): bkz. modül belgesi.
        infos[1].quantize((f64::from(acc[i]) / f64::from(ts)).ln() as f32)
    });
    encoded.push(encode_full(&codes, &grid)?);
    drop(codes);
    drop(elev);
    println!("CTI: {:.0} s", t0.elapsed().as_secs_f64());

    // 7. Nehre ve kıyıya uzaklık. Natural Earth orta hatları ek kaynak:
    // düz deltalarda D8 hattı gerçek yataktan kayabiliyor (ör. New Orleans'ta
    // Mississippi'ye ~2 km); büyük nehirlerin konumunu bunlar sabitler.
    let ne_cells = rasterize_lines(rivers_shp, &grid)?;
    println!("Natural Earth nehir hücresi: {}", ne_cells.len());
    let rivers = (0..n)
        .filter(|&i| ctx.water[i] == LAND && acc[i] >= RIVER_KM2)
        .count();
    println!(
        "nehir hücresi (≥ {RIVER_KM2} km²): {rivers} ({:.1}% kara)",
        100.0 * rivers as f64 / land as f64
    );
    for (li, src) in [
        (
            2usize,
            // İç sular da kaynak: Vs30 göl kodu geniş nehir yataklarını
            // (ör. Amazon ana kolu) da kapsıyor; bunlar havuz sayıldığı için
            // içlerinde birikim yok.
            Box::new(|i: usize| {
                ctx.water[i] == LAKE || (ctx.water[i] == LAND && acc[i] >= RIVER_KM2)
            }) as Box<dyn Fn(usize) -> bool + Sync>,
        ),
        (3, Box::new(|i: usize| ctx.water[i] == OCEAN)),
    ] {
        let mut dist: Vec<f32> = (0..n)
            .into_par_iter()
            .map(|i| if src(i) { 0.0 } else { f32::INFINITY })
            .collect();
        if li == 2 {
            for &i in &ne_cells {
                dist[i as usize] = 0.0;
            }
        }
        chamfer(&ctx, &mut dist);
        let codes = codes_with_water(&ctx, |i| infos[li].quantize(dist[i].min(DIST_MAX_KM)));
        encoded.push(encode_full(&codes, &grid)?);
        println!("{}: {:.0} s", infos[li].name, t0.elapsed().as_secs_f64());
    }
    drop(acc);

    // 8. Yağış (CHELSA bio12).
    let mut pr = GeoTiff::open(chelsa)?;
    let (pw, ph) = pr.size();
    let ox = (grid.west - pr.west) / grid.cell;
    let oy = (pr.north - grid.north) / grid.cell;
    ensure!(
        pw as usize == w && (ox.abs() < 0.1) && (oy - oy.round()).abs() < 0.1,
        "CHELSA ızgarası beklenenden farklı (genişlik {pw}, kayma {ox}, {oy})"
    );
    let oy = oy.round() as u32;
    ensure!(oy + grid.height <= ph, "CHELSA güneyde kısa");
    let mut p = vec![f32::NAN; n];
    let band = 480u32;
    for y0 in (0..grid.height).step_by(band as usize) {
        let m = band.min(grid.height - y0);
        let s = y0 as usize * w;
        pr.rows(oy + y0, m, &mut p[s..s + m as usize * w])?;
    }
    // bio12: kg m⁻² yıl⁻¹ = mm/yıl, ölçek 0,1 (CHELSA V2.1 teknik belge;
    // örnek noktalarla aşağıda sınanır).
    let codes = codes_with_water(&ctx, |i| {
        let v = p[i];
        if v.is_nan() {
            CODE_NODATA
        } else {
            infos[4].quantize(v * PRECIP_SCALE)
        }
    });
    encoded.push(encode_full(&codes, &grid)?);
    drop(codes);
    println!("yağış: {:.0} s", t0.elapsed().as_secs_f64());

    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &infos, &encoded)?;
    drop(out);
    let size = std::fs::metadata(output)?.len();
    println!(
        "{}: {:.1} MB, toplam {:.0} s",
        output.display(),
        size as f64 / 1e6,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

/// CHELSA V2.1 bio12 ölçek çarpanı (ham değer × ölçek = mm/yıl).
const PRECIP_SCALE: f32 = 0.1;
