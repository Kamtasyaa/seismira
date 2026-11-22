//! ESRI ArcInfo ikili ızgara (AIG, `hdr.adf` + `w00100?.adf`) okuyucusu.
//!
//! GMTED2010 USGS'te bu biçimde (zip) dağıtılıyor; GeoTIFF döşemeleri 7,5″
//! için ~5 kat büyük. GDAL'ı uygulamaya ve veri aracına sokmamak için saf
//! Rust okuyucu (CLAUDE.md Bölüm 4).
//!
//! Biçim kaynağı: Frank Warmerdam, "Arc/Info Binary Grid Format" (GDAL
//! belgesi, frmts/aigrid). Yalnızca tam sayı ızgaralar ve GMTED'de görülen
//! blok türleri desteklenir; bilinmeyen tür hata verir. Doğrulama: aynı
//! bölgenin USGS GeoTIFF döşemesiyle hücre hücre karşılaştırma (testler).
//!
//! Düzen (tümü büyük-endian):
//! - `hdr.adf`: 16 hücre türü (1 tam sayı), 20 sıkıştırma (0 sıkıştırılmış),
//!   256/264 hücre boyutu x/y, 288/292 dosya başına blok sütunu/satırı,
//!   296 blok genişliği, 304 blok yüksekliği (piksel).
//! - `dblbnd.adf`: sol alt x/y, sağ üst x/y (f64).
//! - Büyük ızgaralar birden çok "döşeme dosyasına" bölünür; her biri
//!   blok_sütunu·blok_genişliği × blok_satırı·blok_yüksekliği piksel.
//! - `wNNNNNNx.adf`: 100 bayt başlık, sonra blok başına (konum, boyut),
//!   ikisi de 16 bitlik sözcük cinsinden.
//! - `wNNNNNN.adf`: blok = 2 bayt boyut, 1 bayt tür, 1 bayt RMin boyutu,
//!   RMin (işaretli), veri.

use anyhow::{Context, Result, bail, ensure};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Tam sayı nodata (ESRI_GRID_NO_DATA).
const ESRI_NODATA: i32 = -2_147_483_647;
/// Çıktıdaki nodata.
pub const NODATA: i16 = i16::MIN;

struct TileFile {
    file: File,
    /// Blok başına (bayt konumu, bayt boyutu); boyut 0 ise blok boş.
    index: Vec<(u64, usize)>,
}

pub struct Aig {
    pub west: f64,
    pub north: f64,
    pub cell: f64,
    pub width: u32,
    pub height: u32,
    block_w: u32,
    block_h: u32,
    blocks_x: u32,
    blocks_y: u32,
    tiles_x: u32,
    /// Döşeme dosyaları, satır satır (kuzeyden güneye); eksik olan `None`.
    tiles: Vec<Option<TileFile>>,
    /// Şimdiye dek RMin ile yaklaşık doldurulan (CCITT) blok sayısı.
    pub ccitt_blocks: u32,
}

fn be_i32(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}

fn be_f64(b: &[u8], o: usize) -> f64 {
    f64::from_be_bytes(b[o..o + 8].try_into().unwrap())
}

