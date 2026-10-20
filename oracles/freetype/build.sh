#!/usr/bin/env bash
# Builds FreeType at the pinned tag and the ft_dump oracle against it.
#
# Output goes to .cache/freetype-<tag>. The tag is read from
# oracles/versions.toml so that results can record it.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
tag=$(sed -n 's/^freetype = "\(.*\)"/\1/p' "$root/oracles/versions.toml")
out="$root/.cache/freetype-$tag"

if [ ! -x "$out/ft_dump" ] || [ "$root/oracles/freetype/ft_dump.c" -nt "$out/ft_dump" ]; then
    if [ ! -f "$out/build/libfreetype.a" ]; then
        rm -rf "$out"
        git clone -q --depth 1 --branch "$tag" \
            https://gitlab.freedesktop.org/freetype/freetype.git "$out/src"
        meson setup "$out/build" "$out/src" --buildtype=release \
            -Ddefault_library=static -Dzlib=disabled -Dbzip2=disabled \
            -Dpng=disabled -Dharfbuzz=disabled -Dbrotli=disabled > /dev/null
        ninja -C "$out/build" > /dev/null
    fi
    cc -O2 -I "$out/src/include" -o "$out/ft_dump" \
        "$root/oracles/freetype/ft_dump.c" "$out/build/libfreetype.a" -lm
fi
echo "$out/ft_dump"
