# skrifa crater

Compares [skrifa](https://github.com/googlefonts/fontations/tree/main/skrifa)
with FreeType and HarfBuzz over a set of fonts chosen to reach every code
path, for any fontations ref. Results are published to this repository's
GitHub Pages site.

Today it compares unhinted outlines and advances, in font units and at
12, 16 and 24 ppem, against FreeType. Hinted outlines, HarfBuzz metrics,
coverage-driven font selection and built fonts come next.

## Running

On Linux, with Rust, Python 3.11+, meson, ninja and a C compiler:

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
  builds FreeType at that tag and a small dumper against it. Its output
  depends only on the fonts and the version, so it is cached.
- `crates/crater` dumps the same records from skrifa, built against the
  fontations ref under test, and compares them. Paths are regularized
  first: FreeType closes contours implicitly and both sides emit some
  degenerate elements the other does not.
- `known_diffs.toml` lists differences that are expected, with reasons.
- `site/` is the static page.

A ref the harness does not build against gets a result marked as a failed
build, with the log.
