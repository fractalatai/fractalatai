#!/usr/bin/env python3
"""Zero-shot SIF classification via Ollama — single-model approach.

Tests whether a base model (Qwen 3 8B) can do mechanism + energy analysis
in one pass from narrative text alone, without fine-tuning.

Usage (on RunPod):
    python3 /workspace/sif/scripts/zeroshot_sif.py --model qwen3:8b
    python3 /workspace/sif/scripts/zeroshot_sif.py --model qwen3:4b
"""

import argparse
import json
import time
from pathlib import Path

import requests

OLLAMA_URL = "http://localhost:11434/api/chat"
DATA_FILE = "/workspace/sif/data/qq_zeroshot_sample.json"
OUTPUT_BASE = Path("/workspace/sif/output")

SYSTEM_PROMPT = """You are a workplace safety classifier. Given an incident or near-miss narrative, analyse it for Serious Injury or Fatality (SIF) potential.

SIF potential asks: given the energy present in this event, could it have killed or permanently injured someone — regardless of the actual outcome? A near-miss where a scaffold board fell 20m but hit nobody has the same SIF potential as if it had hit someone.

Classify the event by answering ALL of the following:

1. MECHANISM: What physical process caused or could have caused harm? Choose ONE:
   transport, fall, struck, caught_in, explosion, fire, electrical, thermal, chemical, breathing, pressure, structural_collapse, assault, overexertion, slip_no_fall, abrasion, radiation_noise, animal_insect

2. ENERGY TYPE: What type of energy was involved? Choose ONE OR MORE:
   gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological

3. SOURCE PROPERTIES: What energy source was present? Extract specific values if mentioned (height in metres, speed, voltage, mass, temperature, concentration).

4. CARRIER/ENVIRONMENT: What was the energy carrier (sharp object, rotating shaft, vehicle) and the default receiving environment (concrete floor, rebar, water)?

5. BODY VULNERABILITY: If a body part is mentioned, which region? (head, neck, spine, torso, limbs, or unknown)

6. SEVERITY QUANTILES: If this event's energy reached a person with NO engineered controls (no PPE, no barriers, no safety nets), estimate the distribution of outcomes:
   - P10 (10th percentile — mild outcome): one of no_injury, first_aid, medical_treatment, serious_injury, fatality
   - P50 (median — most likely outcome): same scale
   - P90 (90th percentile — severe outcome): same scale

7. REASONING: One sentence explaining your severity assessment.

Respond ONLY with valid JSON in this exact format:
{
  "mechanism": "...",
  "energy_types": ["..."],
  "source_properties": "...",
  "carrier_environment": "...",
  "body_vulnerability": "...",
  "severity_p10": "...",
  "severity_p50": "...",
  "severity_p90": "...",
  "reasoning": "..."
}"""


def classify_event(narrative: str, model: str) -> dict:
    """Send a narrative to Ollama and parse the JSON response."""
    resp = requests.post(
        OLLAMA_URL,
        json={
            "model": model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": narrative},
            ],
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="qwen3:8b")
    parser.add_argument("--input", default=DATA_FILE)
    args = parser.parse_args()

    run_name = args.model.replace(":", "-")
    output_dir = OUTPUT_BASE / f"zeroshot-{run_name}"
    output_dir.mkdir(parents=True, exist_ok=True)

    with open(args.input) as f:
        events = json.load(f)

    print(f"Model: {args.model}")
    print(f"Events: {len(events)}")
    print(f"Output: {output_dir}")
    print()

    results = []
    output_path = output_dir / "predictions.json"
    t0 = time.time()

    # Resume from partial results if they exist
    if output_path.exists():
        with open(output_path) as f:
            results = json.load(f)
        done_ids = {r["event_id"] for r in results}
        events = [e for e in events if e["event_id"] not in done_ids]
        print(f"Resuming: {len(results)} done, {len(events)} remaining")

    for i, evt in enumerate(events):
        narrative = evt["narrative"]
        t_start = time.time()
        prediction = classify_event(narrative, args.model)
        elapsed = time.time() - t_start

        result = {
            "event_id": evt["event_id"],
            "qq_sifp": evt["qq_sifp"],
            "qq_hazard": evt.get("hazard_category", ""),
            "report_type": evt["report_type"],
            "narrative_preview": narrative[:100],
            "prediction": prediction,
            "inference_time_s": round(elapsed, 1),
        }
        results.append(result)

        # Persist every 10 events
        if (i + 1) % 10 == 0 or (i + 1) == len(events):
            with open(output_path, "w") as f:
                json.dump(results, f)

        # Print progress
        mech = prediction.get("mechanism", "?")
        p50 = prediction.get("severity_p50", "?")
        sifp = evt["qq_sifp"]
        total_done = len(results)
        print(f"  [{total_done}] {elapsed:.1f}s  sifp={sifp:15s}  mech={mech:20s}  p50={p50:20s}  {narrative[:60]}...")

    total_time = time.time() - t0
    print(f"\nTotal: {total_time:.0f}s ({total_time/len(events):.1f}s/event)")

    # Save results
    output_path = output_dir / "predictions.json"
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"→ {output_path}")

    # Summary
    from collections import Counter
    mechs = Counter(r["prediction"].get("mechanism", "error") for r in results)
    p50s = Counter(r["prediction"].get("severity_p50", "error") for r in results)
    errors = sum(1 for r in results if "error" in r["prediction"])

    print(f"\n=== Summary ===")
    print(f"Errors: {errors}/{len(results)}")
    print(f"\nMechanism distribution:")
    for m, n in mechs.most_common():
        print(f"  {m:25s} {n:>3}")
    print(f"\nP50 severity distribution:")
    for s, n in p50s.most_common():
        print(f"  {s:25s} {n:>3}")

    # Cross-tab: QQ SIFp vs model P50
    print(f"\n=== QQ SIFp vs Model P50 ===")
    for sifp in ["1 Fatal", "2 Massive", "3 Very High", "4 High", "9 Not SIFp"]:
        subset = [r for r in results if r["qq_sifp"] == sifp]
        if not subset:
            continue
        p50_dist = Counter(r["prediction"].get("severity_p50", "?") for r in subset)
        top = ", ".join(f"{s}:{n}" for s, n in p50_dist.most_common(3))
        print(f"  {sifp:15s} (n={len(subset):>2}): {top}")


if __name__ == "__main__":
    main()
