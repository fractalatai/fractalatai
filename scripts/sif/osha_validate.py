#!/usr/bin/env python3
"""Validate calibration curves against OSHA ITA Case Detail data.

Extracts magnitudes from OSHA narratives using the same band_selector
prompt pattern, then cross-tabs against OSHA outcome codes to compare
empirical vs literature-based severity distributions.

Two modes:
  --extract   Run Qwen band selection on OSHA narratives (needs RunPod)
  --analyse   Cross-tab extracted bands vs outcomes (no GPU needed)

Usage (on RunPod):
    python3 scripts/sif/osha_validate.py --extract --model qwen

Usage (local, after extraction):
    /usr/bin/python3 scripts/sif/osha_validate.py --analyse
"""

import argparse
import csv
import json
import math
import os
import random
import time
import zipfile
from collections import Counter
from pathlib import Path

# --- Paths ---
# Local paths (for --sample and --analyse)
OSHA_ZIP = Path("data/sif/sources/osha/ITA_Case_Detail_2024.zip")
CAL_DIR = Path("data/sif/calibration")
OUTPUT_DIR = Path("data/sif/benchmarks/osha-validation")
# RunPod paths (for --extract) — override via env or args
RUNPOD_CAL_DIR = Path("/workspace/sif/data/calibration")
RUNPOD_OUTPUT_DIR = Path("/workspace/sif/output/osha-validation")

# OSHA event code → our mechanism label
OSHA_TO_MECHANISM = {
    41: "fall",
    51: "electrical",
    64: "struck",
    65: "caught_in",
    25: "transport",
    26: "transport",
    27: "transport",
    31: "explosion",
    32: "fire",
    53: "thermal",
    55: "chemical",
}

# OSHA outcome codes
OUTCOME_LABELS = {1: "fatality", 2: "hospitalisation", 3: "amputation", 4: "loss_of_eye"}

# Sample sizes per mechanism for extraction
SAMPLE_PER_MECHANISM = 500

MECHANISM_TO_FILE = {
    "transport": "motion_transport.json", "fall": "falls_gravity.json",
    "struck": "motion_struck.json", "caught_in": "caught_in.json",
    "explosion": "explosion.json", "fire": "fire.json",
    "electrical": "electrical.json", "thermal": "thermal.json",
    "chemical": "chemical.json",
}


def load_cal(mechanism, cal_dir=None):
    if cal_dir is None:
        cal_dir = CAL_DIR
    fname = MECHANISM_TO_FILE.get(mechanism)
    if not fname:
        return None
    fpath = cal_dir / fname
    if not fpath.exists():
        return None
    with open(fpath) as f:
        return json.load(f)


def build_prompt(mechanism, cal_data, narrative):
    """Same prompt pattern as band_selector.py."""
    bands = cal_data["magnitude_bands"]
    band_list = []
    for i, b in enumerate(bands):
        band_list.append(f'{i+1}. {b["band"]}: {b["description"]}')
    band_text = "\n".join(band_list)

    return f"""You are classifying a workplace incident narrative. The primary mechanism has been identified as: {mechanism}

Given this narrative, select the most appropriate severity band and extract any magnitude properties mentioned.

NARRATIVE:
{narrative}

AVAILABLE BANDS (ordered least severe to most severe):
{band_text}

RULES:
- Select the band that best matches the energy level described in the narrative
- If the narrative does not contain enough detail to determine the magnitude, select band 1 (least severe)
- Extract any specific magnitude values mentioned (heights, speeds, voltages, temperatures, etc.)
- If a value is not mentioned or cannot be inferred, set it to null

Respond with valid JSON only:
{{
  "band_number": <1-{len(bands)}>,
  "band_name": "<band name from list above>",
  "confidence": "<high|medium|low>",
  "extracted_values": {{
    "height_m": <number or null>,
    "voltage_v": <number or null>,
    "speed_kmh": <number or null>,
    "temperature_c": <number or null>,
    "mass_kg": <number or null>,
    "other": "<any other relevant magnitude>"
  }},
  "reasoning": "<one sentence>"
}}"""


