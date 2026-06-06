#!/usr/bin/env /usr/bin/python3
"""Tier 1 Inheritance QA — Bayesian precision estimation.

Samples inherited provisions from LanceDB, sends each to an LLM for
independent verification, and applies Beta-Binomial Bayesian inference
to estimate precision with credible intervals.

Usage:
    /usr/bin/python3 .claude/skills/tier1-qa/run_qa.py
    /usr/bin/python3 .claude/skills/tier1-qa/run_qa.py --family "OH&S" --sample-size 50
    /usr/bin/python3 .claude/skills/tier1-qa/run_qa.py --dry-run
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
    """Approximate credible interval using normal approximation to Beta.

    For high alpha+beta (>10), the normal approximation is adequate.
    For small samples, this is slightly wider than the exact interval,
    which is conservative (safe for QA purposes).
    """
    mean = beta_mean(alpha, beta)
    var = (alpha * beta) / ((alpha + beta) ** 2 * (alpha + beta + 1))
    std = math.sqrt(var)
    z = 1.96 if confidence == 0.95 else 2.576  # 95% or 99%
    lo = max(0.0, mean - z * std)
    hi = min(1.0, mean + z * std)
    return (lo, hi)


# ── LLM verification ────────────────────────────────────────────────

VERIFICATION_PROMPT = """\
You are a legal analyst verifying automated duty-holder extraction from UK and EU legislation.

A provision was found to have NO explicit duty holder in its text. An automated system
inherited the duty holder from a parent provision in the document hierarchy.

## Parent Provision
Section: {parent_sid}
Text: {parent_text}

## Target Provision (child)
Section: {target_sid}
Text: {target_text}

## Inherited Result
Duty holder: {inherited_actor}
Ancestor distance: {distance} (1 = immediate parent, 2 = grandparent, etc.)

## Task
Is the inherited duty holder CORRECT for the target provision?

Consider:
- Does the parent provision establish an obligation on the inherited actor?
- Does the target provision naturally extend or detail that obligation?
- Or is the target provision about something unrelated to the parent's duty?

Respond with EXACTLY one of:
- CORRECT — the inheritance is right, the target provision is part of the parent's obligation
- INCORRECT — the target provision is about a different actor or is not a duty provision
- AMBIGUOUS — the text is genuinely unclear about who holds the duty

Then a brief explanation (1-2 sentences).

