# skrifa crater

Compares [skrifa](https://github.com/googlefonts/fontations/tree/main/skrifa)
with FreeType and HarfBuzz over a set of fonts chosen to reach every code
path, for any fontations ref. Results are published to this repository's
GitHub Pages site.

Today it compares outlines and advances against FreeType:

| Configuration | Compares |
|---|---|
| `unscaled`, `ppem=N` | unhinted, in font units and at a size |
| `native=N` | the font's own hints: TrueType (v40) or CFF |
| `mono=N` | the font's own hints for a monochrome target |
| `auto=N`, `autolight=N` | the automatic hinter, normal and light |
| `auto-shaped=N`, `autolight-shaped=N` | the same, with skrifa's best-effort shaping against FreeType built with HarfBuzz |

The plain autohinting configurations build skrifa without its
`autohint_shaping` feature and FreeType without HarfBuzz, so both find
glyphs through the character map alone and should agree exactly. The
`-shaped` ones show where skrifa's shaping and HarfBuzz part ways.

HarfBuzz metrics, coverage-driven font selection and built fonts come
next.

## Running

On Linux, with Rust, Python 3.11+, meson, ninja and C and C++ compilers:

```sh
scripts/run.sh                      # fontations main
scripts/run.sh some/branch          # any ref
scripts/run.sh my-branch https://github.com/me/fontations
```

The result lands in `out/<ref>.json`. To see it, publish into a scratch
copy of the site and serve it:

```sh
cp -r site /tmp/site
python3 scripts/publish.py /tmp/site out/main.json
python3 -m http.server -d /tmp/site
```

The `crater` workflow does the same on GitHub Actions, nightly for main
and on demand for any refs, and pushes to the `gh-pages` branch.

## How it works

- `fonts/manifest.toml` lists the fonts, each fetched from a URL pinned to
  a commit and checked by SHA-256. Fonts are not stored here.
- `oracles/versions.toml` pins FreeType and HarfBuzz. `oracles/freetype`
  builds FreeType at that tag, with or without HarfBuzz, and a small dumper
  against it. Its output depends only on the fonts and the versions, so it
  is cached. `CRATER_FREETYPE_TAG` overrides the tag for experiments.
  The patches in `oracles/freetype` are applied first, and results name
  them in the FreeType version: `no-diacritics` turns off the diacritic
  handling FreeType 2.14 added to its autohinter, which skrifa deliberately
  does not have.
- `crates/crater` dumps the same records from skrifa, built against the
  fontations ref under test, and compares them. Paths are regularized
  first: FreeType closes contours implicitly and both sides emit some
  degenerate elements the other does not.
- `known_diffs.toml` lists differences that are expected, with reasons.
- `site/` is the static page.

A ref the harness does not build against gets a result marked as a failed
build, with the log.