impl Aig {
    /// `dir`: kapsam klasörü (`hdr.adf`'nin bulunduğu). `name` döşeme
    /// (sütun, satır) → dosya kök adı (ör. "w001001").
    pub fn open(dir: &Path, name: impl Fn(u32, u32) -> String) -> Result<Self> {
        let hdr = std::fs::read(dir.join("hdr.adf")).context("hdr.adf okunamadı")?;
        ensure!(
            hdr.len() >= 308 && &hdr[..6] == b"GRID1.",
            "hdr.adf: AIG değil"
        );
        ensure!(
            be_i32(&hdr, 16) == 1,
            "yalnızca tam sayı ızgara destekleniyor"
        );
        ensure!(
            be_i32(&hdr, 20) == 0,
            "yalnızca sıkıştırılmış ızgara destekleniyor"
        );
        let (cx, cy) = (be_f64(&hdr, 256), be_f64(&hdr, 264));
        ensure!((cx - cy).abs() < 1e-12, "hücreler kare değil");
        let blocks_x = be_i32(&hdr, 288) as u32;
        let blocks_y = be_i32(&hdr, 292) as u32;
        let block_w = be_i32(&hdr, 296) as u32;
        let block_h = be_i32(&hdr, 304) as u32;
        let bnd = std::fs::read(dir.join("dblbnd.adf")).context("dblbnd.adf okunamadı")?;
        ensure!(bnd.len() >= 32, "dblbnd.adf kısa");
        let (llx, lly, urx, ury) = (
            be_f64(&bnd, 0),
            be_f64(&bnd, 8),
            be_f64(&bnd, 16),
            be_f64(&bnd, 24),
        );
        let width = ((urx - llx) / cx).round() as u32;
        let height = ((ury - lly) / cy).round() as u32;
        let (tile_w, tile_h) = (blocks_x * block_w, blocks_y * block_h);
        let tiles_x = width.div_ceil(tile_w);
        let tiles_y = height.div_ceil(tile_h);
        let mut tiles = Vec::new();
        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let base = name(tx, ty);
                let data: PathBuf = dir.join(format!("{base}.adf"));
                let idx: PathBuf = dir.join(format!("{base}x.adf"));
                if !data.exists() {
                    tiles.push(None);
                    continue;
                }
                let raw =
                    std::fs::read(&idx).with_context(|| format!("{} okunamadı", idx.display()))?;
                ensure!(raw.len() >= 100, "{}: kısa dizin", idx.display());
                let index = raw[100..]
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|e| {
                        let off = be_i32(e, 0) as u64 * 2;
                        let len = be_i32(e, 4) as usize * 2;
                        (off, len)
                    })
                    .collect();
                tiles.push(Some(TileFile {
                    file: File::open(&data)?,
                    index,
                }));
            }
        }
        println!(
            "{}: {width} × {height}, hücre {cx:.10}°, batı {llx:.6}, kuzey {ury:.6}, \
             blok {block_w} × {block_h}, {tiles_x} × {tiles_y} döşeme dosyası",
            dir.display()
        );
        Ok(Self {
            west: llx,
            north: ury,
            cell: cx,
            width,
            height,
            block_w,
            block_h,
            blocks_x,
            blocks_y,
            tiles_x,
            tiles,
            ccitt_blocks: 0,
        })
    }

    /// Satır `y`'den başlayan blok satırı yüksekliği (döşemeye hizalı okuma
    /// için `rows` bu katlarla çağrılmalı).
    pub fn block_h(&self) -> u32 {
        self.block_h
    }

    /// `y0..y0+n` satırlarını `out`'a (width × n) yazar; `y0` ve `n` blok
    /// yüksekliğinin katı olmalı (son bant hariç). Nodata [`NODATA`].
    pub fn rows(&mut self, y0: u32, n: u32, out: &mut [i16]) -> Result<()> {
        ensure!(
            y0.is_multiple_of(self.block_h),
            "satır blok sınırında değil"
        );
        let w = self.width as usize;
        let (bw, bh) = (self.block_w as usize, self.block_h as usize);
        let tile_w = self.blocks_x as usize * bw;
        let tile_h = self.blocks_y * self.block_h;
        let mut block = vec![0i32; bw * bh];
        let mut buf = Vec::new();
        let mut by = y0;
        while by < (y0 + n).min(self.height) {
            let (ty, ry) = (by / tile_h, (by % tile_h) / self.block_h);
            for tx in 0..self.tiles_x {
                let slot = (ty * self.tiles_x + tx) as usize;
                for bx in 0..self.blocks_x as usize {
                    let x0 = tx as usize * tile_w + bx * bw;
                    if x0 >= w {
                        break;
                    }
                    let bi = ry as usize * self.blocks_x as usize + bx;
                    let entry = match &self.tiles[slot] {
                        Some(t) => t.index.get(bi).copied(),
                        None => None,
                    };
                    match (entry, &mut self.tiles[slot]) {
                        (Some((off, len)), Some(t)) if len > 0 => {
                            buf.resize(len + 2, 0);
                            t.file.seek(SeekFrom::Start(off))?;
                            t.file.read_exact(&mut buf)?;
                            let approx = decode_block(&buf[2..], &mut block)
                                .with_context(|| format!("blok çözülemedi (satır {by}, x {x0})"))?;
                            self.ccitt_blocks += u32::from(approx);
                        }
                        _ => block.fill(ESRI_NODATA),
                    }
                    for r in 0..bh {
                        let y = by as usize + r;
                        if y >= (y0 + n).min(self.height) as usize {
                            break;
                        }
                        let o = (y - y0 as usize) * w;
                        let cols = bw.min(w - x0);
                        for c in 0..cols {
                            let v = block[r * bw + c];
                            out[o + x0 + c] = if v == ESRI_NODATA {
                                NODATA
                            } else {
                                v.clamp(i16::MIN as i32 + 1, i16::MAX as i32) as i16
                            };
                        }
                    }
                }
            }
            by += self.block_h;
        }
        Ok(())
    }
}

