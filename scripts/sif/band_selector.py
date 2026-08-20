#!/usr/bin/env python3
"""Second-pass band selection from calibration curves.

Given a mechanism (from Qwen pass 1), generates a focused prompt
from the calibration JSON and asks the model to select the appropriate
band and extract magnitude properties.

Prompts are auto-generated from calibration files — always in sync.

Usage:
    # Test on sample with Gemini
    /usr/bin/python3 scripts/sif/band_selector.py --sample 50 --model gemini

    # Full run (when Qwen/RunPod available)
    /usr/bin/python3 scripts/sif/band_selector.py --model qwen
"""

import argparse
import json
import os
import re
import subprocess
import time
from collections import Counter
from pathlib import Path

CAL_DIR = Path("data/sif/calibration")
QWEN_FILE = Path("data/sif/benchmarks/qq_zeroshot_full.json")
OUTPUT_DIR = Path("data/sif/benchmarks")

# Same mapping as evaluate_calibrated.py
MECHANISM_MAP = {
    "transport": "transport", "fall": "fall", "struck": "struck",
    "caught_in": "caught_in", "explosion": "explosion", "fire": "fire",
    "electrical": "electrical", "thermal": "thermal", "chemical": "chemical",
    "breathing": "breathing", "pressure": "pressure",
    "structural_collapse": "structural_collapse", "assault": "assault",
    "overexertion": "overexertion", "slip_no_fall": "slip_no_fall",
    "abrasion": "abrasion", "radiation_noise": "radiation_noise",
    "animal_insect": "animal_insect",
    "collision": "transport", "cut": "struck", "cutting": "struck",
    "mechanical": "caught_in", "contact": "struck", "impact": "struck",
    "biological": "animal_insect", "respiratory": "breathing",
    "inhalation": "breathing", "exposure": "chemical",
}

MECHANISM_TO_FILE = {
    "transport": "motion_transport.json", "fall": "falls_gravity.json",
    "struck": "motion_struck.json", "caught_in": "caught_in.json",
    "explosion": "explosion.json", "fire": "fire.json",
    "electrical": "electrical.json", "thermal": "thermal.json",
    "chemical": "chemical.json", "breathing": "breathing.json",
    "pressure": "pressure.json", "structural_collapse": "structural_collapse.json",
    "assault": "assault.json", "overexertion": "overexertion.json",
    "slip_no_fall": "slip_no_fall.json", "abrasion": "abrasion.json",
    "radiation_noise": "radiation_noise.json", "animal_insect": "animal_insect.json",
}


def load_cal(mechanism):
    fname = MECHANISM_TO_FILE.get(mechanism)
    if not fname:
        return None
    fpath = CAL_DIR / fname
    if not fpath.exists():
        return None
    with open(fpath) as f:
        return json.load(f)


