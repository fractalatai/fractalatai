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

If INCORRECT, also provide the corrected classification as JSON. Use the EXACT actor labels from above.

Format:
VERDICT: [CORRECT|INCORRECT|AMBIGUOUS]
REASON: [your explanation]
CORRECTION: {{"drrp_types": ["Duty"], "actors": [{{"label": "Org: Employer", "position": "active", "reason": "..."}}]}}
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
    correction = None
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
        elif line.upper().startswith("CORRECTION:"):
            json_str = line.split(":", 1)[1].strip()
            try:
                correction = json.loads(json_str)
            except json.JSONDecodeError:
                # Try to find JSON in the rest of the text
                pass

    # If no inline correction found, try to extract JSON block from full text
    if correction is None and verdict == "INCORRECT" and "{" in text:
        import re
        # Find last JSON object in the response
        json_matches = re.findall(r'\{[^{}]*"actors"[^{}]*\[.*?\].*?\}', text, re.DOTALL)
        if json_matches:
            try:
                correction = json.loads(json_matches[-1])
            except json.JSONDecodeError:
                pass

    return {"verdict": verdict, "reason": reason, "correction": correction, "raw": text}


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


# ── Correction write-back ────────────────────────────────────────────

def apply_correction(lancedb_path: str, section_id: str, correction: dict):
    """Write a Gemini QA correction back to LanceDB.

    Updates the provision's actors and drrp_types with Gemini's corrected
    classification, stamped as agentic with 0.90 confidence.
    """
    import lancedb as ldb
    import pyarrow as pa

    db = ldb.connect(str(lancedb_path))
    tbl = db.open_table("legislation_text")

    # Build the corrected actors as Arrow struct
    corrected_actors = correction.get("actors", [])
    corrected_drrp = correction.get("drrp_types", [])

    if not corrected_actors:
        return False

    ACTORS_TYPE = pa.list_(pa.struct([
        pa.field("label", pa.string(), nullable=False),
        pa.field("position", pa.string(), nullable=False),
        pa.field("relates_to", pa.string(), nullable=True),
        pa.field("label_source", pa.string(), nullable=False),
        pa.field("reason", pa.string(), nullable=True),
    ]))

    actors_data = [{
        "label": a.get("label", ""),
        "position": a.get("position", "mentioned").lower(),
        "relates_to": a.get("relates_to"),
        "label_source": "canonical",  # Gemini uses our labels
        "reason": a.get("reason"),
    } for a in corrected_actors]

    # Update via merge_insert on section_id
    update_data = pa.table({
        "section_id": [section_id],
        "actors": pa.array([actors_data], type=ACTORS_TYPE),
        "drrp_types": pa.array([corrected_drrp], type=pa.list_(pa.string())),
        "extraction_method": ["agentic"],
        "taxa_confidence": pa.array([0.90], type=pa.float32()),
    })

    try:
        tbl.merge_insert("section_id").when_matched_update_all().execute(update_data)
        return True
    except Exception as e:
        print(f"  WARNING: Failed to write correction for {section_id}: {e}",
              file=sys.stderr)
        return False


# ── Report generation ────────────────────────────────────────────────

def generate_report(lancedb_path: str, law_name: str, data_dir: Path):
    """Print a human-readable DRRP report for a law to stdout.

    Shows regulation-level provisions (article, sub_article, section, sub_section)
    — the level at which DRRP is meaningful. Fragments (paragraph, sub_paragraph)
    are counted but not listed — they inherit from their parent regulation.
    """
    import lancedb as ldb

    db = ldb.connect(str(lancedb_path))
    tbl = db.open_table("legislation_text")

    results = (
        tbl.search()
        .where(f"law_name = '{law_name.replace(chr(39), chr(39)+chr(39))}'", prefilter=True)
        .select([
            "section_id", "section_type", "text", "drrp_types",
            "extraction_method", "taxa_confidence", "actors", "purposes",
        ])
        .limit(10000)
        .to_arrow()
    )

    if results.num_rows == 0:
        print(f"No provisions found for {law_name}")
        return

    # Regulation-level types (where DRRP lives)
    REGULATION_TYPES = {"article", "sub_article", "section", "sub_section"}

    total = results.num_rows
    reg_count = 0
    frag_count = 0
    structural_count = 0
    methods = {}
    has_drrp = 0

    for i in range(total):
        st = results.column("section_type")[i].as_py() or ""
        m = results.column("extraction_method")[i].as_py() or "none"
        methods[m] = methods.get(m, 0) + 1
        drrp = results.column("drrp_types")[i].as_py()
        if drrp and len(drrp) > 0:
            has_drrp += 1
        if st in REGULATION_TYPES:
            reg_count += 1
        elif st in ("paragraph", "sub_paragraph"):
            frag_count += 1
        else:
            structural_count += 1

    print(f"# {law_name}")
    print(f"")
    print(f"Provisions: {total} total, {reg_count} regulations, {frag_count} fragments, {structural_count} structural")
    print(f"DRRP: {has_drrp} classified | Methods: {methods}")
    print()

    # Regulation-level provisions
    print(f"| Section | DRRP | Conf | Method | Actors | Text |")
    print(f"|---|---|---|---|---|---|")

    for i in range(total):
        st = results.column("section_type")[i].as_py() or ""
        if st not in REGULATION_TYPES:
            continue

        drrp = results.column("drrp_types")[i].as_py() or []
        actors = results.column("actors")[i].as_py() or []
        sid = results.column("section_id")[i].as_py() or ""
        short_sid = sid.split(":", 1)[1] if ":" in sid else sid
        method = results.column("extraction_method")[i].as_py() or ""
        conf = results.column("taxa_confidence")[i].as_py()
        conf_str = f"{conf:.2f}" if conf is not None else "-"
        text = (results.column("text")[i].as_py() or "")[:120].replace("|", "\\|").replace("\n", " ")
        drrp_str = ", ".join(drrp) if drrp else "-"

        actor_parts = []
        for a in actors:
            label = a.get("label", "?")
            pos = a.get("position", "?")
            actor_parts.append(f"{label}({pos})")
        actors_str = ", ".join(actor_parts) if actor_parts else "-"

        print(f"| {short_sid} | {drrp_str} | {conf_str} | {method} | {actors_str} | {text} |")


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
    parser.add_argument("--write-back", action="store_true",
                        help="Write Gemini corrections back to LanceDB (stamps as agentic, conf=0.90)")
    parser.add_argument("--report", type=str, default=None,
                        help="Generate human-readable DRRP report for a specific law (e.g., UK_uksi_1992_2793)")
    parser.add_argument("--data-dir", type=str, default="data",
                        help="Data directory (default: data)")
    args = parser.parse_args()

    from datetime import datetime

    data_dir = Path(args.data_dir)
    lancedb_path = data_dir / "lancedb"
    duckdb_path = data_dir / "fractalaw.duckdb"
    ts = datetime.now().strftime("%Y%m%d-%H%M%S")
    output_path = data_dir / "qa-results" / f"drrp-qa-{args.method}-{ts}.json"

    # ── Report mode: dump human-readable DRRP table for a law ──
    if args.report:
        generate_report(str(lancedb_path), args.report, data_dir)
        return

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
            # Write correction back to LanceDB if available and --write-back enabled
            correction = verdict_result.get("correction")
            if correction and args.write_back:
                ok = apply_correction(
                    str(lancedb_path), sample["section_id"], correction
                )
                if ok:
                    print(f"  >> Correction applied to LanceDB (agentic, conf=0.90)")
                else:
                    print(f"  >> Correction not applied (parse failed or no actors)")
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
            "correction": verdict_result.get("correction"),
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
