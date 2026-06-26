#!/usr/bin/env /usr/bin/python3
"""Compare pipeline output against golden benchmarks.

Loads Parquet benchmarks from NAS, queries the provision store for the same
provisions, and reports metrics for:
  1. DRRP type accuracy (Obligation/Liberty/none)
  2. Actor position accuracy (active/counterparty/beneficiary/mentioned)
  3. Legal relation accuracy (the composite: type + actor + position)

A legal relation is correct when BOTH the DRRP type AND actor positions match.
An Obligation with the active actor classified as 'mentioned' is a FAIL —
the duty has no holder.

Usage:
    /usr/bin/python3 scripts/benchmark_report.py                     # all benchmarks
    /usr/bin/python3 scripts/benchmark_report.py --family "OH&S"     # one family
    /usr/bin/python3 scripts/benchmark_report.py --mismatches 20     # show top N
    /usr/bin/python3 scripts/benchmark_report.py --pg postgres://... # read from Postgres
"""

import argparse
import glob
import json
import os
import re
import sys
from collections import Counter


BENCHMARK_DIR = "data/benchmarks"

# Map natural-language gold labels to canonical pipeline labels
LABEL_ALIASES = {
    "employer": "Org: Employer",
    "employee": "Ind: Employee",
    "person": "Ind: Person",
    "any person": "Ind: Person",
    "responsible person": "Ind: Responsible Person",
    "inspector": "Spc: Inspector",
    "hse": "Gvt: Agency: Health and Safety Executive",
    "secretary of state": "Gvt: Minister",
    "local authority": "Gvt: Authority: Local",
    "enforcing authority": "Gvt: Authority: Enforcement",
    "self-employed": "Ind: Self-Employed",
    "occupier": "Ind: Occupier",
    "manufacturer": "Org: Manufacturer",
    "supplier": "Org: Supplier",
    "designer": "Org: Designer",
    "importer": "Org: Importer",
    "installer": "Org: Installer",
    "contractor": "Org: Contractor",
    "owner": "Ind: Owner",
    "duty holder": "Org: Duty Holder",
}


def normalise_label(label):
    """Normalise a gold label to match pipeline canonical form."""
    low = label.strip().lower()
    if low in LABEL_ALIASES:
        return LABEL_ALIASES[low]
    # Already canonical?
    if ":" in label:
        return label.strip()
    return label.strip()


def load_benchmarks(family_filter=None):
    """Load all benchmark Parquet files."""
    import pyarrow as pa
    import pyarrow.parquet as pq

    pattern = os.path.join(BENCHMARK_DIR, "tier2-*.parquet")
    files = sorted(glob.glob(pattern))

    if not files:
        # Try NAS with padding fix
        nas_dir = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"
        files = sorted(glob.glob(os.path.join(nas_dir, "tier2-*.parquet")))

    if not files:
        print(f"No benchmark files found in {BENCHMARK_DIR} or NAS")
        sys.exit(1)

    tables = []
    for f in files:
        # Fix NAS block-padding if needed
        with open(f, "rb") as fh:
            data = fh.read()
        idx = data.rfind(b"PAR1")
        if idx == -1:
            print(f"  SKIP: {os.path.basename(f)} (no PAR1 magic)")
            continue
        if idx + 4 < len(data):
            # Write fixed version to temp
            import tempfile
            tmp = tempfile.NamedTemporaryFile(suffix=".parquet", delete=False)
            tmp.write(data[: idx + 4])
            tmp.close()
            f = tmp.name

        t = pq.read_table(f)
        if family_filter:
            slug = family_filter.lower().replace(" ", "")
            fname = os.path.basename(f).lower()
            if slug not in fname:
                continue
        tables.append(t)
        law = t.column("law_name")[0].as_py() if t.num_rows > 0 else "?"
        print(f"  {law}: {t.num_rows} provisions")

    if not tables:
        print(f"No benchmarks matched filter '{family_filter}'")
        sys.exit(1)

    return pa.concat_tables(tables)


def load_pipeline_pg(section_ids, pg_url):
    """Load pipeline results from Postgres."""
    import psycopg2

    conn = psycopg2.connect(pg_url)
    cur = conn.cursor()

    pipeline = {}
    for i in range(0, len(section_ids), 500):
        chunk = section_ids[i : i + 500]
        placeholders = ",".join(["%s"] * len(chunk))
        cur.execute(
            f"SELECT section_id, drrp_types, actors, extraction_method "
            f"FROM legislation_text WHERE section_id IN ({placeholders})",
            chunk,
        )
        for row in cur.fetchall():
            sid, drrp_types, actors_json, method = row
            actors = []
            if actors_json:
                for a in (actors_json if isinstance(actors_json, list) else json.loads(actors_json)):
                    if isinstance(a, dict):
                        actors.append({
                            "label": a.get("label", ""),
                            "position": a.get("position", ""),
                        })
            pipeline[sid] = {
                "drrp": drrp_types or [],
                "actors": actors,
                "method": method or "",
            }

    cur.close()
    conn.close()
    return pipeline


