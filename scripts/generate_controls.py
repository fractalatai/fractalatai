#!/usr/bin/env python3
"""Generate compliance controls from legal obligations using Gemini Pro.

Queries DuckDB for law metadata and Postgres for governed provisions,
assembles prompts from validated templates, calls Gemini Pro, validates
output, and stores results in DuckDB staging table.

Usage:
    /usr/bin/python3 scripts/generate_controls.py --law UK_uksi_1997_1713
    /usr/bin/python3 scripts/generate_controls.py --law UK_uksi_1997_1713 --dry-run
    /usr/bin/python3 scripts/generate_controls.py --family "OH&S: Occupational / Personal Safety" --limit 5
    /usr/bin/python3 scripts/generate_controls.py --all --significance HIGH,MEDIUM --limit 10
"""

import argparse
import json
import os
import re
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

import duckdb
import psycopg2
import requests

# --- Config ---
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCKDB_PATH = "data/fractalaw.duckdb"
GEMINI_MODEL = "gemini-2.5-pro"
GEMINI_FLASH = "gemini-2.5-flash"
PROMPTS_DIR = Path(".claude/plans/compliance-controls/prompts")
RESULTS_DIR = Path("data/compliance-controls")

# Rate limiting
GEMINI_DELAY = 2.0  # seconds between calls

# Provision filter: exclude these purposes
EXCLUDE_PURPOSES = ["Offence", "Exemption", "Enactment+Citation+Commencement", "Defence+Appeal"]

# Deontic verbs to flag
DEONTIC_VERBS = ["must", "shall", "should", "will ensure", "needs to", "is required to"]

# Judgement terms that should trigger load_bearing_judgement
JUDGEMENT_TERMS = [
    "adequate", "competent", "proportionate", "sufficient", "suitable",
    "effective", "independent", "appropriate", "necessary", "reasonable",
]

VALID_CONTROL_TYPES = {"Preventive", "Detective", "Corrective", "Directive"}
VALID_NATURES = {"Manual", "Automated", "IT-dependent manual"}
VALID_DOMAINS = {"Organisational", "People", "Physical", "Technical"}
VALID_INFO_DISTANCES = {"Direct", "Adjacent", "Mediated", "Remote"}
VALID_BLAST_RADII = {"Local", "Area", "Site", "Enterprise"}
VALID_STRENGTHS = {"Primary", "Supporting", "Ancillary"}


def load_system_prompt():
    path = PROMPTS_DIR / "system-prompt-v1.md"
    return path.read_text()


def load_predicate_prompt():
    path = PROMPTS_DIR / "policy-predicate-prompt-v1.md"
    return path.read_text()


# --- Database queries ---

def get_law_outline(duck_conn, law_name):
    """Get law metadata from DuckDB."""
    row = duck_conn.execute("""
        SELECT name, title, family, sub_family, year, jurisdiction,
               description, body_paras, total_paras, status,
               duty_holder, rights_holder, duty_type,
               fitness_person, fitness_process, fitness_place,
               fitness_plant, fitness_sector, extent_regions,
               explanatory_note
        FROM legislation WHERE name = ?
    """, [law_name]).fetchone()
    if not row:
        return None
    cols = ["name", "title", "family", "sub_family", "year", "jurisdiction",
            "description", "body_paras", "total_paras", "status",
            "duty_holder", "rights_holder", "duty_type",
            "fitness_person", "fitness_process", "fitness_place",
            "fitness_plant", "fitness_sector", "extent_regions",
            "explanatory_note"]
    return dict(zip(cols, row))


def get_laws_by_family(duck_conn, family, significance=None, limit=None):
    """Get law names for a family."""
    sql = "SELECT name FROM legislation WHERE family = ?"
    params = [family]
    if significance:
        # TODO: filter by significance when available at law level
        pass
    sql += " ORDER BY name"
    if limit:
        sql += f" LIMIT {limit}"
    return [r[0] for r in duck_conn.execute(sql, params).fetchall()]


def get_all_laws(duck_conn, limit=None):
    """Get all law names."""
    sql = "SELECT name FROM legislation ORDER BY name"
    if limit:
        sql += f" LIMIT {limit}"
    return [r[0] for r in duck_conn.execute(sql).fetchall()]