def build_prompt(mechanism, cal_data, narrative):
    """Generate a band-selection prompt from the calibration JSON."""
    bands = cal_data["magnitude_bands"]

    band_list = []
    for i, b in enumerate(bands):
        band_list.append(f'{i+1}. {b["band"]}: {b["description"]}')

    band_text = "\n".join(band_list)

    # Collect magnitude properties that might be extractable
    props = set()
    for b in bands:
        for key in b:
            if key.endswith("_low") or key.endswith("_high"):
                base = key.rsplit("_", 1)[0]
                props.add(base)
    props.discard("height_m")  # will be asked explicitly per type

    prompt = f"""You are classifying a workplace incident narrative. The primary mechanism has been identified as: {mechanism}

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

    return prompt


def call_gemini(prompt, api_key):
    """Call Gemini 2.5 Flash for band selection."""
    import requests

    resp = requests.post(
        f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={api_key}",
        json={
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.0,
                "maxOutputTokens": 1024,
                "responseMimeType": "application/json",
                "thinkingConfig": {"thinkingBudget": 1024},
            },
        },
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    try:
        text = data["candidates"][0]["content"]["parts"][-1]["text"]
        return json.loads(text)
    except (KeyError, IndexError, json.JSONDecodeError) as e:
        return {"error": str(e), "raw": str(data)[:300]}


def get_narrative(event_id, qwen_events, qq_data=None):
    """Get the full narrative for an event. Try QQ DuckDB first, fall back to preview."""
    # Try DuckDB
    try:
        import duckdb
        con = duckdb.connect("data/sif.duckdb", read_only=True)
        row = con.execute(
            "SELECT narrative FROM events WHERE event_id = ?", [event_id]
        ).fetchone()
        con.close()
        if row and row[0]:
            return row[0]
    except Exception:
        pass

    # Fall back to Qwen preview
    for evt in qwen_events:
        if evt["event_id"] == event_id:
            return evt.get("narrative_preview", "")
    return ""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sample", type=int, default=50, help="Number of events to process")
    parser.add_argument("--model", default="gemini", choices=["gemini", "qwen"])
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--output", default="band_selection_results.json")
    args = parser.parse_args()

    # Load Qwen results
    with open(QWEN_FILE) as f:
        qwen_events = json.load(f)

    # Get API key for Gemini
    api_key = None
    if args.model == "gemini":
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            result = subprocess.run(
                ["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
                capture_output=True, text=True,
            )
            api_key = result.stdout.strip()
        if not api_key:
            print("ERROR: GEMINI_API_KEY not set")
            return

    # Stratified sample: pick events across mechanisms
    by_mech = {}
    for evt in qwen_events:
        raw_mech = evt["prediction"].get("mechanism", "unknown")
        mech = MECHANISM_MAP.get(raw_mech)
        if mech and mech in MECHANISM_TO_FILE:
            by_mech.setdefault(mech, []).append(evt)

    sample = []
    # Take proportional sample, at least 1 per mechanism
    mechs_sorted = sorted(by_mech.keys(), key=lambda m: len(by_mech[m]), reverse=True)
    per_mech = max(1, args.sample // len(mechs_sorted))
    for mech in mechs_sorted:
        events = by_mech[mech]
        take = min(per_mech, len(events))
        # Mix of SIFp and Not SIFp
        sif_events = [e for e in events if e["qq_sifp"] != "9 Not SIFp"]
        non_sif = [e for e in events if e["qq_sifp"] == "9 Not SIFp"]
        take_sif = min(take // 2, len(sif_events))
        take_non = min(take - take_sif, len(non_sif))
        sample.extend(sif_events[:take_sif])
        sample.extend(non_sif[:take_non])
        if len(sample) >= args.sample:
            break

    sample = sample[:args.sample]
    print(f"Sample: {len(sample)} events across {len(set(MECHANISM_MAP.get(e['prediction'].get('mechanism',''),'?') for e in sample))} mechanisms")

    if args.dry_run:
        # Show one example prompt per mechanism
        seen = set()
        for evt in sample:
            raw_mech = evt["prediction"].get("mechanism", "unknown")
            mech = MECHANISM_MAP.get(raw_mech)
            if mech in seen:
                continue
            seen.add(mech)
            cal = load_cal(mech)
            if not cal:
                continue
            narrative = get_narrative(evt["event_id"], qwen_events)
            prompt = build_prompt(mech, cal, narrative[:300])
            print(f"\n{'='*70}")
            print(f"MECHANISM: {mech}")
            print(f"{'='*70}")
            print(prompt[:500])
            print("...")
            if len(seen) >= 3:
                break
        return

    # Process
    results = []
    output_path = OUTPUT_DIR / args.output

    # Resume from partial
    if output_path.exists():
        with open(output_path) as f:
            results = json.load(f)
        done_ids = {r["event_id"] for r in results}
        sample = [e for e in sample if e["event_id"] not in done_ids]
        print(f"Resuming: {len(results)} done, {len(sample)} remaining")

    for i, evt in enumerate(sample):
        raw_mech = evt["prediction"].get("mechanism", "unknown")
        mech = MECHANISM_MAP.get(raw_mech)
        cal = load_cal(mech)
        if not cal:
            continue

        narrative = get_narrative(evt["event_id"], qwen_events)
        if not narrative or len(narrative) < 10:
            narrative = evt.get("narrative_preview", "No narrative available")

        prompt = build_prompt(mech, cal, narrative)

        t0 = time.time()
        if args.model == "gemini":
            response = call_gemini(prompt, api_key)
        else:
            print("Qwen mode not implemented yet — use --model gemini")
            return
        elapsed = time.time() - t0

        band_name = response.get("band_name", cal["magnitude_bands"][0]["band"])
        confidence = response.get("confidence", "low")

        result = {
            "event_id": evt["event_id"],
            "qq_sifp": evt["qq_sifp"],
            "qq_hazard": evt.get("qq_hazard", ""),
            "mechanism_raw": raw_mech,
            "mechanism_mapped": mech,
            "band_selected": band_name,
            "band_confidence": confidence,
            "extracted_values": response.get("extracted_values", {}),
            "reasoning": response.get("reasoning", ""),
            "inference_time_s": round(elapsed, 1),
        }
        results.append(result)

        # Persist every 5
        if (i + 1) % 5 == 0 or (i + 1) == len(sample):
            with open(output_path, "w") as f:
                json.dump(results, f, indent=2)

        print(f"  [{len(results)}] {elapsed:.1f}s  {mech:20s} → {band_name:25s}  conf={confidence:6s}  sifp={evt['qq_sifp']}")

        # Rate limit for Gemini free tier
        if args.model == "gemini" and elapsed < 4.0:
            time.sleep(4.0 - elapsed)

    print(f"\nDone. {len(results)} results → {output_path}")

    # Quick summary
    band_counts = Counter(r["band_selected"] for r in results)
    conf_counts = Counter(r["band_confidence"] for r in results)
    print(f"\nBand distribution:")
    for b, n in band_counts.most_common(10):
        print(f"  {b:30s} {n:>4}")
    print(f"\nConfidence: {dict(conf_counts)}")


if __name__ == "__main__":
    main()
