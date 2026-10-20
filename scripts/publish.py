"""Adds run results to the site.

Usage: publish.py <site-dir> <result.json>...

Each result is copied to runs/ under a name made from its date, ref and
commit, and runs/index.json lists every run, newest first, with the
totals the overview page shows. The oldest runs past the limit are removed.
"""

import json
import pathlib
import re
import sys

KEEP = 300


def totals(result):
    out = {"glyphs": 0, "match": 0, "off_by_one": 0, "mismatch": 0, "advance": 0, "errors": 0}
    for font in result.get("fonts", []):
        for config in font["configs"]:
            out["glyphs"] += config["glyphs"]
            out["match"] += config["outlines_match"]
            out["off_by_one"] += config["outlines_off_by_one"]
            out["advance"] += config["advance_mismatches"]
            out["errors"] += config["errors_skrifa"] + config["missing"]
    out["mismatch"] = out["glyphs"] - out["match"] - out["off_by_one"]
    return out


def main():
    site = pathlib.Path(sys.argv[1])
    runs = site / "runs"
    runs.mkdir(parents=True, exist_ok=True)
    index_path = runs / "index.json"
    index = json.loads(index_path.read_text()) if index_path.exists() else []
    for path in sys.argv[2:]:
        result = json.loads(pathlib.Path(path).read_text())
        meta = result["meta"]
        stem = re.sub(r"[^A-Za-z0-9._-]", "_", f"{meta['date']}-{meta['ref']}-{meta['sha'][:8]}")
        (runs / f"{stem}.json").write_text(json.dumps(result), encoding="utf-8")
        index.append(
            {
                "file": f"{stem}.json",
                "meta": meta,
                "build_failed": result.get("build_failed", False),
                "totals": totals(result),
            }
        )
    index.sort(key=lambda run: run["meta"]["date"], reverse=True)
    for old in index[KEEP:]:
        (runs / old["file"]).unlink(missing_ok=True)
    index_path.write_text(json.dumps(index[:KEEP], indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
