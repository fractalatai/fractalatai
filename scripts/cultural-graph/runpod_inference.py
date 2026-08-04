#!/usr/bin/env python3
"""Cultural graph inference on RunPod from cleaned JSONL.

Reads JSONL from ingest_qa.py, calls cultural-graph model via Ollama,
writes results as JSONL. Designed for RunPod GPU with 4 parallel workers.

Usage:
    # On RunPod:
    python3 -u runpod_inference.py --input /workspace/cultural-graph/ingest/Redactor_2027-clean.jsonl --workers 4
    python3 -u runpod_inference.py --input /workspace/cultural-graph/ingest/Redactor_2027-clean.jsonl --workers 4 --resume
"""

import argparse
import json
import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from threading import Lock

import requests

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL_NAME = "cultural-graph"
OUTPUT_DIR = Path("/workspace/cultural-graph/results")

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


def load_input(path):
    """Load cleaned JSONL from ingest_qa.py."""
    records = []
    with open(path) as f:
        for line in f:
            records.append(json.loads(line))
    return records


def call_ollama(record):
    """Call Ollama for a single record."""
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
        content = re.sub(r"<think>[\s\S]*?</think>", "", content).strip()
        result = json.loads(content)
        return record, result, duration, True
    except json.JSONDecodeError as e:
        return record, {"error": f"JSON parse: {e}", "raw": content[:500]}, 0, False
    except Exception as e:
        return record, {"error": str(e)}, 0, False


def main():
    parser = argparse.ArgumentParser(description="Cultural graph RunPod inference")
    parser.add_argument("--input", required=True, help="Cleaned JSONL from ingest_qa.py")
    parser.add_argument("--output", help="Output JSONL path (default: auto-named in results/)")
    parser.add_argument("--workers", type=int, default=1, help="Parallel Ollama workers")
    parser.add_argument("--resume", action="store_true", help="Skip already-processed IDs")
    args = parser.parse_args()

    input_path = Path(args.input)
    records = load_input(input_path)
    print(f"Loaded {len(records)} records from {input_path}")

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    if args.output:
        out_path = Path(args.output)
    else:
        out_path = OUTPUT_DIR / f"{input_path.stem}-results.jsonl"

    # Resume support
    done_ids = set()
    if args.resume and out_path.exists():
        with open(out_path) as f:
            for line in f:
                try:
                    done_ids.add(json.loads(line)["id"])
                except (json.JSONDecodeError, KeyError):
                    continue
        print(f"Resume: {len(done_ids)} already processed")

    to_process = [r for r in records if r["id"] not in done_ids]
    if not to_process:
        print("All records already processed")
        return

    print(f"Processing {len(to_process)} records ({len(done_ids)} skipped)")

    write_lock = Lock()
    stats = {"ok": 0, "error": 0, "total_duration": 0}

    with open(out_path, "a") as out:
        def process_one(rec):
            rec, result, duration, valid = call_ollama(rec)
            output = {
                "id": rec["id"],
                "site": rec["site"],
                "report_type": rec["report_type"],
                "fy": rec.get("fy", ""),
                "sector": rec.get("sector", ""),
                "sub_sector": rec.get("sub_sector", ""),
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

        if args.workers > 1:
            with ThreadPoolExecutor(max_workers=args.workers) as executor:
                futures = {executor.submit(process_one, rec): rec for rec in to_process}
                for i, future in enumerate(as_completed(futures), 1):
                    valid, duration = future.result()
                    if valid:
                        stats["ok"] += 1
                        stats["total_duration"] += duration
                    else:
                        stats["error"] += 1
                    if i % 50 == 0:
                        print(f"  [{i}/{len(to_process)}] ok={stats['ok']} err={stats['error']}")
        else:
            for i, rec in enumerate(to_process, 1):
                valid, duration = process_one(rec)
                if valid:
                    stats["ok"] += 1
                    stats["total_duration"] += duration
                else:
                    stats["error"] += 1
                if i % 25 == 0:
                    print(f"  [{i}/{len(to_process)}] ok={stats['ok']} err={stats['error']}")

    print(f"\nDone: {stats['ok']} ok, {stats['error']} errors")
    if stats["ok"] > 0:
        print(f"Avg inference: {stats['total_duration']/stats['ok']:.1f}s/narrative")
    print(f"Output: {out_path}")


if __name__ == "__main__":
    main()
