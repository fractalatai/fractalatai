#!/usr/bin/python3
"""Evidence pack for sentence units (meta-plan phases A and C): everything a justifier or reviewer needs per sentence.

One JSONL row per unit (units.py): the assembled sentence, law title and headings, the labelling context (referenced
and applying provisions, the law's definitions of terms used), the silver labels on its member rows (Gemini,
GPT-mini, GPT-5.5, referee) and Jason's earlier row-level decisions on its members (gold-v3-draft), as precedent.

  unit_evidence.py --reviewed            # units with at least one reviewed member row
  unit_evidence.py --selection           # every unit of the gold selection

Writes data/gold/v4/evidence.jsonl. Reads only.
"""

import argparse
import collections
import csv
import json
import os
import re
import sys

import duckdb
import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from drrp_prompt import applying, applying_index, references, user_prompt  # noqa: E402
from evidence import DEFINES  # noqa: E402
from units import Units  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
V3 = os.path.join(ROOT, "data/gold/v3")


def silver(lab: dict) -> dict:
    """A model label trimmed to what a sentence-level reviewer compares: relation, type, purpose, actors."""
    keep = ("relation", "raw_type", "drrp_type", "purpose", "actors", "rationale", "sided_with")
    return {k: lab[k] for k in keep if k in lab}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--reviewed", action="store_true")
    g.add_argument("--selection", action="store_true")
    ap.add_argument("--out", default=os.path.join(ROOT, "data/gold/v4/evidence.jsonl"))
    args = ap.parse_args()

    # units Jason dropped from gold (e.g. out-of-domain Parts of massive Acts) never come back
    excluded = {r["unit_id"] for r in csv.DictReader(open(os.path.join(ROOT, "data/gold/v4/excluded_units.csv")))}
    sel = {r["section_id"]: r["unit_id"] for r in csv.DictReader(open(os.path.join(V3, "selection_units.csv")))
           if r["unit_id"] not in excluded}
    v3 = {json.loads(l)["section_id"]: json.loads(l) for l in open(os.path.join(V3, "evidence.jsonl"))}
    laws = sorted({s.split(":", 1)[0] for s in sel})
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT section_id, law_name, text, position, section_type, part, hierarchy_path
                   FROM legislation_text WHERE law_name = ANY(%s)""", (laws,))
    raw = cur.fetchall()
    U = Units({r[0]: (r[1], r[2], r[3] or 0, r[4]) for r in raw})
    texts = {r[0]: r[2] for r in raw}
    parts = {r[0]: r[5] for r in raw}
    paths = {r[0]: r[6] or "" for r in raw}
    titles = {(r[1], r[6]): r[2] for r in raw if r[4] in ("part", "chapter", "heading") and r[6]}
    app_idx = applying_index(texts, parts)
    defs: dict[str, dict] = {}
    for r in raw:
        for m in DEFINES.finditer(r[2] or ""):
            defs.setdefault(r[1], {}).setdefault(m.group(1).strip(), (r[0], r[2]))

    cur.execute("""SELECT section_id, field, actor_label, proposed, decision, decided, comment
                   FROM drrp_gold WHERE gold_version = 'gold-v3-draft' AND field <> 'purpose_fine'""")
    decided: dict[str, list] = collections.defaultdict(list)
    for sid, field, actor, proposed, decision, dec, comment in cur.fetchall():
        if decision and sid in U.rows:  # rows of dropped laws (e.g. Companies Act 1989) aren't loaded
            decided[U.root(sid)].append({"row": sid, "field": field, "actor_label": actor,
                                         "value": dec if decision == "change" else proposed,
                                         "decision": decision, "comment": comment})

    units = sorted((set(decided) - excluded) if args.reviewed else set(sel.values()))
    duck = duckdb.connect(DUCK, read_only=True)
    law_title = dict(duck.execute("SELECT name, title FROM legislation WHERE name IN (SELECT unnest(?))", [laws]).fetchall())
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    n = collections.Counter()
    with open(args.out, "w") as f:
        for uid in units:
            u = U.unit(uid)
            law = u["law_name"]
            segs = paths.get(uid, "").split("/")
            heads = [titles[(law, p)] for p in ("/".join(segs[:i]) for i in range(1, len(segs))) if (law, p) in titles]
            refs, seen = [], set(u["members"])
            for m in u["members"]:
                for x, xt in references(m, texts.get(m) or "", texts):
                    if x not in seen:
                        refs.append((x, xt))
                        seen.add(x)
            apps = applying(uid, parts, app_idx)
            used = sorted((t for t in defs.get(law, {}) if re.search(r"(?<!\w)" + re.escape(t) + r"(?!\w)", u["text"], re.I)),
                          key=len, reverse=True)[:8]
            context = user_prompt(uid, u["text"], [], refs, apps).replace(
                "\n\nPROVISION TO LABEL", "\n\nDEFINITIONS (same law; terms used above):\n"
                + ("\n".join(f"[{defs[law][t][0]}] {defs[law][t][1][:600]}" for t in used) or "(none)")
                + "\n\nPROVISION TO LABEL", 1)
            labels = {m: {k: silver(v) for k, v in v3[m]["labels"].items()} for m in u["members"] if m in v3 and v3[m]["labels"]}
            sampled = [m for m in u["members"] if m in sel]
            kind = ("single" if u["n_rows"] == 1 else
                    "stem_reviewed" if any(d["row"] == uid for d in decided.get(uid, [])) else
                    "items_only" if decided.get(uid) else "new")
            n[kind] += 1
            f.write(json.dumps({
                "unit_id": uid, "law_name": law, "law_title": law_title.get(law), "headings": heads,
                "text": u["text"], "members": u["members"], "n_rows": u["n_rows"], "flags": u["flags"],
                "sampled_rows": sampled, "kind": kind, "context": context,
                "silver": labels, "row_decisions": decided.get(uid, []),
            }, ensure_ascii=False) + "\n")
    print(f"{len(units)} units → {args.out}: " + ", ".join(f"{k} {v}" for k, v in n.most_common()))


if __name__ == "__main__":
    main()
