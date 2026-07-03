#!/usr/bin/env /usr/bin/python3
"""Surface missing actors from benchmark or LanceDB provisions.

Finds provisions where the pipeline returned empty drrp_types despite
having a modal verb, then extracts the grammatical subject (likely the
missing duty-bearer). Groups by entity name and counts occurrences
across families to prioritise dictionary additions.

Usage:
    # Against golden benchmarks (requires NAS mount)
    /usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py

    # Against a specific family in LanceDB (no benchmarks needed)
    /usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --family "ENERGY"

    # Show provision text for review
    /usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --text

    # Only show entities appearing 2+ times
    /usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --min-count 2
"""

import argparse
import glob
import json
import os
import re
import sys
from collections import Counter, defaultdict

import lancedb

HAS_MODAL = re.compile(r"(?i)\bshall\b|\bmust\b|\bmay\b|\brequir|\bensur")
# Extract the subject: text from start-of-sentence to first modal
SUBJECT_RE = re.compile(
    r"(?:^|(?<=[.;—]))\s*"  # sentence start
    r"((?:[A-Z][^.;—]*?|[a-z][^.;—]*?))"  # subject phrase
    r"\s+(?:shall|must|may|is required to|has a duty)\b",
    re.IGNORECASE,
)
# Simpler fallback: last N words before modal
BEFORE_MODAL = re.compile(r"(\S+(?:\s+\S+){0,5})\s+(?:shall|must|may)\b", re.IGNORECASE)


def load_actor_dictionary():
    """Load known actor labels from the unified YAML dictionary."""
    import yaml

    with open("crates/fractalaw-core/data/actor-dictionary.yaml") as f:
        lines = [l for l in f if not l.startswith("#")]
        actors = yaml.safe_load("".join(lines))

    labels = set()
    keywords = set()
    for a in actors:
        labels.add(a["label"])
        for pat in a.get("regex_patterns", []):
            # Extract bare words from the pattern for fuzzy matching
            for word in re.findall(r"[A-Za-z]{3,}", pat):
                keywords.add(word.lower())
        for t in a.get("triggers", []):
            keywords.add(t.lower())
    return labels, keywords


def extract_subject(text):
    """Extract the grammatical subject before the first modal verb."""
    # Try structured regex first
    m = SUBJECT_RE.search(text)
    if m:
        subj = m.group(1).strip()
        # Clean up leading conjunctions, paragraph markers
        subj = re.sub(r"^(?:\([a-z0-9]+\)\s*|(?:and|or|but)\s+)", "", subj)
        return subj

    # Fallback: words before modal
    m = BEFORE_MODAL.search(text)
    if m:
        return m.group(1).strip()

    return None


def normalise_entity(subject):
    """Normalise a subject phrase to a potential entity name."""
    if not subject:
        return None

    # Remove common preamble
    s = re.sub(
        r"^(?:Subject to .*?,\s*|Where .*?,\s*|If .*?,\s*|In .*?,\s*|For .*?,\s*)",
        "",
        subject,
        flags=re.IGNORECASE,
    )
    s = s.strip()

    # Remove leading articles and qualifiers
    s = re.sub(
        r"^(?:the|a|an|each|every|any|no|that|this|such)\s+",
        "",
        s,
        flags=re.IGNORECASE,
    )

    # Remove trailing clause fragments
    s = re.sub(r"\s+(?:who|which|that|under|in|of|for)\b.*$", "", s)

    # Lowercase and trim
    s = s.strip().lower()

    # Skip if too short or too long (not a real entity)
    if len(s) < 3 or len(s) > 60:
        return None

    # Skip if it's a thing, not a person/org
    thing_words = [
        "notice",
        "information",
        "report",
        "plan",
        "assessment",
        "application",
        "appeal",
        "order",
        "regulation",
        "provision",
        "requirement",
        "procedure",
        "arrangement",
        "document",
        "certificate",
        "record",
        "register",
        "direction",
        "fee",
        "penalty",
        "offence",
        "fine",
        "amount",
        "sum",
        "cost",
    ]
    if any(s.startswith(w) or s == w for w in thing_words):
        return None

    return s


