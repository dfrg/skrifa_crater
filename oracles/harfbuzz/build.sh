#!/usr/bin/env bash
# Builds HarfBuzz at the pinned tag as a static library, without FreeType,
# and installs it to .cache/harfbuzz-<tag>/install. Prints that prefix.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
tag=$(sed -n 's/^harfbuzz = "\(.*\)"/\1/p' "$root/oracles/versions.toml")
out="$root/.cache/harfbuzz-$tag"

if [ ! -f "$out/install/lib/pkgconfig/harfbuzz.pc" ]; then
    rm -rf "$out"
    git clone -q --depth 1 --branch "$tag" https://github.com/harfbuzz/harfbuzz.git "$out/src" 2> /dev/null
    meson setup "$out/build" "$out/src" --buildtype=release --prefix="$out/install" \
        --libdir=lib -Ddefault_library=static -Dfreetype=disabled -Dglib=disabled \
        -Dgobject=disabled -Dcairo=disabled -Dicu=disabled -Dtests=disabled \
        -Ddocs=disabled -Dutilities=disabled -Dintrospection=disabled > /dev/null
    ninja -C "$out/build" install > /dev/null
fi
echo "$out/install"
