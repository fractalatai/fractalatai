#!/usr/bin/env python3
"""Generate evidence patterns from compliance controls using Gemini Pro.

Queries DuckDB for controls (from suggested_controls), assembles prompts,
calls Gemini Pro, validates output, and stores results in DuckDB staging table.

Usage:
    /usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713
    /usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --dry-run
    /usr/bin/python3 scripts/compliance/generate_evidence.py --family "OH&S: Occupational / Personal Safety"
    /usr/bin/python3 scripts/compliance/generate_evidence.py --qq
    /usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --force
"""

import argparse
import json
import os
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

import duckdb
import requests

# --- Config ---
DUCKDB_PATH = "data/fractalaw.duckdb"
GEMINI_MODEL = "gemini-2.5-pro"
PROMPTS_DIR = Path(__file__).parent / "prompts"
RESULTS_DIR = Path("data/compliance-evidence")

# Rate limiting
GEMINI_DELAY = 10.0  # seconds between calls
GEMINI_MAX_RETRIES = 3
GEMINI_RETRY_BACKOFF = 30  # seconds, doubles each retry

# --- Enums for validation ---
VALID_ARTEFACT_TYPES = {
    "Policy", "Procedure", "Certificate", "Training Record",
    "Report", "Inspection Report",  # accept both — prompt v1 used "Report"
    "Risk Assessment", "Permit", "Licence",
    "Test Result", "Sensor Reading", "Other",
}
VALID_ARTEFACT_CLASSES = {"Activity", "Outcome"}
VALID_SOURCES = {"Upload", "System Generated", "Sensor", "External", "Linked System"}
VALID_LIKELIHOOD_RATIOS = {"Low", "Medium", "High"}
VALID_JUDGEMENT_METHODS = {
    "Visual Inspection", "Functional Test", "Simulation",
    "Interview", "Observation", "Exercise", "Document Review",
}
VALID_VOI_QUADRANTS = {"Table Stakes", "No-Brainer", "Judgement", "Waste"}
VALID_EVIDENCE_STANDARDS = {"Basic", "Focused", "Comprehensive"}
VALID_STALENESS_TOLERANCES = {"Low", "Medium", "High"}

# Control properties needed for deterministic checks
VALID_INFO_DISTANCES = {"Direct", "Adjacent", "Mediated", "Remote"}
VALID_BLAST_RADII = {"Local", "Area", "Site", "Enterprise"}


def load_system_prompt():
    path = PROMPTS_DIR / "evidence-system-prompt-v1.md"
    return path.read_text()


# --- Database queries ---

def get_controls_for_law(duck_conn, law_name):
    """Get specific controls for a law from DuckDB staging table."""
    rows = duck_conn.execute("""
        SELECT id, control_json
        FROM suggested_controls
        WHERE law_name = ? AND control_type = 'specific'
        ORDER BY generated_at
    """, [law_name]).fetchall()
    results = []
    for row in rows:
        ctrl = json.loads(row[1]) if isinstance(row[1], str) else row[1]
        results.append({"id": row[0], "control": ctrl})
    return results


def get_law_outline(duck_conn, law_name):
    """Get minimal law metadata for context."""
    row = duck_conn.execute("""
        SELECT title, family FROM legislation WHERE name = ?
    """, [law_name]).fetchone()
    if not row:
        return None
    return {"title": row[0], "family": row[1]}


def get_laws_with_controls(duck_conn, family=None, qq_only=False, limit=None):
    """Get law names that have controls in the staging table."""
    sql = """
        SELECT DISTINCT sc.law_name
        FROM suggested_controls sc
        JOIN legislation l ON sc.law_name = l.name
        WHERE sc.control_type = 'specific'
    """
    params = []
    if family:
        sql += " AND l.family = ?"
        params.append(family)
    sql += " ORDER BY sc.law_name"
    if limit:
        sql += f" LIMIT {limit}"
    law_names = [r[0] for r in duck_conn.execute(sql, params).fetchall()]

    if qq_only:
        qq_csv = Path("data/sertantai/qq-applicable-laws.csv").read_text().strip()
        qq_set = {n.strip() for n in qq_csv.split(",") if n.strip()}
        law_names = [n for n in law_names if n in qq_set]

    return law_names


# --- Prompt assembly ---