def get_governed_provisions(pg_conn, law_name):
    """Get governed obligations from Postgres, filtered for control generation."""
    cur = pg_conn.cursor()
    cur.execute("""
        SELECT section_id, text, drrp_types, governed_actors, purposes,
               significance_overall, clause_refined
        FROM legislation_text
        WHERE law_name = %s
          AND 'Obligation' = ANY(drrp_types)
          AND governed_actors != '{}'
          AND significance_overall IN ('HIGH', 'MEDIUM')
          AND section_id NOT LIKE '%%[%%'
          AND section_id NOT LIKE '%%sch.%%'
          AND EXISTS (
              SELECT 1 FROM unnest(governed_actors) a
              WHERE a NOT LIKE 'Gvt:%%' AND a NOT LIKE 'Spc: Inspector%%'
                AND a NOT LIKE 'Spc: Authorised Person%%' AND a NOT LIKE 'Spc: Assessor%%'
          )
        ORDER BY sort_key
    """, (law_name,))
    rows = cur.fetchall()
    cur.close()

    # Post-filter: exclude offence/exemption/commencement provisions
    filtered = []
    for row in rows:
        purposes = row[4] or []
        if any(p in EXCLUDE_PURPOSES for p in purposes):
            continue
        filtered.append(row)
    return filtered


# --- Prompt assembly ---

def format_user_prompt(law_outline, provisions):
    """Assemble the user prompt from law metadata and provisions."""
    lo = law_outline
    lines = ["## Law Outline\n"]
    lines.append(f"Title: {lo['title']}")
    lines.append(f"Family: {lo['family']}")
    if lo.get("sub_family"):
        lines.append(f"Sub-family: {lo['sub_family']}")
    lines.append(f"Year: {lo['year']}")
    lines.append(f"Jurisdiction: {lo['jurisdiction']}")
    lines.append(f"Status: {lo['status']}")
    if lo.get("description"):
        lines.append(f"Description: {lo['description']}")
    # Explanatory Note: always include when available, truncated for controls prompt
    expl = lo.get("explanatory_note")
    if expl:
        truncated = expl[:500] + ("..." if len(expl) > 500 else "")
        lines.append(f"Explanatory Note: {truncated}")
    if lo.get("duty_holder"):
        holders = lo["duty_holder"]
        if isinstance(holders, list):
            # Filter to non-government
            holders = [h for h in holders if not h.startswith("Gvt:") and not h.startswith("EU:")]
        lines.append(f"Duty holders: {holders}")
    if lo.get("rights_holder"):
        holders = lo["rights_holder"]
        if isinstance(holders, list):
            holders = [h for h in holders if not h.startswith("Gvt:") and not h.startswith("EU:")]
        if holders:
            lines.append(f"Rights holders: {holders}")

    # Fitness dimensions
    for dim in ["fitness_person", "fitness_process", "fitness_place", "fitness_plant", "fitness_sector"]:
        val = lo.get(dim)
        if val and (isinstance(val, list) and len(val) > 0):
            label = dim.replace("fitness_", "Fitness — ").title()
            lines.append(f"{label}: {val}")

    lines.append(f"\n## Governed Obligations ({len(provisions)} provisions, HIGH + MEDIUM significance)\n")

    for row in provisions:
        section_id, text, drrp_types, governed_actors, purposes, significance, clause_refined = row
        # Extract short ref from section_id (everything after the colon)
        short_ref = section_id.split(":", 1)[1] if ":" in section_id else section_id

        sig_label = f" — {significance}" if significance else ""
        actors_str = ", ".join(governed_actors) if governed_actors else ""
        actor_label = f" [{actors_str}]" if actors_str else ""

        lines.append(f"### {short_ref} — Obligation{actor_label}{sig_label}")
        # Truncate very long provision text
        prov_text = text if len(text) <= 500 else text[:500] + "..."
        lines.append(f'"{prov_text}"')
        lines.append("")

    lines.append("Generate candidate controls for this law.")
    lines.append("- Indicative mood only — no 'must', 'shall', 'should'")
    lines.append("- Consolidate where the operational mechanism is the same")
    lines.append(f"- Use short section references in linked_provisions (e.g. '{provisions[0][0].split(':',1)[1] if provisions else 'reg.3(1)'}')")

    return "\n".join(lines)


# --- Gemini API ---

def call_gemini(api_key, system_prompt, user_prompt, model=GEMINI_MODEL,
                max_tokens=16384, thinking_budget=8192):
    """Call Gemini API with structured JSON output."""
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
    resp = requests.post(url, json=body, timeout=180)
    resp.raise_for_status()
    content = resp.json()
    parts = content.get("candidates", [{}])[0].get("content", {}).get("parts", [])
    text_resp = "".join(p.get("text", "") for p in parts).strip()

    # Parse JSON
    if "```json" in text_resp:
        text_resp = text_resp.split("```json")[1].split("```")[0].strip()
    elif "```" in text_resp:
        text_resp = text_resp.split("```")[1].split("```")[0].strip()

    return json.loads(text_resp)


