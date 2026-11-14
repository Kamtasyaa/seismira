//! Veri hazırlama aracı: ham veriyi (shapefile, GeoTIFF, CSV) uygulamanın
//! hızlı okuduğu kompakt ikili biçime çevirir.
//!
//! Kullanım:
//! ```text
//! seismira-data natural-earth <girdi klasörü> <çıktı klasörü>
//! ```

use anyhow::{Context, Result, bail};
use seismira_core::mapfile::{Layer, LayerKind, MapFile, RESTART};
use seismira_core::mercator;
use shapefile::{Point, Polygon, PolygonRing, Polyline, Shape, ShapeReader};
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, input, output] if cmd == "natural-earth" => {
            natural_earth(Path::new(input), Path::new(output))
        }
        _ => bail!("kullanım: seismira-data natural-earth <girdi klasörü> <çıktı klasörü>"),
    }
}

/// Natural Earth katmanlarını ölçek başına bir `.smap` dosyasına çevirir.
fn natural_earth(input: &Path, output: &Path) -> Result<()> {
    std::fs::create_dir_all(output)?;
    for scale in ["110m", "50m", "10m"] {
        let shp = |name: &str| input.join(format!("ne_{scale}_{name}.shp"));
        let layers = vec![
            polygons("land", &shp("land"))?,
            polygons("lakes", &shp("lakes"))?,
            lines("admin0", &shp("admin_0_boundary_lines_land"))?,
            lines("admin1", &shp("admin_1_states_provinces_lines"))?,
        ];
        let path = output.join(format!("ne_{scale}.smap"));
        let mut file = std::io::BufWriter::new(std::fs::File::create(&path)?);
        MapFile { layers }.write(&mut file)?;
        drop(file);
        let size = std::fs::metadata(&path)?.len();
        println!("→ {} ({:.1} MB)", path.display(), size as f64 / 1e6);
    }
    Ok(())
}

/// Tüm şekilleri okur; boş (NullShape) kayıtlar çağıran tarafta atlanır.
fn read_shapes(path: &Path) -> Result<Vec<Shape>> {
    ShapeReader::from_path(path)
        .and_then(|r| r.read())
        .with_context(|| format!("{} okunamadı", path.display()))
}

fn project(p: &Point) -> [f64; 2] {
    let (x, y) = mercator::to_unit(p.x, p.y);
    [x, y]
}

/// Halkanın kapanış noktasını (ilk noktanın tekrarı) atar.
fn open_ring(points: &[Point]) -> &[Point] {
    match points {
        [first, .., last] if first == last => &points[..points.len() - 1],
        _ => points,
    }
}

/// 180° meridyenini geçen kenar sayısı. Natural Earth geometrisi ±180°'de
/// bölünmüş olmalı (CLAUDE.md Bölüm 8); geçen kenar varsa uyarılır.
/// Kutup boyunca uzanan kenarlar (Antarktika'nın kapanışı, enlem ≈ −90°)
/// gerçek bir geçiş değildir; Mercator'da haritanın alt kenarına düşer.
fn antimeridian_crossings(points: &[Point]) -> usize {
    points
        .windows(2)
        .filter(|w| (w[1].x - w[0].x).abs() > 180.0)
        .filter(|w| w[0].y.abs().min(w[1].y.abs()) < 89.9)
        .count()
}