def format_user_prompt(law_outline, controls):
    """Assemble the user prompt from law metadata and controls."""
    lines = ["## Law Context\n"]
    lines.append(f"Title: {law_outline['title']}")
    lines.append(f"Family: {law_outline['family']}")
    lines.append(f"\n## Controls ({len(controls)} specific controls)\n")

    for idx, item in enumerate(controls, 1):
        ctrl = item["control"]
        lines.append(f"### [{idx}] {ctrl.get('title', 'Untitled')}")
        lines.append(f"Type: {ctrl.get('control_type', '?')} | "
                     f"Nature: {ctrl.get('nature', '?')} | "
                     f"Domain: {ctrl.get('domain', '?')}")
        lines.append(f"Info Distance: {ctrl.get('info_distance', '?')} | "
                     f"Blast Radius: {ctrl.get('blast_radius', '?')}")
        lines.append(f"Frequency: {ctrl.get('frequency', '?')}")
        lines.append(f"Expected Touch: {ctrl.get('expected_touch_frequency', '?')}")
        if ctrl.get("load_bearing_judgement"):
            lines.append(f"Load-bearing judgement: {ctrl['load_bearing_judgement']}")
        if ctrl.get("description"):
            lines.append(f"Description: {ctrl['description'][:300]}")
        if ctrl.get("what_it_checks"):
            lines.append(f"What it checks: {ctrl['what_it_checks'][:300]}")
        if ctrl.get("evidence_hint"):
            eh = ctrl["evidence_hint"]
            if eh.get("type_a"):
                lines.append(f"Evidence hint (Type-A): {eh['type_a'][:200]}")
            if eh.get("type_b"):
                lines.append(f"Evidence hint (Type-B): {eh['type_b'][:200]}")
        if ctrl.get("honest_limit"):
            lines.append(f"Honest limit: {ctrl['honest_limit'][:200]}")
        lines.append("")

    lines.append("Generate evidence patterns for each control.")
    lines.append("- Return one evidence pattern per control, keyed by [N] index")
    lines.append("- At least one Type-B (Outcome) artefact per control")
    lines.append("- Always populate judgement_rationale and drift_conditions")

    return "\n".join(lines)


# --- Gemini API ---

