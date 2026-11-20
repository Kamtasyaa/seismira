//! Döşemeli, kuantalanmış coğrafi raster biçimi (`.sras`).
//!
//! Enlem-boylam ızgarası (eşit açılı), her hücre bir `u8` kod. Kodlar
//! `0..n_degerli` aralığında katmanın değer aralığına (doğrusal ya da
//! logaritmik) eşlenir; kalan kodlar katmana özel "özel değerler"dir
//! (ör. Vs30'da okyanus, göl). Raster `tile × tile` döşemelere bölünür ve
//! her döşeme ayrı bir zstd çerçevesi olarak saklanır; tamamı aynı koddan
//! oluşan döşeme (ör. açık deniz) hiç yazılmaz, yalnızca dizinde kodu tutulur.
//! Böylece uygulama dosyanın tamamını belleğe almadan yalnızca ilgili
//! bölgenin döşemelerini açar.
//!
//! Biçim (little-endian):
//! ```text
//! "SRAS"  u32 sürüm
//! f64 batı  f64 kuzey  f64 hücre (derece)
//! u32 genişlik  u32 yükseklik  u32 döşeme  u32 katman sayısı
//! her katman: u8 ölçek (0 doğrusal, 1 log), u8 n_degerli, 2 bayt boşluk,
//!             f32 alt, f32 üst, u32 ad uzunluğu, ad (UTF-8, 4'e tamamlanır)
//! dizin: katman → döşeme satırı → döşeme sütunu sırasıyla,
//!        her giriş u64 konum, u32 uzunluk, u8 dolgu kodu, 3 bayt boşluk
//!        (uzunluk 0 ise döşemenin tamamı dolgu kodudur)
//! döşeme verisi: zstd çerçeveleri, her biri tile × tile bayt (kenar
//!        döşemeleri de tam boyutta, taşan kısım dolgu)
//! ```

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

const MAGIC: &[u8; 4] = b"SRAS";
pub const VERSION: u32 = 1;
const INDEX_ENTRY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Scale {
    Linear = 0,
    Log = 1,
}

/// Bir katmanın kod → değer eşlemesi.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerInfo {
    pub name: String,
    pub scale: Scale,
    /// `0..n_valid` kodları değerdir; `n_valid..=255` özel kodlardır.
    pub n_valid: u8,
    pub lo: f32,
    pub hi: f32,
}

impl LayerInfo {
    /// Değeri en yakın koda çevirir; aralık dışı değerler uca kırpılır.
    pub fn quantize(&self, v: f32) -> u8 {
        let n = f32::from(self.n_valid - 1);
        let t = match self.scale {
            Scale::Linear => (v - self.lo) / (self.hi - self.lo),
            Scale::Log => (v / self.lo).ln() / (self.hi / self.lo).ln(),
        };
        (t.clamp(0.0, 1.0) * n).round() as u8
    }

    /// Kodu değere çevirir; özel kodlarda `None`.
    pub fn value(&self, code: u8) -> Option<f32> {
        if code >= self.n_valid {
            return None;
        }
        let t = f32::from(code) / f32::from(self.n_valid - 1);
        Some(match self.scale {
            Scale::Linear => self.lo + t * (self.hi - self.lo),
            Scale::Log => self.lo * (self.hi / self.lo).powf(t),
        })
    }
}

/// Izgara geometrisi. `west`/`north` sol üst köşenin KENARIDIR (hücre
/// merkezi değil); hücre (sütun, satır) `west + sütun·cell` ile
/// `west + (sütun+1)·cell` arasını kaplar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub west: f64,
    pub north: f64,
    pub cell: f64,
    pub width: u32,
    pub height: u32,
    pub tile: u32,
}

impl Grid {
    pub fn tiles_x(&self) -> u32 {
        self.width.div_ceil(self.tile)
    }

    pub fn tiles_y(&self) -> u32 {
        self.height.div_ceil(self.tile)
    }

    /// Izgara boylamda tüm dünyayı kaplıyor mu (yatay sarma)?
    pub fn wraps(&self) -> bool {
        (f64::from(self.width) * self.cell - 360.0).abs() < self.cell * 0.5
    }