fn polygons(name: &str, path: &Path) -> Result<Layer> {
    let shapes: Vec<Polygon> = read_shapes(path)?
        .into_iter()
        .filter_map(|s| match s {
            Shape::Polygon(p) => Some(Ok(p)),
            Shape::NullShape => None,
            other => Some(Err(anyhow::anyhow!(
                "{name}: beklenmeyen şekil {}",
                other.shapetype()
            ))),
        })
        .collect::<Result<_>>()?;

    let mut vertices: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut earcut = earcut::Earcut::new();
    let mut tris: Vec<u32> = Vec::new();
    let (mut parts, mut crossings) = (0usize, 0usize);

    let mut orphan_holes = 0usize;
    for shape in &shapes {
        // Her deliği, gerçekten içinde bulunduğu dış halkaya bağla. Shapefile
        // sırası buna güvenmeye yetmez: Natural Earth 10m karada Hazar Denizi
        // deliği Avrasya'nın hemen ardından gelmez.
        let mut groups: Vec<Vec<&[Point]>> = Vec::new();
        let mut bboxes: Vec<[f64; 4]> = Vec::new();
        let mut holes: Vec<&[Point]> = Vec::new();
        for ring in shape.rings() {
            let pts = open_ring(ring.points());
            crossings += antimeridian_crossings(pts);
            match ring {
                PolygonRing::Outer(_) => {
                    bboxes.push(bbox(pts));
                    groups.push(vec![pts]);
                }
                PolygonRing::Inner(_) => holes.push(pts),
            }
        }
        for hole in holes {
            let Some(probe) = hole.first() else { continue };
            // İç içe dış halkalar varsa en küçük kapsayan seçilir.
            let owner = (0..groups.len())
                .filter(|&i| in_bbox(&bboxes[i], probe) && point_in_ring(groups[i][0], probe))
                .min_by(|&a, &b| bbox_area(&bboxes[a]).total_cmp(&bbox_area(&bboxes[b])));
            match owner {
                Some(i) => groups[i].push(hole),
                None => orphan_holes += 1,
            }
        }
        for group in groups {
            let mut flat: Vec<[f64; 2]> = Vec::new();
            let mut holes: Vec<u32> = Vec::new();
            for (i, ring) in group.iter().enumerate() {
                if i > 0 {
                    holes.push(flat.len() as u32);
                }
                flat.extend(ring.iter().map(project));
            }
            if flat.len() < 3 {
                continue;
            }
            earcut.earcut(flat.iter().copied(), &holes, &mut tris);
            let base = vertices.len() as u32;
            vertices.extend(flat.iter().map(|&[x, y]| [x as f32, y as f32]));
            indices.extend(tris.iter().map(|&i| base + i));
            parts += 1;
        }
    }
    report(
        name,
        path,
        shapes.len(),
        parts,
        vertices.len(),
        indices.len() / 3,
        crossings,
    );
    if orphan_holes > 0 {
        bail!("{name}: hiçbir dış halkanın içinde olmayan {orphan_holes} delik");
    }
    Ok(Layer {
        name: name.to_owned(),
        kind: LayerKind::Triangles,
        vertices,
        indices,
    })
}

fn bbox(pts: &[Point]) -> [f64; 4] {
    pts.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
    )
}

fn bbox_area(b: &[f64; 4]) -> f64 {
    (b[2] - b[0]) * (b[3] - b[1])
}

fn in_bbox(b: &[f64; 4], p: &Point) -> bool {
    p.x >= b[0] && p.x <= b[2] && p.y >= b[1] && p.y <= b[3]
}

/// Işın atma (çift-tek kuralı) ile nokta-halka içerme testi.
fn point_in_ring(ring: &[Point], p: &Point) -> bool {
    let mut inside = false;
    let mut j = ring.len().wrapping_sub(1);
    for i in 0..ring.len() {
        let (a, b) = (&ring[i], &ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn lines(name: &str, path: &Path) -> Result<Layer> {
    let shapes: Vec<Polyline> = read_shapes(path)?
        .into_iter()
        .filter_map(|s| match s {
            Shape::Polyline(p) => Some(Ok(p)),
            Shape::NullShape => None,
            other => Some(Err(anyhow::anyhow!(
                "{name}: beklenmeyen şekil {}",
                other.shapetype()
            ))),
        })
        .collect::<Result<_>>()?;

    let mut vertices: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let (mut parts, mut crossings) = (0usize, 0usize);
    for shape in &shapes {
        for part in shape.parts() {
            if part.len() < 2 {
                continue;
            }
            crossings += antimeridian_crossings(part);
            if !indices.is_empty() {
                indices.push(RESTART);
            }
            let base = vertices.len() as u32;
            vertices.extend(part.iter().map(|p| {
                let [x, y] = project(p);
                [x as f32, y as f32]
            }));
            indices.extend(base..vertices.len() as u32);
            parts += 1;
        }
    }
    report(
        name,
        path,
        shapes.len(),
        parts,
        vertices.len(),
        0,
        crossings,
    );
    Ok(Layer {
        name: name.to_owned(),
        kind: LayerKind::LineStrips,
        vertices,
        indices,
    })
}

fn report(
    name: &str,
    path: &Path,
    shapes: usize,
    parts: usize,
    verts: usize,
    tris: usize,
    crossings: usize,
) {
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    print!("{file:48} {name:7} şekil {shapes:5}  parça {parts:6}  köşe {verts:8}");
    if tris > 0 {
        print!("  üçgen {tris:8}");
    }
    if crossings > 0 {
        print!("  UYARI: 180°'yi geçen {crossings} kenar");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn nokta_halka_icinde() {
        let square = [pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)];
        assert!(point_in_ring(&square, &pt(5.0, 5.0)));
        assert!(!point_in_ring(&square, &pt(15.0, 5.0)));
        assert!(!point_in_ring(&square, &pt(5.0, -1.0)));
        // İçbükey (U şekli): girintideki nokta dışarıda.
        let u = [
            pt(0.0, 0.0),
            pt(9.0, 0.0),
            pt(9.0, 9.0),
            pt(6.0, 9.0),
            pt(6.0, 3.0),
            pt(3.0, 3.0),
            pt(3.0, 9.0),
            pt(0.0, 9.0),
        ];
        assert!(!point_in_ring(&u, &pt(4.5, 6.0)));
        assert!(point_in_ring(&u, &pt(1.5, 6.0)));
    }
}
