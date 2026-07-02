#!/usr/bin/env /usr/bin/python3
"""Apply accepted adjudication decisions to LanceDB.

Reads an adjudication trail JSON, writes accepted corrections to LanceDB
with extraction_method="adjudicated", and appends to drrp_history.

Usage:
    /usr/bin/python3 .claude/skills/human-review/scripts/apply_adjudication.py \
        --adjudication data/audit/UK_uksi_2002_2788_adjudicated.json
"""

import argparse
import json
import sys
from datetime import datetime, timezone

import lancedb
import pyarrow as pa


def main():
    parser = argparse.ArgumentParser(description="Apply adjudication to LanceDB")
    parser.add_argument("--adjudication", required=True, help="Path to adjudication JSON")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be written")
    args = parser.parse_args()

    with open(args.adjudication) as f:
        adj = json.load(f)

    accepted = [d for d in adj.get("decisions", []) if d.get("decision") == "accept"]

    if not accepted:
        print("No accepted corrections to apply.")
        return

    print(f"Applying {len(accepted)} adjudicated corrections...")

    if args.dry_run:
        for d in accepted:
            print(f"  {d['section_id']}: {d.get('pre_llm_drrp', '?')} -> {d.get('llm_drrp', '?')}")
        print(f"\nDry run — {len(accepted)} corrections would be written.")
        return

    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    now_iso = datetime.now(timezone.utc).isoformat()

    for d in accepted:
        sid = d["section_id"]
        new_drrp = d.get("llm_drrp", "none")

        # Read existing drrp_history
        existing = (
            tbl.search()
            .where(f"section_id = '{sid}'")
            .select(["section_id", "drrp_history"])
            .limit(1)
            .to_arrow()
        )
        if existing.num_rows == 0:
            print(f"  WARNING: {sid} not found in LanceDB, skipping")
            continue

        hist_raw = existing.column("drrp_history")[0].as_py() or "[]"
        try:
            history = json.loads(hist_raw)
        except Exception:
            history = []

        # Append adjudicated entry
        history.append(
            {
                "tier": "adjudicated",
                "drrp": new_drrp,
                "confidence": 1.0,
                "timestamp": now_iso,
                "reason": d.get("reason", "human adjudication"),
            }
        )

        # Build update batch
        drrp_list = [new_drrp] if new_drrp != "none" else []
        update = pa.table(
            {
                "section_id": [sid],
                "drrp_types": [drrp_list],
                "extraction_method": ["adjudicated"],
                "drrp_history": [json.dumps(history)],
            }
        )

        tbl.merge_insert("section_id").when_matched_update_all().execute(update)
        print(f"  {sid}: {d.get('pre_llm_drrp', '?')} -> {new_drrp} [adjudicated]")

    print(f"\n{len(accepted)} corrections applied with extraction_method='adjudicated'.")
    print("These provisions are now protected from lower-tier overwrites (source_tier=7).")


if __name__ == "__main__":
    main()
