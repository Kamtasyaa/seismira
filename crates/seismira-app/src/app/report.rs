//! Otomatik teknik rapor (README "Önemli"): açık simülasyonun özeti,
//! Markdown ve tek dosyalık HTML (tarayıcıda "Yazdır → PDF" ile PDF;
//! PDF kütüphanesi eklenmedi). Dışa aktarma klasörüne yazılır.
//!
//! İçerik yalnızca uygulamanın hesapladıklarıdır; metin seçili dilde.

use super::SeismiraApp;
use crate::i18n::Lang;

/// Biçimden bağımsız rapor bloğu.
enum Block {
    H1(String),
    H2(String),
    P(String),
    List(Vec<String>),
    Table(Vec<String>, Vec<Vec<String>>),
}

fn md(blocks: &[Block]) -> String {
    let cell = |s: &str| s.replace('|', "\\|");
    let mut out = String::new();
    for b in blocks {
        match b {
            Block::H1(t) => out.push_str(&format!("# {t}\n\n")),
            Block::H2(t) => out.push_str(&format!("## {t}\n\n")),
            Block::P(t) => out.push_str(&format!("{t}\n\n")),
            Block::List(items) => {
                for i in items {
                    out.push_str(&format!("- {i}\n"));
                }
                out.push('\n');
            }
            Block::Table(h, rows) => {
                out.push_str(&format!(
                    "| {} |\n",
                    h.iter().map(|x| cell(x)).collect::<Vec<_>>().join(" | ")
                ));
                out.push_str(&format!("|{}\n", "---|".repeat(h.len())));
                for r in rows {
                    out.push_str(&format!(
                        "| {} |\n",
                        r.iter().map(|x| cell(x)).collect::<Vec<_>>().join(" | ")
                    ));
                }
                out.push('\n');
            }
        }
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn html(title: &str, lang: Lang, blocks: &[Block]) -> String {
    let mut body = String::new();
    for b in blocks {
        match b {
            Block::H1(t) => body.push_str(&format!("<h1>{}</h1>\n", esc(t))),
            Block::H2(t) => body.push_str(&format!("<h2>{}</h2>\n", esc(t))),
            Block::P(t) => body.push_str(&format!("<p>{}</p>\n", esc(t))),
            Block::List(items) => {
                body.push_str("<ul>\n");
                for i in items {
                    body.push_str(&format!("<li>{}</li>\n", esc(i)));
                }
                body.push_str("</ul>\n");
            }
            Block::Table(h, rows) => {
                body.push_str("<table>\n<thead><tr>");
                for x in h {
                    body.push_str(&format!("<th>{}</th>", esc(x)));
                }
                body.push_str("</tr></thead>\n<tbody>\n");
                for r in rows {
                    body.push_str("<tr>");
                    for x in r {
                        body.push_str(&format!("<td>{}</td>", esc(x)));
                    }
                    body.push_str("</tr>\n");
                }
                body.push_str("</tbody>\n</table>\n");
            }
        }
    }
    format!(
        "<!doctype html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title>\
<style>body{{font-family:'Noto Sans',system-ui,sans-serif;max-width:60rem;margin:2rem auto;padding:0 1rem;\
color:#1d1f24;line-height:1.45}}h1{{font-size:1.6rem}}h2{{font-size:1.2rem;margin-top:1.8rem;\
border-bottom:1px solid #ccd;padding-bottom:.2rem}}table{{border-collapse:collapse;width:100%;\
font-size:.88rem;margin:.6rem 0}}th,td{{border:1px solid #ccd;padding:.25rem .45rem;text-align:left}}\
th{{background:#eef0f5}}p.warn,.warn{{background:#fff4d6;padding:.5rem .7rem;border-left:4px solid #e8b44c}}\
@media print{{body{{margin:0}}h2{{break-after:avoid}}tr{{break-inside:avoid}}}}</style></head>\
<body>\n{}</body></html>\n",
        lang.code(),
        esc(title),
        body
    )
}

impl SeismiraApp {
    /// (Markdown, HTML) rapor; simülasyon yoksa `None`.
    pub(super) fn build_report(&self) -> Option<(String, String)> {
        let sim = self.sim.as_ref()?;
        let results = self.results.as_ref()?;
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let n0 = |x: f64| lang.num(x, 0);
        let n1 = |x: f64| lang.num(x, 1);
        let q = sim.quake;
        let r = seismira_core::rupture::Rupture::from_quake(&q);
        let mut b = Vec::new();
        let title = t("report-title");
        b.push(Block::H1(title.clone()));
        // "20261126-090515" → "2026-11-26 09:05:15".
        let ts = super::user_ui::timestamp();
        let pretty = format!(
            "{}-{}-{} {}:{}:{}",
            &ts[0..4],
            &ts[4..6],
            &ts[6..8],
            &ts[9..11],
            &ts[11..13],
            &ts[13..15]
        );
        b.push(Block::P(format!("{} {pretty}", t("report-generated"))));
        b.push(Block::P(t("report-disclaimer")));

        // Deprem.
        b.push(Block::H2(t("report-quake")));
        let setting = t(match q.setting {
            seismira_core::quake::Setting::ShallowCrust => "setting-crust",
            seismira_core::quake::Setting::Interface => "setting-interface",
            seismira_core::quake::Setting::Intraslab => "setting-intraslab",
        });
        b.push(Block::List(vec![
            lang.t_args("report-q-mw", &[("v", n1(q.mw))]),
            lang.t_args(
                "report-q-epi",
                &[
                    ("lat", lang.num(q.epicenter.1, 3)),
                    ("lon", lang.num(q.epicenter.0, 3)),
                    ("d", n0(q.depth_km)),
                ],
            ),
            lang.t_args("report-q-setting", &[("setting", setting)]),
            lang.t_args(
                "report-q-fault",
                &[
                    ("strike", n0(r.strike)),
                    ("dip", n0(r.dip)),
                    ("rake", n0(q.rake)),
                ],
            ),
            lang.t_args(
                "report-q-rupture",
                &[
                    ("l", n0(r.length)),
                    ("w", n0(r.width)),
                    ("ztor", n1(r.ztor)),
                    ("n", n0(r.segments.len() as f64)),
                ],
            ),
            t(match q.setting {
                seismira_core::quake::Setting::ShallowCrust => "report-q-gmm-crust",
                _ => "report-q-gmm-sub",
            }),
        ]));

        // Şehirler: en güçlü 20.
        let Ok(layer) = &self.cities else {
            return None;
        };
        let mut idx: Vec<usize> = (0..results.sites.len())
            .filter(|&i| results.sites[i].motion.is_some())
            .collect();
        idx.sort_by(|&a, &c| {
            let ja = results.sites[a].motion.map_or(0.0, |m| m.jma);
            let jc = results.sites[c].motion.map_or(0.0, |m| m.jma);
            jc.total_cmp(&ja)
        });
        // Shindo sınıfı başına şehir sayısı.
        let mut by_class = std::collections::BTreeMap::new();
        for &i in &idx {
            if let Some(m) = results.sites[i].motion {
                *by_class
                    .entry(std::cmp::Reverse(m.shindo() as u8))
                    .or_insert(0usize) += 1;
            }
        }
        b.push(Block::H2(t("report-cities")));
        if let Some(&top) = idx.first()
            && let Some(m) = results.sites[top].motion
        {
            b.push(Block::P(lang.t_args(
                "report-max",
                &[
                    ("shindo", m.shindo().label().to_string()),
                    ("mmi", n1(m.mmi)),
                    ("city", layer.cities[top].name.clone()),
                ],
            )));
        }
        let classes: Vec<String> = by_class
            .iter()
            .map(|(c, n)| {
                let s = seismira_core::intensity::Shindo::ALL
                    .iter()
                    .find(|s| **s as u8 == c.0)
                    .map_or("?", |s| s.label());
                format!("Shindo {s}: {n}")
            })
            .collect();
        b.push(Block::P(lang.t_args(
            "report-class-counts",
            &[("list", classes.join(", "))],
        )));
        let rows: Vec<Vec<String>> = idx
            .iter()
            .take(20)
            .filter_map(|&i| {
                let m = results.sites[i].motion?;
                let c = &layer.cities[i];
                let site = &results.sites[i];
                let dmg = self
                    .damage
                    .as_ref()
                    .and_then(|d| d.cities.get(i).copied().flatten())
                    .map_or("–".to_string(), |d| {
                        format!(
                            "%{} (%{}–{})",
                            n1(100.0 * seismira_core::damage::RegionDamage::heavy(&d[0])),
                            n1(100.0 * seismira_core::damage::RegionDamage::heavy(&d[1])),
                            n1(100.0 * seismira_core::damage::RegionDamage::heavy(&d[2]))
                        )
                    });
                Some(vec![
                    format!("{} ({})", c.name, String::from_utf8_lossy(&c.country)),
                    m.shindo().label().to_string(),
                    n1(m.mmi),
                    lang.num(m.pga.median(), 2),
                    n0(m.pgv.median()),
                    n0(site.dist.rrup),
                    dmg,
                ])
            })
            .collect();
        b.push(Block::Table(
            [
                "report-c-city",
                "report-c-shindo",
                "report-c-mmi",
                "report-c-pga",
                "report-c-pgv",
                "report-c-rrup",
                "report-c-damage",
            ]
            .iter()
            .map(|k| t(k))
            .collect(),
            rows,
        ));

        // Erken uyarı.
        if let Some(a) = &self.alert {
            b.push(Block::H2(t("report-warning")));
            b.push(Block::P(lang.t_args(
                "report-warning-text",
                &[("t", n1(a.t)), ("n", n0(a.regions.len() as f64))],
            )));
        }

        // Bölgeler: ağır hasara göre en çok 25 (yoksa en yüksek Shindo).
        if let Some(reg) = &self.regions
            && !self.region_results.is_empty()
        {
            let heavy = |id| {
                self.damage
                    .as_ref()
                    .and_then(|d| d.region(id))
                    .map(|d| seismira_core::damage::RegionDamage::heavy(&d.median))
            };
            let mut rr: Vec<&crate::compute::RegionResult> = self.region_results.iter().collect();
            rr.sort_by(|a, c| {
                heavy(c.summary.id)
                    .unwrap_or(-1.0)
                    .total_cmp(&heavy(a.summary.id).unwrap_or(-1.0))
                    .then((c.summary.max as u8).cmp(&(a.summary.max as u8)))
            });
            let total: f64 = rr.iter().filter_map(|x| heavy(x.summary.id)).sum();
            b.push(Block::H2(t("report-regions")));
            if total > 0.0 {
                b.push(Block::P(
                    lang.t_args("report-heavy-total", &[("n", n0(total))]),
                ));
            }
            let gf = |k: usize, id| {
                self.ground_failure[k]
                    .0
                    .iter()
                    .find(|g| g.id == id)
                    .map_or("–".to_string(), |g| n0(g.km2[0]))
            };
            let rows: Vec<Vec<String>> = rr
                .iter()
                .take(25)
                .filter_map(|x| {
                    let info = reg.info(x.summary.id)?;
                    let name = if lang == Lang::Tr {
                        &info.name_tr
                    } else {
                        &info.name_en
                    };
                    Some(vec![
                        format!("{name} ({})", String::from_utf8_lossy(&info.country)),
                        x.summary.max.label().to_string(),
                        heavy(x.summary.id).map_or("–".to_string(), n0),
                        gf(0, x.summary.id),
                        gf(1, x.summary.id),
                    ])
                })
                .collect();
            b.push(Block::Table(
                [
                    "report-r-region",
                    "report-r-max",
                    "report-r-heavy",
                    "report-r-liq",
                    "report-r-ls",
                ]
                .iter()
                .map(|k| t(k))
                .collect(),
                rows,
            ));
        }

        // Tsunami.
        if let Some(ts) = &self.tsunami {
            b.push(Block::H2(t("report-tsunami")));
            if ts.coast.is_empty() {
                b.push(Block::P(t("report-tsunami-none")));
            } else {
                use crate::tsunami::Category;
                let mut items = Vec::new();
                for c in [Category::Major, Category::Warning, Category::Advisory] {
                    let pts: Vec<_> = ts.coast.iter().filter(|p| p.category == c).collect();
                    if pts.is_empty() {
                        continue;
                    }
                    let first = pts.iter().map(|p| p.eta_s).fold(f64::INFINITY, f64::min);
                    let hmax = pts.iter().filter_map(|p| p.height).fold(0.0f64, f64::max);
                    let mut places: Vec<String> = pts
                        .iter()
                        .filter_map(|p| p.place.as_ref().map(|(n, _)| n.clone()))
                        .collect();
                    places.dedup();
                    places.truncate(8);
                    items.push(lang.t_args(
                        "report-tsunami-cat",
                        &[
                            ("cat", t(c.key())),
                            ("n", n0(pts.len() as f64)),
                            ("eta", n0(first / 60.0)),
                            ("h", n1(hmax)),
                            ("places", places.join(", ")),
                        ],
                    ));
                }
                b.push(Block::List(items));
            }
        }

        // Kullanıcı envanteri.
        if let (Some(inv), Some(res)) = (&self.user.inventory, &self.user.inventory_results) {
            let ([m, lo, hi], covered) = res.totals(inv);
            b.push(Block::H2(t("report-inventory")));
            b.push(Block::P(lang.t_args(
                "user-inv-result",
                &[
                    ("m", n0(m)),
                    ("lo", n0(lo)),
                    ("hi", n0(hi)),
                    ("c", n0(covered)),
                ],
            )));
        }

        // Sınırlar ve kaynaklar.
        b.push(Block::H2(t("report-limits")));
        b.push(Block::List(
            [
                "report-limit-1",
                "report-limit-2",
                "report-limit-3",
                "report-limit-4",
                "report-limit-5",
            ]
            .iter()
            .map(|k| t(k))
            .collect(),
        ));
        if let Ok(src) = &self.sources {
            b.push(Block::H2(t("report-sources")));
            b.push(Block::List(
                src.iter()
                    .filter(|s| s.distributed)
                    .map(|s| format!("{}: {} ({})", s.name, s.attribution, s.license))
                    .collect(),
            ));
        }
        Some((md(&b), html(&title, lang, &b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bicimler() {
        let b = vec![
            Block::H1("Başlık".into()),
            Block::P("a < b & c".into()),
            Block::List(vec!["x".into()]),
            Block::Table(
                vec!["A".into(), "B".into()],
                vec![vec!["1|2".into(), "3".into()]],
            ),
        ];
        let m = md(&b);
        assert!(m.contains("# Başlık") && m.contains("| 1\\|2 | 3 |") && m.contains("|---|---|"));
        let h = html("T", Lang::Tr, &b);
        assert!(
            h.contains("a &lt; b &amp; c")
                && h.contains("<td>1|2</td>")
                && h.contains("lang=\"tr\"")
        );
    }
}
