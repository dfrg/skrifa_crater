#!/usr/bin/env bash
# Runs crater for one fontations ref.
#
# Usage: scripts/run.sh [fontations-ref] [fontations-repo]
#
# The plain configurations compare skrifa without its best-effort shaping
# against FreeType built without HarfBuzz; there the automatic hinters find
# glyphs through the character map alone and should agree exactly. The
# `-shaped` configurations compare skrifa's shaping against FreeType built
# with HarfBuzz.
#
# FreeType's dumps depend only on the fonts and its version, so they are
# cached under .cache. The result lands in out/<ref>.json, with slashes in
# the ref replaced by underscores. A ref the harness does not build against
# still gets a result, marked as a failed build.

set -euo pipefail
# A failed oracle build inside $(...) must stop the run, not cache an
# empty dump.
shopt -s inherit_errexit

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ref=${1:-main}
repo=${2:-https://github.com/googlefonts/fontations}
# CRATER_PLAIN and CRATER_SHAPED replace the configurations, for
# experiments.
plain="${CRATER_PLAIN:-unscaled ppem=12 ppem=16 ppem=24 native=12 native=16 native=24 mono=16 auto=12 auto=16 autolight=16}"
shaped="${CRATER_SHAPED:-auto-shaped=16 autolight-shaped=16}"
cd "$root"

python3 scripts/fetch_fonts.py
mkdir -p out .cache
ls .cache/fonts/* | while read -r path; do
    name=$(basename "$path")
    echo "${name%.*} $path"
done > .cache/fonts.txt

ft_tag=${CRATER_FREETYPE_TAG:-$(sed -n 's/^freetype = "\(.*\)"/\1/p' oracles/versions.toml)}
# The oracle's patches are part of its version.
patches=()
if [ -z "${CRATER_FREETYPE_NO_PATCHES:-}" ]; then
    shopt -s nullglob
    patches=(oracles/freetype/*.patch)
    shopt -u nullglob
fi
for patch in "${patches[@]}"; do
    ft_tag="$ft_tag+$(basename "$patch" .patch)"
done
hb_tag=$(sed -n 's/^harfbuzz = "\(.*\)"/\1/p' oracles/versions.toml)

# Dumps FreeType once for each build, fonts and set of configurations.
oracle_dump() {
    local variant=$1
    shift
    local ft_dump key out
    ft_dump=$(bash oracles/freetype/build.sh $variant)
    key=$(cat fonts/manifest.toml oracles/versions.toml oracles/freetype/ft_dump.c \
        "${patches[@]}" \
        <(echo "$ft_tag $variant $*") | sha256sum | cut -c1-16)
    out=".cache/oracle-$key.txt"
    if [ ! -f "$out" ]; then
        "$ft_dump" .cache/fonts.txt "$@" > "$out.tmp"
        mv "$out.tmp" "$out"
    fi
    echo "$out"
}
oracle_plain=$(oracle_dump "" $plain)
oracle_shaped=$(oracle_dump hb $shaped)
cat "$oracle_plain" "$oracle_shaped" > out/oracle.txt

checkout=".cache/fontations-$(echo "$repo" | sha256sum | cut -c1-12)"
if [ ! -d "$checkout" ]; then
    git clone -q "$repo" "$checkout"
fi
git -C "$checkout" fetch -q origin "$ref"
git -C "$checkout" checkout -q --detach FETCH_HEAD
sha=$(git -C "$checkout" rev-parse HEAD)
date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
safe_ref=${ref//\//_}
result="out/$safe_ref.json"

patch="patch.\"https://github.com/googlefonts/fontations\".skrifa.path=\"$checkout/skrifa\""
crater() {
    cargo run -q --release --config "$patch" -p crater "$@"
}
if ! { crater --no-default-features -- dump .cache/fonts.txt $plain > out/skrifa.txt &&
        crater -- dump .cache/fonts.txt $shaped >> out/skrifa.txt; } 2> out/build.log; then
    python3 scripts/failed_result.py "$ref" "$repo" "$sha" "$ft_tag" "$date" \
        out/build.log > "$result"
    echo "build or dump failed for $ref" >&2
    echo "$result"
    exit 0
fi
crater -- compare out/oracle.txt out/skrifa.txt \
    "ref=$ref" "repo=$repo" "sha=$sha" "freetype=$ft_tag" "harfbuzz=$hb_tag" \
    "date=$date" > "$result"
echo "$result"
