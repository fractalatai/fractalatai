#!/usr/bin/python3
"""Stratified provision sample for the drrp-v1.1 SLM training labels (session parsing/2026-10-01-training-labels-slm.md).

One unit = one provision (the definitive prompt labels per provision). Universe: substantive, non-repealed
provisions with text, outside Schedules, in live hub laws that are not benchmarks (gold_benchmarks, DuckDB
is_benchmark), not gold v2 laws (no benchmark leakage) and not enabling_extent scoped LAT (#66).

Strata (a provision takes the first that applies; all flags are kept):
  prot     a duty word (shall/must) and protective-purpose wording, in the provision or its stem
  ben      any tier put an actor at beneficiary, or the text has a protective-purpose cue
  cp       any tier put an actor at counterparty
  app      holder-unknown Obligation with an applying provision (#60)
  hu       other holder-unknown Obligation (no active actor)
  lib      Liberty
  none     no Obligation/Liberty (offences, definitions, procedure: the negatives)
  general  everything else
The tier positions are noisy (that's why we relabel); they only steer the sample.

Each stratum is drawn round-robin across laws (a per-law cap spreads it over the corpus). The test split
is by LAW (deterministic hash), so no law appears in both splits.

  sample_drrp_training.py --summary            # availability per stratum, no sample written
  sample_drrp_training.py                      # write data/training/drrp-v1.1/sample.csv
  sample_drrp_training.py --quota ben=1500 --quota cp=1200 --test-share 0.1 --seed 60
"""

import argparse
import collections
import csv
import hashlib
import os
import random
import re
import sys

import duckdb
import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import PROMPT_VERSION, ancestors, applying, applying_index  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
ORDER = ["prot", "ben", "cp", "app", "hu", "lib", "none", "general"]
# prot: the pilot showed old-tier beneficiary flags yield 0.02 beneficiaries per provision, protective-purpose
# duty wording 0.36 (2026-10-05), so prot takes everything available and ben shrinks to 500
QUOTA = {"prot": 2500, "ben": 500, "cp": 1200, "app": 400, "hu": 800, "lib": 500, "none": 1000, "general": 1000}
MODAL = re.compile(r"\b(?:shall|must)\b", re.I)
PROT_CUE = re.compile(
    r"\b(?:ensure|secure|protect|safeguard|prevent|reduce|minimi[sz]e)\b[^.;]{0,120}\b(?:health|safety|welfare|risks?|harm|injur\w*|danger)\b"
    r"|\b(?:health|safety|welfare)\b[^.;]{0,40}\bof\b|\bexposed\s+to\s+(?:risks?|danger)|\bwell-?being\b", re.I)
# Protective-purpose wording: the duty protects a party who doesn't receive its act (beneficiary)
BEN_CUE = re.compile(
    r"\b(?:health|safety|welfare)\b[^.;]{0,40}\bof\b[^.;]{0,30}\b(?:employees|persons|workers|people|public|children|patients|passengers|residents|users)\b"
    r"|\bnot\s+(?:thereby\s+)?exposed\s+to\s+risks?\b|\bprotect(?:ion|ing)?\s+(?:of\s+)?(?:the\s+)?(?:health|safety|persons|people|public)\b"
    r"|\bfor\s+the\s+benefit\s+of\b|\bin\s+the\s+interests?\s+of\b[^.;]{0,30}\b(?:safety|health)\b", re.I)


