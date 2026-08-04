#!/usr/bin/env python3
"""Open IE emergence pass on safety narratives via Gemini.

Extracts 5P entities, relationships, and cultural signals from safety
narratives. Outputs JSON-lines. Supports multiple source types.

Usage:
    /usr/bin/python3 scripts/cultural-graph/emergence_pass.py --sample 10 --dry-run
    /usr/bin/python3 scripts/cultural-graph/emergence_pass.py --all
    /usr/bin/python3 scripts/cultural-graph/emergence_pass.py --input data/qq/cultural-graph/hazards.csv --id-prefix HZ --source-type hazard_report --all
"""

import argparse
import csv
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import requests

# --- Config ---
GEMINI_MODEL = "gemini-2.5-flash"
GEMINI_DELAY = 1.5  # seconds between calls (rate limit)
DATA_DIR = Path("data/qq/cultural-graph")
INPUT_CSV = DATA_DIR / "Positive_Observations_Redacted(ReportTable).csv"
OUTPUT_DIR = DATA_DIR / "emergence"
PROMPTS_DIR = Path("scripts/cultural-graph/prompts")


def load_system_prompt():
    return (PROMPTS_DIR / "emergence-system-v1.md").read_text()


def load_narratives(path, id_prefix="PO", sample=None):
    """Load narratives from CSV, return list of dicts."""
    with open(path, encoding="utf-8-sig") as f:
        rows = list(csv.DictReader(f))
    # Add sequential ID and combine narrative fields
    for i, row in enumerate(rows):
        row["narrative_id"] = f"{id_prefix}-{i+1:04d}"
        # Combine RE_What + RE_ActionImmediate when present
        action = row.get("RE_ActionImmediate", "").strip()
        if action:
            row["RE_What"] = row["RE_What"].rstrip() + "\n\nImmediate action taken: " + action
    if sample:
        # Deterministic sample: take first N (data is already randomised)
        rows = rows[:sample]
    return rows


def call_gemini(system_prompt, narrative_text, site_code, api_key):
    """Call Gemini API for a single narrative extraction."""
    url = (
        f"https://generativelanguage.googleapis.com/v1beta/models/"
        f"{GEMINI_MODEL}:generateContent?key={api_key}"
    )
    user_prompt = (
        f"Site: {site_code}\n\n"
        f"Narrative:\n\"{narrative_text}\"\n\n"
        f"Extract entities, relationships, and cultural signals. "
        f"Return ONLY the JSON object, no markdown fences."
    )
    payload = {
        "contents": [
            {"role": "user", "parts": [{"text": user_prompt}]},
        ],
        "systemInstruction": {"parts": [{"text": system_prompt}]},
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": 8192,
            "responseMimeType": "application/json",
            "thinkingConfig": {"thinkingBudget": 2048},
        },
    }
    resp = requests.post(url, json=payload, timeout=60)
    resp.raise_for_status()
    data = resp.json()

    # Extract text from response
    try:
        parts = data["candidates"][0]["content"]["parts"]
        text = next(p["text"] for p in parts if "text" in p)
        return json.loads(text)
    except (KeyError, IndexError, json.JSONDecodeError) as e:
        return {"error": str(e), "raw": str(data)[:500]}


def validate_extraction(result):
    """Basic validation of extraction result."""
    if "error" in result:
        return False, result.get("error", "unknown error")
    if not isinstance(result.get("entities"), list):
        return False, "missing entities array"
    if not isinstance(result.get("relationships"), list):
        return False, "missing relationships array"
    if not isinstance(result.get("cultural_signals"), dict):
        return False, "missing cultural_signals object"
    # Check entity types
    valid_types = {"People", "Plant", "Process", "Place", "Provision"}
    for ent in result["entities"]:
        if ent.get("type") not in valid_types:
            return False, f"invalid entity type: {ent.get('type')}"
    return True, "ok"