def call_qwen(prompt, ollama_url="http://localhost:11434/api/chat", model="qwen3:8b"):
    import requests
    resp = requests.post(
        ollama_url,
        json={
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": False,
            "options": {"temperature": 0.0, "num_predict": 500},
            "format": "json",
            "think": False,
        },
        timeout=120,
    )
    resp.raise_for_status()
    content = resp.json()["message"]["content"]
    try:
        return json.loads(content)
    except json.JSONDecodeError:
        return {"error": "invalid_json", "raw": content[:500]}


def load_osha_sample():
    """Load stratified sample from OSHA CSV."""
    import duckdb
    con = duckdb.connect()

    # Extract ZIP if needed
    if OSHA_ZIP.exists():
        with zipfile.ZipFile(OSHA_ZIP) as z:
            csv_name = z.namelist()[0]
            csv_path = Path("/tmp/osha") / csv_name
            if not csv_path.exists():
                z.extract(csv_name, "/tmp/osha")
    else:
        csv_path = Path("/tmp/osha/ITA_Case_Detail_Data_2024_through_12-31-2025.csv")

    events_by_mech = {}
    for osha_code, mechanism in OSHA_TO_MECHANISM.items():
        rows = con.execute(f"""
            SELECT id, event_code_pred, incident_outcome,
                   NEW_NAR_WHAT_HAPPENED, NEW_NAR_BEFORE_INCIDENT,
                   NEW_NAR_INJURY_ILLNESS, NEW_INCIDENT_DESCRIPTION,
                   nature_title_pred, part_title_pred
            FROM read_csv('{csv_path}', auto_detect=true, sample_size=10000, ignore_errors=true)
            WHERE event_code_pred = {osha_code}
              AND NEW_NAR_WHAT_HAPPENED IS NOT NULL
              AND LENGTH(CAST(NEW_NAR_WHAT_HAPPENED AS VARCHAR)) > 20
            ORDER BY RANDOM()
            LIMIT {SAMPLE_PER_MECHANISM}
        """).fetchall()

        for row in rows:
            # Combine narrative fields
            narrative_parts = [p for p in [row[3], row[4], row[5], row[6]] if p]
            narrative = " ".join(narrative_parts)

            evt = {
                "osha_id": str(row[0]),
                "event_code": osha_code,
                "mechanism": mechanism,
                "outcome": row[2],
                "outcome_label": OUTCOME_LABELS.get(row[2], f"unknown_{row[2]}"),
                "narrative": narrative,
                "nature": row[7] or "",
                "body_part": row[8] or "",
            }
            events_by_mech.setdefault(mechanism, []).append(evt)

    con.close()

    # Flatten
    all_events = []
    for mech, evts in events_by_mech.items():
        all_events.extend(evts)
        print(f"  {mech:20s}: {len(evts)} events")

    print(f"  Total: {len(all_events)} events")
    return all_events


def sample(args):
    """Extract stratified OSHA sample locally (no GPU). Saves JSON for RunPod."""
    print("Extracting OSHA sample...")
    events = load_osha_sample()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    sample_path = OUTPUT_DIR / "osha_sample.json"
    with open(sample_path, "w") as f:
        json.dump(events, f)
    print(f"Sample: {len(events)} events → {sample_path}")
    print("Copy to RunPod: scp ... osha_sample.json root@<pod>:/workspace/sif/output/osha-validation/")


