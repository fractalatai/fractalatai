#!/usr/bin/env python3
"""Production cultural graph inference on full QQ corpus via Ollama.

Reads yearly CSVs (no headers), calls the fine-tuned model via Ollama,
writes graph triple JSONL output. Designed for RunPod GPU.

Usage:
    # On RunPod (after ollama create cultural-graph -f Modelfile):
    python3 -u production_inference.py --year 2024 --workers 4
    python3 -u production_inference.py --all --workers 4
    python3 -u production_inference.py --year 2024 --workers 4 --resume

    # Dry run (no inference):
    python3 -u production_inference.py --year 2024 --dry-run
"""

import argparse
import csv
import json
import os
import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from threading import Lock

import requests

# --- Config ---
OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL_NAME = "cultural-graph"
DATA_DIR = Path("/workspace/cultural-graph/qq-data")
OUTPUT_DIR = Path("/workspace/cultural-graph/production-output")
COLS = ["Id", "Site", "What", "Type", "AtWork", "Action", "FY", "AP", "Sector", "SubSector"]

SYSTEM_INSTRUCTION = (
    "You are a safety culture analyst. Given a workplace safety narrative, "
    "extract entities and cultural relationships.\n\n"
    "Entity types: People, Plant, Process, Place, Provision.\n\n"
    "Cultural relationship types:\n"
    "- shares-information-with: briefing, explaining, informing another person\n"
    "- monitors: watching, checking, reviewing another's work\n"
    "- learns-from: acquiring knowledge from another person or experience\n"
    "- cooperates-with: working together, coordinating, jointly participating\n"
    "- speaks-up-to: raising concerns, challenging decisions, stopping work, suggesting improvements\n"
    "- recognises: acknowledging competence, effort, or good practice\n"
    "- adapts-to: adjusting behaviour or improving a process in response to conditions\n"
    "- responds-to-failure-of: reacting when something goes wrong or is found deficient\n"
    "- normalises: treating a deviation from procedure as acceptable or routine\n"
    "- directs: giving orders or leading activities with authority\n"
    "- cares-for: welfare gestures — looking after someone's wellbeing\n"
    "- protects: proactive safeguarding — designing or maintaining controls that prevent harm\n"
    "- operational: a person performing a task — not an interpersonal cultural relationship\n\n"
    "Return a JSON object with entities and relationships. "
    "Each relationship has source, target, edge_type, and detail fields. "
    "If there are no cultural relationships, return an empty relationships array."
)

VALID_EDGE_TYPES = {
    "shares-information-with", "monitors", "learns-from", "cooperates-with",
    "speaks-up-to", "recognises", "adapts-to", "responds-to-failure-of",
    "normalises", "directs", "cares-for", "protects", "operational",
}


def load_year(year):
    """Load all records for a given year."""
    path = DATA_DIR / f"Redactor_{year}.csv"
    if not path.exists():
        print(f"WARNING: {path} not found", file=sys.stderr)
        return []
    records = []
    with open(path, encoding="cp1252") as f:
        reader = csv.reader(f)
        for row in reader:
            if len(row) < 6:
                continue
            rec = dict(zip(COLS, row + [""] * (len(COLS) - len(row))))
            # Combine What + Action
            text = rec["What"]
            action = rec.get("Action", "").strip()
            if action:
                text = text.rstrip() + "\n\nImmediate action taken: " + action
            rec["narrative_text"] = text
            rec["word_count"] = len(text.split())
            records.append(rec)
    return records


def call_ollama(record):
    """Call Ollama for a single record. Returns (record, result, duration)."""
    user_msg = f'Extract entities and cultural relationships from this narrative:\n\n"{record["narrative_text"]}"'
    payload = {
        "model": MODEL_NAME,
        "messages": [
            {"role": "system", "content": SYSTEM_INSTRUCTION},
            {"role": "user", "content": user_msg},
        ],
        "stream": False,
        "options": {"temperature": 0.1, "num_predict": 4096},
    }
    try:
        resp = requests.post(OLLAMA_URL, json=payload, timeout=300)
        resp.raise_for_status()
        data = resp.json()
        content = data["message"]["content"]
        duration = data.get("eval_duration", 0) / 1e9

        # Strip Qwen3 thinking tags
        content = re.sub(r"<think>[\s\S]*?</think>", "", content).strip()

        result = json.loads(content)
        return record, result, duration, True
    except json.JSONDecodeError as e:
        return record, {"error": f"JSON parse: {e}", "raw": content[:500]}, 0, False
    except Exception as e:
        return record, {"error": str(e)}, 0, False


