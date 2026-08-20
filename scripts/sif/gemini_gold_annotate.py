#!/usr/bin/env python3
"""Annotate gold benchmark with Gemini Pro for independent SIF classification.

Produces mechanism, energy type, and severity quantile annotations
as an independent "expert" classification to compare against Qwen zero-shot.

Persists every 5 events. Resumable.

Usage:
    /usr/bin/python3 scripts/sif/gemini_gold_annotate.py [--dry-run] [--limit 10]
"""

import argparse
import json
import os
import time
from pathlib import Path

INPUT = Path("data/sif/benchmarks/gold_sample_200.json")
OUTPUT = Path("data/sif/benchmarks/gold_gemini_annotations.json")

SYSTEM_PROMPT = """You are a workplace safety expert classifying incident and near-miss narratives for Serious Injury or Fatality (SIF) potential.

SIF potential asks: given the energy present in this event, could it have killed or permanently injured someone — regardless of actual outcome? Define the event as having happened (chance P = 1). Assess the OUTCOME distribution only. Exclude all engineered controls (PPE, barriers, nets) — assess unmitigated energy.

Classify the event:

1. MECHANISM: Choose ONE from: transport, fall, struck, caught_in, explosion, fire, electrical, thermal, chemical, breathing, pressure, structural_collapse, assault, overexertion, slip_no_fall, abrasion, radiation_noise, animal_insect, unknown

2. ENERGY_TYPES: Choose ONE OR MORE from: gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological, none

3. SOURCE_PROPERTIES: What energy source? Extract specific values if mentioned (height, speed, voltage, mass, temperature).

4. SEVERITY_P10: 10th percentile outcome (mild). One of: no_injury, first_aid, medical_treatment, serious_injury, fatality
5. SEVERITY_P50: Median outcome. Same scale.
6. SEVERITY_P90: 90th percentile outcome (severe). Same scale.

7. SIF_POTENTIAL: Is there SIF potential? One of: SIF, ELEVATED, NON_SIF
   - SIF: P50 >= serious_injury, or P90 = fatality
   - ELEVATED: P50 = medical_treatment AND P90 >= serious_injury
   - NON_SIF: P50 <= first_aid AND P90 <= medical_treatment

8. REASONING: One sentence explaining your assessment.

9. CONFIDENCE: How confident are you? One of: high, medium, low
   - high: narrative clearly describes a specific energy event
   - medium: narrative describes a situation but energy details are vague
   - low: narrative is too short, vague, or doesn't describe a physical event

Respond ONLY with valid JSON."""


def annotate_one(narrative: str, api_key: str) -> dict:
    """Call Gemini Pro to annotate one narrative."""
    import requests

    resp = requests.post(
        f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={api_key}",
        json={
            "contents": [
                {"parts": [{"text": f"{SYSTEM_PROMPT}\n\nNarrative:\n{narrative}"}]}
            ],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 2048,
                "responseMimeType": "application/json",
                "thinkingConfig": {"thinkingBudget": 2048},
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()

    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        # Try sourcing from bashrc
        import subprocess
        result = subprocess.run(
            ["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
            capture_output=True, text=True,
        )
        api_key = result.stdout.strip()
    if not api_key:
        print("ERROR: GEMINI_API_KEY not set")
        return

    with open(INPUT) as f:
        events = json.load(f)

    if args.limit:
        events = events[: args.limit]

    # Resume from partial results
    results = []
    if OUTPUT.exists():
        with open(OUTPUT) as f:
            results = json.load(f)
        done_ids = {r["event_id"] for r in results}
        events = [e for e in events if e["event_id"] not in done_ids]
        print(f"Resuming: {len(results)} done, {len(events)} remaining")

    print(f"Events to annotate: {len(events)}")

    if args.dry_run:
        print("DRY RUN — would annotate with Gemini Pro")
        return

    for i, evt in enumerate(events):
        t0 = time.time()
        annotation = annotate_one(evt["narrative"], api_key)
        elapsed = time.time() - t0

        result = {
            "event_id": evt["event_id"],
            "qq_sifp": evt["qq_sifp"],
            "report_type": evt["report_type"],
            "hazard_category": evt["hazard_category"],
            "length_band": evt["length_band"],
            "narrative_preview": evt["narrative"][:100],
            "gemini": annotation,
            "inference_time_s": round(elapsed, 1),
        }
        results.append(result)

        # Persist every 5
        if (i + 1) % 5 == 0 or (i + 1) == len(events):
            with open(OUTPUT, "w") as f:
                json.dump(results, f, indent=2)

        mech = annotation.get("mechanism", annotation.get("MECHANISM", "?"))
        sif = annotation.get("sif_potential", annotation.get("SIF_POTENTIAL", "?"))
        conf = annotation.get("confidence", annotation.get("CONFIDENCE", "?"))
        print(f"  [{len(results)}/{len(results)+len(events)-i-1}] {elapsed:.1f}s  sifp={evt['qq_sifp']:15s}  mech={mech:20s}  sif={sif:10s}  conf={conf}")

        # Rate limit — Gemini free tier is 15 RPM
        if elapsed < 4.0:
            time.sleep(4.0 - elapsed)

    print(f"\nDone. {len(results)} annotations → {OUTPUT}")


if __name__ == "__main__":
    main()
