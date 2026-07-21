#!/usr/bin/env python3
"""Prepare control title generation training data for SLM fine-tuning.

Uses existing Gemini-generated legislation controls (1,556) as exemplars
of good indicative-mood control titles. The SLM learns the style from
legislation controls and applies it to JSP artefacts.

Training data combines:
1. Legislation controls (gold standard titles from Gemini)
2. JSP controls (template titles to be improved)

The SLM learns: given artefact_type + obligation text + role → produce
an indicative-mood control title + description + what_it_checks.

Output: data/jsp-control-title-training/train.jsonl, val.jsonl

Usage:
    /usr/bin/python3 scripts/ml/prepare_control_title_training.py
"""

import json
import os
import random

import duckdb

SYSTEM_PROMPT = """You are a compliance controls architect. Generate a control specification from a JSP obligation.

The control must be in indicative mood — a statement that is observably true or false right now.
Not "must ensure" but "is ensured." Not "shall provide" but "is provided."

Output JSON:
{
  "title": "indicative-mood statement that can be verified on site",
  "description": "what reality this control stands for, not what document proves it",
  "what_it_checks": "what would look different if this control had failed"
}

Rules:
- Title must be checkable: can you walk onto a site and verify it?
- Description refers to the state of the work, not the state of a file
- what_it_checks describes what would look DIFFERENT if the control failed
- Use domain-specific language from the obligation, not generic compliance terms"""

OUTPUT_DIR = "data/jsp-control-title-training"


def main():
    conn = duckdb.connect("data/fractalaw.duckdb", read_only=True)

    # Load legislation controls as exemplars (Gemini-generated, high quality)
    leg_controls = conn.execute("""
        SELECT control_json FROM suggested_controls
        WHERE source_id IS NULL AND status IN ('validated', 'generated')
        LIMIT 1000
    """).fetchall()
    print(f"Legislation controls (exemplars): {len(leg_controls)}")

    # Load JSP controls with their source data
    jsp_data = conn.execute("""
        SELECT c.control_json, a.artefact_type, o.text AS obligation_text,
               r.role_label, o.competence_requirements
        FROM suggested_controls c
        JOIN jsp_mandated_artefacts a ON c.id = a.artefact_id
        JOIN jsp_obligations o ON a.obligation_id = o.obligation_id
        LEFT JOIN jsp_raci r ON r.obligation_id = a.obligation_id AND r.assignment_type = 'R'
        WHERE c.source_id IS NOT NULL
        ORDER BY c.id
    """).fetchall()
    print(f"JSP controls with source data: {len(jsp_data)}")
    conn.close()

    examples = []

    # Legislation controls as positive examples (learn the style)
    for (cj,) in leg_controls:
        try:
            c = json.loads(cj)
        except json.JSONDecodeError:
            continue

        title = c.get("title", "")
        desc = c.get("description", "")
        checks = c.get("what_it_checks", "")

        if not title or not desc:
            continue

        # The "user" input is what we'd have from a JSP — artefact type + obligation
        # For legislation, we use the description as the obligation proxy
        linked = c.get("linked_provisions", [])
        control_type = c.get("control_type", "")
        domain = c.get("domain", "")

        user_msg = f"Control type: {control_type}\nDomain: {domain}\n\nObligation: {desc[:500]}"

        target = {
            "title": title,
            "description": desc,
            "what_it_checks": checks,
        }

        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_msg},
                {"role": "assistant", "content": json.dumps(target)},
            ]
        })

    print(f"Legislation exemplars prepared: {len(examples)}")

    # JSP controls — these have template titles that need improvement
    # We include them as training data too (the SLM sees JSP-style inputs)
    for cj, atype, otext, role, competence in jsp_data:
        try:
            c = json.loads(cj)
        except json.JSONDecodeError:
            continue

        role_str = role or "(unassigned)"
        user_msg = f"Artefact type: {atype}\nResponsible role: {role_str}\n\nObligation: {otext[:500]}"

        # For JSP controls, the existing template title is the target
        # (the SLM will learn to generate these, then we evaluate quality)
        target = {
            "title": c.get("title", ""),
            "description": otext[:300],
            "what_it_checks": c.get("what_it_checks", ""),
        }

        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_msg},
                {"role": "assistant", "content": json.dumps(target)},
            ]
        })

    print(f"Total examples (legislation + JSP): {len(examples)}")

    # Split 90/10
    random.seed(42)
    random.shuffle(examples)
    split = int(len(examples) * 0.9)
    train = examples[:split]
    val = examples[split:]

    os.makedirs(OUTPUT_DIR, exist_ok=True)
    with open(os.path.join(OUTPUT_DIR, "train.jsonl"), "w") as f:
        for ex in train:
            f.write(json.dumps(ex) + "\n")
    with open(os.path.join(OUTPUT_DIR, "val.jsonl"), "w") as f:
        for ex in val:
            f.write(json.dumps(ex) + "\n")

    print(f"\nWritten to {OUTPUT_DIR}/")
    print(f"  train.jsonl: {len(train)}")
    print(f"  val.jsonl: {len(val)}")


if __name__ == "__main__":
    main()
