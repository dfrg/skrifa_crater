#!/usr/bin/env bash
# Builds FreeType at the pinned tag and the ft_dump oracle against it.
#
# Usage: build.sh [hb]
#
# CRATER_FREETYPE_TAG and CRATER_FREETYPE_NO_PATCHES change what is built;
# see below.
#
# With `hb`, FreeType is built with HarfBuzz, which its autohinter uses to
# find the glyphs each script and feature reaches. Every patch in
# oracles/freetype/*.patch, if any, is applied first; see the patches for why.
# Output goes to .cache/freetype-<tag>[-hb]-<patches hash>. Prints the path
# of ft_dump.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
# CRATER_FREETYPE_TAG overrides the pinned tag, for experiments.
tag=${CRATER_FREETYPE_TAG:-$(sed -n 's/^freetype = "\(.*\)"/\1/p' "$root/oracles/versions.toml")}
variant=${1:-}
# CRATER_FREETYPE_NO_PATCHES builds the tag as released, for a tag the
# patches don't apply to.
patches=()
if [ -z "${CRATER_FREETYPE_NO_PATCHES:-}" ]; then
    shopt -s nullglob
    patches=("$root"/oracles/freetype/*.patch)
    shopt -u nullglob
fi
patches_hash=$(cat "${patches[@]}" /dev/null | sha256sum | cut -c1-8)
out="$root/.cache/freetype-$tag${variant:+-$variant}-$patches_hash"
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
        for patch in "${patches[@]}"; do
            git -C "$out/src" apply "$patch"
        done
        meson setup "$out/build" "$out/src" --buildtype=release \
            -Ddefault_library=static -Dzlib=disabled -Dbzip2=disabled \
            -Dpng=disabled -Dharfbuzz=$harfbuzz -Dbrotli=disabled > /dev/null
        ninja -C "$out/build" > /dev/null
    fi
    # FreeType 2.13.3 builds in bzip2 support whenever pkg-config finds the
    # library, whatever -Dbzip2 says.
    if grep -q '^#define FT_CONFIG_OPTION_USE_BZIP2' "$out/build/ftoption.h"; then
        libs+=(-lbz2)
    fi
    cc -O2 -I "$out/src/include" -o "$out/ft_dump" \
        "$root/oracles/freetype/ft_dump.c" "$out/build/libfreetype.a" "${libs[@]}" -lm
fi
echo "$out/ft_dump"