def load_pipeline_lance(section_ids):
    """Load pipeline results from LanceDB."""
    import lancedb

    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    pipeline = {}
    for i in range(0, len(section_ids), 500):
        chunk = section_ids[i : i + 500]
        filter_expr = " OR ".join([f"section_id = '{sid}'" for sid in chunk])
        data = (
            tbl.search()
            .where(filter_expr)
            .select(["section_id", "drrp_types", "actors", "extraction_method"])
            .limit(len(chunk) + 10)
            .to_arrow()
        )
        for j in range(data.num_rows):
            sid = data.column("section_id")[j].as_py()
            actors = data.column("actors")[j].as_py() or []
            pipeline[sid] = {
                "drrp": data.column("drrp_types")[j].as_py() or [],
                "actors": [
                    {"label": a.get("label", ""), "position": a.get("position", "")}
                    for a in actors
                    if isinstance(a, dict)
                ],
                "method": data.column("extraction_method")[j].as_py() or "",
            }
    return pipeline


def compute_metrics(gold, pipeline):
    """Compute DRRP, position, and legal relation metrics."""
    drrp_confusion = Counter()
    pos_confusion = Counter()
    relation_results = {"correct": 0, "wrong_type": 0, "wrong_position": 0, "both_wrong": 0, "missing": 0}
    mismatches = []
    pos_mismatches = []

    matched = 0
    for sid, g in gold.items():
        if sid not in pipeline:
            relation_results["missing"] += 1
            continue
        matched += 1
        p = pipeline[sid]

        # --- DRRP type ---
        g_type = g["drrp"][0] if g["drrp"] else "none"
        p_type = p["drrp"][0] if p["drrp"] else "none"
        drrp_confusion[(g_type, p_type)] += 1
        type_correct = g_type == p_type

        if not type_correct:
            mismatches.append({
                "section_id": sid,
                "gold_drrp": g_type,
                "pipeline_drrp": p_type,
                "method": p["method"],
                "reasoning": g.get("reasoning", ""),
            })

        # --- Actor positions ---
        # Normalise gold labels to canonical form for matching
        g_actors = {normalise_label(a["label"]): a["position"] for a in g.get("actors", [])}
        p_actors = {a["label"]: a["position"] for a in p["actors"]}

        position_correct = True
        for label in g_actors:
            g_pos = g_actors[label]
            p_pos = p_actors.get(label, "missing")
            if p_pos == "missing":
                # Try fuzzy: check if any pipeline label contains the gold label
                for p_label, p_position in p_actors.items():
                    if label.lower() in p_label.lower() or p_label.lower() in label.lower():
                        p_pos = p_position
                        break
            if p_pos != "missing":
                pos_confusion[(g_pos, p_pos)] += 1
                if g_pos != p_pos:
                    position_correct = False
                    pos_mismatches.append({
                        "section_id": sid,
                        "actor": label,
                        "gold_position": g_pos,
                        "pipeline_position": p_pos,
                        "method": p["method"],
                    })
            else:
                position_correct = False

        # --- Legal relation (composite) ---
        if type_correct and position_correct:
            relation_results["correct"] += 1
        elif not type_correct and not position_correct:
            relation_results["both_wrong"] += 1
        elif not type_correct:
            relation_results["wrong_type"] += 1
        else:
            relation_results["wrong_position"] += 1

    return matched, drrp_confusion, pos_confusion, relation_results, mismatches, pos_mismatches


def print_confusion_matrix(confusion, title):
    """Print a confusion matrix."""
    types = sorted(set(t for pair in confusion for t in pair))
    if not types:
        return

    print(f"\n{title}")
    col_width = max(max(len(t) for t in types) + 2, 14)
    header = f"{'gold↓ pipe→':>{col_width}}"
    for t in types:
        header += f"{t:>{col_width}}"
    print(header)

    for g in types:
        row = f"{g:>{col_width}}"
        for p in types:
            c = confusion.get((g, p), 0)
            row += f"{c:>{col_width}}"
        print(row)


def print_per_class_metrics(confusion, title):
    """Print per-class precision, recall, F1."""
    types = sorted(set(t for pair in confusion for t in pair))
    if not types:
        return

    print(f"\n{title}")
    print(f"{'Class':>15s} {'Precision':>10s} {'Recall':>10s} {'F1':>10s} {'Support':>10s}")
    total_tp = total_fp = total_fn = 0

    for cls in types:
        tp = confusion.get((cls, cls), 0)
        fp = sum(confusion.get((g, cls), 0) for g in types if g != cls)
        fn = sum(confusion.get((cls, p), 0) for p in types if p != cls)
        precision = tp / (tp + fp) if (tp + fp) > 0 else 0
        recall = tp / (tp + fn) if (tp + fn) > 0 else 0
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
        support = tp + fn
        print(f"{cls:>15s} {precision:>10.1%} {recall:>10.1%} {f1:>10.1%} {support:>10d}")
        total_tp += tp
        total_fp += fp
        total_fn += fn

    micro_p = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
    micro_r = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0
    micro_f1 = 2 * micro_p * micro_r / (micro_p + micro_r) if (micro_p + micro_r) > 0 else 0
    total = sum(confusion.values())
    print(f"{'micro avg':>15s} {micro_p:>10.1%} {micro_r:>10.1%} {micro_f1:>10.1%} {total:>10d}")


