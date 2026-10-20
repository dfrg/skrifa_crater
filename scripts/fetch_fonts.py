"""Fetches the fonts in fonts/manifest.toml into .cache/fonts.

Each font is checked against its SHA-256. With --pin, a font that has no
hash yet gets the hash of what was downloaded, written back to the
manifest.
"""

import argparse
import hashlib
import pathlib
import re
import sys
import tomllib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "fonts" / "manifest.toml"
CACHE = ROOT / ".cache" / "fonts"


def extension(url):
    name = url.rsplit("/", 1)[-1].split("?")[0]
    return name.rsplit(".", 1)[-1].lower() if "." in name else "bin"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pin", action="store_true", help="record missing hashes")
    args = parser.parse_args()

    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    CACHE.mkdir(parents=True, exist_ok=True)
    pinned = {}
    failed = False
    for font in manifest["font"]:
        path = CACHE / f"{font['id']}.{extension(font['url'])}"
        if not path.exists():
            print(f"fetch {font['id']}", file=sys.stderr)
            with urllib.request.urlopen(font["url"]) as response:
                path.write_bytes(response.read())
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        expected = font.get("sha256")
        if expected is None:
            if args.pin:
                pinned[font["id"]] = digest
            else:
                print(f"{font['id']}: no sha256 (run with --pin)", file=sys.stderr)
                failed = True
        elif expected != digest:
            print(f"{font['id']}: sha256 {digest}, expected {expected}", file=sys.stderr)
            path.unlink()
            failed = True
    if pinned:
        text = MANIFEST.read_text(encoding="utf-8")
        for font_id, digest in pinned.items():
            # Insert the hash after the font's url line.
            text = re.sub(
                rf'(id = "{re.escape(font_id)}"\nurl = "[^"]*"\n)',
                rf'\1sha256 = "{digest}"\n',
                text,
            )
        MANIFEST.write_text(text, encoding="utf-8", newline="\n")
        print(f"pinned {len(pinned)} fonts", file=sys.stderr)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
