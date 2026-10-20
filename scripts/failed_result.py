"""Writes the result of a run whose harness did not build or run.

Usage: failed_result.py <ref> <repo> <sha> <freetype> <date> <log>
"""

import json
import sys

ref, repo, sha, freetype, date, log_path = sys.argv[1:7]
with open(log_path, encoding="utf-8", errors="replace") as log:
    tail = log.read()[-4000:]
json.dump(
    {
        "meta": {"ref": ref, "repo": repo, "sha": sha, "freetype": freetype, "date": date},
        "build_failed": True,
        "log": tail,
        "fonts": [],
    },
    sys.stdout,
    indent=1,
)
