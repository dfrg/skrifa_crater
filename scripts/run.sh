#!/usr/bin/env bash
# Runs crater for one fontations ref.
#
# Usage: scripts/run.sh [fontations-ref] [fontations-repo]
#
# FreeType's dumps depend only on the fonts and its version, so they are
# cached under .cache. The result lands in out/<ref>.json, with slashes in
# the ref replaced by underscores. A ref the harness does not build against
# still gets a result, marked as a failed build.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ref=${1:-main}
repo=${2:-https://github.com/googlefonts/fontations}
sizes="0 12 16 24"
cd "$root"

python3 scripts/fetch_fonts.py
mkdir -p out .cache
ls .cache/fonts/* | while read -r path; do
    name=$(basename "$path")
    echo "${name%.*} $path"
done > .cache/fonts.txt

ft_dump=$(bash oracles/freetype/build.sh)
ft_tag=$(sed -n 's/^freetype = "\(.*\)"/\1/p' oracles/versions.toml)
key=$(cat fonts/manifest.toml oracles/freetype/ft_dump.c | sha256sum | cut -c1-16)
ft_out=".cache/freetype-$ft_tag-$key-${sizes// /_}.txt"
if [ ! -f "$ft_out" ]; then
    "$ft_dump" .cache/fonts.txt $sizes > "$ft_out.tmp"
    mv "$ft_out.tmp" "$ft_out"
fi

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
if ! cargo run -q --release --config "$patch" -p crater -- dump .cache/fonts.txt $sizes \
        > out/skrifa.txt 2> out/build.log; then
    python3 scripts/failed_result.py "$ref" "$repo" "$sha" "$ft_tag" "$date" \
        out/build.log > "$result"
    echo "build or dump failed for $ref" >&2
    echo "$result"
    exit 0
fi
cargo run -q --release --config "$patch" -p crater -- compare "$ft_out" out/skrifa.txt \
    "ref=$ref" "repo=$repo" "sha=$sha" "freetype=$ft_tag" "date=$date" > "$result"
echo "$result"