def law_hash(law: str, seed: int) -> float:
    return int(hashlib.sha256(f"{seed}:{law}".encode()).hexdigest()[:8], 16) / 0xFFFFFFFF


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv"))
    ap.add_argument("--quota", action="append", default=[], help="stratum=n (repeatable)")
    ap.add_argument("--per-law-cap", type=float, default=0.03, help="max share of a stratum from one law (default 0.03)")
    ap.add_argument("--test-share", type=float, default=0.10, help="share of laws held out as test (default 0.10)")
    ap.add_argument("--seed", type=int, default=60)
    ap.add_argument("--summary", action="store_true", help="print availability only")
    args = ap.parse_args()
    quota = dict(QUOTA)
    for q in args.quota:
        k, v = q.split("=")
        quota[k] = int(v)

    duck = duckdb.connect(DUCK, read_only=True)
    pg = psycopg2.connect(PG)
    cur = pg.cursor()
    cur.execute("SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks")
    excluded = {r[0] for r in cur.fetchall()}
    cur.execute("SELECT DISTINCT law_name FROM gold_v2_raw")
    excluded |= {r[0] for r in cur.fetchall()}
    cur.execute("SELECT law_name FROM lat_sync_state WHERE 'enabling_extent' = ANY(scope_purposes)")
    excluded |= {r[0] for r in cur.fetchall()}
    excluded |= {r[0] for r in duck.execute("SELECT name FROM legislation WHERE is_benchmark OR status = 'revoked'").fetchall()}

    cur.execute(
        """SELECT t.law_name, t.section_id, t.text, t.part, coalesce(t.drrp_types, '{}'),
                  coalesce(bool_or('counterparty' IN (a.position, a.slm_position, a.llm_position, a.cls_position, a.regex_position)), false),
                  coalesce(bool_or('beneficiary' IN (a.position, a.slm_position, a.llm_position, a.cls_position, a.regex_position)), false),
                  coalesce(bool_or(a.position = 'active'), false), count(a.actor_label)
           FROM legislation_text t LEFT JOIN provision_actors a USING (section_id)
           WHERE t.scope = 'substantive' AND coalesce(t.status, '') <> 'repealed'
             AND t.text IS NOT NULL AND length(t.text) >= 20 AND t.section_id NOT LIKE '%%:sch.%%'
             AND t.law_name <> ALL(%s)
           GROUP BY t.law_name, t.section_id, t.text, t.part, t.drrp_types""",
        (sorted(excluded),),
    )
    rows = cur.fetchall()
    texts = {r[1]: r[2] for r in rows}
    parts = {r[1]: r[3] for r in rows}
    idx = applying_index(texts, parts)

    pool: dict[str, list] = collections.defaultdict(list)
    for law, sid, text, _, types, cp, ben, active, n_actors in rows:
        obligation, liberty = "Obligation" in types, "Liberty" in types
        hu = obligation and not active
        ctx = " ".join([texts.get(a) or "" for a in ancestors(sid)] + [text])
        flags = {"prot": bool(MODAL.search(ctx) and PROT_CUE.search(ctx)),
                 "ben": ben or bool(BEN_CUE.search(text)), "cp": cp,
                 "app": hu and bool(applying(sid, parts, idx)), "hu": hu, "lib": liberty,
                 "none": not (obligation or liberty)}
        stratum = next((s for s in ORDER[:-1] if flags[s]), "general")
        pool[stratum].append({"law_name": law, "section_id": sid, "stratum": stratum, "n_actors": n_actors,
                              "text_md5": hashlib.md5(text.encode()).hexdigest(),
                              **{f"f_{k}": int(v) for k, v in flags.items()}})

    laws = sorted({r[0] for r in rows})
    test_laws = {law for law in laws if law_hash(law, args.seed) < args.test_share}
    print(f"universe: {len(rows):,} provisions in {len(laws)} laws "
          f"(excluded {len(excluded)} benchmark/gold-v2/enabling_extent/revoked laws); test laws: {len(test_laws)}")
    for s in ORDER:
        p = pool[s]
        print(f"  {s:8s} available {len(p):7,} in {len({x['law_name'] for x in p}):4d} laws; quota {quota[s]:5,}")
    if args.summary:
        return

    rnd = random.Random(args.seed)
    sample = []
    for s in ORDER:
        by_law = collections.defaultdict(list)
        for x in pool[s]:
            by_law[x["law_name"]].append(x)
        for v in by_law.values():
            rnd.shuffle(v)
        cap = max(1, int(quota[s] * args.per_law_cap))
        order = sorted(by_law)
        rnd.shuffle(order)
        taken, depth = [], 0
        while len(taken) < quota[s] and depth < cap:  # round-robin: one per law per pass
            added = False
            for law in order:
                if depth < len(by_law[law]) and len(taken) < quota[s]:
                    taken.append(by_law[law][depth])
                    added = True
            if not added:
                break
            depth += 1
        sample += taken

    for x in sample:
        x["split"] = "test" if x["law_name"] in test_laws else "train"
        x["prompt_version"] = PROMPT_VERSION
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    cols = ["section_id", "law_name", "stratum", "split", "n_actors", "text_md5", "prompt_version",
            *[f"f_{k}" for k in ORDER[:-1]]]
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols, extrasaction="ignore")
        w.writeheader()
        w.writerows(sorted(sample, key=lambda x: x["section_id"]))

    by = collections.Counter((x["stratum"], x["split"]) for x in sample)
    print(f"\nsample: {len(sample):,} provisions in {len({x['law_name'] for x in sample})} laws → {args.out}")
    for s in ORDER:
        print(f"  {s:8s} train {by[(s, 'train')]:5,}  test {by[(s, 'test')]:4,}")
    print(f"  flags in sample: ben {sum(x['f_ben'] for x in sample):,}, cp {sum(x['f_cp'] for x in sample):,}, "
          f"hu {sum(x['f_hu'] for x in sample):,}, app {sum(x['f_app'] for x in sample):,}; "
          f"current-tier actors {sum(x['n_actors'] for x in sample):,}")


if __name__ == "__main__":
    main()
