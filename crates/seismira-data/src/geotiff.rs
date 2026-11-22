//! GeoTIFF okuma (veri hazırlama aracı). Saf Rust: `tiff` crate'i +
//! GeoTIFF konum etiketleri (ModelPixelScale, ModelTiepoint, GTRasterType).

use anyhow::{Context, Result, bail, ensure};
use seismira_core::raster::Grid;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tiff::decoder::{ChunkType, Decoder, DecodingResult, Limits};
use tiff::tags::Tag;

/// Tek bantlı GeoTIFF okuyucu: satır bantları (tam genişlik) ya da
/// rastgele noktalar (parça önbelleğiyle). Değerler f32'ye çevrilir;
/// nodata NaN olur.
pub struct GeoTiff {
    dec: Decoder<BufReader<File>>,
    width: u32,
    height: u32,
    chunk_w: u32,
    chunk_h: u32,
    chunks_x: u32,
    nodata: Option<f32>,
    /// Toleranslı LZW yedeği için: ham dosya, parça konumları/boyutları.
    raw: File,
    offsets: Vec<u64>,
    counts: Vec<u64>,
    /// Son okunan parça satırı (chunk row) ve verisi: width × chunk_h.
    cached: Option<(u32, Vec<f32>)>,
    /// Rastgele erişim önbelleği: parça → (veri, satır adımı).
    chunks: HashMap<u32, (Vec<f32>, usize)>,
    pub west: f64,
    pub north: f64,
    pub cell: f64,
    pub lenient_chunks: u32,
}