# --- Validation (Phase 2 lint) ---

def lint_control(control, provision_refs):
    """Run deterministic lint checks on a generated control. Returns list of flags."""
    flags = []
    title = control.get("title", "")

    # Deontic verb check
    title_lower = title.lower()
    for verb in DEONTIC_VERBS:
        if verb in title_lower:
            flags.append(f"DEONTIC: title contains '{verb}'")

    # Paperwork referent check
    desc = control.get("description", "").lower()
    paperwork_phrases = ["document exists", "record is maintained", "form is completed",
                         "certificate is on file", "log is kept"]
    for phrase in paperwork_phrases:
        if phrase in desc:
            flags.append(f"PAPERWORK: description contains '{phrase}'")

    # Missing judgement check — look in the linked provision texts
    load_bearing = control.get("load_bearing_judgement")
    if not load_bearing:
        combined = (title + " " + desc).lower()
        for term in JUDGEMENT_TERMS:
            if term in combined:
                flags.append(f"JUDGEMENT_MISSING: '{term}' in control text but load_bearing_judgement is null")
                break

    # Provision linkage
    linked = control.get("linked_provisions", [])
    if not linked:
        flags.append("NO_PROVISIONS: linked_provisions is empty")
    for ref in linked:
        if ref not in provision_refs:
            flags.append(f"INVALID_REF: '{ref}' not in input provisions")

    # Enum validation
    ct = control.get("control_type")
    if ct not in VALID_CONTROL_TYPES:
        flags.append(f"INVALID_ENUM: control_type '{ct}' not in {VALID_CONTROL_TYPES}")
    nature = control.get("nature")
    if nature not in VALID_NATURES:
        flags.append(f"INVALID_ENUM: nature '{nature}' not in {VALID_NATURES}")
    domain = control.get("domain")
    if domain not in VALID_DOMAINS:
        flags.append(f"INVALID_ENUM: domain '{domain}' not in {VALID_DOMAINS}")
    info_dist = control.get("info_distance")
    if info_dist not in VALID_INFO_DISTANCES:
        flags.append(f"INVALID_ENUM: info_distance '{info_dist}' not in {VALID_INFO_DISTANCES}")
    blast = control.get("blast_radius")
    if blast not in VALID_BLAST_RADII:
        flags.append(f"INVALID_ENUM: blast_radius '{blast}' not in {VALID_BLAST_RADII}")
    strength = control.get("mapping_strength")
    if strength not in VALID_STRENGTHS:
        flags.append(f"INVALID_ENUM: mapping_strength '{strength}' not in {VALID_STRENGTHS}")

    # Required fields
    for field in ["title", "description", "what_it_checks", "evidence_hint"]:
        if not control.get(field):
            flags.append(f"MISSING_FIELD: '{field}' is empty or missing")

    return flags


# --- DuckDB staging table ---

def ensure_staging_table(duck_conn):
    """Create the suggested_controls staging table if it doesn't exist."""
    duck_conn.execute("""
        CREATE TABLE IF NOT EXISTS suggested_controls (
            id VARCHAR PRIMARY KEY,
            law_name VARCHAR NOT NULL,
            control_type VARCHAR,  -- 'specific' or 'predicate'
            control_json JSON NOT NULL,
            status VARCHAR DEFAULT 'generated',
            validation_flags JSON,
            generation_model VARCHAR,
            generated_at TIMESTAMP DEFAULT current_timestamp,
            base_hash VARCHAR
        )
    """)


def store_controls(duck_conn, law_name, controls, model, validation_results):
    """Store generated controls in DuckDB staging table."""
    ensure_staging_table(duck_conn)
    now = datetime.now(timezone.utc).isoformat()
    for i, control in enumerate(controls):
        control_id = str(uuid.uuid4())
        flags = validation_results[i] if i < len(validation_results) else []
        status = "validated" if not flags else "flagged"
        # Hash for three-way merge
        base_hash = str(hash(json.dumps(control, sort_keys=True)))
        duck_conn.execute("""
            INSERT INTO suggested_controls (id, law_name, control_type, control_json, status,
                                            validation_flags, generation_model, generated_at, base_hash)
            VALUES (?, ?, 'specific', ?, ?, ?, ?, ?, ?)
        """, [control_id, law_name, json.dumps(control), status,
              json.dumps(flags), model, now, base_hash])