def process_year(year, records, workers, resume_ids, out_path):
    """Process all records for a year."""
    write_lock = Lock()
    stats = {"ok": 0, "error": 0, "skipped": 0, "total_duration": 0}

    to_process = []
    for rec in records:
        if rec["Id"] in resume_ids:
            stats["skipped"] += 1
            continue
        to_process.append(rec)

    if not to_process:
        print(f"  All {len(records)} records already processed")
        return stats

    print(f"  Processing {len(to_process)} records ({stats['skipped']} skipped)")

    with open(out_path, "a") as out:
        def process_and_write(rec):
            rec, result, duration, valid = call_ollama(rec)
            # Build output record
            output = {
                "id": rec["Id"],
                "site": rec["Site"],
                "report_type": rec["Type"],
                "fy": rec["FY"],
                "sector": rec.get("Sector", ""),
                "sub_sector": rec.get("SubSector", ""),
                "word_count": rec["word_count"],
                "extraction": result,
                "valid": valid,
                "duration": round(duration, 2),
                "extracted_at": datetime.now(timezone.utc).isoformat(),
            }
            with write_lock:
                out.write(json.dumps(output) + "\n")
                out.flush()
            return valid, duration

        if workers > 1:
            with ThreadPoolExecutor(max_workers=workers) as executor:
                futures = {executor.submit(process_and_write, rec): rec for rec in to_process}
                for i, future in enumerate(as_completed(futures), 1):
                    valid, duration = future.result()
                    if valid:
                        stats["ok"] += 1
                        stats["total_duration"] += duration
                    else:
                        stats["error"] += 1
                    if i % 50 == 0:
                        print(f"    [{i}/{len(to_process)}] ok={stats['ok']} err={stats['error']} "
                              f"({stats['total_duration']:.0f}s total inference)")
        else:
            for i, rec in enumerate(to_process, 1):
                valid, duration = process_and_write(rec)
                if valid:
                    stats["ok"] += 1
                    stats["total_duration"] += duration
                else:
                    stats["error"] += 1
                if i % 25 == 0:
                    print(f"    [{i}/{len(to_process)}] ok={stats['ok']} err={stats['error']}")

    return stats


def main():
    parser = argparse.ArgumentParser(description="Production cultural graph inference")
    parser.add_argument("--year", type=str, help="Process a single year (e.g., 2024)")
    parser.add_argument("--all", action="store_true", help="Process all years")
    parser.add_argument("--workers", type=int, default=1, help="Parallel Ollama workers")
    parser.add_argument("--resume", action="store_true", help="Skip already-processed IDs")
    parser.add_argument("--dry-run", action="store_true", help="Profile data, no inference")
    args = parser.parse_args()

    if not args.year and not args.all:
        parser.error("Specify --year YYYY or --all")

    years = [args.year] if args.year else ["2022", "2023", "2024", "2025", "2026"]

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for year in years:
        print(f"\n{'='*60}")
        print(f"Year: {year}")
        records = load_year(year)
        if not records:
            continue
        print(f"  Records: {len(records)}")

        # Report type breakdown
        from collections import Counter
        type_counts = Counter(r["Type"] for r in records)
        for t, c in type_counts.most_common():
            print(f"    {t}: {c}")

        if args.dry_run:
            import statistics
            words = [r["word_count"] for r in records]
            print(f"  Words: min={min(words)}, median={statistics.median(words):.0f}, max={max(words)}")
            continue

        out_path = OUTPUT_DIR / f"cultural-graph-{year}.jsonl"

        # Load resume IDs
        resume_ids = set()
        if args.resume and out_path.exists():
            with open(out_path) as f:
                for line in f:
                    try:
                        resume_ids.add(json.loads(line)["id"])
                    except (json.JSONDecodeError, KeyError):
                        continue
            print(f"  Resume: {len(resume_ids)} already processed")

        stats = process_year(year, records, args.workers, resume_ids, out_path)
        print(f"  Done: {stats['ok']} ok, {stats['error']} errors, {stats['skipped']} skipped")
        if stats["ok"] > 0:
            print(f"  Avg inference: {stats['total_duration']/stats['ok']:.1f}s/narrative")
        print(f"  Output: {out_path}")

    print(f"\n{'='*60}")
    print("ALL YEARS COMPLETE")


if __name__ == "__main__":
    main()
