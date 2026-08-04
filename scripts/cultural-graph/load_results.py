#!/usr/bin/env python3
"""Load cultural graph inference results into DuckDB.

Reads results JSONL, normalises edge types from config, appends to
DuckDB tables, and recomputes site profiles.

Usage:
    /usr/bin/python3 scripts/cultural-graph/load_results.py --input data/qq/cultural-graph/results/Redactor_2027-clean-results.jsonl
    /usr/bin/python3 scripts/cultural-graph/load_results.py --input data/qq/cultural-graph/results/Redactor_2027-clean-results.jsonl --dry-run
"""

import argparse
import json
import sys
from pathlib import Path

import duckdb
import yaml

DUCKDB_PATH = Path("data/cultural-graph.duckdb")
CONFIG_PATH = Path("scripts/cultural-graph/config/normalise.yaml")


def load_config():
    """Load normalisation config."""
    with open(CONFIG_PATH) as f:
        return yaml.safe_load(f)


def load_results(path):
    """Load results JSONL."""
    records = []
    with open(path) as f:
        for line in f:
            records.append(json.loads(line))
    return records


def normalise_and_flatten(records, config):
    """Normalise edge/entity types and flatten into table rows."""
    edge_norm = config["edge_normalise"]
    entity_norm = config["entity_normalise"]
    valid_edges = set(config["valid_edge_types"])
    valid_entities = set(config["valid_entity_types"])

    narr_rows = []
    ent_rows = []
    edge_rows = []
    stats = {"normalised": 0, "dropped": 0, "valid": 0, "invalid": 0}

    for r in records:
        if not r["valid"]:
            stats["invalid"] += 1
            continue
        stats["valid"] += 1

        ext = r["extraction"]
        ents = [e for e in ext.get("entities", []) if isinstance(e, dict)]
        rels = [rel for rel in ext.get("relationships", []) if isinstance(rel, dict)]

        cultural_count = 0
        operational_count = 0

        for rel in rels:
            et = rel.get("edge_type", "unknown")
            if et in edge_norm:
                et = edge_norm[et]
                stats["normalised"] += 1
            elif et not in valid_edges:
                stats["dropped"] += 1
                continue
            is_cultural = et != "operational"
            if is_cultural:
                cultural_count += 1
            else:
                operational_count += 1
            edge_rows.append((
                r["id"], rel.get("source", ""), rel.get("target", ""),
                et, rel.get("detail", ""), is_cultural
            ))

        for e in ents:
            et = e.get("type", "unknown")
            if et in entity_norm:
                et = entity_norm[et]
            elif et not in valid_entities:
                et = "unknown"
            ent_rows.append((r["id"], e.get("text", ""), et))

        narr_rows.append((
            r["id"], r["site"], r["report_type"],
            int(r["fy"]) if r.get("fy") else None,
            r.get("sector", ""), r.get("sub_sector", ""),
            r["word_count"], len(ents), cultural_count, operational_count,
            r["extracted_at"]
        ))

    return narr_rows, ent_rows, edge_rows, stats


def check_dupes(con, narr_rows):
    """Check for duplicate IDs already in DuckDB."""
    new_ids = {r[0] for r in narr_rows}
    existing = set(con.execute("SELECT id FROM narratives").fetchdf()["id"].tolist())
    dupes = new_ids & existing
    return dupes


def load_into_duckdb(narr_rows, ent_rows, edge_rows, dry_run=False):
    """Append rows to DuckDB tables."""
    con = duckdb.connect(str(DUCKDB_PATH))

    # Check for duplicates
    dupes = check_dupes(con, narr_rows)
    if dupes:
        print(f"WARNING: {len(dupes)} duplicate IDs found — skipping these")
        narr_rows = [r for r in narr_rows if r[0] not in dupes]
        ent_rows = [r for r in ent_rows if r[0] not in dupes]
        edge_rows = [r for r in edge_rows if r[0] not in dupes]

    if dry_run:
        print(f"\nDRY RUN — would insert:")
        print(f"  Narratives: {len(narr_rows)}")
        print(f"  Entities: {len(ent_rows)}")
        print(f"  Edges: {len(edge_rows)}")
        con.close()
        return

    # Get counts before
    before = {
        "narratives": con.execute("SELECT count(*) FROM narratives").fetchone()[0],
        "entities": con.execute("SELECT count(*) FROM entities").fetchone()[0],
        "edges": con.execute("SELECT count(*) FROM edges").fetchone()[0],
    }

    # Insert
    if narr_rows:
        con.executemany("INSERT INTO narratives VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", narr_rows)
    if ent_rows:
        con.executemany("INSERT INTO entities VALUES (?, ?, ?)", ent_rows)
    if edge_rows:
        con.executemany("INSERT INTO edges VALUES (?, ?, ?, ?, ?, ?)", edge_rows)

    # Get counts after
    after = {
        "narratives": con.execute("SELECT count(*) FROM narratives").fetchone()[0],
        "entities": con.execute("SELECT count(*) FROM entities").fetchone()[0],
        "edges": con.execute("SELECT count(*) FROM edges").fetchone()[0],
    }

    print(f"\nDuckDB updated:")
    for table in ["narratives", "entities", "edges"]:
        print(f"  {table}: {before[table]} → {after[table]} (+{after[table] - before[table]})")

    # Quick site profile for the new data
    print(f"\nNew data site summary:")
    new_ids_str = ",".join(f"'{r[0]}'" for r in narr_rows[:1000])
    if narr_rows:
        df = con.execute(f"""
            SELECT
                n.site,
                count(DISTINCT n.id) AS narratives,
                ROUND(AVG(n.cultural_edge_count), 1) AS avg_cultural,
                ROUND(100.0 * count(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with'))
                    / NULLIF(count(*) FILTER (e.is_cultural), 0), 0) AS "voice%",
                ROUND(100.0 * count(*) FILTER (e.edge_type IN ('normalises', 'adapts-to'))
                    / NULLIF(count(*) FILTER (e.is_cultural), 0), 0) AS "drift%"
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE n.fy = (SELECT MAX(fy) FROM narratives)
            GROUP BY n.site
            HAVING count(DISTINCT n.id) >= 5
            ORDER BY count(DISTINCT n.id) DESC
            LIMIT 15
        """).fetchdf()
        print(df.to_string(index=False))

    con.close()


def main():
    parser = argparse.ArgumentParser(description="Load cultural graph results into DuckDB")
    parser.add_argument("--input", required=True, help="Results JSONL from RunPod inference")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be loaded, don't write")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"ERROR: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    config = load_config()
    print(f"Config: {len(config['edge_normalise'])} edge normalisations, "
          f"{len(config['entity_normalise'])} entity normalisations")

    records = load_results(input_path)
    print(f"Loaded {len(records)} records from {input_path}")

    narr_rows, ent_rows, edge_rows, stats = normalise_and_flatten(records, config)
    print(f"\nNormalisation stats:")
    print(f"  Valid records: {stats['valid']}")
    print(f"  Invalid records: {stats['invalid']}")
    print(f"  Edge types normalised: {stats['normalised']}")
    print(f"  Edge types dropped: {stats['dropped']}")

    load_into_duckdb(narr_rows, ent_rows, edge_rows, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