def surface_from_benchmarks(known_keywords, show_text=False):
    """Surface missing actors from golden benchmark provisions."""
    import pyarrow.parquet as pq

    BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"
    if not os.path.isdir(BENCHMARK_DIR):
        print(f"NAS not mounted at {BENCHMARK_DIR}")
        return {}

    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    # Load gold
    gold = {}
    for f in sorted(glob.glob(os.path.join(BENCHMARK_DIR, "tier2-*.parquet"))):
        t = pq.read_table(f)
        for i in range(t.num_rows):
            sid = t.column("section_id")[i].as_py()
            drrp = t.column("gold_drrp_types")[i].as_py() or []
            if drrp:  # only provisions with gold DRRP
                gold[sid] = {
                    "drrp": drrp[0],
                    "family": t.column("family")[i].as_py() or "",
                }

    entities = defaultdict(lambda: {"count": 0, "families": set(), "examples": []})

    for sid, g in gold.items():
        data = (
            tbl.search()
            .where(f"section_id = '{sid}'")
            .select(["drrp_types", "text", "governed_actors", "government_actors"])
            .limit(1)
            .to_arrow()
        )
        if data.num_rows == 0:
            continue

        pipe_drrp = data.column("drrp_types")[0].as_py() or []
        if pipe_drrp:
            continue  # pipeline already classified

        text = data.column("text")[0].as_py() or ""
        governed = data.column("governed_actors")[0].as_py() or []
        govt = data.column("government_actors")[0].as_py() or []

        if not HAS_MODAL.search(text):
            continue  # no modal — LLM territory

        # Extract subject
        subject = extract_subject(text)
        entity = normalise_entity(subject)

        if not entity:
            continue

        # Skip if already in dictionary
        if entity in known_keywords or any(
            kw in entity for kw in known_keywords if len(kw) > 4
        ):
            continue

        entities[entity]["count"] += 1
        entities[entity]["families"].add(g["family"])
        if len(entities[entity]["examples"]) < 3:
            entities[entity]["examples"].append(
                {
                    "sid": sid,
                    "text": text[:200],
                    "governed": governed,
                    "government": govt,
                    "gold_drrp": g["drrp"],
                }
            )

    return entities


def surface_from_lancedb(known_keywords, family_filter=None, show_text=False):
    """Surface missing actors from LanceDB provisions with no DRRP."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    filt = "drrp_types IS NULL"
    if family_filter:
        # Need to join with DuckDB for family — use law_name prefix instead
        pass  # TODO: family filter via DuckDB join

    data = (
        tbl.search()
        .where(filt)
        .select(["section_id", "text", "governed_actors", "government_actors", "law_name"])
        .limit(50000)
        .to_arrow()
    )

    entities = defaultdict(lambda: {"count": 0, "families": set(), "examples": []})

    for i in range(data.num_rows):
        text = data.column("text")[i].as_py() or ""
        if not HAS_MODAL.search(text):
            continue

        governed = data.column("governed_actors")[i].as_py() or []
        govt = data.column("government_actors")[i].as_py() or []

        subject = extract_subject(text)
        entity = normalise_entity(subject)
        if not entity:
            continue
        if entity in known_keywords or any(
            kw in entity for kw in known_keywords if len(kw) > 4
        ):
            continue

        law = data.column("law_name")[i].as_py() or ""
        sid = data.column("section_id")[i].as_py() or ""

        entities[entity]["count"] += 1
        entities[entity]["families"].add(law)
        if len(entities[entity]["examples"]) < 2:
            entities[entity]["examples"].append(
                {"sid": sid, "text": text[:200], "governed": governed, "government": govt}
            )

    return entities


def main():
    parser = argparse.ArgumentParser(description="Surface missing actors")
    parser.add_argument(
        "--family", help="Filter to a specific law family (LanceDB mode)"
    )
    parser.add_argument(
        "--text", action="store_true", help="Show provision text examples"
    )
    parser.add_argument(
        "--min-count",
        type=int,
        default=1,
        help="Only show entities appearing N+ times",
    )
    parser.add_argument(
        "--source",
        choices=["benchmark", "lancedb", "both"],
        default="benchmark",
        help="Data source to scan",
    )
    args = parser.parse_args()

    print("Loading actor dictionary...")
    labels, keywords = load_actor_dictionary()
    print(f"  {len(labels)} labels, {len(keywords)} keywords\n")

    all_entities = defaultdict(lambda: {"count": 0, "families": set(), "examples": []})

    if args.source in ("benchmark", "both"):
        print("=== Scanning golden benchmarks ===")
        bench_entities = surface_from_benchmarks(keywords, args.text)
        for entity, data in bench_entities.items():
            all_entities[entity]["count"] += data["count"]
            all_entities[entity]["families"] |= data["families"]
            all_entities[entity]["examples"].extend(data["examples"])

    if args.source in ("lancedb", "both"):
        print("=== Scanning LanceDB ===")
        lance_entities = surface_from_lancedb(keywords, args.family, args.text)
        for entity, data in lance_entities.items():
            all_entities[entity]["count"] += data["count"]
            all_entities[entity]["families"] |= data["families"]
            all_entities[entity]["examples"].extend(data["examples"])

    # Filter by min count
    filtered = {
        e: d for e, d in all_entities.items() if d["count"] >= args.min_count
    }

    if not filtered:
        print("\nNo missing actors found.")
        return

    print(f"\n{'=' * 70}")
    print(f"MISSING ACTORS ({len(filtered)} entities, min_count={args.min_count})")
    print(f"{'=' * 70}\n")

    for entity, data in sorted(filtered.items(), key=lambda x: -x[1]["count"]):
        families = ", ".join(sorted(data["families"]))
        print(f"  {entity} ({data['count']}x) — {families}")
        if args.text:
            for ex in data["examples"][:2]:
                print(f"    {ex['sid']}: {ex['text'][:150]}")
            print()


if __name__ == "__main__":
    main()
