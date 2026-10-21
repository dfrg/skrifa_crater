//! Dumps glyph outlines and advances from skrifa in the record format.

use crate::record::{Element, write_path};
use skrifa::{DrawMemory, GlyphId, HintOptions, PathSink, RasterTarget, ScalerFont, font::Font};
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

/// How a configuration draws.
enum Mode {
    /// Font units.
    Unscaled,
    /// At a size, unhinted.
    Unhinted(f32),
    /// At a size, hinted as the options ask.
    Hinted(f32, HintOptions),
}

/// Parses a configuration name: `unscaled`, or `<mode>=<ppem>` where the
/// mode may carry a suffix after a hyphen that only names it.
///
/// The hinted modes match FreeType's load flags: `native` runs the font's
/// own hints with FreeType's TrueType interpreter behavior, `mono` does so
/// for a monochrome target, and `auto` and `autolight` run the automatic
/// hinter. The TrueType modes use the classic ClearType rules that FreeType's
/// v40 interpreter follows, or the newer ones with an `enhanced` suffix, as in
/// `native-enhanced`.
fn parse_mode(config: &str) -> Option<Mode> {
    if config == "unscaled" {
        return Some(Mode::Unscaled);
    }
    let (name, ppem) = config.split_once('=')?;
    let ppem: f32 = ppem.parse().ok()?;
    let base = name.split('-').next()?;
    let classic = !name.split('-').skip(1).any(|suffix| suffix == "enhanced");
    let options = match base {
        "ppem" => return Some(Mode::Unhinted(ppem)),
        "native" => HintOptions::interpreter().with_classic_cleartype(classic),
        "mono" => HintOptions::interpreter()
            .with_classic_cleartype(classic)
            .with_target(RasterTarget::Mono),
        "auto" => HintOptions::auto(),
        "autolight" => HintOptions::auto().with_target(RasterTarget::Light),
        _ => return None,
    };
    Some(Mode::Hinted(ppem, options))
}

/// Writes the records of one font for each configuration.
pub fn dump_font(
    out: &mut impl Write,
    id: &str,
    data: Vec<u8>,
    configs: &[String],
) -> std::io::Result<()> {
    writeln!(out, "font {id}")?;
    let Some(font) = Font::new(data, 0) else {
        return writeln!(out, "error load");
    };
    for config in configs {
        let Some(mode) = parse_mode(config) else {
            continue;
        };
        writeln!(out, "config {config}")?;
        let (scaler, hinted) = match mode {
            Mode::Unscaled => (ScalerFont::unhinted(font.clone(), None), false),
            Mode::Unhinted(ppem) => (ScalerFont::unhinted(font.clone(), Some(ppem)), false),
            Mode::Hinted(ppem, options) => (ScalerFont::hinted(font.clone(), ppem, options), true),
        };
        let scaled = scaler.ppem().is_some();
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
            let mut drawn_advance = None;
            if let Some(outline) = scaler.outline(glyph) {
                match outline.draw(DrawMemory::Scratch, &mut recorder) {
                    Ok(drawn) => drawn_advance = drawn.h_advance,
                    Err(error) => {
                        writeln!(out, "glyph {gid} error {error:?}")?;
                        continue;
                    }
                }
            }
            let linear = metrics.h_advance(glyph);
            let advance = if hinted {
                (drawn_advance.unwrap_or(linear) * 64.0).round() as i64
            } else if scaled {
                (linear * 65536.0).round() as i64
            } else {
                linear.round() as i64
            };
            writeln!(out, "glyph {gid} {advance} {}", write_path(&recorder.path))?;
        }
    }
    Ok(())
}
