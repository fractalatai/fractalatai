#!/usr/bin/env /usr/bin/python3
"""DRRP QA — Bayesian precision estimation for actor position classification.

Samples provisions from LanceDB, sends each to Gemini for independent
verification of DRRP type and actor position (Hohfeldian model), and
applies Beta-Binomial Bayesian inference to estimate precision.

Usage:
    /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --family "OH&S" --sample-size 30
    /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --method agentic --sample-size 20
    /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --method all --sample-size 40
    /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --dry-run --family "OH&S"
"""

import argparse
import json
import math
import os
import random
import sys
from pathlib import Path


# ── Beta distribution (no scipy dependency) ─────────────────────────

def beta_mean(alpha: float, beta: float) -> float:
    return alpha / (alpha + beta)


def beta_credible_interval(alpha: float, beta: float, confidence: float = 0.95) -> tuple:
    mean = beta_mean(alpha, beta)
    var = (alpha * beta) / ((alpha + beta) ** 2 * (alpha + beta + 1))
    std = math.sqrt(var)
    z = 1.96 if confidence == 0.95 else 2.576
    lo = max(0.0, mean - z * std)
    hi = min(1.0, mean + z * std)
    return (lo, hi)


# ── LLM verification ────────────────────────────────────────────────

VERIFICATION_PROMPT = """\
You are a legal analyst verifying automated DRRP extraction from UK and EU legislation.

A provision has been classified with a DRRP type and actors have been assigned
Hohfeldian positions (active = bears/exercises the obligation, counterparty = other
side of the legal relation, beneficiary = benefits without direct legal relation,
mentioned = referenced but no active role).

## Provision
Section: {section_id}
Text: {text}

## Automated Classification
DRRP types: {drrp_types}
Extraction method: {extraction_method}

Actors:
{actors_display}

## Task
Verify the classification. Consider:
1. Is the DRRP type correct? (Is this really a Duty/Right/Responsibility/Power?)
2. For each actor with position "active" — do they actually bear/exercise the obligation in this text?
3. For each actor with position "counterparty" — are they on the receiving end?
4. Are any actors misclassified? (e.g., an active actor that should be counterparty, or vice versa)

Respond with EXACTLY one of:
- CORRECT — the DRRP type and all actor positions are right
- INCORRECT — at least one classification is wrong
- AMBIGUOUS — the text is genuinely unclear

Then a brief explanation (1-2 sentences).

Format:
VERDICT: [CORRECT|INCORRECT|AMBIGUOUS]
REASON: [your explanation]
"""


def verify_with_gemini(prompt: str, api_key: str) -> dict:
    from google import genai

    client = genai.Client(api_key=api_key)
    response = client.models.generate_content(
        model="gemini-2.5-flash",
        contents=prompt,
        config={"http_options": {"timeout": 30_000}},
    )
    text = response.text.strip()

    verdict = "UNKNOWN"
    reason = text
    for line in text.split("\n"):
        line = line.strip()
        if line.upper().startswith("VERDICT:"):
            v = line.split(":", 1)[1].strip().upper()
            if "CORRECT" in v and "INCORRECT" not in v:
                verdict = "CORRECT"
            elif "INCORRECT" in v:
                verdict = "INCORRECT"
            elif "AMBIGUOUS" in v:
                verdict = "AMBIGUOUS"
        elif line.upper().startswith("REASON:"):
            reason = line.split(":", 1)[1].strip()

    return {"verdict": verdict, "reason": reason, "raw": text}


# ── Sample assembly ─────────────────────────────────────────────────

def get_family_laws(duckdb_path: str, family_prefix: str) -> set:
    try:
        import duckdb
        conn = duckdb.connect(str(duckdb_path), read_only=True)
        rows = conn.execute(
            "SELECT name FROM legislation WHERE family LIKE ?",
            [f"{family_prefix}%"]
        ).fetchall()
        conn.close()
        return {r[0] for r in rows}
    except Exception as e:
        print(f"WARNING: Could not query DuckDB for family filter: {e}", file=sys.stderr)
        return set()


