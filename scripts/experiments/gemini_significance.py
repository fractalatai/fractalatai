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

1. **scope_duty_bearer** — breadth of who bears the duty.
   - HIGH: universal ("every employer", "any person", "all self-employed persons")
   - MEDIUM: categorical ("an employer who operates...", "a competent person", "the responsible person")
   - LOW: individual/specific ("the person", "an inspector", "the authority")

2. **scope_protected_class** — breadth of who is protected by the duty.
   - HIGH: universal ("all employees", "persons", "the public", "any person affected")
   - MEDIUM: categorical ("employees in that workplace", "young persons", "new or expectant mothers")
   - LOW: specific ("the document", "the premises", "the apparatus", "the record")

3. **gravity** — what is at stake if the duty is breached.
   - HIGH: health, safety, life, welfare, serious environmental harm, risk of death or serious injury
   - MEDIUM: property damage, financial loss, moderate environmental impact, risk of minor injury
   - LOW: administrative, procedural, record-keeping, notification, display of notices

4. **strength** — how absolute is the obligation.
   - HIGH: absolute unqualified duty ("shall ensure" with NO qualification, "must provide", "shall maintain" — where no defence or mitigation language appears)
   - MEDIUM: qualified duty ("shall ensure so far as is reasonably practicable", "shall take all reasonable steps", "shall have regard to", "due diligence") or duty with built-in discretion
   - LOW: procedural obligation ("shall notify", "shall keep records", "shall display", "shall produce", "shall inform")

Respond with ONLY a JSON object:
{"scope_duty_bearer": "HIGH"|"MEDIUM"|"LOW", "scope_protected_class": "HIGH"|"MEDIUM"|"LOW", "gravity": "HIGH"|"MEDIUM"|"LOW", "strength": "HIGH"|"MEDIUM"|"LOW"}"""


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
        dims = ["scope_duty_bearer", "scope_protected_class", "gravity", "strength"]
        if all(parsed.get(d, "").upper() in valid for d in dims):
            return {d: parsed[d].upper() for d in dims}
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

    dims = ["scope_duty_bearer", "scope_protected_class", "gravity", "strength"]
    results = []
    errors = 0
    dim_counts = {d: Counter() for d in dims}
    t0 = time.time()

    for i, (sid, text, stype, law) in enumerate(provisions):
        rating = rate_provision(api_key, sid, text, stype)

        if rating:
            results.append({"section_id": sid, "law_name": law, **rating})
            for d in dims:
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

    for d in dims:
        print(f"\n{d}:")
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
