#!/usr/bin/python3
"""Score each model's training labels against the Claude referee's final labels (same provisions).

Per model: exact agreement (no diff by compare_training_labels.diff), and per field: relation, raw_type,
purpose, actors (same position/holds/inferred for every non-mentioned actor). Uses each model's latest label at
--prompt-version for the referee's section_ids.

  score_vs_referee.py --referee data/training/drrp-v1.1/referee/batch1.jsonl \
      --models gemini-3.8-flash,gpt-5.5:low,gpt-5.4-mini:low
"""

import argparse
import collections
import json
import os
import sys

import psycopg

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts/ml"))
sys.path.insert(0, os.path.join(ROOT, "scripts/benchmarks/gold_v2"))
from common import PG, PROMPT_VERSION  # noqa: E402
from compare_training_labels import diff  # noqa: E402


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--referee", required=True)
    ap.add_argument("--models", required=True)
    ap.add_argument("--prompt-version", default=PROMPT_VERSION)
    args = ap.parse_args()
    ref = {r["section_id"]: r["final"] for r in map(json.loads, open(args.referee))}
    models = [m.strip() for m in args.models.split(",")]
    with psycopg.connect(PG) as conn:
        rows = conn.execute(
            "SELECT DISTINCT ON (section_id, model) section_id, model, response FROM drrp_training_labels_raw "
            "WHERE prompt_version = %s AND response IS NOT NULL AND model = ANY(%s) AND section_id = ANY(%s) "
            "ORDER BY section_id, model, created_at DESC", (args.prompt_version, models, sorted(ref))).fetchall()
    got = collections.defaultdict(dict)
    for sid, model, resp in rows:
        got[model][sid] = resp
    print(f"{len(ref)} refereed provisions; prompt {args.prompt_version}")
    print(f"{'model':22s} {'n':>4s} {'exact':>7s} {'relation':>9s} {'raw_type':>9s} {'purpose':>8s} {'actors':>7s}  top misses")
    for m in models:
        n, c, miss = 0, collections.Counter(), collections.Counter()
        for sid, final in ref.items():
            resp = got[m].get(sid)
            if not resp:
                continue
            n += 1
            d = diff(resp, final)
            cats = {k.split(":")[0] for k, _ in d}
            c["exact"] += not d
            c["relation"] += "relation" not in cats
            c["raw_type"] += "raw_type" not in cats
            c["purpose"] += "purpose" not in cats
            c["actors"] += not (cats & {"position", "holds", "inferred", "label", "actor_only_gemini", "actor_only_gpt"})
            for k, _ in d:
                miss[k] += 1
        pct = lambda k: f"{c[k] / n:.0%}" if n else "-"  # noqa: E731
        top = ", ".join(f"{k} {v}" for k, v in miss.most_common(4))
        print(f"{m:22s} {n:4d} {pct('exact'):>7s} {pct('relation'):>9s} {pct('raw_type'):>9s} {pct('purpose'):>8s} {pct('actors'):>7s}  {top}")


if __name__ == "__main__":
    main()