    /// Noktanın hücresi (sütun, satır); ızgara dışındaysa `None`.
    pub fn cell_of(&self, lon: f64, lat: f64) -> Option<(u32, u32)> {
        let mut dx = lon - self.west;
        if self.wraps() {
            dx = dx.rem_euclid(360.0);
        }
        let col = (dx / self.cell).floor();
        let row = ((self.north - lat) / self.cell).floor();
        let ok =
            col >= 0.0 && row >= 0.0 && col < f64::from(self.width) && row < f64::from(self.height);
        ok.then_some((col as u32, row as u32))
    }
}

/// Yazmaya hazır döşeme.
#[derive(Clone, Debug, PartialEq)]
pub enum Encoded {
    /// Tamamı aynı kod; dosyaya yazılmaz.
    Fill(u8),
    /// zstd çerçevesi (açılınca tile² bayt).
    Zstd(Vec<u8>),
}

/// Döşemeyi (tile² kod) kodlar. `compress` bir zstd çerçevesi üretmelidir
/// (veri aracı C zstd kullanır, çekirdek yalnızca çözer).
pub fn encode(
    codes: &[u8],
    compress: impl FnOnce(&[u8]) -> io::Result<Vec<u8>>,
) -> io::Result<Encoded> {
    match codes.first() {
        Some(&c0) if codes.iter().all(|&c| c == c0) => Ok(Encoded::Fill(c0)),
        _ => Ok(Encoded::Zstd(compress(codes)?)),
    }
}

/// Yazma: `tiles[katman][ty * tiles_x + tx]`.
pub fn write(
    w: &mut impl Write,
    grid: &Grid,
    layers: &[LayerInfo],
    tiles: &[Vec<Encoded>],
) -> io::Result<()> {
    let n_tiles = (grid.tiles_x() * grid.tiles_y()) as usize;
    if tiles.len() != layers.len() || tiles.iter().any(|l| l.len() != n_tiles) {
        return Err(bad("döşeme sayısı ızgarayla uyuşmuyor"));
    }

    let mut head = Vec::new();
    head.extend_from_slice(MAGIC);
    head.extend_from_slice(&VERSION.to_le_bytes());
    for v in [grid.west, grid.north, grid.cell] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    let n_layers = u32::try_from(layers.len()).map_err(|_| bad("katman sayısı"))?;
    for v in [grid.width, grid.height, grid.tile, n_layers] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    for l in layers {
        if l.n_valid < 2 {
            return Err(bad("n_valid en az 2 olmalı"));
        }
        head.extend_from_slice(&[l.scale as u8, l.n_valid, 0, 0]);
        head.extend_from_slice(&l.lo.to_le_bytes());
        head.extend_from_slice(&l.hi.to_le_bytes());
        let name = l.name.as_bytes();
        head.extend_from_slice(&(name.len() as u32).to_le_bytes());
        head.extend_from_slice(name);
        head.resize(head.len().next_multiple_of(4), 0);
    }

    let mut offset = (head.len() + layers.len() * n_tiles * INDEX_ENTRY) as u64;
    let mut index = Vec::with_capacity(layers.len() * n_tiles * INDEX_ENTRY);
    for t in tiles.iter().flatten() {
        let (pos, len, fill) = match t {
            Encoded::Fill(c) => (0, 0, *c),
            Encoded::Zstd(z) if z.is_empty() => return Err(bad("boş döşeme")),
            Encoded::Zstd(z) => {
                let len = u32::try_from(z.len()).map_err(|_| bad("döşeme çok büyük"))?;
                (offset, len, 0)
            }
        };
        index.extend_from_slice(&pos.to_le_bytes());
        index.extend_from_slice(&len.to_le_bytes());
        index.extend_from_slice(&[fill, 0, 0, 0]);
        offset += u64::from(len);
    }

    w.write_all(&head)?;
    w.write_all(&index)?;
    for t in tiles.iter().flatten() {
        if let Encoded::Zstd(z) = t {
            w.write_all(z)?;
        }
    }
    Ok(())
}