def extract(args):
    """Run Qwen band selection on OSHA narratives (RunPod)."""
    # Use RunPod paths
    cal_dir = RUNPOD_CAL_DIR if RUNPOD_CAL_DIR.exists() else CAL_DIR
    output_dir = RUNPOD_OUTPUT_DIR if RUNPOD_OUTPUT_DIR.parent.exists() else OUTPUT_DIR
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load sample from pre-extracted JSON
    sample_path = output_dir / "osha_sample.json"
    if not sample_path.exists():
        print(f"ERROR: {sample_path} not found. Run --sample locally first, then copy to RunPod.")
        return
    with open(sample_path) as f:
        events = json.load(f)
    print(f"Loaded {len(events)} events from {sample_path}")

    output_path = output_dir / "osha_band_selection.json"

    # Resume from partial
    results = []
    if output_path.exists():
        with open(output_path) as f:
            results = json.load(f)
        done_ids = {r["osha_id"] for r in results}
        events = [e for e in events if e["osha_id"] not in done_ids]
        print(f"Resuming: {len(results)} done, {len(events)} remaining")

    print(f"Extracting bands for {len(events)} events...")

    print(f"GPU: check VRAM with nvidia-smi — Qwen 8B Q4 needs ~5GB")
    print()

    for i, evt in enumerate(events):
        mech = evt["mechanism"]
        cal = load_cal(mech, cal_dir)
        if not cal:
            continue

        prompt = build_prompt(mech, cal, evt["narrative"][:1500])

        t0 = time.time()
        response = call_qwen(prompt, ollama_url=args.ollama_url, model=args.qwen_model)
        elapsed = time.time() - t0

        # Validate band
        valid_bands = [b["band"] for b in cal["magnitude_bands"]]
        band_name = response.get("band_name", "")
        if band_name not in valid_bands:
            band_num = response.get("band_number")
            if isinstance(band_num, int) and 1 <= band_num <= len(valid_bands):
                band_name = valid_bands[band_num - 1]
            else:
                band_name = valid_bands[0]

        result = {
            "osha_id": evt["osha_id"],
            "event_code": evt["event_code"],
            "mechanism": mech,
            "outcome": evt["outcome"],
            "outcome_label": evt["outcome_label"],
            "band_selected": band_name,
            "band_confidence": response.get("confidence", "low"),
            "extracted_values": response.get("extracted_values", {}),
            "reasoning": response.get("reasoning", ""),
            "nature": evt["nature"],
            "body_part": evt["body_part"],
            "inference_time_s": round(elapsed, 1),
        }
        results.append(result)

        # Persist every 10
        if (i + 1) % 10 == 0 or (i + 1) == len(events):
            with open(output_path, "w") as f:
                json.dump(results, f)

        if (i + 1) % 50 == 0:
            print(f"  [{len(results)}] {elapsed:.1f}s  {mech:15s} → {band_name:25s}  outcome={evt['outcome_label']}")

    print(f"\nDone. {len(results)} results → {output_path}")


# --- Metalog P(SIF) ---

def logit(y):
    return math.log(y / (1.0 - y))

def bounded_transform(x, lb=0.0, ub=1.0):
    return math.log((x - lb) / (ub - x))

def spt_fit(p10, p50, p90):
    l90 = logit(0.9)
    return p50, (p90 - p10) / (2 * l90), (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)

def metalog_q(y, a1, a2, a3):
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l

def p_sif_bounded(p10, p50, p90, threshold=0.10):
    if abs(p10 - p90) < 1e-6:
        return 1.0 if p50 >= threshold else 0.0
    z10, z50, z90 = bounded_transform(p10), bounded_transform(p50), bounded_transform(p90)
    a1, a2, a3 = spt_fit(z10, z50, z90)
    z_thresh = bounded_transform(threshold)
    if metalog_q(0.999, a1, a2, a3) < z_thresh:
        return 0.0
    if metalog_q(0.001, a1, a2, a3) >= z_thresh:
        return 1.0
    lo, hi = 0.001, 0.999
    for _ in range(50):
        mid = (lo + hi) / 2.0
        if metalog_q(mid, a1, a2, a3) < z_thresh:
            lo = mid
        else:
            hi = mid
    return 1.0 - (lo + hi) / 2.0


