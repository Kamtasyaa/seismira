//! Harita dosya biçimi (`.smap`): GPU'ya doğrudan yüklenebilen hazır geometri.
//!
//! Koordinatlar Web Mercator birim karesindedir ([`crate::mercator`]).
//! Kara/göl çokgenleri veri hazırlamada üçgenlenmiştir; sınırlar çizgi
//! dizileri (line strip) olarak, aralarında [`RESTART`] ile saklanır.
//!
//! Biçim (little-endian):
//! ```text
//! "SMAP"  u32 sürüm  u32 katman sayısı
//! her katman: u8 tür, 3 bayt boşluk, u32 ad uzunluğu, ad (UTF-8, 4'e
//!             tamamlanır), u32 köşe sayısı, u32 indeks sayısı,
//!             köşeler [f32; 2] × n, indeksler u32 × m
//! ```

use std::io::{self, Read, Write};

#[cfg(target_endian = "big")]
compile_error!("harita dosya biçimi little-endian varsayar");

const MAGIC: &[u8; 4] = b"SMAP";
pub const VERSION: u32 = 1;

/// Çizgi dizileri arasındaki ayraç (wgpu primitive restart değeri).
pub const RESTART: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LayerKind {
    /// Üçgen listesi (dolgu).
    Triangles = 0,
    /// Çizgi dizileri, [`RESTART`] ile ayrılmış.
    LineStrips = 1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub kind: LayerKind,
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapFile {
    pub layers: Vec<Layer>,
}

impl MapFile {
    pub fn layer(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.name == name)
    }

    pub fn write(&self, w: &mut impl Write) -> io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&VERSION.to_le_bytes())?;
        w.write_all(&len_u32(self.layers.len())?.to_le_bytes())?;
        for layer in &self.layers {
            w.write_all(&[layer.kind as u8, 0, 0, 0])?;
            let name = layer.name.as_bytes();
            w.write_all(&len_u32(name.len())?.to_le_bytes())?;
            w.write_all(name)?;
            w.write_all(&[0u8; 3][..pad4(name.len())])?;
            w.write_all(&len_u32(layer.vertices.len())?.to_le_bytes())?;
            w.write_all(&len_u32(layer.indices.len())?.to_le_bytes())?;
            w.write_all(bytemuck::cast_slice(&layer.vertices))?;
            w.write_all(bytemuck::cast_slice(&layer.indices))?;
        }
        Ok(())
    }

    pub fn read(r: &mut impl Read) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid("SMAP dosyası değil"));
        }
        let version = read_u32(r)?;
        if version != VERSION {
            return Err(invalid(&format!("desteklenmeyen sürüm {version}")));
        }
        let count = read_u32(r)?;
        let mut layers = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut head = [0u8; 4];
            r.read_exact(&mut head)?;
            let kind = match head[0] {
                0 => LayerKind::Triangles,
                1 => LayerKind::LineStrips,
                k => return Err(invalid(&format!("bilinmeyen katman türü {k}"))),
            };
            let name_len = read_u32(r)? as usize;
            let mut name = vec![0u8; name_len + pad4(name_len)];
            r.read_exact(&mut name)?;
            name.truncate(name_len);
            let name = String::from_utf8(name).map_err(|_| invalid("katman adı UTF-8 değil"))?;
            let nv = read_u32(r)? as usize;
            let ni = read_u32(r)? as usize;
            let mut vertices = vec![[0f32; 2]; nv];
            r.read_exact(bytemuck::cast_slice_mut(&mut vertices))?;
            let mut indices = vec![0u32; ni];
            r.read_exact(bytemuck::cast_slice_mut(&mut indices))?;
            if indices.iter().any(|&i| i != RESTART && i as usize >= nv) {
                return Err(invalid(&format!("{name}: indeks köşe sayısını aşıyor")));
            }
            layers.push(Layer {
                name,
                kind,
                vertices,
                indices,
            });
        }
        Ok(Self { layers })
    }
}

fn pad4(n: usize) -> usize {
    (4 - n % 4) % 4
}

fn len_u32(n: usize) -> io::Result<u32> {
    u32::try_from(n).map_err(|_| invalid("boyut u32 sınırını aşıyor"))
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaz_oku_ayni() {
        let map = MapFile {
            layers: vec![
                Layer {
                    name: "land".into(),
                    kind: LayerKind::Triangles,
                    vertices: vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
                    indices: vec![0, 1, 2],
                },
                Layer {
                    name: "sınır".into(), // 6 bayt UTF-8: dolgu testi
                    kind: LayerKind::LineStrips,
                    vertices: vec![[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]],
                    indices: vec![0, 1, RESTART, 1, 2],
                },
            ],
        };
        let mut buf = Vec::new();
        map.write(&mut buf).unwrap();
        assert_eq!(MapFile::read(&mut buf.as_slice()).unwrap(), map);
    }

    #[test]
    fn bozuk_dosya_reddedilir() {
        assert!(MapFile::read(&mut &b"XXXX"[..]).is_err());
        let mut buf = Vec::new();
        MapFile {
            layers: vec![Layer {
                name: "x".into(),
                kind: LayerKind::Triangles,
                vertices: vec![[0.0, 0.0]],
                indices: vec![5],
            }],
        }
        .write(&mut buf)
        .unwrap();
        assert!(MapFile::read(&mut buf.as_slice()).is_err());
    }
}
