#!/usr/bin/env /usr/bin/python3
"""Compare pipeline output against golden benchmarks.

Loads Parquet benchmarks from NAS, queries LanceDB for the same provisions,
and reports precision/recall/F1 for DRRP types and actor positions.

Usage:
    /usr/bin/python3 scripts/benchmark_report.py                    # all benchmarks
    /usr/bin/python3 scripts/benchmark_report.py --family "OH&S"    # one family
    /usr/bin/python3 scripts/benchmark_report.py --mismatches 20    # show top N mismatches
"""

import argparse
import glob
import json
import os
import sys
from collections import Counter

import lancedb
import pyarrow.parquet as pq

BENCHMARK_DIR = "data/benchmarks"


def load_benchmarks(family_filter=None):
    """Load all benchmark Parquet files from NAS."""
    pattern = os.path.join(BENCHMARK_DIR, "tier2-*.parquet")
    files = sorted(glob.glob(pattern))

    if not files:
        print(f"No benchmark files found in {BENCHMARK_DIR}")
        sys.exit(1)

    tables = []
    for f in files:
        t = pq.read_table(f)
        if family_filter:
            slug = family_filter.lower().replace(" ", "")
            fname = os.path.basename(f).lower()
            if slug not in fname:
                continue
        tables.append(t)
        print(f"  Loaded {os.path.basename(f)}: {t.num_rows} provisions")

    if not tables:
        print(f"No benchmarks matched filter '{family_filter}'")
        sys.exit(1)

    import pyarrow as pa
    return pa.concat_tables(tables)


def load_pipeline(section_ids):
    """Load pipeline results for matching section_ids from LanceDB."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    # Query in chunks to avoid filter length limits
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
    """Compute DRRP and position metrics."""
    drrp_confusion = Counter()
    pos_confusion = Counter()
    mismatches = []

    matched = 0
    for sid, g in gold.items():
        if sid not in pipeline:
            continue
        matched += 1
        p = pipeline[sid]

        g_type = g["drrp"][0] if g["drrp"] else "none"
        p_type = p["drrp"][0] if p["drrp"] else "none"
        drrp_confusion[(g_type, p_type)] += 1

        if g_type != p_type:
            mismatches.append(
                {
                    "section_id": sid,
                    "gold_drrp": g_type,
                    "pipeline_drrp": p_type,
                    "method": p["method"],
                    "reasoning": g.get("reasoning", ""),
                }
            )

        # Actor position comparison
        g_actors = {a["label"]: a["position"] for a in g.get("actors", [])}
        p_actors = {a["label"]: a["position"] for a in p["actors"]}
        for label in set(g_actors) & set(p_actors):
            pos_confusion[(g_actors[label], p_actors[label])] += 1

    return matched, drrp_confusion, pos_confusion, mismatches


def print_confusion_matrix(confusion, title):
    """Print a confusion matrix from a Counter of (gold, predicted) tuples."""
    types = sorted(set(t for pair in confusion for t in pair))
    if not types:
        return

    print(f"\n{title}")
    col_width = max(len(t) for t in types) + 2
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

    # Micro average
    micro_p = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
    micro_r = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0
    micro_f1 = 2 * micro_p * micro_r / (micro_p + micro_r) if (micro_p + micro_r) > 0 else 0
    total = sum(confusion.values())
    print(f"{'micro avg':>15s} {micro_p:>10.1%} {micro_r:>10.1%} {micro_f1:>10.1%} {total:>10d}")


def main():
    parser = argparse.ArgumentParser(description="Benchmark comparison report")
    parser.add_argument("--family", help="Filter benchmarks by family")
    parser.add_argument(
        "--mismatches", type=int, default=10, help="Number of mismatches to show"
    )
    args = parser.parse_args()

    print("=== Golden Benchmark Report ===\n")
    print("Loading benchmarks...")
    bench = load_benchmarks(args.family)

    # Build gold dict
    gold = {}
    for i in range(bench.num_rows):
        sid = bench.column("section_id")[i].as_py()
        gold[sid] = {
            "drrp": bench.column("gold_drrp_types")[i].as_py() or [],
            "actors": json.loads(bench.column("gold_actors")[i].as_py() or "[]"),
            "reasoning": bench.column("gold_reasoning")[i].as_py() or "",
        }

    print(f"\nLoading pipeline results for {len(gold)} provisions...")
    pipeline = load_pipeline(list(gold.keys()))

    matched, drrp_confusion, pos_confusion, mismatches = compute_metrics(gold, pipeline)
    total = sum(drrp_confusion.values())
    correct = sum(drrp_confusion.get((t, t), 0) for t in set(t for pair in drrp_confusion for t in pair))

    print(f"\n{'='*60}")
    print(f"Benchmark: {bench.num_rows} provisions, {matched} matched in pipeline")
    print(f"DRRP accuracy: {correct}/{total} ({100*correct/total:.1f}%)")
    print(f"{'='*60}")

    print_confusion_matrix(drrp_confusion, "DRRP Confusion Matrix (gold → pipeline):")
    print_per_class_metrics(drrp_confusion, "DRRP Per-Class Metrics:")

    pos_total = sum(pos_confusion.values())
    if pos_total > 0:
        pos_correct = sum(
            pos_confusion.get((t, t), 0)
            for t in set(t for pair in pos_confusion for t in pair)
        )
        print(f"\nActor position accuracy: {pos_correct}/{pos_total} ({100*pos_correct/pos_total:.1f}%)")
        print_confusion_matrix(pos_confusion, "Position Confusion Matrix (gold → pipeline):")
        print_per_class_metrics(pos_confusion, "Position Per-Class Metrics:")

    if mismatches and args.mismatches > 0:
        print(f"\n{'='*60}")
        print(f"Top {min(args.mismatches, len(mismatches))} DRRP mismatches:")
        print(f"{'='*60}")
        for m in mismatches[: args.mismatches]:
            print(f"  {m['section_id']}: gold={m['gold_drrp']} pipeline={m['pipeline_drrp']} method={m['method']}")
            if m["reasoning"]:
                print(f"    reason: {m['reasoning'][:120]}")


if __name__ == "__main__":
    main()
