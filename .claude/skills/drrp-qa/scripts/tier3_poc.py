#!/usr/bin/env /usr/bin/python3
"""Tier 3 LLM Proof of Concept — holder vs recipient distinction.

Tests whether an LLM can correctly identify duty HOLDERS vs RECIPIENTS
in provisions where Tier 1 inheritance produced false positives.

Uses the INCORRECT cases from the latest Tier 1 QA run as test set.

Usage:
    source ~/.bashrc && /usr/bin/python3 .claude/skills/tier1-qa/tier3_poc.py
"""

import glob
import json
import os
import sys
from pathlib import Path

TIER3_PROMPT = """\
You are a legal analyst identifying duty holders in UK and EU legislation.

A provision mentions multiple actors. Your task is to identify which actor
HOLDS the duty (the one who must act) versus which actors are RECIPIENTS,
BENEFICIARIES, or merely MENTIONED.

## Actors found in the text
{actor_list}

## Parent Provision
Section: {parent_sid}
Text: {parent_text}

## Target Provision (child)
Section: {target_sid}
Text: {target_text}

## Task
For each actor, classify their ROLE in this provision:

- HOLDER — this actor bears the obligation (must do something)
- RECIPIENT — this actor receives something (information, training, protection)
- BENEFICIARY — this actor benefits from the provision but has no active role
- MENTIONED — this actor is referenced but neither holds nor receives

Respond in JSON:
{{
  "actors": [
    {{"label": "Org: Employer", "role": "HOLDER", "reason": "..."}},
    {{"label": "Ind: Employee", "role": "RECIPIENT", "reason": "..."}}
  ],
  "primary_holder": "Org: Employer",
  "reasoning": "The employer shall provide training TO employees — employer acts, employee receives."
}}

If NO actor is a clear HOLDER, set "primary_holder": null.
"""


def run_tier3(prompt: str, api_key: str) -> dict:
    """Call Gemini API for holder/recipient distinction."""
    from google import genai

    client = genai.Client(api_key=api_key)
    response = client.models.generate_content(
        model="gemini-2.5-flash",
        contents=prompt,
        config={"http_options": {"timeout": 30_000}},
    )
    text = response.text.strip()

    # Try to parse JSON from response (may be wrapped in markdown code block)
    json_text = text
    if "```json" in text:
        json_text = text.split("```json")[1].split("```")[0].strip()
    elif "```" in text:
        json_text = text.split("```")[1].split("```")[0].strip()

    try:
        parsed = json.loads(json_text)
    except json.JSONDecodeError:
        parsed = {"raw": text, "parse_error": True}

    return {"parsed": parsed, "raw": text}


def main():
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        print("ERROR: Set GEMINI_API_KEY environment variable", file=sys.stderr)
        sys.exit(1)

    # Load latest QA results
    files = sorted(glob.glob("data/qa-results/inherited-qa-*.json"))
    if not files:
        print("ERROR: No QA results found in data/qa-results/", file=sys.stderr)
        sys.exit(1)

    with open(files[-1]) as f:
        qa_data = json.load(f)

    incorrect = [r for r in qa_data["results"] if r["verdict"] == "INCORRECT"]
    print(f"QA file: {files[-1]}")
    print(f"INCORRECT cases: {len(incorrect)}")
    print()

    # Load parent text from LanceDB for each case
    import lancedb

    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    results = []
    for i, case in enumerate(incorrect):
        target_sid = case["target_sid"]
        parent_sid = case["parent_sid"]
        actors = case["inherited_actor"]

        # Fetch full text (QA results truncate at 300 chars)
        target_rows = (
            tbl.search()
            .where(
                f"section_id = '{target_sid.replace(chr(39), chr(39)+chr(39))}'",
                prefilter=True,
            )
            .select(["text"])
            .limit(1)
            .to_arrow()
        )
        parent_rows = (
            tbl.search()
            .where(
                f"section_id = '{parent_sid.replace(chr(39), chr(39)+chr(39))}'",
                prefilter=True,
            )
            .select(["text"])
            .limit(1)
            .to_arrow()
        )

        target_text = (
            (target_rows.column("text")[0].as_py() or "").strip()[:500]
            if target_rows.num_rows > 0
            else case["target_text"]
        )
        parent_text = (
            (parent_rows.column("text")[0].as_py() or "").strip()[:500]
            if parent_rows.num_rows > 0
            else case["parent_text"]
        )

        # Build actor list for prompt
        actor_labels = [a.strip() for a in actors.split(",")]
        actor_list = "\n".join(f"- {a}" for a in actor_labels)

        print(f"--- Case {i+1}/{len(incorrect)} ---")
        print(f"  Target: {target_sid}")
        print(f"  Parent: {parent_sid}")
        print(f"  Actors: {actors}")
        sys.stdout.flush()

        prompt = TIER3_PROMPT.format(
            actor_list=actor_list,
            parent_sid=parent_sid,
            parent_text=parent_text,
            target_sid=target_sid,
            target_text=target_text,
        )

        response = run_tier3(prompt, api_key)
        parsed = response["parsed"]

        if parsed.get("parse_error"):
            print(f"  PARSE ERROR — raw response:")
            print(f"  {response['raw'][:300]}")
        else:
            primary = parsed.get("primary_holder", "???")
            reasoning = parsed.get("reasoning", "")[:150]
            print(f"  Primary holder: {primary}")
            print(f"  Reasoning: {reasoning}")

            if parsed.get("actors"):
                for a in parsed["actors"]:
                    print(
                        f"    {a.get('label', '?')}: {a.get('role', '?')} — {a.get('reason', '')[:100]}"
                    )

        # Compare with QA verdict
        qa_reason = case["reason"][:150]
        print(f"  QA said: {qa_reason}")
        print()
        sys.stdout.flush()

        results.append(
            {
                "target_sid": target_sid,
                "parent_sid": parent_sid,
                "inherited_actors": actor_labels,
                "tier3_response": parsed,
                "qa_reason": case["reason"],
            }
        )

    # Save results
    from datetime import datetime

    ts = datetime.now().strftime("%Y%m%d-%H%M%S")
    output_path = Path(f"data/qa-results/tier3-poc-{ts}.json")
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"Results saved to: {output_path}")

    # Summary
    holders_identified = sum(
        1
        for r in results
        if r["tier3_response"].get("primary_holder")
        and not r["tier3_response"].get("parse_error")
    )
    print(f"\nSummary: {holders_identified}/{len(results)} cases got a primary_holder identification")


if __name__ == "__main__":
    main()