def sample_provisions(
    lancedb_path: str,
    sample_size: int,
    method: str = "all",
    family_laws: set = None,
) -> list:
    """Sample random provisions with actors data."""
    import lancedb as ldb

    db = ldb.connect(str(lancedb_path))
    tbl = db.open_table("legislation_text")

    if method == "all":
        filter_expr = "actors IS NOT NULL"
    else:
        filter_expr = f"extraction_method = '{method}'"

    rows = (
        tbl.search()
        .where(filter_expr, prefilter=True)
        .select([
            "section_id", "law_name", "text",
            "drrp_types", "extraction_method", "actors",
        ])
        .limit(500_000)
        .to_arrow()
    )

    candidates = []
    for i in range(rows.num_rows):
        law = rows.column("law_name")[i].as_py()
        if family_laws and law not in family_laws:
            continue
        actors = rows.column("actors")[i].as_py()
        if not actors:
            continue
        candidates.append({
            "section_id": rows.column("section_id")[i].as_py(),
            "law_name": law,
            "text": (rows.column("text")[i].as_py() or "").strip(),
            "drrp_types": rows.column("drrp_types")[i].as_py() or [],
            "extraction_method": rows.column("extraction_method")[i].as_py() or "regex",
            "actors": actors,
        })

    if not candidates:
        print(f"No provisions found for method='{method}'.", file=sys.stderr)
        return []

    sample_size = min(sample_size, len(candidates))
    return random.sample(candidates, sample_size)


def format_actors(actors: list) -> str:
    lines = []
    for a in actors:
        label = a.get("label", "?")
        position = a.get("position", a.get("role", "?"))
        relates_to = a.get("relates_to", "")
        label_source = a.get("label_source", "canonical")
        reason = a.get("reason", "")

        line = f"  - {label}: position={position}"
        if relates_to:
            line += f", relates_to={relates_to}"
        if label_source == "invented":
            line += " [invented label]"
        if reason:
            line += f" ({reason[:100]})"
        lines.append(line)
    return "\n".join(lines)