def call_gemini(api_key, system_prompt, user_prompt, model=GEMINI_MODEL,
                max_tokens=32768, thinking_budget=8192):
    """Call Gemini API with structured JSON output and retry on 429."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    body = {
        "contents": [{"role": "user", "parts": [{"text": user_prompt}]}],
        "systemInstruction": {"parts": [{"text": system_prompt}]},
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": max_tokens,
            "responseMimeType": "application/json",
            "thinkingConfig": {"thinkingBudget": thinking_budget},
        },
    }
    last_err = None
    for attempt in range(GEMINI_MAX_RETRIES + 1):
        resp = requests.post(url, json=body, timeout=300)
        if resp.status_code == 429:
            wait = GEMINI_RETRY_BACKOFF * (2 ** attempt)
            print(f"    Rate limited (429), waiting {wait}s (attempt {attempt + 1}/{GEMINI_MAX_RETRIES + 1})...")
            time.sleep(wait)
            last_err = resp
            continue
        resp.raise_for_status()
        break
    else:
        last_err.raise_for_status()  # raise after all retries exhausted
    content = resp.json()
    parts = content.get("candidates", [{}])[0].get("content", {}).get("parts", [])
    text_resp = "".join(p.get("text", "") for p in parts).strip()

    # Parse JSON — handle markdown fences
    if "```json" in text_resp:
        text_resp = text_resp.split("```json")[1].split("```")[0].strip()
    elif "```" in text_resp:
        text_resp = text_resp.split("```")[1].split("```")[0].strip()

    return json.loads(text_resp)


# --- Validation (Phase 2 lint) ---

def lint_evidence(evidence, control):
    """Run deterministic lint checks on a generated evidence pattern."""
    flags = []

    # --- Artefact checks ---
    artefacts = evidence.get("artefacts", [])
    if not artefacts:
        flags.append("NO_ARTEFACTS: artefacts array is empty")

    has_type_b = False
    for i, art in enumerate(artefacts):
        prefix = f"artefact[{i}]"
        at = art.get("artefact_type")
        if at not in VALID_ARTEFACT_TYPES:
            flags.append(f"INVALID_ENUM: {prefix}.artefact_type '{at}' invalid")
        ac = art.get("artefact_class")
        if ac not in VALID_ARTEFACT_CLASSES:
            flags.append(f"INVALID_ENUM: {prefix}.artefact_class '{ac}' invalid")
        elif ac == "Outcome":
            has_type_b = True
        src = art.get("source")
        if src not in VALID_SOURCES:
            flags.append(f"INVALID_ENUM: {prefix}.source '{src}' invalid")
        lr = art.get("likelihood_ratio")
        if lr not in VALID_LIKELIHOOD_RATIOS:
            flags.append(f"INVALID_ENUM: {prefix}.likelihood_ratio '{lr}' invalid")
        # Type-A should not have High likelihood ratio
        if ac == "Activity" and lr == "High":
            flags.append(f"CONSISTENCY: {prefix} is Activity but likelihood_ratio is High")
        for field in ["title", "what_it_proves"]:
            if not art.get(field):
                flags.append(f"MISSING_FIELD: {prefix}.{field} is empty")

    if not has_type_b:
        flags.append("NO_TYPE_B: no Outcome artefact — every control needs at least one discriminating artefact")

    # --- Judgement checks ---
    judgement = evidence.get("judgement", {})
    needs_j = judgement.get("needs_judgement")
    rationale = judgement.get("judgement_rationale")
    if not rationale:
        flags.append("MISSING_FIELD: judgement_rationale must always be populated")
    drift_cond = judgement.get("drift_conditions")
    if not drift_cond:
        flags.append("MISSING_FIELD: drift_conditions must always be populated")

    if needs_j:
        method = judgement.get("recommended_method")
        if method not in VALID_JUDGEMENT_METHODS:
            flags.append(f"INVALID_ENUM: recommended_method '{method}' invalid")
        for field in ["basis_guidance", "discriminating_question", "drift_signal"]:
            if not judgement.get(field):
                flags.append(f"MISSING_FIELD: judgement.{field} is empty (required when needs_judgement=true)")

    # Consistency: load_bearing_judgement present but needs_judgement=false
    lbj = control.get("load_bearing_judgement")
    if lbj and not needs_j:
        flags.append(f"CONSISTENCY: control has load_bearing_judgement='{lbj}' but needs_judgement=false")

    # --- Strategy checks ---
    strategy = evidence.get("strategy", {})
    vq = strategy.get("voi_quadrant")
    if vq not in VALID_VOI_QUADRANTS:
        flags.append(f"INVALID_ENUM: voi_quadrant '{vq}' invalid")
    es = strategy.get("evidence_standard")
    if es not in VALID_EVIDENCE_STANDARDS:
        flags.append(f"INVALID_ENUM: evidence_standard '{es}' invalid")
    st = strategy.get("staleness_tolerance")
    if st not in VALID_STALENESS_TOLERANCES:
        flags.append(f"INVALID_ENUM: staleness_tolerance '{st}' invalid")
    for field in ["voi_rationale", "recommended_interval", "nature_strategy"]:
        if not strategy.get(field):
            flags.append(f"MISSING_FIELD: strategy.{field} is empty")

    # Deterministic override checks
    blast = control.get("blast_radius")
    info_dist = control.get("info_distance")
    nature = control.get("nature")

    # VoI consistency: Enterprise + Manual should not be Table Stakes
    if blast == "Enterprise" and nature == "Manual" and vq == "Table Stakes":
        flags.append("CONSISTENCY: Enterprise blast_radius + Manual nature but voi_quadrant is Table Stakes")

    # Evidence standard consistency
    if blast == "Enterprise" and es == "Basic":
        flags.append("CONSISTENCY: blast_radius=Enterprise but evidence_standard=Basic")

    # Staleness tolerance consistency
    if info_dist == "Remote" and blast in ("Site", "Enterprise") and st == "High":
        flags.append("CONSISTENCY: Remote info_distance + high blast_radius but staleness_tolerance=High")

    return flags


# --- DuckDB staging table ---

def ensure_staging_table(duck_conn):
    """Create the suggested_evidence staging table if it doesn't exist."""
    duck_conn.execute("""
        CREATE TABLE IF NOT EXISTS suggested_evidence (
            id VARCHAR PRIMARY KEY,
            law_name VARCHAR NOT NULL,
            control_id VARCHAR NOT NULL,
            control_title VARCHAR,
            evidence_json JSON NOT NULL,
            status VARCHAR DEFAULT 'generated',
            validation_flags JSON,
            generation_model VARCHAR,
            generated_at TIMESTAMP DEFAULT current_timestamp,
            base_hash VARCHAR,
            customer_edits JSON
        )
    """)