impl GeoTiff {
    pub fn open(path: &Path) -> Result<Self> {
        let f = File::open(path).with_context(|| format!("{} açılamadı", path.display()))?;
        let mut limits = Limits::default();
        limits.ifd_value_size = 64 << 20;
        let mut dec = Decoder::new(BufReader::new(f))?.with_limits(limits);
        let (width, height) = dec.dimensions()?;
        let scale = dec.get_tag_f64_vec(Tag::ModelPixelScaleTag)?;
        let tie = dec.get_tag_f64_vec(Tag::ModelTiepointTag)?;
        ensure!(
            scale.len() >= 2 && tie.len() >= 6,
            "GeoTIFF konum etiketleri eksik"
        );
        ensure!(
            (scale[0] - scale[1]).abs() < 1e-12,
            "hücreler kare değil: {} × {}",
            scale[0],
            scale[1]
        );
        ensure!(
            tie[0] == 0.0 && tie[1] == 0.0,
            "bağlama noktası (0,0) pikselinde değil"
        );
        // GTRasterTypeGeoKey (1025): 1 PixelIsArea (köşe), 2 PixelIsPoint (merkez).
        let keys = dec
            .get_tag_u16_vec(Tag::GeoKeyDirectoryTag)
            .unwrap_or_default();
        let pixel_is_point = keys
            .as_chunks::<4>()
            .0
            .iter()
            .skip(1)
            .any(|k| k[0] == 1025 && k[1] == 0 && k[3] == 2);
        let half = if pixel_is_point { scale[0] / 2.0 } else { 0.0 };
        let (west, north, cell) = (tie[3] - half, tie[4] + half, scale[0]);
        let nodata = dec
            .get_tag_ascii_string(Tag::GdalNodata)
            .ok()
            .and_then(|s| s.trim_matches(char::from(0)).trim().parse::<f32>().ok());
        let (chunk_w, chunk_h) = dec.chunk_dimensions();
        let chunks_x = match dec.get_chunk_type() {
            ChunkType::Tile => width.div_ceil(chunk_w),
            ChunkType::Strip => 1,
        };
        println!(
            "{}: {width} × {height}, hücre {:.10}°, batı {:.6}, kuzey {:.6}, parça {chunk_w} × {chunk_h}, \
             {}, nodata {nodata:?}",
            path.display(),
            cell,
            west,
            north,
            if pixel_is_point {
                "PixelIsPoint"
            } else {
                "PixelIsArea"
            },
        );
        let tiled = matches!(dec.get_chunk_type(), ChunkType::Tile);
        let (off_tag, cnt_tag) = if tiled {
            (Tag::TileOffsets, Tag::TileByteCounts)
        } else {
            (Tag::StripOffsets, Tag::StripByteCounts)
        };
        let offsets = dec.get_tag_u64_vec(off_tag)?;
        let counts = dec.get_tag_u64_vec(cnt_tag)?;
        let raw = File::open(path)?;
        Ok(Self {
            chunks: HashMap::new(),
            west,
            north,
            cell,
            raw,
            offsets,
            counts,
            lenient_chunks: 0,
            dec,
            width,
            height,
            chunk_w,
            chunk_h,
            chunks_x,
            nodata,
            cached: None,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Kaynak ızgara, verilen döşeme boyutuyla.
    pub fn grid(&self, tile: u32) -> Grid {
        Grid {
            west: self.west,
            north: self.north,
            cell: self.cell,
            width: self.width,
            height: self.height,
            tile,
        }
    }

    /// Bir parçayı f32 olarak çözer: (veri, satır adımı, veri genişliği, veri yüksekliği).
    fn read_chunk(&mut self, idx: u32) -> Result<(Vec<f32>, usize, u32, u32)> {
        let (dw, dh) = self.dec.chunk_data_dimensions(idx);
        let mut data = match self.dec.read_chunk(idx) {
            Ok(DecodingResult::F32(v)) => v,
            Ok(DecodingResult::F64(v)) => v.into_iter().map(|x| x as f32).collect(),
            Ok(DecodingResult::U32(v)) => v.into_iter().map(|x| x as f32).collect(),
            Ok(DecodingResult::U16(v)) => v.into_iter().map(f32::from).collect(),
            Ok(DecodingResult::I16(v)) => v.into_iter().map(f32::from).collect(),
            Ok(DecodingResult::U8(v)) => v.into_iter().map(f32::from).collect(),
            Ok(_) => bail!("desteklenmeyen örnek türü"),
            Err(e) if e.to_string().contains("lzw end code") => {
                self.lenient_chunks += 1;
                self.lenient_lzw(idx, dh)?
            }
            Err(e) => return Err(e.into()),
        };
        if let Some(nd) = self.nodata {
            data.iter_mut()
                .filter(|v| **v == nd)
                .for_each(|v| *v = f32::NAN);
        }
        // Kenar parçalarında satır genişliği dw (veri) olabilir.
        let stride = data.len() / dh as usize;
        Ok((data, stride, dw, dh))
    }

    /// Noktadaki değer (en yakın hücre); ızgara dışı ya da nodata ise NaN.
    /// Parçalar önbellekte tutulur (sınır aşılınca boşaltılır); yakın
    /// noktaları art arda sormak hızlıdır.
    pub fn value_at(&mut self, lon: f64, lat: f64) -> Result<f32> {
        let Some((col, row)) = self.grid(1).cell_of(lon, lat) else {
            return Ok(f32::NAN);
        };
        let idx = (row / self.chunk_h) * self.chunks_x + col / self.chunk_w;
        if !self.chunks.contains_key(&idx) {
            if self.chunks.len() >= 512 {
                self.chunks.clear();
            }
            let (data, stride, _, _) = self.read_chunk(idx)?;
            self.chunks.insert(idx, (data, stride));
        }
        let (data, stride) = &self.chunks[&idx];
        let (x, y) = ((col % self.chunk_w) as usize, (row % self.chunk_h) as usize);
        Ok(data.get(y * stride + x).copied().unwrap_or(f32::NAN))
    }

    /// Parça satırını (tüm genişlik) çözer.
    fn chunk_row(&mut self, cy: u32) -> Result<&[f32]> {
        if self.cached.as_ref().is_none_or(|(c, _)| *c != cy) {
            let w = self.width as usize;
            let mut row = vec![f32::NAN; w * self.chunk_h as usize];
            for cx in 0..self.chunks_x {
                let (data, stride, dw, dh) = self.read_chunk(cy * self.chunks_x + cx)?;
                for y in 0..dh as usize {
                    let x0 = (cx * self.chunk_w) as usize;
                    let dst = &mut row[y * w + x0..y * w + x0 + dw as usize];
                    dst.copy_from_slice(&data[y * stride..y * stride + dw as usize]);
                }
            }
            self.cached = Some((cy, row));
        }
        Ok(&self
            .cached
            .as_ref()
            .map(|(_, r)| r)
            .expect("az önce dolduruldu")[..])
    }

    /// `tiff` 0.11.3, USGS SD dosyasının 180° kenarındaki 23 parçada
    /// "no lzw end code found" hatası veriyor; oysa ham akış eksiksiz ve
    /// EOI koduyla bitiyor (weezl ile doğrudan çözüldüğünde `Done`,
    /// 2026-11-14 denendi). Bu parçaları aynı ayarlarla (MSB, 8 bit, TIFF
    /// kod boyu geçişi) kendimiz çözüyoruz; YALNIZCA akış `Done` ile bitip
    /// çıktı tam parça boyutundaysa kabul ediliyor.
    fn lenient_lzw(&mut self, idx: u32, dh: u32) -> Result<Vec<f32>> {
        use std::io::{Read, Seek, SeekFrom};
        let pred = self.dec.get_tag_u32(Tag::Predictor).unwrap_or(1);
        let bits = self.dec.get_tag_u32(Tag::BitsPerSample)?;
        let format = self.dec.get_tag_u32(Tag::SampleFormat).unwrap_or(1);
        ensure!(
            pred == 1 && bits == 32 && (format == 1 || format == 3),
            "toleranslı LZW yalnızca öngörücüsüz 32 bit (u32/f32) için"
        );
        let i = idx as usize;
        let mut src = vec![0u8; usize::try_from(self.counts[i])?];
        self.raw.seek(SeekFrom::Start(self.offsets[i]))?;
        self.raw.read_exact(&mut src)?;
        let full = (self.chunk_w * self.chunk_h * 4) as usize;
        let mut out = vec![0u8; full];
        let mut dec = weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
        let (mut ci, mut co) = (0, 0);
        let done = loop {
            let r = dec.decode_bytes(&src[ci..], &mut out[co..]);
            ci += r.consumed_in;
            co += r.consumed_out;
            match r.status {
                Ok(weezl::LzwStatus::Done) => break true,
                Ok(_) if r.consumed_in == 0 && r.consumed_out == 0 => break false,
                Ok(_) => {}
                Err(e) => bail!("parça {idx}: LZW: {e}"),
            }
        };
        ensure!(
            done && co == full,
            "parça {idx}: toleranslı LZW {co} / {full} bayt üretti (bitti: {done})"
        );
        let le = matches!(self.dec.byte_order(), tiff::tags::ByteOrder::LittleEndian);
        let vals = out[..(self.chunk_w * dh * 4) as usize]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| {
                let bits = if le {
                    u32::from_le_bytes(*b)
                } else {
                    u32::from_be_bytes(*b)
                };
                if format == 3 {
                    f32::from_bits(bits)
                } else {
                    bits as f32
                }
            })
            .collect();
        Ok(vals)
    }

    /// `y0..y0+n` satırlarını `out`'a (width × n) kopyalar.
    pub fn rows(&mut self, y0: u32, n: u32, out: &mut [f32]) -> Result<()> {
        let w = self.width as usize;
        for y in y0..(y0 + n).min(self.height) {
            let (cy, ry) = (y / self.chunk_h, (y % self.chunk_h) as usize);
            let row = self.chunk_row(cy)?;
            let o = (y - y0) as usize * w;
            out[o..o + w].copy_from_slice(&row[ry * w..(ry + 1) * w]);
        }
        Ok(())
    }
}