def main():
    parser = argparse.ArgumentParser(description="Benchmark comparison report")
    parser.add_argument("--family", help="Filter benchmarks by family")
    parser.add_argument("--mismatches", type=int, default=10, help="Number of mismatches to show")
    parser.add_argument("--pg", help="Postgres URL (default: LanceDB)")
    args = parser.parse_args()

    print("=== Golden Benchmark Report ===\n")
    print("Loading benchmarks...")
    bench = load_benchmarks(args.family)

    # Build gold dict
    gold = {}
    for i in range(bench.num_rows):
        sid = bench.column("section_id")[i].as_py()
        actors_raw = bench.column("gold_actors")[i].as_py() or "[]"
        gold[sid] = {
            "drrp": bench.column("gold_drrp_types")[i].as_py() or [],
            "actors": json.loads(actors_raw) if isinstance(actors_raw, str) else actors_raw,
            "reasoning": bench.column("gold_reasoning")[i].as_py() or "",
        }

    print(f"\nLoading pipeline results for {len(gold)} provisions...")
    if args.pg:
        pipeline = load_pipeline_pg(list(gold.keys()), args.pg)
    else:
        pipeline = load_pipeline_lance(list(gold.keys()))

    matched, drrp_confusion, pos_confusion, relation, mismatches, pos_mismatches = compute_metrics(gold, pipeline)

    # --- Headline: Legal Relation accuracy ---
    relation_total = sum(relation.values()) - relation["missing"]
    relation_correct = relation["correct"]
    relation_pct = 100 * relation_correct / relation_total if relation_total > 0 else 0

    drrp_total = sum(drrp_confusion.values())
    drrp_correct = sum(drrp_confusion.get((t, t), 0) for t in set(t for pair in drrp_confusion for t in pair))
    drrp_pct = 100 * drrp_correct / drrp_total if drrp_total > 0 else 0

    pos_total = sum(pos_confusion.values())
    pos_correct = sum(pos_confusion.get((t, t), 0) for t in set(t for pair in pos_confusion for t in pair))
    pos_pct = 100 * pos_correct / pos_total if pos_total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"Benchmark: {bench.num_rows} gold provisions, {matched} matched in pipeline")
    print(f"{'=' * 60}")
    print(f"\n  LEGAL RELATION accuracy:  {relation_correct}/{relation_total} ({relation_pct:.1f}%)")
    print(f"    Correct (type + position):  {relation['correct']}")
    print(f"    Wrong DRRP type only:       {relation['wrong_type']}")
    print(f"    Wrong position only:        {relation['wrong_position']}")
    print(f"    Both wrong:                 {relation['both_wrong']}")
    print(f"    Not in pipeline:            {relation['missing']}")
    print(f"\n  DRRP type accuracy:       {drrp_correct}/{drrp_total} ({drrp_pct:.1f}%)")
    print(f"  Actor position accuracy:  {pos_correct}/{pos_total} ({pos_pct:.1f}%)")

    # --- Detail ---
    print_confusion_matrix(drrp_confusion, "DRRP Confusion Matrix (gold ↓ pipeline →):")
    print_per_class_metrics(drrp_confusion, "DRRP Per-Class Metrics:")

    if pos_total > 0:
        print_confusion_matrix(pos_confusion, "Position Confusion Matrix (gold ↓ pipeline →):")
        print_per_class_metrics(pos_confusion, "Position Per-Class Metrics:")

    # --- Mismatches ---
    if mismatches and args.mismatches > 0:
        n = min(args.mismatches, len(mismatches))
        print(f"\n{'=' * 60}")
        print(f"Top {n} DRRP type mismatches:")
        for m in mismatches[:n]:
            print(f"  {m['section_id']}: gold={m['gold_drrp']} pipeline={m['pipeline_drrp']} method={m['method']}")

    if pos_mismatches and args.mismatches > 0:
        n = min(args.mismatches, len(pos_mismatches))
        print(f"\n{'=' * 60}")
        print(f"Top {n} position mismatches:")
        for m in pos_mismatches[:n]:
            print(f"  {m['section_id']}: {m['actor']} gold={m['gold_position']} pipeline={m['pipeline_position']} method={m['method']}")


if __name__ == "__main__":
    main()
