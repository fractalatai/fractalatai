#!/usr/bin/python3
"""Load justified gold v3 labels into Postgres drrp_gold, with per-field evidence (phase 0a).

Input: the justifier's JSONL (JUSTIFY_GOLD.md) plus data/gold/v3/evidence.jsonl. For every proposed field it
attaches what each model, the referee and the pipeline said, and sets `agree` (the Gemini and referee labels
that exist say the same; mini, GPT-5.5 and the pipeline tiers are shown but not counted).
An actor that a model or the referee listed but the justifier didn't becomes its own row, proposed
`{"listed": false}`, difficulty hard, so the review sees the omission.

Rows Jason has decided (decision IS NOT NULL) are never overwritten.

  load.py data/gold/v3/justified/pilot.jsonl              # dry run: summary only
  load.py data/gold/v3/justified/pilot.jsonl --write
"""

import argparse
import collections
import json
import os
import subprocess
import sys

import psycopg2
from psycopg2.extras import Json

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from coarse_purpose import FROM_18  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
CATALOGUE = "docs/architecture/DRRP-RULE-CATALOGUE.md"
MODELS = ("gemini", "mini", "gpt55", "referee")
# agreement counts the referee and Gemini only: GPT-5.4-mini and GPT-5.5 scored far below Gemini against the
# referee (exact 36% / 27% vs 71%), so they are shown as evidence but not counted
COUNTED = ("gemini", "referee")
FINE = {"Requirement": "Requirement", "Procedure+Detail": "Procedure/Detail"}


def act(a):
    return "notify" if a == "serve" else a  # serve folded into notify (2026-10-06)


def actor_view(a: dict) -> dict:
    return {"position": a.get("position"), "holds": a.get("holds"), "inferred": bool(a.get("inferred")), "act": act(a.get("act"))}


def model_value(lab: dict, field: str, actor: str):
    if field == "relation":
        return lab.get("relation")
    if field == "raw_type":
        return lab.get("raw_type")
    if field == "purpose":
        return FROM_18.get(lab.get("purpose"), lab.get("purpose"))
    if field == "purpose_fine":
        return FINE.get(lab.get("purpose"))
    hit = [a for a in lab.get("actors") or [] if a.get("label") == actor]
    return actor_view(hit[0]) if hit else {"listed": False}


def pipeline_value(ev: dict, field: str, actor: str):
    t = ev["tiers"]
    if field == "relation":
        return "yes" if t.get("drrp_types") else "no"
    if field == "purpose":
        return [FROM_18.get(p, p) for p in t.get("purposes") or []]
    if field == "actor":
        hit = [a for a in t.get("actors") or [] if a["label"] == actor]
        return {k: v for k, v in hit[0].items() if k != "label"} if hit else {"listed": False}
    return None


def same(field: str, proposed, value) -> bool:
    if field == "actor" and isinstance(proposed, dict) and isinstance(value, dict):
        keys = ("position", "holds", "inferred", "act") if proposed.get("listed", True) else ("listed",)
        return all(proposed.get(k, True if k == "listed" else None) == value.get(k, True if k == "listed" else None) for k in keys)
    return proposed == value


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("justified")
    ap.add_argument("--evidence", default=os.path.join(ROOT, "data/gold/v3/evidence.jsonl"))
    ap.add_argument("--gold-version", default="gold-v3-draft")
    ap.add_argument("--write", action="store_true")
    args = ap.parse_args()

    evidence = {e["section_id"]: e for e in map(json.loads, open(args.evidence))}
    cat_ver = subprocess.run(["git", "log", "-1", "--format=%h", "--", CATALOGUE], cwd=ROOT,
                             capture_output=True, text=True).stdout.strip() or "uncommitted"
    out, problems = [], []
    for line in open(args.justified):
        j = json.loads(line)
        sid = j["section_id"]
        ev = evidence.get(sid)
        if not ev:
            problems.append(f"{sid}: not in evidence")
            continue
        if j.get("text_md5") != ev["text_md5"]:
            problems.append(f"{sid}: text_md5 differs from evidence (text changed?)")
        labs = {m: ev["labels"][m] for m in MODELS if m in ev["labels"]}
        fields = list(j["fields"])
        proposed_actors = {f.get("actor_label") for f in fields if f["field"] == "actor"}
        for lab_actor in sorted({a["label"] for lab in labs.values() for a in lab.get("actors") or []} - proposed_actors):
            who = [m for m, lab in labs.items() if any(a.get("label") == lab_actor for a in lab.get("actors") or [])]
            fields.append({"field": "actor", "actor_label": lab_actor, "proposed": {"listed": False}, "rule_ids": ["LBL"],
                           "reason": f"Listed by {', '.join(who)} but not by the justifier: confirm it is not an actor here, "
                                     f"or give its role.", "difficulty": "hard"})
        for f in fields:
            field, actor = f["field"], f.get("actor_label") or ""
            proposed = f["proposed"]
            if field == "actor" and isinstance(proposed, dict) and "act" in proposed:
                proposed = {**proposed, "act": act(proposed["act"])}
            evid = {m: model_value(lab, field, actor) for m, lab in labs.items()}
            if field == "purpose":
                evid["cue"] = ev["cue"]["coarse_purpose"]
            pv = pipeline_value(ev, field, actor)
            if pv is not None:
                evid["pipeline"] = pv
            counted = [evid[m] for m in COUNTED if m in evid and not (field == "purpose_fine" and evid[m] is None)]
            agree = all(same(field, proposed, v) for v in counted) if counted else None
            out.append((args.gold_version, sid, ev["text_md5"], field, actor, Json(proposed), f.get("rule_ids") or [],
                        f.get("reason") or "", Json(evid), agree, f.get("difficulty") or "hard", cat_ver))

    by = collections.Counter((r[3], r[10]) for r in out)
    print(f"{len(out):,} gold rows from {args.justified} (catalogue {cat_ver})")
    for field in ("relation", "raw_type", "purpose", "purpose_fine", "actor"):
        print(f"  {field:13s} " + "  ".join(f"{d} {by[(field, d)]}" for d in ("easy", "hard", "new_edge")))
    agree = collections.Counter(r[9] for r in out)
    print(f"  agree with all model/referee labels: yes {agree[True]}, no {agree[False]}, no labels {agree[None]}")
    for p in problems:
        print("  !", p)
    if not args.write:
        print("dry run: --write to load")
        return
    with psycopg2.connect(PG) as conn, conn.cursor() as cur:
        cur.executemany(
            """INSERT INTO drrp_gold (gold_version, section_id, text_md5, field, actor_label, proposed, rule_ids, reason,
                                      evidence, agree, difficulty, catalogue_ver)
               VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
               ON CONFLICT (gold_version, section_id, field, actor_label) DO UPDATE SET
                 text_md5 = EXCLUDED.text_md5, proposed = EXCLUDED.proposed, rule_ids = EXCLUDED.rule_ids,
                 reason = EXCLUDED.reason, evidence = EXCLUDED.evidence, agree = EXCLUDED.agree,
                 difficulty = EXCLUDED.difficulty, catalogue_ver = EXCLUDED.catalogue_ver, created_at = now()
               WHERE drrp_gold.decision IS NULL""", out)
    print(f"written to drrp_gold ({args.gold_version}); decided rows untouched")


if __name__ == "__main__":
    main()