/// Okuyucu. `B` dosyanın tamamı (ör. memmap) ya da test için bir `Vec`.
/// Açılan döşemeler sınırlı bir önbellekte tutulur; iş parçacıkları
/// arasında paylaşılabilir.
pub struct Raster<B> {
    bytes: B,
    pub grid: Grid,
    pub layers: Vec<LayerInfo>,
    index_start: usize,
    cache: Mutex<HashMap<u64, Arc<[u8]>>>,
    cache_cap: usize,
}

#[derive(Clone)]
pub enum Tile {
    /// Tamamı aynı kod.
    Fill(u8),
    Data(Arc<[u8]>),
}

impl Tile {
    pub fn get(&self, tile: u32, x: u32, y: u32) -> u8 {
        match self {
            Tile::Fill(c) => *c,
            Tile::Data(d) => d[(y * tile + x) as usize],
        }
    }
}

impl<B: AsRef<[u8]>> Raster<B> {
    pub fn open(bytes: B) -> io::Result<Self> {
        let mut r = Cursor {
            b: bytes.as_ref(),
            p: 0,
        };
        if r.take(4)? != MAGIC {
            return Err(bad("SRAS dosyası değil"));
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(bad(&format!("desteklenmeyen sürüm {version}")));
        }
        let (west, north, cell) = (r.f64()?, r.f64()?, r.f64()?);
        let (width, height, tile, n_layers) = (r.u32()?, r.u32()?, r.u32()?, r.u32()?);
        if tile == 0 || width == 0 || height == 0 || !cell.is_finite() || cell <= 0.0 {
            return Err(bad("geçersiz ızgara"));
        }
        let mut layers = Vec::new();
        for _ in 0..n_layers {
            let h = r.take(4)?;
            let scale = match h[0] {
                0 => Scale::Linear,
                1 => Scale::Log,
                s => return Err(bad(&format!("bilinmeyen ölçek {s}"))),
            };
            let n_valid = h[1];
            let (lo, hi) = (r.f32()?, r.f32()?);
            let len = r.u32()? as usize;
            let name =
                String::from_utf8(r.take(len)?.to_vec()).map_err(|_| bad("ad UTF-8 değil"))?;
            r.p = r.p.next_multiple_of(4);
            layers.push(LayerInfo {
                name,
                scale,
                n_valid,
                lo,
                hi,
            });
        }
        let grid = Grid {
            west,
            north,
            cell,
            width,
            height,
            tile,
        };
        let index_start = r.p;
        let n_entries = layers.len() * (grid.tiles_x() * grid.tiles_y()) as usize;
        if bytes.as_ref().len() < index_start + n_entries * INDEX_ENTRY {
            return Err(bad("dizin kesik"));
        }
        Ok(Self {
            bytes,
            grid,
            layers,
            index_start,
            cache: Mutex::new(HashMap::new()),
            // 240² bayt × 512 ≈ 30 MB
            cache_cap: 512,
        })
    }