def store_predicate(duck_conn, law_name, predicate, model):
    """Store policy predicate in DuckDB staging table."""
    ensure_staging_table(duck_conn)
    now = datetime.now(timezone.utc).isoformat()
    control_id = str(uuid.uuid4())
    base_hash = str(hash(json.dumps(predicate, sort_keys=True)))
    duck_conn.execute("""
        INSERT INTO suggested_controls (id, law_name, control_type, control_json, status,
                                        validation_flags, generation_model, generated_at, base_hash)
        VALUES (?, ?, 'predicate', ?, 'validated', '[]', ?, ?, ?)
    """, [control_id, law_name, json.dumps(predicate), model, now, base_hash])


# --- Main pipeline ---

def law_has_controls(duck_conn, law_name):
    """Check if a law already has generated controls in the staging table."""
    try:
        row = duck_conn.execute(
            "SELECT count(*) FROM suggested_controls WHERE law_name = ? AND control_type = 'specific'",
            [law_name]
        ).fetchone()
        return row[0] > 0
    except Exception:
        return False


def process_law(law_name, duck_conn, pg_conn, api_key, system_prompt, predicate_prompt,
                dry_run=False, skip_predicate=False, force=False):
    """Process a single law: generate controls, validate, store."""
    print(f"\n{'='*60}")
    print(f"Processing: {law_name}")

    # 0. Skip if already generated (unless --force)
    if not force and not dry_run and law_has_controls(duck_conn, law_name):
        print(f"  SKIP: controls already exist (use --force to regenerate)")
        return {"law": law_name, "skipped": True}

    # 1. Get law outline
    outline = get_law_outline(duck_conn, law_name)
    if not outline:
        print(f"  SKIP: law not found in DuckDB")
        return None

    # 2. Get governed provisions
    provisions = get_governed_provisions(pg_conn, law_name)
    if not provisions:
        print(f"  SKIP: no governed obligations found")
        return None
    print(f"  Provisions: {len(provisions)} governed obligations")

    # 3. Build provision ref set (short refs for validation)
    provision_refs = set()
    for row in provisions:
        sid = row[0]
        short = sid.split(":", 1)[1] if ":" in sid else sid
        provision_refs.add(short)

    # 4. Assemble prompt
    user_prompt = format_user_prompt(outline, provisions)
    token_est = len(user_prompt.split()) * 1.3  # rough token estimate
    print(f"  Prompt: ~{int(token_est)} tokens (est)")

    if dry_run:
        print(f"  DRY RUN — prompt assembled, not calling Gemini")
        # Save prompt to file
        out_path = RESULTS_DIR / "dry-run" / f"{law_name}.txt"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(user_prompt)
        print(f"  Saved to: {out_path}")
        return {"law": law_name, "provisions": len(provisions), "dry_run": True}

    # 5. If --force, clear existing controls for this law
    if force:
        duck_conn.execute("DELETE FROM suggested_controls WHERE law_name = ?", [law_name])

    # 6. Call Gemini Pro
    print(f"  Calling Gemini Pro...")
    try:
        controls = call_gemini(api_key, system_prompt, user_prompt)
    except Exception as e:
        print(f"  ERROR: Gemini call failed: {e}")
        return None

    if not isinstance(controls, list):
        print(f"  ERROR: expected JSON array, got {type(controls)}")
        return None
    print(f"  Generated: {len(controls)} controls")

    # 6. Validate (Phase 2 lint)
    validation_results = []
    total_flags = 0
    for ctrl in controls:
        flags = lint_control(ctrl, provision_refs)
        validation_results.append(flags)
        if flags:
            total_flags += len(flags)
    print(f"  Validation: {total_flags} flags across {sum(1 for f in validation_results if f)} controls")

    # 7. Store
    store_controls(duck_conn, law_name, controls, GEMINI_MODEL, validation_results)
    print(f"  Stored: {len(controls)} controls in suggested_controls")

    # 8. Policy predicate
    if not skip_predicate:
        print(f"  Generating policy predicate...")
        control_summary = ", ".join(c.get("title", "")[:60] for c in controls)
        expl_raw = outline.get("explanatory_note")
        if expl_raw:
            # Policy predicate gets more of the note — this is its primary input
            expl_note = expl_raw[:2000] + ("..." if len(expl_raw) > 2000 else "")
        else:
            expl_note = "Not available"
        pred_user = (
            f"Law: {outline['title']}\n"
            f"Description: {outline.get('description', 'Not available')}\n"
            f"Explanatory Note: {expl_note}\n"
            f"Controls generated ({len(controls)}): {control_summary}\n\n"
            f"Generate a policy predicate for this law."
        )
        time.sleep(GEMINI_DELAY)
        try:
            predicate = call_gemini(api_key, predicate_prompt, pred_user,
                                    max_tokens=2048, thinking_budget=2048)
            if isinstance(predicate, list) and len(predicate) == 1:
                predicate = predicate[0]
            store_predicate(duck_conn, law_name, predicate, GEMINI_MODEL)
            print(f"  Predicate: \"{predicate.get('title', '')[:80]}...\"")
        except Exception as e:
            print(f"  PREDICATE ERROR: {e}")

    # 9. Save raw output
    out_path = RESULTS_DIR / "generated" / f"{law_name}.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    output = {
        "law_name": law_name,
        "provisions_count": len(provisions),
        "controls": controls,
        "validation": validation_results,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "model": GEMINI_MODEL,
    }
    out_path.write_text(json.dumps(output, indent=2))

    time.sleep(GEMINI_DELAY)

    return {
        "law": law_name,
        "provisions": len(provisions),
        "controls": len(controls),
        "flags": total_flags,
        "ratio": f"{len(provisions)/len(controls):.1f}:1" if controls else "N/A",
    }