Format:
VERDICT: [CORRECT|INCORRECT|AMBIGUOUS]
REASON: [your explanation]
"""


def verify_with_gemini(prompt: str, api_key: str) -> dict:
    """Call Gemini API for verification."""
    try:
        from google import genai
    except ImportError:
        print("ERROR: google-genai not installed. Run: pip install google-genai",
              file=sys.stderr)
        sys.exit(1)

    client = genai.Client(api_key=api_key)
    response = client.models.generate_content(
        model="gemini-2.5-flash",
        contents=prompt,
        config={"http_options": {"timeout": 30_000}},
    )
    text = response.text.strip()

    # Parse verdict
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


def verify_with_anthropic(prompt: str, api_key: str) -> dict:
    """Call Anthropic API for verification."""
    try:
        import anthropic
    except ImportError:
        print("ERROR: anthropic not installed. Run: pip install anthropic",
              file=sys.stderr)
        sys.exit(1)

    client = anthropic.Anthropic(api_key=api_key)
    response = client.messages.create(
        model="claude-sonnet-4-20250514",
        max_tokens=256,
        messages=[{"role": "user", "content": prompt}],
    )
    text = response.content[0].text.strip()

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
    """Get law names matching a family prefix from DuckDB."""
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


def sample_inherited_provisions(
    lancedb_path: str,
    sample_size: int,
    method: str = "inherited",
    family_laws: set = None,
) -> list:
    """Sample random inherited provisions with their parent context."""
    import lancedb as ldb

    db = ldb.connect(str(lancedb_path))
    tbl = db.open_table("legislation_text")

    # Fetch all inherited provisions
    filter_expr = f"extraction_method = '{method}'"
    rows = (
        tbl.search()
        .where(filter_expr, prefilter=True)
        .select([
            "section_id", "law_name", "text", "hierarchy_path", "depth",
            "governed_actors", "holder_inferred_from", "ancestor_distance",
            "drrp_types", "extraction_method",
        ])
        .limit(500_000)
        .to_arrow()
    )

    # Build candidate list
    candidates = []
    for i in range(rows.num_rows):
        law = rows.column("law_name")[i].as_py()
        if family_laws and law not in family_laws:
            continue
        inferred_from = rows.column("holder_inferred_from")[i].as_py()
        if not inferred_from:
            continue
        candidates.append({
            "target_sid": rows.column("section_id")[i].as_py(),
            "law_name": law,
            "target_text": (rows.column("text")[i].as_py() or "").strip(),
            "hierarchy_path": rows.column("hierarchy_path")[i].as_py(),
            "depth": rows.column("depth")[i].as_py(),
            "governed_actors": rows.column("governed_actors")[i].as_py(),
            "holder_inferred_from": inferred_from,
            "ancestor_distance": rows.column("ancestor_distance")[i].as_py(),
            "drrp_types": rows.column("drrp_types")[i].as_py(),
        })

    if not candidates:
        print(f"No {method} provisions found.", file=sys.stderr)
        return []

    # Random sample
    sample_size = min(sample_size, len(candidates))
    sampled = random.sample(candidates, sample_size)

    # Fetch parent text for each sample
    for item in sampled:
        parent_sids = item["holder_inferred_from"].split(",")
        parent_sid = parent_sids[0].strip()
        parent_filter = f"section_id = '{parent_sid.replace(chr(39), chr(39)+chr(39))}'"
        try:
            parent_rows = (
                tbl.search()
                .where(parent_filter, prefilter=True)
                .select(["section_id", "text", "governed_actors"])
                .limit(1)
                .to_arrow()
            )
            if parent_rows.num_rows > 0:
                item["parent_sid"] = parent_rows.column("section_id")[0].as_py()
                item["parent_text"] = (parent_rows.column("text")[0].as_py() or "").strip()
                item["parent_actors"] = parent_rows.column("governed_actors")[0].as_py()
            else:
                item["parent_sid"] = parent_sid
                item["parent_text"] = "[not found in LanceDB]"
                item["parent_actors"] = []
        except Exception:
            item["parent_sid"] = parent_sid
            item["parent_text"] = "[query error]"
            item["parent_actors"] = []

    return sampled


# ── Main ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Tier 1 Inheritance QA")
    parser.add_argument("--family", type=str, default=None,
                        help="Filter to laws in this family prefix (e.g., 'OH&S')")
    parser.add_argument("--sample-size", type=int, default=30,
                        help="Number of provisions to sample (default: 30)")
    parser.add_argument("--method", type=str, default="inherited",
                        help="Extraction method to QA (default: inherited)")
    parser.add_argument("--provider", type=str, default="gemini",
                        choices=["gemini", "anthropic"],
                        help="LLM provider (default: gemini)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Assemble samples without LLM calls")
    parser.add_argument("--data-dir", type=str, default="data",
                        help="Data directory (default: data)")
    parser.add_argument("--output", type=str, default=None,
                        help="Output JSON path (default: data/qa-results/tier1-qa-TIMESTAMP.json)")
    args = parser.parse_args()

    from datetime import datetime

    data_dir = Path(args.data_dir)
    lancedb_path = data_dir / "lancedb"
    duckdb_path = data_dir / "fractalaw.duckdb"
    if args.output:
        output_path = Path(args.output)
    else:
        ts = datetime.now().strftime("%Y%m%d-%H%M%S")
        output_path = data_dir / "qa-results" / f"{args.method}-qa-{ts}.json"

    # Resolve API key
    api_key = None
    if not args.dry_run:
        if args.provider == "gemini":
            api_key = os.environ.get("GEMINI_API_KEY")
            if not api_key:
                print("ERROR: Set GEMINI_API_KEY environment variable", file=sys.stderr)
                sys.exit(1)
        elif args.provider == "anthropic":
            api_key = os.environ.get("ANTHROPIC_API_KEY")
            if not api_key:
                print("ERROR: Set ANTHROPIC_API_KEY environment variable", file=sys.stderr)
                sys.exit(1)

    # Family filter
    family_laws = None
    if args.family:
        family_laws = get_family_laws(str(duckdb_path), args.family)
        if not family_laws:
            print(f"No laws found for family '{args.family}'")
            sys.exit(1)
        print(f"Family '{args.family}': {len(family_laws)} laws")

    # Sample provisions
    print(f"Sampling {args.sample_size} {args.method} provisions...")
    samples = sample_inherited_provisions(
        str(lancedb_path), args.sample_size, args.method, family_laws
    )
    if not samples:
        sys.exit(1)
    print(f"Sampled {len(samples)} provisions from {len(set(s['law_name'] for s in samples))} laws")

    # Bayesian state
    alpha = 1.0  # Beta prior: uniform
    beta_param = 1.0
    results = []
    correct = 0
    incorrect = 0
    ambiguous = 0

    for i, sample in enumerate(samples):
        inherited_actor = ", ".join(sample["governed_actors"]) if sample["governed_actors"] else "Unknown"
        target_text = sample["target_text"][:500] if sample["target_text"] else "[empty]"
        parent_text = sample["parent_text"][:500] if sample["parent_text"] else "[empty]"

        print(f"\n--- Sample {i+1}/{len(samples)} ---")
        print(f"  Target:  {sample['target_sid']}")
        print(f"  Parent:  {sample['parent_sid']}")
        print(f"  Actor:   {inherited_actor}")
        print(f"  Distance: {sample['ancestor_distance']}")

        if args.dry_run:
            print(f"  Target text: {target_text[:120]}...")
            print(f"  Parent text: {parent_text[:120]}...")
            verdict_result = {"verdict": "DRY_RUN", "reason": "skipped", "raw": ""}
        else:
            prompt = VERIFICATION_PROMPT.format(
                parent_sid=sample["parent_sid"],
                parent_text=parent_text,
                target_sid=sample["target_sid"],
                target_text=target_text,
                inherited_actor=inherited_actor,
                distance=sample["ancestor_distance"],
            )
            if args.provider == "gemini":
                verdict_result = verify_with_gemini(prompt, api_key)
            else:
                verdict_result = verify_with_anthropic(prompt, api_key)

        verdict = verdict_result["verdict"]
        print(f"  Verdict: {verdict}")
        print(f"  Reason:  {verdict_result['reason'][:150]}")

        # Bayesian update
        if verdict == "CORRECT":
            alpha += 1
            correct += 1
        elif verdict == "INCORRECT":
            beta_param += 1
            incorrect += 1
        elif verdict == "AMBIGUOUS":
            ambiguous += 1
            # Ambiguous doesn't update the Beta — we're measuring precision
            # of clear correct vs clear incorrect judgements

        results.append({
            "index": i + 1,
            "target_sid": sample["target_sid"],
            "parent_sid": sample["parent_sid"],
            "law_name": sample["law_name"],
            "inherited_actor": inherited_actor,
            "ancestor_distance": sample["ancestor_distance"],
            "verdict": verdict,
            "reason": verdict_result["reason"],
            "target_text": sample["target_text"][:300],
            "parent_text": sample["parent_text"][:300],
        })

        # Running estimate
        if correct + incorrect > 0:
            mean = beta_mean(alpha, beta_param)
            lo, hi = beta_credible_interval(alpha, beta_param)
            width = hi - lo
            print(f"  Precision: {mean:.1%} [{lo:.1%}, {hi:.1%}] (width={width:.3f})")
        sys.stdout.flush()

        # Incremental save — preserve results even if a later sample fails
        _save_results(output_path, args, samples, results,
                      correct, incorrect, ambiguous, alpha, beta_param)

    # Final summary
    print("\n" + "=" * 60)
    print("TIER 1 INHERITANCE QA — FINAL RESULTS")
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

    _save_results(output_path, args, samples, results,
                  correct, incorrect, ambiguous, alpha, beta_param)


def _save_results(output_path, args, samples, results,
                  correct, incorrect, ambiguous, alpha, beta_param):
    """Save results to JSON (called incrementally and at end)."""
    output = {
        "method": args.method,
        "family": args.family,
        "provider": args.provider,
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