    pub fn layer(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.name == name)
    }

    /// Döşemeyi açar (önbellekten ya da zstd çözerek).
    pub fn tile(&self, layer: usize, tx: u32, ty: u32) -> io::Result<Tile> {
        let n = u64::from(self.grid.tiles_x() * self.grid.tiles_y());
        let key = layer as u64 * n + u64::from(ty * self.grid.tiles_x() + tx);
        let e = self.index_start + key as usize * INDEX_ENTRY;
        let b = self.bytes.as_ref();
        let offset = u64::from_le_bytes(b[e..e + 8].try_into().unwrap_or_default()) as usize;
        let len = u32::from_le_bytes(b[e + 8..e + 12].try_into().unwrap_or_default()) as usize;
        if len == 0 {
            return Ok(Tile::Fill(b[e + 12]));
        }
        if let Some(t) = self.cache.lock().map_err(|_| bad("önbellek"))?.get(&key) {
            return Ok(Tile::Data(t.clone()));
        }
        let src = b
            .get(offset..offset + len)
            .ok_or_else(|| bad("döşeme dosya dışında"))?;
        let t2 = (self.grid.tile * self.grid.tile) as usize;
        let data: Arc<[u8]> = decompress(src, t2)?.into();
        let mut cache = self.cache.lock().map_err(|_| bad("önbellek"))?;
        if cache.len() >= self.cache_cap {
            cache.clear();
        }
        cache.insert(key, data.clone());
        Ok(Tile::Data(data))
    }

    /// Dikdörtgen pencere: sol üst hücre (`col0`, `row0`), `w × h` hücre,
    /// satır satır kodlar. Izgara yatayda sarıyorsa sütunlar sarılır;
    /// ızgara dışındaki hücreler `outside` kodunu alır. Her döşeme bir kez
    /// açılır (nokta nokta okumaktan çok daha hızlı).
    pub fn window(
        &self,
        layer: usize,
        col0: i64,
        row0: i64,
        w: u32,
        h: u32,
        outside: u8,
    ) -> io::Result<Vec<u8>> {
        let g = self.grid;
        let t = i64::from(g.tile);
        let (gw, gh) = (i64::from(g.width), i64::from(g.height));
        let mut out = vec![outside; w as usize * h as usize];
        // Sütun → kaynak sütun (sarma / dışarı).
        let src_col = |c: i64| -> Option<i64> {
            if g.wraps() {
                Some(c.rem_euclid(gw))
            } else {
                (0..gw).contains(&c).then_some(c)
            }
        };
        let mut tile: Option<((i64, i64), Tile)> = None;
        for y in 0..i64::from(h) {
            let r = row0 + y;
            if !(0..gh).contains(&r) {
                continue;
            }
            for x in 0..i64::from(w) {
                let Some(c) = src_col(col0 + x) else { continue };
                let key = (c / t, r / t);
                if tile.as_ref().is_none_or(|(k, _)| *k != key) {
                    tile = Some((key, self.tile(layer, key.0 as u32, key.1 as u32)?));
                }
                let (_, tl) = tile.as_ref().expect("az önce açıldı");
                out[(y * i64::from(w) + x) as usize] =
                    tl.get(g.tile, (c % t) as u32, (r % t) as u32);
            }
        }
        Ok(out)
    }

    /// Noktadaki kod (en yakın hücre); ızgara dışındaysa `None`.
    pub fn code_at(&self, layer: usize, lon: f64, lat: f64) -> io::Result<Option<u8>> {
        let Some((col, row)) = self.grid.cell_of(lon, lat) else {
            return Ok(None);
        };
        let t = self.grid.tile;
        let tile = self.tile(layer, col / t, row / t)?;
        Ok(Some(tile.get(t, col % t, row % t)))
    }
}

