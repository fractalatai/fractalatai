#!/usr/bin/python3
"""Select the gold v3 provisions (phase 0a, session parsing/2026-10-06-phase0a-gold-set-scaffolding.md).

Only laws held out of SLM training (the 61 test-split laws of the drrp-v1.1 sample, seed 60), so the gold
set never overlaps training data. Three parts:
  labelled  every test-split provision already labelled (Gemini v1.3, etc.): the tier evidence is richest
  natural   a seeded random draw from the rest of the pool: the corpus's own mix, for unbiased rates
  target    rare cases over-sampled: applying-provision holders, functions lists, Liberty, and each rare
            coarse purpose class (by the coarse cue, coarse_purpose.py)

  sample_drrp_training.py --pool test --out data/gold/v3/pool.csv   # first
  select_gold.py                                   # → data/gold/v3/selection.csv
  select_gold.py --size 1000 --natural 150 --seed 63
"""

import argparse
import collections
import csv
import os
import random
import re
import sys

import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from coarse_purpose import classify, coarse  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
FUNCTIONS = re.compile(r"\b(?:shall have the following functions|the functions of [^.]{1,60} (?:shall be|are)\s*[—–-])", re.I)
# target quotas, in order; anything left after them goes to Liberty
TARGETS = [
    ("app", 25), ("functions", 10),
    ("Constitution", 10), ("Enforcement", 12), ("Fees and charges", 10), ("Transitional and saving", 10),
    ("Appeals and defences", 10), ("Amendment and revocation", 8), ("Citation and commencement", 8),
    ("Interpretation", 6),
]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pool", default=os.path.join(ROOT, "data/gold/v3/pool.csv"))
    ap.add_argument("--sample", default=os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv"))
    ap.add_argument("--out", default=os.path.join(ROOT, "data/gold/v3/selection.csv"))
    ap.add_argument("--size", type=int, default=1000)
    ap.add_argument("--natural", type=int, default=150)
    ap.add_argument("--per-law-cap", type=int, default=3, help="max target rows per law per target")
    ap.add_argument("--seed", type=int, default=63)
    args = ap.parse_args()
    rnd = random.Random(args.seed)

    pool = {x["section_id"]: x for x in csv.DictReader(open(args.pool))}
    labelled = [s["section_id"] for s in csv.DictReader(open(args.sample)) if s["split"] == "test"]
    missing = [s for s in labelled if s not in pool]
    if missing:
        sys.exit(f"{len(missing)} labelled test rows not in the pool (re-run the pool first): {missing[:3]}")
    cur = psycopg2.connect(PG).cursor()
    cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)",
                (sorted({x["law_name"] for x in pool.values()}),))
    texts = dict(cur.fetchall())
    purpose = {sid: classify(sid, texts) for sid in pool}

    chosen: dict[str, str] = {sid: "labelled" for sid in labelled}
    rest = sorted(sid for sid in pool if sid not in chosen)
    rnd.shuffle(rest)
    for sid in rest[: args.natural]:
        chosen[sid] = "natural"

    def is_target(sid: str, t: str) -> bool:
        x = pool[sid]
        if t == "app":
            return x["f_app"] == "1"
        if t == "functions":
            return bool(FUNCTIONS.search(texts.get(sid) or ""))
        if t == "lib":
            return x["f_lib"] == "1"
        return purpose[sid][0] == t

    def take(t: str, n: int) -> None:
        per_law = collections.Counter()
        for sid in rest:
            if n <= 0 or len(chosen) >= args.size:
                return
            if sid in chosen or not is_target(sid, t) or per_law[pool[sid]["law_name"]] >= args.per_law_cap:
                continue
            chosen[sid] = f"target:{t}"
            per_law[pool[sid]["law_name"]] += 1
            n -= 1

    for t, n in TARGETS:
        take(t, n)
    take("lib", args.size - len(chosen))
    if len(chosen) < args.size:  # Liberty ran out under the per-law cap: fill naturally
        for sid in rest:
            if len(chosen) >= args.size:
                break
            chosen.setdefault(sid, "natural")

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    cols = ["section_id", "law_name", "part", "stratum", "coarse_purpose_cue", "cue_how",
            "f_prot", "f_ben", "f_cp", "f_app", "f_hu", "f_lib", "f_none"]
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols, extrasaction="ignore")
        w.writeheader()
        for sid in sorted(chosen):
            w.writerow({**pool[sid], "part": chosen[sid], "coarse_purpose_cue": coarse(purpose[sid][0]), "cue_how": purpose[sid][1]})

    print(f"gold v3 selection: {len(chosen):,} provisions in {len({pool[s]['law_name'] for s in chosen})} laws → {args.out}")
    for k, v in sorted(collections.Counter(chosen.values()).items()):
        print(f"  {k:36s}{v:5d}")
    print("  coarse purpose (cue):", dict(collections.Counter(coarse(purpose[s][0]) for s in chosen).most_common()))
    print("  strata:", dict(collections.Counter(pool[s]["stratum"] for s in chosen).most_common()))
    print(f"  flags: lib {sum(pool[s]['f_lib'] == '1' for s in chosen)}, ben {sum(pool[s]['f_ben'] == '1' for s in chosen)}, "
          f"cp {sum(pool[s]['f_cp'] == '1' for s in chosen)}, app {sum(pool[s]['f_app'] == '1' for s in chosen)}, "
          f"hu {sum(pool[s]['f_hu'] == '1' for s in chosen)}")


if __name__ == "__main__":
    main()
