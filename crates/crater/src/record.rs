//! The record format shared by the oracles and the skrifa dumper.
//!
//! ```text
//! font <id>
//! config <name>                 unscaled, or ppem=<n>
//! glyph <gid> <advance> <path>
//! glyph <gid> error <reason>
//! ```
//!
//! Unscaled coordinates and advances are font units. Scaled coordinates are
//! 26.6 and scaled advances 16.16, all integers. A path is a sequence of
//! `M x y`, `L x y`, `Q cx cy x y`, `C cx0 cy0 cx1 cy1 x y` and `Z`.

use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Element {
    Move(i64, i64),
    Line(i64, i64),
    Quad(i64, i64, i64, i64),
    Cubic(i64, i64, i64, i64, i64, i64),
    Close,
}

impl Element {
    fn end(&self) -> Option<(i64, i64)> {
        match *self {
            Self::Move(x, y) | Self::Line(x, y) | Self::Quad(_, _, x, y) => Some((x, y)),
            Self::Cubic(_, _, _, _, x, y) => Some((x, y)),
            Self::Close => None,
        }
    }

    fn letter(&self) -> &'static str {
        match self {
            Self::Move(..) => "M",
            Self::Line(..) => "L",
            Self::Quad(..) => "Q",
            Self::Cubic(..) => "C",
            Self::Close => "Z",
        }
    }
}

/// A glyph's advance and outline, or why it has none.
#[derive(Clone, PartialEq, Debug)]
pub enum Glyph {
    Drawn { advance: i64, path: Vec<Element> },
    Error(String),
}

/// Glyphs by font, then configuration, then glyph id.
pub type Records = BTreeMap<String, BTreeMap<String, BTreeMap<u32, Glyph>>>;

pub fn write_path(path: &[Element]) -> String {
    let mut out = String::new();
    for element in path {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(element.letter());
        let values: &[i64] = match element {
            Element::Move(x, y) | Element::Line(x, y) => &[*x, *y],
            Element::Quad(a, b, c, d) => &[*a, *b, *c, *d],
            Element::Cubic(a, b, c, d, e, f) => &[*a, *b, *c, *d, *e, *f],
            Element::Close => &[],
        };
        for value in values {
            write!(out, " {value}").unwrap();
        }
    }
    out
}

fn parse_path(tokens: &[&str]) -> Option<Vec<Element>> {
    let mut path = Vec::new();
    let mut i = 0;
    let num = |i: usize| -> Option<i64> { tokens.get(i)?.parse().ok() };
    while i < tokens.len() {
        let element = match tokens[i] {
            "M" => Element::Move(num(i + 1)?, num(i + 2)?),
            "L" => Element::Line(num(i + 1)?, num(i + 2)?),
            "Q" => Element::Quad(num(i + 1)?, num(i + 2)?, num(i + 3)?, num(i + 4)?),
            "C" => Element::Cubic(
                num(i + 1)?,
                num(i + 2)?,
                num(i + 3)?,
                num(i + 4)?,
                num(i + 5)?,
                num(i + 6)?,
            ),
            "Z" => Element::Close,
            _ => return None,
        };
        i += 1 + match element {
            Element::Move(..) | Element::Line(..) => 2,
            Element::Quad(..) => 4,
            Element::Cubic(..) => 6,
            Element::Close => 0,
        };
        path.push(element);
    }
    Some(path)
}

pub fn parse(text: &str) -> Records {
    let mut records = Records::new();
    let mut font = String::new();
    let mut config = String::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_ascii_whitespace().collect();
        match tokens.as_slice() {
            ["font", id] => font = id.to_string(),
            ["config", name] => config = name.to_string(),
            ["glyph", gid, "error", reason @ ..] => {
                let glyph = Glyph::Error(reason.join(" "));
                insert(&mut records, &font, &config, gid, glyph);
            }
            ["glyph", gid, advance, path @ ..] => {
                let (Ok(advance), Some(path)) = (advance.parse(), parse_path(path)) else {
                    continue;
                };
                insert(
                    &mut records,
                    &font,
                    &config,
                    gid,
                    Glyph::Drawn { advance, path },
                );
            }
            _ => {}
        }
    }
    records
}

fn insert(records: &mut Records, font: &str, config: &str, gid: &str, glyph: Glyph) {
    if let Ok(gid) = gid.parse() {
        records
            .entry(font.to_string())
            .or_default()
            .entry(config.to_string())
            .or_default()
            .insert(gid, glyph);
    }
}

/// Removes the differences in how FreeType and skrifa emit the same
/// outline.
///
/// FreeType closes contours implicitly, so a close becomes a line back to
/// the contour's start when the contour does not already end there. Both
/// emit some degenerate elements the other does not: a move followed by
/// another move, and a line to the current point, are dropped.
pub fn regularize(path: &[Element]) -> Vec<Element> {
    let mut out = Vec::with_capacity(path.len());
    let mut pending_move: Option<(i64, i64)> = None;
    let mut start = (0, 0);
    let mut current: Option<(i64, i64)> = None;
    for element in path {
        match *element {
            Element::Move(x, y) => {
                pending_move = Some((x, y));
                start = (x, y);
                current = Some((x, y));
            }
            Element::Line(x, y) => {
                if current != Some((x, y)) {
                    if let Some((mx, my)) = pending_move.take() {
                        out.push(Element::Move(mx, my));
                    }
                    out.push(Element::Line(x, y));
                    current = Some((x, y));
                }
            }
            Element::Close => {
                if pending_move.is_none() && current != Some(start) {
                    out.push(Element::Line(start.0, start.1));
                    current = Some(start);
                }
            }
            curve => {
                if let Some((mx, my)) = pending_move.take() {
                    out.push(Element::Move(mx, my));
                }
                out.push(curve);
                current = curve.end();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use Element::*;

    #[test]
    fn paths_round_trip() {
        let path = vec![
            Move(1, 2),
            Line(3, -4),
            Quad(5, 6, 7, 8),
            Cubic(1, 2, 3, 4, 5, 6),
            Close,
        ];
        let text = write_path(&path);
        let tokens: Vec<&str> = text.split_ascii_whitespace().collect();
        assert_eq!(parse_path(&tokens), Some(path));
    }

    #[test]
    fn closes_become_lines_and_degenerates_drop() {
        let path = [
            Move(0, 0),
            Move(1, 1),
            Line(1, 1),
            Line(5, 1),
            Line(5, 5),
            Close,
        ];
        assert_eq!(
            regularize(&path),
            [Move(1, 1), Line(5, 1), Line(5, 5), Line(1, 1)]
        );
        // A contour that already ends at its start gains nothing.
        let path = [Move(0, 0), Line(5, 0), Line(0, 0), Close];
        assert_eq!(regularize(&path), [Move(0, 0), Line(5, 0), Line(0, 0)]);
    }
}
