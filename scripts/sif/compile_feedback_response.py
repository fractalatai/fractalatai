#!/usr/bin/env python3
"""Compile per-item feedback responses into one document for a source.

Reads docs/sif/feedback/register.md (YAML frontmatter), takes the items for
one source in ID order, and joins their response_doc files under a header.
Items without a response_doc are listed as still open.

Usage:
    /usr/bin/python3 scripts/sif/compile_feedback_response.py QQ-FB-2026-11-19
    /usr/bin/python3 scripts/sif/compile_feedback_response.py QQ-FB-2026-11-19 --output path.md
"""

import argparse
import re
import sys
from datetime import date
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
FEEDBACK = ROOT / "docs" / "sif" / "feedback"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("source", help="Source id from the register, e.g. QQ-FB-2026-11-19")
    ap.add_argument("--output", type=Path, default=None,
                    help="Default: docs/sif/feedback/response-<source>.md")
    args = ap.parse_args()

    text = (FEEDBACK / "register.md").read_text(encoding="utf-8")
    fm = yaml.safe_load(re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL).group(1))
    src = next((s for s in fm.get("sources") or [] if s["id"] == args.source), None)
    if not src:
        sys.exit(f"Unknown source: {args.source}")
    items = sorted((i for i in fm.get("items") or [] if i.get("source") == args.source),
                   key=lambda i: i["id"])

    parts = [
        f"# Response to feedback on {src.get('about', 'the SIF papers')}",
        "",
        f"Feedback from: {src.get('from')}, received {src.get('received')}. "
        f"Response compiled {date.today().isoformat()}.",
        "",
        "| Item | Kind | Status | Summary |",
        "|---|---|---|---|",
    ]
    for i in items:
        summary = " ".join(str(i.get("response") or "Still open.").split())
        parts.append(f"| {i['id']} | {i.get('kind')} | {i.get('status')} | {summary} |")
    parts.append("")

    missing = []
    for i in items:
        doc = i.get("response_doc")
        if doc and (FEEDBACK / doc).exists():
            parts += ["---", "", (FEEDBACK / doc).read_text(encoding="utf-8").strip(), ""]
        else:
            missing.append(i["id"])

    out = args.output or FEEDBACK / f"response-{args.source}.md"
    out.write_text("\n".join(parts), encoding="utf-8")
    print(f"{len(items) - len(missing)}/{len(items)} responses → {out}")
    if missing:
        print("Still open: " + ", ".join(missing))


if __name__ == "__main__":
    main()
