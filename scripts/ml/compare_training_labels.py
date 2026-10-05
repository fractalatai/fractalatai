#!/usr/bin/python3
"""Compare Gemini and GPT training labels (drrp-v1.1) per provision; size the disagreement and write disputes
for the Claude referee. Nothing is written to the database.

A provision agrees when both models give the same relation, raw_type and purpose, and the same
(position, holds, inferred) for every actor. An actor only one model lists is tolerated when that model has it
`mentioned` (gold v2 compare.py rule). Diffs are categorised (relation, raw_type, purpose, position pairs such
as counterparty↔beneficiary, holds, actor only in one model, label-only) so the size of each kind is visible.

  compare_training_labels.py --family "NUCLEAR,FIRE,…" --out data/training/drrp-v1.1/disputes_batch1.jsonl
"""

import argparse
import collections
import csv
import json
import os
import sys

import duckdb
import psycopg

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts/benchmarks/gold_v2"))
from common import MODELS, PG, PROMPT_VERSION, ancestors, applying, applying_index, references, user_prompt  # noqa: E402
from compare import actors_of  # noqa: E402

SAMPLE = os.path.join(ROOT, "data/training/drrp-v1.1/sample.csv")
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")


def _aliases() -> dict[str, str]:
    import yaml
    return {old: e["label"] for e in yaml.safe_load(open(os.path.join(ROOT, "crates/fractalaw-core/data/actor-dictionary.yaml")))
            for old in e.get("renamed_from", [])}


ALIASES = _aliases()


def canon(resp: dict) -> dict:
    """The response with renamed/alias labels mapped to the canonical label (a naming slip isn't a dispute)."""
    return {**resp, "actors": [{**a, "label": ALIASES.get(a["label"], a["label"])} for a in resp.get("actors") or []]}


