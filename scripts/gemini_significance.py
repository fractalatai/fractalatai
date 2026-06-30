#!/usr/bin/env python3
"""Rate significance of Obligation provisions via Gemini Flash.

Queries unique Obligation provisions from benchmark laws, sends to Gemini
for 4-dimension significance rating, stores results.

Usage:
    source ~/.bashrc
    /usr/bin/python3 scripts/gemini_significance.py --dry-run
    /usr/bin/python3 scripts/gemini_significance.py --limit 10
    /usr/bin/python3 scripts/gemini_significance.py
"""

import argparse
import json
import os
import sys
import time
from collections import Counter

import psycopg2
import requests

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
GEMINI_MODEL = "gemini-2.5-flash"

PROMPT = """\
You are rating the significance of a UK statutory provision that creates a legal obligation.

Rate on 4 dimensions, each HIGH / MEDIUM / LOW:

1. **scope** — breadth of who is affected.
   - HIGH: universal duty-bearer AND universal protected class ("every employer... all employees")
   - MEDIUM: categorical ("an employer who operates...") or one side is broad
   - LOW: individual/specific ("the person", "an inspector", "the authority")

2. **gravity** — what is at stake if the duty is breached.
   - HIGH: health, safety, life, welfare, serious environmental harm
   - MEDIUM: property, financial loss, moderate environmental impact
   - LOW: administrative, procedural, record-keeping, notification

3. **strength** — how absolute is the obligation.
   - HIGH: absolute duty ("shall ensure", "must provide", "shall maintain") with no qualification
   - MEDIUM: qualified duty ("so far as is reasonably practicable", "shall have regard to") or discretionary element
   - LOW: procedural obligation ("shall notify", "shall keep records", "shall display")

4. **hierarchy** — structural importance within the law.
   - HIGH: general duties section (Part I, reg.3-5), primary obligations
   - MEDIUM: specific regulations, named duties
   - LOW: sub-paragraphs, schedules, transitional provisions, procedural annexes

Respond with ONLY a JSON object:
{"scope": "HIGH"|"MEDIUM"|"LOW", "gravity": "HIGH"|"MEDIUM"|"LOW", "strength": "HIGH"|"MEDIUM"|"LOW", "hierarchy": "HIGH"|"MEDIUM"|"LOW"}"""


def query_obligation_provisions(conn, limit=None):
    """Fetch unique Obligation provisions from benchmark laws."""
    sql = """
        SELECT DISTINCT ON (pa.section_id)
            pa.section_id, lt.text, lt.section_type, lt.law_name
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE lt.law_name IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
        AND pa.slm_drrp = 'Obligation'
        AND lt.scope = 'substantive'
        ORDER BY pa.section_id
    """
    if limit:
        sql = f"SELECT * FROM ({sql}) sub LIMIT {limit}"
    cur = conn.cursor()
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows


def rate_provision(api_key, section_id, text, section_type):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL}:generateContent?key={api_key}"
    user_msg = (
        f"Provision ({section_id}, section_type={section_type}):\n\n"
        f"{text}\n\n"
        f"Rate the significance of this obligation."
    )
    body = {
        "contents": [
            {"role": "user", "parts": [{"text": PROMPT + "\n\n" + user_msg}]}
        ],
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": 100,
            "thinkingConfig": {"thinkingBudget": 0}
        }
    }
    try:
        resp = None
        for attempt in range(3):
            resp = requests.post(url, json=body, timeout=30)
            if resp.status_code in (429, 503):
                time.sleep(5 * (attempt + 1))
                continue
            break
        resp.raise_for_status()
        content = resp.json()
        text_resp = content.get("candidates", [{}])[0].get("content", {}).get("parts", [{}])[0].get("text", "").strip()

        if "```json" in text_resp:
            text_resp = text_resp.split("```json")[1].split("```")[0].strip()
        elif "```" in text_resp:
            text_resp = text_resp.split("```")[1].split("```")[0].strip()

        parsed = json.loads(text_resp)
        valid = {"HIGH", "MEDIUM", "LOW"}
        if all(parsed.get(d, "").upper() in valid for d in ["scope", "gravity", "strength", "hierarchy"]):
            return {d: parsed[d].upper() for d in ["scope", "gravity", "strength", "hierarchy"]}
        return None
    except json.JSONDecodeError as e:
        print(f"  PARSE ERROR {section_id}: {e} — [{text_resp[:100]}]", file=sys.stderr)
        return None
    except Exception as e:
        print(f"  ERROR {section_id}: {e}", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(description="Rate obligation significance via Gemini")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    api_key = os.environ.get("GEMINI_API_KEY") or ""
    if not api_key:
        import subprocess
        result = subprocess.run(["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
                                capture_output=True, text=True)
        api_key = result.stdout.strip()
    if not api_key:
        print("ERROR: GEMINI_API_KEY not set")
        sys.exit(1)

    conn = psycopg2.connect(PG_DSN)
    provisions = query_obligation_provisions(conn, limit=args.limit)
    print(f"Loaded {len(provisions):,} Obligation provisions from benchmark laws")

    if args.dry_run:
        for sid, text, stype, law in provisions[:5]:
            print(f"  {sid} ({stype}) | {text[:100]}...")
        if len(provisions) > 5:
            print(f"  ... and {len(provisions) - 5} more")
        conn.close()
        return

    results = []
    errors = 0
    dim_counts = {d: Counter() for d in ["scope", "gravity", "strength", "hierarchy"]}
    t0 = time.time()

    for i, (sid, text, stype, law) in enumerate(provisions):
        rating = rate_provision(api_key, sid, text, stype)

        if rating:
            results.append({"section_id": sid, "law_name": law, **rating})
            for d in ["scope", "gravity", "strength", "hierarchy"]:
                dim_counts[d][rating[d]] += 1
        else:
            errors += 1

        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            print(f"  [{i+1:,}/{len(provisions):,}] {len(results):,} rated, "
                  f"{errors} errors, {rate:.1f}/s")

        time.sleep(0.1)

    elapsed = time.time() - t0
    print(f"\n{'=' * 60}")
    print(f"Significance Rating Complete")
    print(f"{'=' * 60}")
    print(f"Total:    {len(provisions):,}")
    print(f"Rated:    {len(results):,}")
    print(f"Errors:   {errors}")
    print(f"Time:     {elapsed/60:.1f} min")

    for d in ["scope", "gravity", "strength", "hierarchy"]:
        print(f"\n{d.capitalize()}:")
        for level in ["HIGH", "MEDIUM", "LOW"]:
            print(f"  {level:8s}: {dim_counts[d].get(level, 0):,}")

    # Save results
    out_path = "data/significance_benchmark.json"
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {out_path}")

    conn.close()


if __name__ == "__main__":
    main()