def main():
    parser = argparse.ArgumentParser(description="Cultural graph emergence pass")
    parser.add_argument("--sample", type=int, help="Process first N narratives")
    parser.add_argument("--all", action="store_true", help="Process all narratives")
    parser.add_argument("--dry-run", action="store_true", help="Show prompts, don't call API")
    parser.add_argument("--resume", action="store_true", help="Skip already-processed IDs")
    parser.add_argument("--input", type=str, help="Input CSV path (default: positive observations)")
    parser.add_argument("--id-prefix", type=str, default="PO", help="Narrative ID prefix (default: PO)")
    parser.add_argument("--source-type", type=str, default="positive_observation",
                        help="Source type label (default: positive_observation)")
    args = parser.parse_args()

    if not args.sample and not args.all:
        parser.error("Specify --sample N or --all")

    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key and not args.dry_run:
        print("ERROR: GEMINI_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    input_csv = Path(args.input) if args.input else INPUT_CSV
    system_prompt = load_system_prompt()
    narratives = load_narratives(input_csv, id_prefix=args.id_prefix,
                                 sample=args.sample if not args.all else None)
    print(f"Loaded {len(narratives)} narratives from {input_csv}")

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    output_path = OUTPUT_DIR / f"emergence_{timestamp}.jsonl"

    # Load already-processed IDs for --resume
    done_ids = set()
    if args.resume:
        for p in OUTPUT_DIR.glob("emergence_*.jsonl"):
            with open(p) as f:
                for line in f:
                    try:
                        rec = json.loads(line)
                        done_ids.add(rec.get("narrative_id"))
                    except json.JSONDecodeError:
                        continue
        print(f"Resuming: {len(done_ids)} already processed")

    if args.dry_run:
        # Show first narrative prompt
        row = narratives[0]
        print(f"\n--- System prompt ({len(system_prompt)} chars) ---")
        print(system_prompt[:500] + "...")
        print(f"\n--- User prompt for {row['narrative_id']} ---")
        print(f"Site: {row['R_SiteCode']}")
        print(f"Narrative ({len(row['RE_What'].split())} words):")
        print(row["RE_What"][:300] + "...")
        print(f"\n--- Would process {len(narratives)} narratives ---")
        return

    # Process
    stats = {"ok": 0, "error": 0, "skipped": 0}
    with open(output_path, "w") as out:
        for i, row in enumerate(narratives):
            nid = row["narrative_id"]
            if nid in done_ids:
                stats["skipped"] += 1
                continue

            print(f"[{i+1}/{len(narratives)}] {nid} ({row['R_SiteCode']}) "
                  f"— {len(row['RE_What'].split())} words ... ", end="", flush=True)

            try:
                result = call_gemini(
                    system_prompt, row["RE_What"], row["R_SiteCode"], api_key
                )
                valid, msg = validate_extraction(result)

                record = {
                    "narrative_id": nid,
                    "site_code": row["R_SiteCode"],
                    "source_type": args.source_type,
                    "word_count": len(row["RE_What"].split()),
                    "extraction": result,
                    "valid": valid,
                    "validation_msg": msg,
                    "model": GEMINI_MODEL,
                    "extracted_at": datetime.now(timezone.utc).isoformat(),
                }
                out.write(json.dumps(record) + "\n")
                out.flush()

                if valid:
                    n_ent = len(result.get("entities", []))
                    n_rel = len(result.get("relationships", []))
                    print(f"OK ({n_ent} entities, {n_rel} relationships)")
                    stats["ok"] += 1
                else:
                    print(f"INVALID: {msg}")
                    stats["error"] += 1

            except Exception as e:
                print(f"ERROR: {e}")
                record = {
                    "narrative_id": nid,
                    "site_code": row["R_SiteCode"],
                    "source_type": args.source_type,
                    "word_count": len(row["RE_What"].split()),
                    "extraction": {"error": str(e)},
                    "valid": False,
                    "validation_msg": str(e),
                    "model": GEMINI_MODEL,
                    "extracted_at": datetime.now(timezone.utc).isoformat(),
                }
                out.write(json.dumps(record) + "\n")
                out.flush()
                stats["error"] += 1

            time.sleep(GEMINI_DELAY)

    print(f"\nDone: {stats['ok']} ok, {stats['error']} errors, {stats['skipped']} skipped")
    print(f"Output: {output_path}")


if __name__ == "__main__":
    main()