pub(crate) fn decompress(src: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    use ruzstd::io::Read as _;
    let mut s = src;
    let mut dec =
        ruzstd::decoding::StreamingDecoder::new(&mut s).map_err(|e| bad(&format!("zstd: {e}")))?;
    let mut out = Vec::with_capacity(expected);
    dec.read_to_end(&mut out)?;
    if out.len() != expected {
        return Err(bad("döşeme boyutu yanlış"));
    }
    Ok(out)
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let s = self
            .b
            .get(self.p..self.p + n)
            .ok_or_else(|| bad("dosya kesik"))?;
        self.p += n;
        Ok(s)
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().unwrap_or_default(),
        ))
    }
    fn f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_le_bytes(
            self.take(4)?.try_into().unwrap_or_default(),
        ))
    }
    fn f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_le_bytes(
            self.take(8)?.try_into().unwrap_or_default(),
        ))
    }
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zstd(b: &[u8]) -> io::Result<Vec<u8>> {
        Ok(ruzstd::encoding::compress_to_vec(
            b,
            ruzstd::encoding::CompressionLevel::Fastest,
        ))
    }

    fn vs30_layer() -> LayerInfo {
        LayerInfo {
            name: "vs30".into(),
            scale: Scale::Log,
            n_valid: 251,
            lo: 80.0,
            hi: 2500.0,
        }
    }

    #[test]
    fn log_kuantalama_hatasi_kucuk() {
        let l = vs30_layer();
        for v in [98.0f32, 180.0, 400.0, 760.0, 1130.0, 2197.0] {
            let back = l.value(l.quantize(v)).unwrap();
            assert!((back / v - 1.0).abs() < 0.008, "{v} → {back}");
        }
        assert_eq!(l.quantize(10.0), 0);
        assert_eq!(l.quantize(1e5), 250);
        assert_eq!(l.value(251), None);
    }

    #[test]
    fn dogrusal_kuantalama() {
        let l = LayerInfo {
            name: "sd".into(),
            scale: Scale::Linear,
            n_valid: 251,
            lo: 0.0,
            hi: 1.25,
        };
        assert_eq!(l.quantize(0.0), 0);
        assert_eq!(l.quantize(1.25), 250);
        assert!((l.value(l.quantize(0.3)).unwrap() - 0.3).abs() < 0.003);
    }

    #[test]
    fn hucre_bulma_ve_sarma() {
        let g = Grid {
            west: -180.0,
            north: 90.0,
            cell: 1.0,
            width: 360,
            height: 180,
            tile: 64,
        };
        assert!(g.wraps());
        assert_eq!(g.cell_of(-180.0, 90.0), Some((0, 0)));
        assert_eq!(g.cell_of(179.5, -89.5), Some((359, 179)));
        assert_eq!(g.cell_of(190.5, 0.5), Some((10, 89)));
        assert_eq!(g.cell_of(0.0, -90.5), None);
        let g2 = Grid { width: 10, ..g };
        assert!(!g2.wraps());
        assert_eq!(g2.cell_of(-169.0, 0.0), None);
    }

    #[test]
    fn yaz_oku_gidis_donus() {
        // 5×3 ızgara, döşeme 2 → 3×2 döşeme; kenar döşemeleri dolgulu.
        let grid = Grid {
            west: 10.0,
            north: 50.0,
            cell: 0.5,
            width: 5,
            height: 3,
            tile: 2,
        };
        let code = |c: u32, r: u32| (r * 10 + c) as u8;
        let mut tiles = Vec::new();
        for ty in 0..grid.tiles_y() {
            for tx in 0..grid.tiles_x() {
                let mut t = vec![255u8; 4];
                for y in 0..2 {
                    for x in 0..2 {
                        let (c, r) = (tx * 2 + x, ty * 2 + y);
                        if c < 5 && r < 3 {
                            t[(y * 2 + x) as usize] = code(c, r);
                        }
                    }
                }
                tiles.push(t);
            }
        }
        // İkinci katman: tamamen tek kod (yazılmaz, dizinde dolgu).
        let fill = vec![vec![7u8; 4]; tiles.len()];
        let layers = [
            vs30_layer(),
            LayerInfo {
                name: "ölçü".into(),
                ..vs30_layer()
            },
        ];
        let enc = |l: Vec<Vec<u8>>| -> Vec<Encoded> {
            l.iter().map(|t| encode(t, zstd).unwrap()).collect()
        };
        let mut buf = Vec::new();
        write(&mut buf, &grid, &layers, &[enc(tiles), enc(fill)]).unwrap();

        let r = Raster::open(buf).unwrap();
        assert_eq!(r.grid, grid);
        assert_eq!(r.layers, layers);
        assert_eq!(r.layer("ölçü"), Some(1));
        for row in 0..3 {
            for col in 0..5 {
                let lon = 10.0 + 0.5 * f64::from(col) + 0.25;
                let lat = 50.0 - 0.5 * f64::from(row) - 0.25;
                assert_eq!(r.code_at(0, lon, lat).unwrap(), Some(code(col, row)));
                assert_eq!(r.code_at(1, lon, lat).unwrap(), Some(7));
            }
        }
        assert_eq!(r.code_at(0, 9.9, 49.0).unwrap(), None);
        // Pencere: ızgaradan taşan kısım `outside`.
        let win = r.window(0, 3, 1, 3, 3, 99).unwrap();
        assert_eq!(
            win,
            vec![
                code(3, 1),
                code(4, 1),
                99,
                code(3, 2),
                code(4, 2),
                99,
                99,
                99,
                99
            ]
        );
        assert!(matches!(r.tile(1, 0, 0).unwrap(), Tile::Fill(7)));
    }

    #[test]
    fn bozuk_dosya_reddedilir() {
        assert!(Raster::open(b"SMAP\x01\0\0\0".to_vec()).is_err());
        assert!(Raster::open(b"SRAS\x01\0\0\0".to_vec()).is_err());
    }
}
