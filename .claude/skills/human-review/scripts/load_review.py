#!/usr/bin/env /usr/bin/python3
"""Load and summarise an LLM validation audit log for human review.

Usage:
    /usr/bin/python3 .claude/skills/human-review/scripts/load_review.py --law UK_uksi_2002_2788
    /usr/bin/python3 .claude/skills/human-review/scripts/load_review.py --all
"""

import argparse
import glob
import json
import os
import sys


def summarise_audit(path):
    """Print a summary of an audit log file."""
    with open(path) as f:
        audit = json.load(f)

    law = audit.get("law_name", "?")
    total = audit.get("provisions_count", 0)
    corrections = audit.get("corrections", [])
    n_corrections = len(corrections)

    overrides = [c for c in corrections if c.get("delta") == "drrp_override"]
    no_change = [c for c in corrections if c.get("delta") == "no_change"]

    print(f"Law: {law}")
    print(f"  Total provisions: {total}")
    print(f"  LLM corrections:  {n_corrections}")
    print(f"    drrp_override:  {len(overrides)} (need human review)")
    print(f"    no_change:      {len(no_change)} (LLM confirmed)")
    print(f"  Model: {audit.get('model', '?')}")
    print(f"  Timestamp: {audit.get('timestamp', '?')}")
    print()

    if overrides:
        print("  Overrides to review:")
        for c in overrides:
            print(f"    {c['section_id']}: {c['pre_llm_drrp']} -> {c['llm_drrp']}")
            print(f"      Reason: {c.get('llm_reason', '?')[:100]}")
        print()

    return overrides


def main():
    parser = argparse.ArgumentParser(description="Load LLM audit log for review")
    parser.add_argument("--law", help="Law name to review")
    parser.add_argument("--all", action="store_true", help="List all pending audit logs")
    parser.add_argument(
        "--audit-dir", default="data/llm-audit", help="Directory with audit JSON files"
    )
    args = parser.parse_args()

    if args.all:
        files = sorted(glob.glob(os.path.join(args.audit_dir, "*.json")))
        # Exclude adjudication files
        files = [f for f in files if "_adjudicated" not in f]
        if not files:
            print(f"No audit logs found in {args.audit_dir}/")
            sys.exit(0)
        for path in files:
            summarise_audit(path)
        return

    if not args.law:
        print("Usage: --law LAW_NAME or --all")
        sys.exit(1)

    path = os.path.join(args.audit_dir, f"{args.law}.json")
    if not os.path.exists(path):
        print(f"No audit log found at {path}")
        print(f"Run: taxa validate --laws {args.law}")
        sys.exit(1)

    overrides = summarise_audit(path)
    if not overrides:
        print("No overrides to review.")
    else:
        print(f"{len(overrides)} corrections ready for human review.")
        print("Invoke /human-review to step through them.")


if __name__ == "__main__":
    main()