def diff(g: dict, o: dict) -> list[tuple[str, str]]:
    """[(category, detail)]: empty when the two labels agree."""
    g, o = canon(g), canon(o)
    out = []
    if g["relation"] != o["relation"]:
        out.append(("relation", f"{g['relation']} vs {o['relation']}"))
    if (g.get("raw_type") or None) != (o.get("raw_type") or None):
        out.append(("raw_type", f"{g.get('raw_type')} vs {o.get('raw_type')}"))
    if g.get("purpose") != o.get("purpose"):
        out.append(("purpose", f"{g.get('purpose')} vs {o.get('purpose')}"))
    ga, oa = actors_of(g), actors_of(o)
    only_g = {k: v for k, v in ga.items() if k not in oa and v[0] != "mentioned"}
    only_o = {k: v for k, v in oa.items() if k not in ga and v[0] != "mentioned"}
    # the same role under different labels (one each side): a label difference, not a role difference
    for lg, vg in list(only_g.items()):
        match = next((lo for lo, vo in only_o.items() if vo == vg), None)
        if match:
            out.append(("label", f"{lg} vs {match} ({vg[0]})"))
            del only_g[lg], only_o[match]
    for k, v in only_g.items():
        out.append(("actor_only_gemini", f"{k} {v[0]}/{v[1]}"))
    for k, v in only_o.items():
        out.append(("actor_only_gpt", f"{k} {v[0]}/{v[1]}"))
    for k in sorted(set(ga) & set(oa)):
        (pg, hg, ig), (po, ho, io) = ga[k], oa[k]
        if pg != po:
            out.append(("position:" + "↔".join(sorted([pg, po])), f"{k} {pg} vs {po}"))
        elif hg != ho:
            out.append(("holds", f"{k} {hg} vs {ho}"))
        elif ig != io:
            out.append(("inferred", f"{k} {ig} vs {io}"))
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--family", help="comma-separated DuckDB family substrings (as label_drrp_training.py)")
    ap.add_argument("--out", required=True, help="disputes JSONL for the referee")
    ap.add_argument("--second-model", default=MODELS["openai"], help="the second model's name (default gpt-5.5:low)")
    ap.add_argument("--ids", help="file of section_ids (one per line) to compare instead of --family")
    args = ap.parse_args()

    rows = list(csv.DictReader(open(SAMPLE)))
    if args.family:
        fam = dict(duckdb.connect(DUCK, read_only=True).execute("SELECT name, coalesce(family, '') FROM legislation").fetchall())
        wanted = [f.strip().lower() for f in args.family.split(",") if f.strip()]
        rows = [r for r in rows if any((w == "(none)" and not fam.get(r["law_name"], "").strip())
                                       or (w != "(none)" and w in fam.get(r["law_name"], "").lower()) for w in wanted)]
    if args.ids:
        wanted_ids = {line.strip() for line in open(args.ids) if line.strip()}
        rows = [r for r in rows if r["section_id"] in wanted_ids]
    ids = {r["section_id"] for r in rows}
    meta = {r["section_id"]: r for r in rows}

    with psycopg.connect(PG) as conn:
        latest = conn.execute(
            "SELECT DISTINCT ON (section_id, model) section_id, model, text_md5, response FROM drrp_training_labels_raw "
            "WHERE prompt_version = %s AND response IS NOT NULL AND section_id = ANY(%s) "
            "ORDER BY section_id, model, created_at DESC", (PROMPT_VERSION, sorted(ids))).fetchall()
        laws = sorted({meta[s]["law_name"] for s in ids})
        law_rows = conn.execute("SELECT section_id, text, part FROM legislation_text WHERE law_name = ANY(%s)", (laws,)).fetchall()
    texts = {s: t for s, t, _ in law_rows}
    parts = {s: p for s, _, p in law_rows}
    apps = applying_index(texts, parts)

    by = collections.defaultdict(dict)
    for sid, model, md5, resp in latest:
        by[sid][model] = (md5, resp)
    g_model, o_model = MODELS["gemini"], args.second_model
    pairs = {sid: m for sid, m in by.items() if g_model in m and o_model in m and m[g_model][0] == m[o_model][0]}

    cats, cat_provs, per_stratum = collections.Counter(), collections.Counter(), collections.defaultdict(collections.Counter)
    disputes = []
    for sid, m in sorted(pairs.items()):
        g, o = m[g_model][1], m[o_model][1]
        d = diff(g, o)
        stratum = meta[sid]["stratum"]
        per_stratum[stratum]["provisions"] += 1
        if not d:
            per_stratum[stratum]["agree"] += 1
            continue
        for c in {c for c, _ in d}:
            cat_provs[c] += 1
        for c, _ in d:
            cats[c] += 1
        text = texts.get(sid) or ""
        stems = [(a, texts[a]) for a in ancestors(sid) if texts.get(a)]
        disputes.append({"section_id": sid, "law_name": meta[sid]["law_name"], "stratum": stratum, "split": meta[sid]["split"],
                         "prompt": user_prompt(sid, text, stems, references(sid, text, texts), applying(sid, parts, apps)),
                         "gemini": g, "gpt": o, "diffs": [f"{c}: {x}" for c, x in d]})

    n = len(pairs)
    agree = n - len(disputes)
    print(f"compared {n} provisions (both models, same text) of {len(ids)} selected; "
          f"agree {agree} ({agree / max(n, 1):.1%}), disputed {len(disputes)} ({len(disputes) / max(n, 1):.1%})")
    print("disputed provisions by kind (a provision can have several):")
    for c, v in cat_provs.most_common():
        print(f"  {c:32s} {v:5d} provisions ({cats[c]} diffs)")
    print("agreement by stratum:")
    for s, c in sorted(per_stratum.items()):
        print(f"  {s:8s} {c['agree']:4d}/{c['provisions']:4d} ({c['agree'] / max(c['provisions'], 1):.0%})")
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w") as f:
        for d in disputes:
            f.write(json.dumps(d, ensure_ascii=False) + "\n")
    print(f"disputes → {args.out}")


if __name__ == "__main__":
    main()
