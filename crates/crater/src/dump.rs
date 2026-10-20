//! Dumps glyph outlines and advances from skrifa in the record format.

use crate::record::{Element, write_path};
use skrifa::{DrawMemory, GlyphId, PathSink, ScalerFont, font::Font};
use std::io::Write;

/// Collects a path as record elements, converting each coordinate.
struct Recorder<F> {
    path: Vec<Element>,
    convert: F,
}

impl<F: Fn(f32) -> i64> PathSink for Recorder<F> {
    fn move_to(&mut self, x: f32, y: f32) {
        let c = &self.convert;
        self.path.push(Element::Move(c(x), c(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let c = &self.convert;
        self.path.push(Element::Line(c(x), c(y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let c = &self.convert;
        self.path.push(Element::Quad(c(cx0), c(cy0), c(x), c(y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let c = &self.convert;
        self.path
            .push(Element::Cubic(c(cx0), c(cy0), c(cx1), c(cy1), c(x), c(y)));
    }

    fn close(&mut self) {
        self.path.push(Element::Close);
    }
}

/// Writes the records of one font for each size in `sizes`, where 0 means
/// font units.
pub fn dump_font(
    out: &mut impl Write,
    id: &str,
    data: Vec<u8>,
    sizes: &[u32],
) -> std::io::Result<()> {
    writeln!(out, "font {id}")?;
    let Some(font) = Font::new(data, 0) else {
        return writeln!(out, "error load");
    };
    for &ppem in sizes {
        let scaled = ppem != 0;
        if scaled {
            writeln!(out, "config ppem={ppem}")?;
        } else {
            writeln!(out, "config unscaled")?;
        }
        let scaler = ScalerFont::unhinted(font.clone(), scaled.then_some(ppem as f32));
        let metrics = scaler.linear_glyph_metrics();
        for gid in 0..font.num_glyphs() {
            let glyph = GlyphId::new(gid);
            // Unscaled, FreeType truncates the midpoints it infers between
            // off-curve points, where skrifa keeps the half unit.
            let convert = move |v: f32| {
                if scaled {
                    (v * 64.0).round() as i64
                } else {
                    v.trunc() as i64
                }
            };
            let mut recorder = Recorder {
                path: Vec::new(),
                convert,
            };
            if let Some(outline) = scaler.outline(glyph)
                && let Err(error) = outline.draw(DrawMemory::Scratch, &mut recorder)
            {
                writeln!(out, "glyph {gid} error {error:?}")?;
                continue;
            }
            let advance = metrics.h_advance(glyph);
            let advance = if scaled {
                (advance * 65536.0).round() as i64
            } else {
                advance.round() as i64
            };
            writeln!(out, "glyph {gid} {advance} {}", write_path(&recorder.path))?;
        }
    }
    Ok(())
}