/// Tam sayı bloğunu çözer (`raw`: boyut alanından sonraki veri). Blok
/// yaklaşıkla doldurulduysa (CCITT) `true`.
fn decode_block(raw: &[u8], out: &mut [i32]) -> Result<bool> {
    ensure!(raw.len() >= 2, "blok çok kısa");
    let (kind, min_size) = (raw[0], raw[1] as usize);
    ensure!(
        min_size <= 4 && raw.len() >= 2 + min_size,
        "RMin boyutu geçersiz"
    );
    let mut rmin: i64 = 0;
    for &b in &raw[2..2 + min_size] {
        rmin = rmin * 256 + i64::from(b);
    }
    if min_size > 0 && raw[2] > 127 {
        rmin -= 1i64 << (8 * min_size);
    }
    let rmin = rmin as i32;
    let d = &raw[2 + min_size..];
    let n = out.len();
    let short = || anyhow::anyhow!("blok verisi eksik (tür {kind:#04x})");
    match kind {
        0x00 => out.fill(rmin),
        // Ham 1 ve 4 bit: baytın yüksek bitlerinden başlayarak.
        0x01 => {
            ensure!(d.len() * 8 >= n, short());
            for (i, o) in out.iter_mut().enumerate() {
                *o = i32::from((d[i / 8] >> (7 - i % 8)) & 1) + rmin;
            }
        }
        0x04 => {
            ensure!(d.len() * 2 >= n, short());
            for (i, o) in out.iter_mut().enumerate() {
                let b = d[i / 2];
                *o = i32::from(if i % 2 == 0 { b >> 4 } else { b & 15 }) + rmin;
            }
        }
        0x08 => {
            ensure!(d.len() >= n, short());
            for (o, &b) in out.iter_mut().zip(d) {
                *o = i32::from(b) + rmin;
            }
        }
        0x10 => {
            ensure!(d.len() >= 2 * n, short());
            for (o, b) in out.iter_mut().zip(d.as_chunks::<2>().0) {
                *o = i32::from(i16::from_be_bytes([b[0], b[1]])) + rmin;
            }
        }
        0x20 => {
            ensure!(d.len() >= 4 * n, short());
            for (o, b) in out.iter_mut().zip(d.as_chunks::<4>().0) {
                *o = be_i32(b, 0).wrapping_add(rmin);
            }
        }
        // Sabit uzunluklu çalışmalar: sayı (1 bayt) + değer (1/2/4 bayt).
        0xE0 | 0xF0 | 0xF8 | 0xFC => {
            let vb = match kind {
                0xE0 => 4,
                0xF0 => 2,
                _ => 1,
            };
            let (mut i, mut p) = (0, 0);
            while i < n {
                ensure!(p + 1 + vb <= d.len(), short());
                let count = d[p] as usize;
                let v = match vb {
                    4 => be_i32(d, p + 1),
                    2 => i32::from(i16::from_be_bytes([d[p + 1], d[p + 2]])),
                    _ => i32::from(d[p + 1]),
                };
                p += 1 + vb;
                let end = (i + count).min(n);
                out[i..end].fill(v.wrapping_add(rmin));
                i = end;
            }
        }
        // İşaretçi < 128: o kadar değer (0xCF 16 bit, 0xD7 8 bit, 0xDF RMin);
        // ≥ 128: 256 − işaretçi kadar nodata.
        0xCF | 0xD7 | 0xDF => {
            let (mut i, mut p) = (0, 0);
            while i < n {
                ensure!(p < d.len(), short());
                let m = d[p] as usize;
                p += 1;
                if m < 128 {
                    let end = (i + m).min(n);
                    match kind {
                        0xCF => {
                            ensure!(p + 2 * m <= d.len(), short());
                            for (k, o) in out[i..end].iter_mut().enumerate() {
                                let q = p + 2 * k;
                                *o = i32::from(i16::from_be_bytes([d[q], d[q + 1]])) + rmin;
                            }
                            p += 2 * m;
                        }
                        0xD7 => {
                            ensure!(p + m <= d.len(), short());
                            for (k, o) in out[i..end].iter_mut().enumerate() {
                                *o = i32::from(d[p + k]) + rmin;
                            }
                            p += m;
                        }
                        _ => out[i..end].fill(rmin),
                    }
                    i = end;
                } else {
                    let end = (i + 256 - m).min(n);
                    out[i..end].fill(ESRI_NODATA);
                    i = end;
                }
            }
        }
        // CCITT 1 bit (değerler yalnızca RMin ya da RMin + 1). Çözücü
        // yazılmadı: blok RMin ile doldurulur, hata ≤ 1 birim. GMTED md30'da
        // 177 450 bloğun 56'sı (%0,03); sayı `Aig::ccitt_blocks`'ta.
        0xFF => {
            out.fill(rmin);
            return Ok(true);
        }
        _ => bail!("desteklenmeyen blok türü {kind:#04x}"),
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_runs_and_literals() {
        // RMin = −10 (1 bayt), 0xD7: 2 değer (5, 7), sonra 2 nodata.
        let raw = [0xD7, 1, 0xF6, 2, 5, 7, 254];
        let mut out = [0; 4];
        decode_block(&raw, &mut out).unwrap();
        assert_eq!(out, [-5, -3, ESRI_NODATA, ESRI_NODATA]);
        // 0xF8: 3 × (2 + RMin 100), 1 × (0 + 100).
        let raw = [0xF8, 1, 100, 3, 2, 1, 0];
        let mut out = [0; 4];
        decode_block(&raw, &mut out).unwrap();
        assert_eq!(out, [102, 102, 102, 100]);
        // Sabit blok, 2 baytlık RMin = 1000.
        let raw = [0x00, 2, 0x03, 0xE8];
        let mut out = [0; 3];
        decode_block(&raw, &mut out).unwrap();
        assert_eq!(out, [1000; 3]);
    }
}
