#!/usr/bin/env /usr/bin/python3
"""Generate golden benchmarks for DRRP pipeline QA.

Sends regulation-level provisions to Gemini for independent DRRP + actor
position classification. Stores results as Parquet on NAS.

Usage:
    # Pilot: one law
    GEMINI_API_KEY="$GEMINI_API_KEY" /usr/bin/python3 scripts/generate_benchmarks.py \
        --law UK_ukpga_1974_37 --family "OH&S"

    # Full family
    GEMINI_API_KEY="$GEMINI_API_KEY" /usr/bin/python3 scripts/generate_benchmarks.py \
        --family "OH&S" --auto-select

    # All QQ families
    GEMINI_API_KEY="$GEMINI_API_KEY" /usr/bin/python3 scripts/generate_benchmarks.py --all
"""

import argparse
import json
import os
import re
import sys
import time
from datetime import datetime, timezone

import google.genai as genai
import lancedb
import pyarrow as pa
import pyarrow.parquet as pq

# ── Constants ───────────────────────────────────────────────────────

REGULATION_TYPES = {"section", "sub_section", "article", "sub_article"}

BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"

BENCHMARK_SCHEMA = pa.schema([
    pa.field("section_id", pa.utf8()),
    pa.field("law_name", pa.utf8()),
    pa.field("family", pa.utf8()),
    pa.field("text", pa.utf8()),
    pa.field("gold_drrp_types", pa.list_(pa.utf8())),
    pa.field("gold_actors", pa.utf8()),  # JSON
    pa.field("gold_reasoning", pa.utf8()),
    pa.field("gold_source", pa.utf8()),
    pa.field("created_at", pa.timestamp("ns")),
])

SYSTEM_PROMPT = """You are an expert legal analyst specialising in UK/EU ESH (Environment, Safety, Health) legislation and Hohfeldian legal relations.

For each provision, classify:
1. DRRP type: Obligation / Liberty / none
   - Obligation: a legal obligation imposed on someone (shall, must, is required to, has a duty)
   - Liberty: a permission, entitlement, or discretionary power granted to someone (may, is entitled to, has a right to, power to)
   - none: definitions, commencement, repeals, cross-references, structural, offence/penalty provisions

   IMPORTANT — classify as 'none' if the provision:
   - References, conditions, or details an obligation/right created ELSEWHERE (e.g., "An employee's right to return under regulation 13 is a right to return—" just details the right from reg 13)
   - Creates an exemption or exception to an obligation (e.g., "A client is not required to comply with this Part where..." is a detail of the obligation, not a new Liberty)
   - Describes a procedural protection (e.g., "No answer... is admissible in evidence" — this is a procedural rule, not a new right)
   - States a consequence without a modal verb (e.g., "is guilty of an offence", "is liable to forfeiture")
   - Only a provision that CREATES a new legal relation counts as Obligation or Liberty

2. For each actor, their Hohfeldian position:
   - active: bears the obligation / exercises the liberty (the doer)
   - counterparty: on the receiving end of the legal relation
   - beneficiary: benefits without a direct legal relation
   - mentioned: referenced but no active legal role
3. Brief reasoning (1-2 sentences)

Respond in JSON only, no markdown:
{"drrp_type": "Obligation|Liberty|none", "actors": [{"label": "...", "position": "active|counterparty|beneficiary|mentioned"}], "reasoning": "..."}"""


# ── Helpers ─────────────────────────────────────────────────────────

def slugify(family):
    """Convert family name to filename-safe slug."""
    s = family.lower().strip()
    s = re.sub(r"[^\w\s-]", "", s)
    s = re.sub(r"[\s_]+", "-", s)
    return s.strip("-")[:60]


def load_provisions(law_name):
    """Load regulation-level provisions for a law from LanceDB."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")
    data = tbl.search().where(
        f"law_name = '{law_name}'"
    ).select(
        ["section_id", "text", "section_type", "actors", "drrp_types"]
    ).limit(10000).to_arrow()

    provisions = []
    for i in range(data.num_rows):
        st = data.column("section_type")[i].as_py() or ""
        if st not in REGULATION_TYPES:
            continue
        text = data.column("text")[i].as_py() or ""
        if len(text) < 20:
            continue

        actors = data.column("actors")[i].as_py() or []
        actor_labels = [
            a.get("label", "?") for a in actors if isinstance(a, dict)
        ]

        provisions.append({
            "section_id": data.column("section_id")[i].as_py(),
            "text": text,
            "actor_labels": actor_labels,
        })

    return provisions


def classify_provision(client, cache_name, provision):
    """Send a single provision to Gemini for classification."""
    prompt = f"""Provision: {provision['section_id']}
