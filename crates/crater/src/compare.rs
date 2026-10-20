//! Compares skrifa's records with an oracle's.

use crate::record::{Element, Glyph, Records, regularize, write_path};
use serde::Serialize;
use std::collections::BTreeMap;

/// The most mismatching glyphs kept per configuration, with their paths.
const MAX_EXAMPLES: usize = 12;

#[derive(Serialize)]
pub struct Report {
    pub meta: BTreeMap<String, String>,
    pub fonts: Vec<FontReport>,
}

#[derive(Serialize)]
pub struct FontReport {
    pub id: String,
    pub configs: Vec<ConfigReport>,
}

#[derive(Serialize, Default)]
pub struct ConfigReport {
    pub name: String,
    /// Glyphs both sides drew.
    pub glyphs: u32,
    /// Glyphs whose outlines match after regularization.
    pub outlines_match: u32,
    /// Glyphs whose outlines have the same elements with points at most
    /// one unit apart.
    pub outlines_off_by_one: u32,
    /// Glyphs whose advances differ by more than the tolerance.
    pub advance_mismatches: u32,
    /// Glyphs the oracle drew and skrifa could not, or the reverse.
    pub errors_oracle: u32,
    pub errors_skrifa: u32,
    /// Glyphs present on one side only.
    pub missing: u32,
    pub examples: Vec<Example>,
}

#[derive(Serialize)]
pub struct Example {
    pub gid: u32,
    pub kind: &'static str,
    pub oracle: String,
    pub skrifa: String,
}

/// How far apart two outlines are.
enum Distance {
    Same,
    /// Same elements, points at most this far apart.
    Points(i64),
    /// Different elements.
    Structure,
}

fn distance(a: &[Element], b: &[Element]) -> Distance {
    if a == b {
        return Distance::Same;
    }
    if a.len() != b.len() {
        return Distance::Structure;
    }
    let mut max = 0;
    for (x, y) in a.iter().zip(b) {
        let pairs: Vec<(i64, i64)> = match (x, y) {
            (Element::Move(a0, a1), Element::Move(b0, b1))
            | (Element::Line(a0, a1), Element::Line(b0, b1)) => vec![(*a0, *b0), (*a1, *b1)],
            (Element::Quad(a0, a1, a2, a3), Element::Quad(b0, b1, b2, b3)) => {
                vec![(*a0, *b0), (*a1, *b1), (*a2, *b2), (*a3, *b3)]
            }
            (Element::Cubic(a0, a1, a2, a3, a4, a5), Element::Cubic(b0, b1, b2, b3, b4, b5)) => {
                vec![
                    (*a0, *b0),
                    (*a1, *b1),
                    (*a2, *b2),
                    (*a3, *b3),
                    (*a4, *b4),
                    (*a5, *b5),
                ]
            }
            (Element::Close, Element::Close) => vec![],
            _ => return Distance::Structure,
        };
        for (p, q) in pairs {
            max = max.max((p - q).abs());
        }
    }
    Distance::Points(max)
}

/// Returns the advance tolerance for a configuration. Unscaled and hinted
/// advances are whole font units and 26.6 values, and must match. Unhinted
/// scaled advances are 16.16 and may differ by 1/64 of a pixel.
fn advance_tolerance(config: &str) -> i64 {
    if config.starts_with("ppem=") { 1024 } else { 0 }
}

pub fn compare(oracle: &Records, skrifa: &Records, meta: BTreeMap<String, String>) -> Report {
    let mut fonts = Vec::new();
    for (font_id, oracle_configs) in oracle {
        let skrifa_configs = skrifa.get(font_id);
        let mut configs = Vec::new();
        for (config_name, oracle_glyphs) in oracle_configs {
            let empty = BTreeMap::new();
            let skrifa_glyphs = skrifa_configs
                .and_then(|configs| configs.get(config_name))
                .unwrap_or(&empty);
            configs.push(compare_config(config_name, oracle_glyphs, skrifa_glyphs));
        }
        fonts.push(FontReport {
            id: font_id.clone(),
            configs,
        });
    }
    Report { meta, fonts }
}

fn compare_config(
    name: &str,
    oracle: &BTreeMap<u32, Glyph>,
    skrifa: &BTreeMap<u32, Glyph>,
) -> ConfigReport {
    let mut report = ConfigReport {
        name: name.to_string(),
        ..Default::default()
    };
    // Worst first: structure, then by point distance, then advances.
    let mut candidates: Vec<(i64, Example)> = Vec::new();
    let tolerance = advance_tolerance(name);
    let gids: std::collections::BTreeSet<u32> =
        oracle.keys().chain(skrifa.keys()).copied().collect();
    for gid in gids {
        let (Some(o), Some(s)) = (oracle.get(&gid), skrifa.get(&gid)) else {
            report.missing += 1;
            continue;
        };
        let (o_adv, o_path, s_adv, s_path) = match (o, s) {
            (
                Glyph::Drawn {
                    advance: oa,
                    path: op,
                },
                Glyph::Drawn {
                    advance: sa,
                    path: sp,
                },
            ) => (*oa, regularize(op), *sa, regularize(sp)),
            (Glyph::Error(_), Glyph::Error(_)) => continue,
            (Glyph::Error(_), _) => {
                report.errors_oracle += 1;
                continue;
            }
            (_, Glyph::Error(e)) => {
                report.errors_skrifa += 1;
                candidates.push((
                    i64::MAX,
                    Example {
                        gid,
                        kind: "skrifa error",
                        oracle: String::new(),
                        skrifa: e.clone(),
                    },
                ));
                continue;
            }
        };
        report.glyphs += 1;
        match distance(&o_path, &s_path) {
            Distance::Same => report.outlines_match += 1,
            Distance::Points(d) if d <= 1 => report.outlines_off_by_one += 1,
            distance => {
                let (kind, score) = match distance {
                    Distance::Points(d) => ("outline points", d),
                    _ => ("outline structure", i64::MAX - 1),
                };
                candidates.push((
                    score,
                    Example {
                        gid,
                        kind,
                        oracle: write_path(&o_path),
                        skrifa: write_path(&s_path),
                    },
                ));
            }
        }
        if (o_adv - s_adv).abs() > tolerance {
            report.advance_mismatches += 1;
            candidates.push((
                0,
                Example {
                    gid,
                    kind: "advance",
                    oracle: o_adv.to_string(),
                    skrifa: s_adv.to_string(),
                },
            ));
        }
    }
    candidates.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    report.examples = candidates
        .into_iter()
        .take(MAX_EXAMPLES)
        .map(|(_, example)| example)
        .collect();
    report
}