def main():
    parser = argparse.ArgumentParser(description="Generate compliance controls from legal obligations")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--law", help="Generate for a single law (e.g. UK_uksi_1997_1713)")
    group.add_argument("--family", help="Generate for all laws in a family")
    group.add_argument("--all", action="store_true", help="Generate for all laws")
    parser.add_argument("--qq", action="store_true", help="Scope to QQ applicable laws only")
    parser.add_argument("--dry-run", action="store_true", help="Assemble prompts without calling Gemini")
    parser.add_argument("--limit", type=int, help="Limit number of laws to process")
    parser.add_argument("--skip-predicate", action="store_true", help="Skip policy predicate generation")
    parser.add_argument("--force", action="store_true", help="Regenerate even if controls already exist")
    args = parser.parse_args()

    # API key
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key and not args.dry_run:
        # Try sourcing from bashrc
        import subprocess
        result = subprocess.run(
            ["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
            capture_output=True, text=True
        )
        api_key = result.stdout.strip()
    if not api_key and not args.dry_run:
        print("ERROR: GEMINI_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    # Load prompts
    system_prompt = load_system_prompt()
    predicate_prompt = load_predicate_prompt()

    # Connect to databases
    duck_conn = duckdb.connect(DUCKDB_PATH)
    pg_conn = psycopg2.connect(PG_DSN)

    # Determine laws to process
    if args.law:
        law_names = [args.law]
    elif args.family:
        law_names = get_laws_by_family(duck_conn, args.family, limit=args.limit)
    elif args.all:
        law_names = get_all_laws(duck_conn, limit=args.limit)

    # Scope to QQ applicable laws if requested
    if args.qq:
        qq_csv = Path("data/sertantai/qq-applicable-laws.csv").read_text().strip()
        qq_set = {n.strip() for n in qq_csv.split(",") if n.strip()}
        before = len(law_names)
        law_names = [n for n in law_names if n in qq_set]
        print(f"Laws to process: {len(law_names)} (scoped to QQ from {before})")
    else:
        print(f"Laws to process: {len(law_names)}")

    # Ensure staging table exists
    ensure_staging_table(duck_conn)

    # Process
    results = []
    for law_name in law_names:
        result = process_law(
            law_name, duck_conn, pg_conn, api_key, system_prompt, predicate_prompt,
            dry_run=args.dry_run, skip_predicate=args.skip_predicate,
            force=args.force,
        )
        if result:
            results.append(result)

    # Summary
    skipped = sum(1 for r in results if r.get("skipped"))
    generated = [r for r in results if not r.get("skipped") and not r.get("dry_run")]
    print(f"\n{'='*60}")
    print(f"SUMMARY: {len(results)} laws processed ({skipped} skipped, {len(generated)} generated)")
    if generated and not args.dry_run:
        total_prov = sum(r["provisions"] for r in generated)
        total_ctrl = sum(r.get("controls", 0) for r in generated)
        total_flags = sum(r.get("flags", 0) for r in generated)
        print(f"  Provisions: {total_prov}")
        print(f"  Controls:   {total_ctrl}")
        print(f"  Ratio:      {total_prov/total_ctrl:.1f}:1" if total_ctrl else "  Ratio: N/A")
        print(f"  Flags:      {total_flags}")

    duck_conn.close()
    pg_conn.close()


if __name__ == "__main__":
    main()