Text: {provision['text']}
Actors found by regex: {', '.join(provision['actor_labels']) or 'none'}"""

    try:
        response = client.models.generate_content(
            model="gemini-2.5-flash",
            contents=prompt,
            config={
                "cached_content": cache_name,
                "temperature": 0.1,
                "max_output_tokens": 512,
                "thinking_config": {"thinking_budget": 256},
            },
        )
        text = response.text or ""
        # Strip markdown fences
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0].strip()
        elif "```" in text:
            text = text.split("```")[1].split("```")[0].strip()
        return json.loads(text)
    except Exception as e:
        print(f"    Error classifying {provision['section_id']}: {e}")
        return None


def create_cache(client, law_name, provisions):
    """Create a Gemini cache with the law's provisions as context."""
    # Build context: all provision texts
    context_parts = [f"## {p['section_id']}\n{p['text']}" for p in provisions[:200]]
    context = f"# {law_name}\n\n" + "\n\n".join(context_parts)

    cache = client.caches.create(
        model="gemini-2.5-flash",
        config={
            "display_name": f"benchmark-{law_name}",
            "system_instruction": SYSTEM_PROMPT,
            "contents": [
                {"role": "user", "parts": [{"text": f"Here is the full text of {law_name} for reference:\n\n{context}"}]},
                {"role": "model", "parts": [{"text": f"I have the text of {law_name}. Ready to classify individual provisions."}]},
            ],
            "ttl": "3600s",
        },
    )
    return cache.name


def generate_benchmark(client, law_name, family, provisions):
    """Generate benchmark for a single law."""
    print(f"\n  {law_name}: {len(provisions)} regulation-level provisions")

    # Create cache for this law
    cache_name = create_cache(client, law_name, provisions)
    print(f"    Cache created: {cache_name}")

    now = datetime.now(timezone.utc)
    records = {
        "section_id": [],
        "law_name": [],
        "family": [],
        "text": [],
        "gold_drrp_types": [],
        "gold_actors": [],
        "gold_reasoning": [],
        "gold_source": [],
        "created_at": [],
    }

    classified = 0
    for i, prov in enumerate(provisions):
        result = classify_provision(client, cache_name, prov)
        if result is None:
            continue

        drrp_type = result.get("drrp_type", "none")
        drrp_list = [drrp_type] if drrp_type != "none" else []
        actors_json = json.dumps(result.get("actors", []))
        reasoning = result.get("reasoning", "")

        records["section_id"].append(prov["section_id"])
        records["law_name"].append(law_name)
        records["family"].append(family)
        records["text"].append(prov["text"])
        records["gold_drrp_types"].append(drrp_list)
        records["gold_actors"].append(actors_json)
        records["gold_reasoning"].append(reasoning)
        records["gold_source"].append("gemini")
        records["created_at"].append(now)

        classified += 1
        if (i + 1) % 25 == 0:
            print(f"    {i + 1}/{len(provisions)} classified...")

        # Rate limit: ~2 requests/sec to stay within Gemini limits
        time.sleep(0.5)

    print(f"    {classified}/{len(provisions)} classified")
    return records


# ── Main ────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Generate golden benchmarks")
    parser.add_argument("--law", help="Specific law to benchmark")
    parser.add_argument("--family", help="Family name (for output path)")
    parser.add_argument("--output", help="Override output path")
    parser.add_argument("--max-provisions", type=int, default=150,
                        help="Max provisions per law (random sample if exceeded)")
    args = parser.parse_args()

    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        print("Error: GEMINI_API_KEY not set")
        sys.exit(1)

    client = genai.Client(api_key=api_key)

    if not args.law:
        print("Error: --law is required")
        sys.exit(1)

    family = args.family or "unknown"

    # Load provisions
    print(f"Loading provisions for {args.law}...")
    provisions = load_provisions(args.law)
    print(f"  {len(provisions)} regulation-level provisions")

    if not provisions:
        print("No provisions found.")
        sys.exit(1)

    # Random sample if too many provisions
    import random
    if len(provisions) > args.max_provisions:
        random.seed(42)  # reproducible
        provisions = random.sample(provisions, args.max_provisions)
        print(f"  Sampled {len(provisions)} provisions (--max-provisions {args.max_provisions})")

    # Generate benchmark
    records = generate_benchmark(client, args.law, family, provisions)

    # Write Parquet
    table = pa.table(records, schema=BENCHMARK_SCHEMA)

    output_path = args.output
    if not output_path:
        os.makedirs(BENCHMARK_DIR, exist_ok=True)
        slug = slugify(family)
        output_path = f"{BENCHMARK_DIR}/tier2-{slug}-{args.law}.parquet"

    pq.write_table(table, output_path)
    rows = table.num_rows
    size = os.path.getsize(output_path)
    print(f"\n  Written: {output_path} ({rows} rows, {size:,} bytes)")


if __name__ == "__main__":
    main()