def store_evidence(duck_conn, law_name, control_id, control_title,
                   evidence, model, flags):
    """Store a single evidence pattern in DuckDB staging table."""
    evidence_id = str(uuid.uuid4())
    status = "validated" if not flags else "flagged"
    base_hash = str(hash(json.dumps(evidence, sort_keys=True)))
    now = datetime.now(timezone.utc).isoformat()
    duck_conn.execute("""
        INSERT INTO suggested_evidence
            (id, law_name, control_id, control_title, evidence_json, status,
             validation_flags, generation_model, generated_at, base_hash)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, [evidence_id, law_name, control_id, control_title,
          json.dumps(evidence), status, json.dumps(flags), model, now, base_hash])


# --- Main pipeline ---

def law_has_evidence(duck_conn, law_name):
    """Check if a law already has generated evidence patterns."""
    try:
        row = duck_conn.execute(
            "SELECT count(*) FROM suggested_evidence WHERE law_name = ?",
            [law_name]
        ).fetchone()
        return row[0] > 0
    except Exception:
        return False


def process_law(law_name, duck_conn, api_key, system_prompt,
                dry_run=False, force=False, no_db=False,
                model=GEMINI_MODEL, thinking_budget=8192):
    """Process a single law: generate evidence patterns, validate, store."""
    print(f"\n{'='*60}", flush=True)
    print(f"Processing: {law_name}", flush=True)

    # 0. Skip if already generated (unless --force or --no-db)
    if not force and not no_db and not dry_run and law_has_evidence(duck_conn, law_name):
        print(f"  SKIP: evidence patterns already exist (use --force to regenerate)", flush=True)
        return {"law": law_name, "skipped": True}

    # 1. Get controls
    controls = get_controls_for_law(duck_conn, law_name)
    if not controls:
        print(f"  SKIP: no controls found in suggested_controls", flush=True)
        return None
    print(f"  Controls: {len(controls)}", flush=True)

    # 2. Get law outline
    outline = get_law_outline(duck_conn, law_name)
    if not outline:
        print(f"  SKIP: law not found in DuckDB legislation table", flush=True)
        return None

    # 3. Assemble prompt
    user_prompt = format_user_prompt(outline, controls)
    token_est = len(user_prompt.split()) * 1.3
    print(f"  Prompt: ~{int(token_est)} tokens (est)", flush=True)

    if dry_run:
        print(f"  DRY RUN — prompt assembled, not calling Gemini")
        out_path = RESULTS_DIR / "dry-run" / f"{law_name}.txt"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(user_prompt)
        print(f"  Saved to: {out_path}")
        return {"law": law_name, "controls": len(controls), "dry_run": True}

    # 4. Call Gemini
    print(f"  Calling {model}...", flush=True)
    try:
        evidence_patterns = call_gemini(api_key, system_prompt, user_prompt,
                                        model=model, thinking_budget=thinking_budget)
    except Exception as e:
        print(f"  ERROR: Gemini call failed: {e}", flush=True)
        return None

    if not isinstance(evidence_patterns, list):
        print(f"  ERROR: expected JSON array, got {type(evidence_patterns)}", flush=True)
        return None
    print(f"  Generated: {len(evidence_patterns)} evidence patterns", flush=True)

    # 5. Save raw output to disk FIRST (survives crashes)
    out_path = RESULTS_DIR / "generated" / f"{law_name}.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    output = {
        "law_name": law_name,
        "controls_count": len(controls),
        "evidence_patterns": evidence_patterns,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "model": model,
    }
    out_path.write_text(json.dumps(output, indent=2))

    # 6. Match patterns to controls by control_index
    idx_to_control = {}
    for idx, item in enumerate(controls, 1):
        idx_to_control[idx] = item

    # 7. Validate and store in DuckDB (skip if --no-db)
    total_flags = 0
    flagged_count = 0
    stored_count = 0

    for pattern in evidence_patterns:
        cidx = pattern.get("control_index")
        if cidx not in idx_to_control:
            print(f"  WARNING: control_index {cidx} not in input controls", flush=True)
            continue

        ctrl_item = idx_to_control[cidx]
        ctrl = ctrl_item["control"]
        ctrl_id = ctrl_item["id"]
        ctrl_title = ctrl.get("title", "Untitled")[:200]

        flags = lint_evidence(pattern, ctrl)
        if flags:
            total_flags += len(flags)
            flagged_count += 1

        if not no_db:
            if force and stored_count == 0:
                duck_conn.execute("DELETE FROM suggested_evidence WHERE law_name = ?", [law_name])
            store_evidence(duck_conn, law_name, ctrl_id, ctrl_title,
                           pattern, model, flags)
        stored_count += 1

    if no_db:
        print(f"  Validated: {stored_count} patterns ({flagged_count} flagged, {total_flags} flags) — NOT stored (--no-db)", flush=True)
    else:
        print(f"  Stored: {stored_count} evidence patterns ({flagged_count} flagged, {total_flags} total flags)", flush=True)

    time.sleep(GEMINI_DELAY)

    return {
        "law": law_name,
        "controls": len(controls),
        "patterns": stored_count,
        "flags": total_flags,
    }


def main():
    parser = argparse.ArgumentParser(description="Generate evidence patterns from compliance controls")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--law", help="Generate for a single law (e.g. UK_uksi_1997_1713)")
    group.add_argument("--family", help="Generate for all laws in a family")
    group.add_argument("--all", action="store_true", help="Generate for all laws with controls")
    parser.add_argument("--qq", action="store_true", help="Scope to QQ applicable laws only")
    parser.add_argument("--dry-run", action="store_true", help="Assemble prompts without calling Gemini")
    parser.add_argument("--limit", type=int, help="Limit number of laws to process")
    parser.add_argument("--force", action="store_true", help="Regenerate even if evidence already exists")
    parser.add_argument("--no-db", action="store_true", help="Skip DuckDB storage (JSON only — for model comparison)")
    parser.add_argument("--model", default=GEMINI_MODEL, help=f"Gemini model (default: {GEMINI_MODEL})")
    parser.add_argument("--thinking", type=int, default=8192, help="Thinking budget tokens (default: 8192)")
    args = parser.parse_args()

    # API key
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key and not args.dry_run:
        import subprocess
        result = subprocess.run(
            ["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
            capture_output=True, text=True
        )
        api_key = result.stdout.strip()
    if not api_key and not args.dry_run:
        print("ERROR: GEMINI_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    # Load prompt
    system_prompt = load_system_prompt()

    # Connect to DuckDB
    duck_conn = duckdb.connect(DUCKDB_PATH)

    # Determine laws to process
    if args.law:
        law_names = [args.law]
    elif args.family:
        law_names = get_laws_with_controls(duck_conn, family=args.family,
                                           qq_only=args.qq, limit=args.limit)
    elif args.all:
        law_names = get_laws_with_controls(duck_conn, qq_only=args.qq,
                                           limit=args.limit)

    # Apply QQ filter for --law mode
    if args.qq and args.law:
        qq_csv = Path("data/sertantai/qq-applicable-laws.csv").read_text().strip()
        qq_set = {n.strip() for n in qq_csv.split(",") if n.strip()}
        law_names = [n for n in law_names if n in qq_set]

    print(f"Laws to process: {len(law_names)}", flush=True)
    if args.model != GEMINI_MODEL:
        print(f"Model: {args.model} (thinking: {args.thinking})", flush=True)
    if args.no_db:
        print(f"Mode: --no-db (JSON only, no DuckDB writes)", flush=True)

    # Ensure staging table
    if not args.no_db:
        ensure_staging_table(duck_conn)

    # Process
    results = []
    for law_name in law_names:
        result = process_law(law_name, duck_conn, api_key, system_prompt,
                             dry_run=args.dry_run, force=args.force,
                             no_db=args.no_db, model=args.model,
                             thinking_budget=args.thinking)
        if result:
            results.append(result)

    # Summary
    skipped = sum(1 for r in results if r.get("skipped"))
    generated = [r for r in results if not r.get("skipped") and not r.get("dry_run")]
    print(f"\n{'='*60}", flush=True)
    print(f"SUMMARY: {len(results)} laws processed ({skipped} skipped, {len(generated)} generated)", flush=True)
    if generated and not args.dry_run:
        total_ctrl = sum(r["controls"] for r in generated)
        total_pat = sum(r.get("patterns", 0) for r in generated)
        total_flags = sum(r.get("flags", 0) for r in generated)
        print(f"  Controls: {total_ctrl}")
        print(f"  Patterns: {total_pat}")
        print(f"  Flags:    {total_flags}")

    duck_conn.close()


if __name__ == "__main__":
    main()
