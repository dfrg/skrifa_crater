#!/usr/bin/env bash
# Builds FreeType at the pinned tag and the ft_dump oracle against it.
#
# Usage: build.sh [hb]
#
# With `hb`, FreeType is built with HarfBuzz, which its autohinter uses to
# find the glyphs each script and feature reaches. Output goes to
# .cache/freetype-<tag>[-hb]. Prints the path of ft_dump.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
# CRATER_FREETYPE_TAG overrides the pinned tag, for experiments.
tag=${CRATER_FREETYPE_TAG:-$(sed -n 's/^freetype = "\(.*\)"/\1/p' "$root/oracles/versions.toml")}
variant=${1:-}
out="$root/.cache/freetype-$tag${variant:+-$variant}"
harfbuzz=disabled
libs=()
if [ "$variant" = hb ]; then
    hb_prefix=$(bash "$root/oracles/harfbuzz/build.sh")
    export PKG_CONFIG_PATH="$hb_prefix/lib/pkgconfig"
    harfbuzz=enabled
    libs=("$hb_prefix/lib/libharfbuzz.a" -lstdc++)
fi

if [ ! -x "$out/ft_dump" ] || [ "$root/oracles/freetype/ft_dump.c" -nt "$out/ft_dump" ]; then
    if [ ! -f "$out/build/libfreetype.a" ]; then
        rm -rf "$out"
        git clone -q --depth 1 --branch "$tag" \
            https://gitlab.freedesktop.org/freetype/freetype.git "$out/src" 2> /dev/null
        meson setup "$out/build" "$out/src" --buildtype=release \
            -Ddefault_library=static -Dzlib=disabled -Dbzip2=disabled \
            -Dpng=disabled -Dharfbuzz=$harfbuzz -Dbrotli=disabled > /dev/null
        ninja -C "$out/build" > /dev/null
    fi
    cc -O2 -I "$out/src/include" -o "$out/ft_dump" \
        "$root/oracles/freetype/ft_dump.c" "$out/build/libfreetype.a" "${libs[@]}" -lm
fi
echo "$out/ft_dump"