def analyse(args):
    """Cross-tab extracted bands vs OSHA outcomes."""
    # Try RunPod output first, then local
    input_path = OUTPUT_DIR / "osha_band_selection.json"
    if not input_path.exists():
        input_path = RUNPOD_OUTPUT_DIR / "osha_band_selection.json"
    if not input_path.exists():
        print(f"ERROR: osha_band_selection.json not found in {OUTPUT_DIR} or {RUNPOD_OUTPUT_DIR}")
        print("Run --extract on RunPod first, then copy results back.")
        return

    with open(input_path) as f:
        results = json.load(f)

    print(f"Loaded {len(results)} OSHA events with band selection")
    print()

    # Load calibration for P(SIF) lookup
    cal_cache = {}
    for mech, fname in MECHANISM_TO_FILE.items():
        cal = load_cal(mech)
        if cal:
            cal_cache[mech] = cal

    # === Cross-tab by mechanism ===
    print("=" * 90)
    print("EMPIRICAL OUTCOME DISTRIBUTION BY MECHANISM × BAND")
    print("=" * 90)

    for mech in sorted(set(r["mechanism"] for r in results)):
        mech_results = [r for r in results if r["mechanism"] == mech]
        cal = cal_cache.get(mech)
        if not cal:
            continue

        print(f"\n--- {mech} ({len(mech_results)} events) ---")
        print(f"{'Band':25s} {'n':>5} {'Fatal':>6} {'Hosp':>6} {'Amput':>6} {'Eye':>6}  {'Fatal%':>7}  {'Cal P(SIF)':>10}")

        band_counts = Counter(r["band_selected"] for r in mech_results)
        for band_name in [b["band"] for b in cal["magnitude_bands"]]:
            subset = [r for r in mech_results if r["band_selected"] == band_name]
            if not subset:
                continue
            n = len(subset)
            fatal = sum(1 for r in subset if r["outcome"] == 1)
            hosp = sum(1 for r in subset if r["outcome"] == 2)
            amput = sum(1 for r in subset if r["outcome"] == 3)
            eye = sum(1 for r in subset if r["outcome"] == 4)
            fatal_pct = 100 * fatal / n if n > 0 else 0

            # Get calibrated P(SIF) for this band
            band_data = next((b for b in cal["magnitude_bands"] if b["band"] == band_name), None)
            if band_data:
                cal_psif = p_sif_bounded(band_data["p10"], band_data["p50"], band_data["p90"])
                cal_str = f"{cal_psif:.3f}"
            else:
                cal_str = "?"

            print(f"{band_name:25s} {n:>5} {fatal:>6} {hosp:>6} {amput:>6} {eye:>6}  {fatal_pct:>6.1f}%  {cal_str:>10}")

    # === Summary: does severity increase with band? ===
    print()
    print("=" * 90)
    print("VALIDATION: Does fatality rate increase with band severity?")
    print("=" * 90)

    for mech in ["fall", "electrical", "struck", "transport", "caught_in"]:
        mech_results = [r for r in results if r["mechanism"] == mech]
        cal = cal_cache.get(mech)
        if not cal or not mech_results:
            continue

        print(f"\n  {mech}:")
        prev_fatal_rate = -1
        monotonic = True
        for band_data in cal["magnitude_bands"]:
            band_name = band_data["band"]
            subset = [r for r in mech_results if r["band_selected"] == band_name]
            if len(subset) < 5:
                continue
            fatal_rate = sum(1 for r in subset if r["outcome"] == 1) / len(subset)
            hosp_rate = sum(1 for r in subset if r["outcome"] == 2) / len(subset)
            arrow = "↑" if fatal_rate >= prev_fatal_rate else "↓"
            if fatal_rate < prev_fatal_rate and prev_fatal_rate > 0:
                monotonic = False
            prev_fatal_rate = fatal_rate
            print(f"    {band_name:25s}  n={len(subset):>4}  fatal={100*fatal_rate:>5.1f}%  hosp={100*hosp_rate:>5.1f}%  {arrow}")

        print(f"    Monotonic severity increase: {'YES' if monotonic else 'NO — investigate'}")

    # === Confidence distribution ===
    print()
    conf_counts = Counter(r["band_confidence"] for r in results)
    print(f"Confidence: {dict(conf_counts)}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sample", action="store_true", help="Extract OSHA sample locally (no GPU)")
    parser.add_argument("--extract", action="store_true", help="Run Qwen band extraction on RunPod")
    parser.add_argument("--analyse", action="store_true", help="Cross-tab bands vs outcomes (no GPU)")
    parser.add_argument("--qwen-model", default="qwen3:8b")
    parser.add_argument("--ollama-url", default="http://localhost:11434/api/chat")
    args = parser.parse_args()

    if not args.sample and not args.extract and not args.analyse:
        args.analyse = True

    if args.sample:
        sample(args)
    if args.extract:
        extract(args)
    if args.analyse:
        analyse(args)


if __name__ == "__main__":
    main()