# ── Main ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="DRRP QA — Classification Quality Assurance")
    parser.add_argument("--family", type=str, default=None,
                        help="Filter to laws in this family prefix (e.g., 'OH&S')")
    parser.add_argument("--sample-size", type=int, default=30,
                        help="Number of provisions to sample (default: 30)")
    parser.add_argument("--method", type=str, default="all",
                        help="Extraction method: all, regex, inherited, agentic (default: all)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Assemble samples without LLM calls")
    parser.add_argument("--data-dir", type=str, default="data",
                        help="Data directory (default: data)")
    args = parser.parse_args()

    from datetime import datetime

    data_dir = Path(args.data_dir)
    lancedb_path = data_dir / "lancedb"
    duckdb_path = data_dir / "fractalaw.duckdb"
    ts = datetime.now().strftime("%Y%m%d-%H%M%S")
    output_path = data_dir / "qa-results" / f"drrp-qa-{args.method}-{ts}.json"

    api_key = None
    if not args.dry_run:
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            print("ERROR: Set GEMINI_API_KEY environment variable", file=sys.stderr)
            sys.exit(1)

    family_laws = None
    if args.family:
        family_laws = get_family_laws(str(duckdb_path), args.family)
        if not family_laws:
            print(f"No laws found for family '{args.family}'")
            sys.exit(1)
        print(f"Family '{args.family}': {len(family_laws)} laws")

    print(f"Sampling {args.sample_size} provisions (method={args.method})...")
    samples = sample_provisions(
        str(lancedb_path), args.sample_size, args.method, family_laws
    )
    if not samples:
        sys.exit(1)

    methods = {}
    for s in samples:
        m = s["extraction_method"]
        methods[m] = methods.get(m, 0) + 1
    print(f"Sampled {len(samples)} provisions from {len(set(s['law_name'] for s in samples))} laws")
    print(f"Methods: {methods}")

    alpha = 1.0
    beta_param = 1.0
    results = []
    correct = 0
    incorrect = 0
    ambiguous = 0

    for i, sample in enumerate(samples):
        drrp = ", ".join(sample["drrp_types"]) if sample["drrp_types"] else "none"
        actors_display = format_actors(sample["actors"])
        text = sample["text"][:500] if sample["text"] else "[empty]"

        print(f"\n--- Sample {i+1}/{len(samples)} ---")
        print(f"  Section:  {sample['section_id']}")
        print(f"  Method:   {sample['extraction_method']}")
        print(f"  DRRP:     {drrp}")
        print(f"  Actors:   {len(sample['actors'])}")

        if args.dry_run:
            print(f"  Text: {text[:120]}...")
            print(actors_display)
            verdict_result = {"verdict": "DRY_RUN", "reason": "skipped", "raw": ""}
        else:
            prompt = VERIFICATION_PROMPT.format(
                section_id=sample["section_id"],
                text=text,
                drrp_types=drrp,
                extraction_method=sample["extraction_method"],
                actors_display=actors_display,
            )
            verdict_result = verify_with_gemini(prompt, api_key)

        verdict = verdict_result["verdict"]
        print(f"  Verdict: {verdict}")
        print(f"  Reason:  {verdict_result['reason'][:150]}")

        if verdict == "CORRECT":
            alpha += 1
            correct += 1
        elif verdict == "INCORRECT":
            beta_param += 1
            incorrect += 1
        elif verdict == "AMBIGUOUS":
            ambiguous += 1

        results.append({
            "index": i + 1,
            "section_id": sample["section_id"],
            "law_name": sample["law_name"],
            "extraction_method": sample["extraction_method"],
            "drrp_types": sample["drrp_types"],
            "actors": sample["actors"],
            "verdict": verdict,
            "reason": verdict_result["reason"],
            "text": sample["text"][:300],
        })

        if correct + incorrect > 0:
            mean = beta_mean(alpha, beta_param)
            lo, hi = beta_credible_interval(alpha, beta_param)
            width = hi - lo
            print(f"  Precision: {mean:.1%} [{lo:.1%}, {hi:.1%}] (width={width:.3f})")
        sys.stdout.flush()

        _save_results(output_path, args, results,
                      correct, incorrect, ambiguous, alpha, beta_param)

    print("\n" + "=" * 60)
    print("DRRP QA — FINAL RESULTS")
    print("=" * 60)
    print(f"  Samples:    {len(results)}")
    print(f"  Correct:    {correct}")
    print(f"  Incorrect:  {incorrect}")
    print(f"  Ambiguous:  {ambiguous}")

    if correct + incorrect > 0:
        mean = beta_mean(alpha, beta_param)
        lo, hi = beta_credible_interval(alpha, beta_param)
        width = hi - lo
        print(f"  Precision:  {mean:.1%}")
        print(f"  95% CI:     [{lo:.1%}, {hi:.1%}]")
        print(f"  CI width:   {width:.3f}")
        if width < 0.05:
            print(f"  Status:     CONVERGED (width < 0.05)")
        else:
            more_needed = max(0, int(50 / (width ** 2)) - len(results))
            print(f"  Status:     NOT CONVERGED (suggest ~{more_needed} more samples)")

    _save_results(output_path, args, results,
                  correct, incorrect, ambiguous, alpha, beta_param)


def _save_results(output_path, args, results,
                  correct, incorrect, ambiguous, alpha, beta_param):
    output = {
        "method": args.method,
        "family": args.family,
        "sample_size": len(results),
        "correct": correct,
        "incorrect": incorrect,
        "ambiguous": ambiguous,
        "precision_mean": beta_mean(alpha, beta_param) if correct + incorrect > 0 else None,
        "precision_ci_lo": beta_credible_interval(alpha, beta_param)[0] if correct + incorrect > 0 else None,
        "precision_ci_hi": beta_credible_interval(alpha, beta_param)[1] if correct + incorrect > 0 else None,
        "alpha": alpha,
        "beta": beta_param,
        "results": results,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"  [saved to {output_path}]")
    sys.stdout.flush()


if __name__ == "__main__":
    main()
