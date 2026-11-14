//! Şehir listesi (GeoNames, CC BY 4.0), ikili biçim `.scty`.
//!
//! Biçim (little-endian):
//! ```text
//! "SCTY"  u32 sürüm  u32 şehir sayısı
//! her şehir: f32 boylam, f32 enlem, u32 nüfus, [u8; 2] ülke kodu,
//!            u8 başkent mi, u8 boşluk, u16 ad uzunluğu, ad (UTF-8),
//!            u16 il/eyalet adı uzunluğu, il/eyalet adı (UTF-8)
//! ```
//! Şehirler nüfusa göre azalan sırada saklanır (gösterim önceliği).

use std::io::{self, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct City {
    pub name: String,
    /// Birinci düzey idari bölge adı (İngilizce, GeoNames admin1).
    pub admin1: String,
    /// ISO 3166-1 alfa-2.
    pub country: [u8; 2],
    pub lon: f64,
    pub lat: f64,
    pub population: u32,
    pub capital: bool,
}

const MAGIC: &[u8; 4] = b"SCTY";
const VERSION: u32 = 1;

pub fn write(cities: &[City], w: &mut impl Write) -> io::Result<()> {
    w.write_all(MAGIC)?;
    w.write_all(&VERSION.to_le_bytes())?;
    w.write_all(
        &u32::try_from(cities.len())
            .map_err(|_| bad("çok fazla şehir"))?
            .to_le_bytes(),
    )?;
    for c in cities {
        w.write_all(&(c.lon as f32).to_le_bytes())?;
        w.write_all(&(c.lat as f32).to_le_bytes())?;
        w.write_all(&c.population.to_le_bytes())?;
        w.write_all(&c.country)?;
        w.write_all(&[u8::from(c.capital), 0])?;
        for s in [&c.name, &c.admin1] {
            let b = s.as_bytes();
            w.write_all(
                &u16::try_from(b.len())
                    .map_err(|_| bad("ad çok uzun"))?
                    .to_le_bytes(),
            )?;
            w.write_all(b)?;
        }
    }
    Ok(())
}

pub fn read(r: &mut impl Read) -> io::Result<Vec<City>> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(bad("SCTY dosyası değil"));
    }
    if read_u32(r)? != VERSION {
        return Err(bad("desteklenmeyen sürüm"));
    }
    let n = read_u32(r)? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let lon = f64::from(f32::from_le_bytes(read_arr(r)?));
        let lat = f64::from(f32::from_le_bytes(read_arr(r)?));
        let population = read_u32(r)?;
        let country: [u8; 2] = read_arr(r)?;
        let flags: [u8; 2] = read_arr(r)?;
        let name = read_str(r)?;
        let admin1 = read_str(r)?;
        out.push(City {
            name,
            admin1,
            country,
            lon,
            lat,
            population,
            capital: flags[0] != 0,
        });
    }
    Ok(out)
}

fn read_arr<const N: usize>(r: &mut impl Read) -> io::Result<[u8; N]> {
    let mut b = [0u8; N];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_arr(r)?))
}

fn read_str(r: &mut impl Read) -> io::Result<String> {
    let n = u16::from_le_bytes(read_arr(r)?) as usize;
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    String::from_utf8(b).map_err(|_| bad("ad UTF-8 değil"))
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaz_oku() {
        let cities = vec![City {
            name: "İzmit".into(),
            admin1: "Kocaeli".into(),
            country: *b"TR",
            lon: 29.92928,
            lat: 40.76499,
            population: 196_571,
            capital: false,
        }];
        let mut buf = Vec::new();
        write(&cities, &mut buf).unwrap();
        let back = read(&mut buf.as_slice()).unwrap();
        assert_eq!(back[0].name, "İzmit");
        assert_eq!(back[0].population, 196_571);
        assert!((back[0].lon - 29.92928).abs() < 1e-5);
    }
}
