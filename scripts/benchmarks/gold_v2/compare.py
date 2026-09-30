#!/usr/bin/python3
"""Compare the two models' labels (gold v2). Agreement is gold; disagreements go to the referee.

A provision is agreed when both models give the same relation/raw_type and the
same (position, holds, inferred) for every actor. An actor that only one model
lists is tolerated when that model has it `mentioned`/`none` (no legal role).
Agreed provisions are written to gold_v2 / gold_v2_provision (source
'consensus'); the rest go to a JSONL for the Claude referee.

  /usr/bin/python3 scripts/benchmarks/gold_v2/compare.py --laws UK_asp_2005_13 [--write]
"""

import argparse
import json
import sys
from collections import Counter

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import MODELS, PROMPT_VERSION, ancestors, connect, norm_label  # noqa: E402

RANK = {"active": 3, "counterparty": 2, "beneficiary": 1, "mentioned": 0}


def actors_of(resp: dict) -> dict:
    """label -> (position, holds, inferred); a repeated label keeps its strongest role."""
    out = {}
    for a in resp.get("actors") or []:
        label = norm_label(a["label"])
        cur = (a["position"], a["holds"] if a["position"] == "active" else "none", bool(a.get("inferred")))
        if label not in out or RANK[cur[0]] > RANK[out[label][0]]:
            out[label] = cur
    return out


def compare(g: dict, o: dict) -> tuple[bool, dict, list[str]]:
    diffs = []
    if g["relation"] != o["relation"]:
        diffs.append(f"relation {g['relation']} vs {o['relation']}")
    if (g.get("raw_type") or None) != (o.get("raw_type") or None):
        diffs.append(f"raw_type {g.get('raw_type')} vs {o.get('raw_type')}")
    ga, oa = actors_of(g), actors_of(o)
    merged = {}
    for label in sorted(set(ga) | set(oa)):
        a, b = ga.get(label), oa.get(label)
        if a == b:
            merged[label] = a
        elif a is None and b[0] == "mentioned":
            merged[label] = b
        elif b is None and a[0] == "mentioned":
            merged[label] = a
        else:
            diffs.append(f"{label}: {a} vs {b}")
    return not diffs, merged, diffs


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--laws", required=True)
    ap.add_argument("--write", action="store_true", help="write consensus to gold_v2 and disputes to JSONL")
    ap.add_argument("--disputes", default="data/audit/gold_v2_disputes.jsonl")
    args = ap.parse_args()
    laws = [x.strip() for x in args.laws.split(",") if x.strip()]

    with connect() as conn:
        raw = conn.execute(
            "SELECT section_id, law_name, text_md5, model, response FROM gold_v2_raw "
            "WHERE law_name = ANY(%s) AND prompt_version = %s AND error IS NULL", (laws, PROMPT_VERSION)).fetchall()
        texts = dict(conn.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)", (laws,)).fetchall())
        by = {}
        for sid, law, md5, model, resp in raw:
            by.setdefault((sid, law, md5), {})[model] = resp

        stats, disputes, consensus = Counter(), [], []
        actor_stats = Counter()
        for (sid, law, md5), r in sorted(by.items()):
            if set(r) != set(MODELS.values()):
                stats["one_model_only"] += 1
                continue
            g, o = r[MODELS["gemini"]], r[MODELS["openai"]]
            agreed, merged, diffs = compare(g, o)
            ga, oa = actors_of(g), actors_of(o)
            for label in set(ga) | set(oa):
                actor_stats["agree" if ga.get(label) == oa.get(label) else "differ"] += 1
            if agreed:
                stats["agreed"] += 1
                consensus.append((sid, law, md5, g["relation"], g.get("raw_type"), merged))
            else:
                stats["disputed"] += 1
                disputes.append({
                    "section_id": sid, "law_name": law, "text_md5": md5, "text": texts.get(sid),
                    "stems": [[a, texts[a]] for a in ancestors(sid) if texts.get(a)],
                    "gemini": g, "openai": o, "diffs": diffs,
                })

        n = stats["agreed"] + stats["disputed"]
        print(f"provisions compared: {n}  agreed: {stats['agreed']} ({100 * stats['agreed'] / max(n, 1):.0f}%)  "
              f"disputed: {stats['disputed']}  one model only: {stats['one_model_only']}")
        m = sum(actor_stats.values())
        print(f"actor labels: {m}  identical: {actor_stats['agree']} ({100 * actor_stats['agree'] / max(m, 1):.0f}%)")
        kinds = Counter(d.split(":")[0] if d.startswith(("relation", "raw_type")) else "actor" for x in disputes for d in x["diffs"])
        print("dispute kinds:", dict(kinds))

        if args.write:
            for sid, law, md5, relation, raw_type, merged in consensus:
                conn.execute("DELETE FROM gold_v2 WHERE section_id = %s", (sid,))
                for label, (pos, holds, inferred) in merged.items():
                    conn.execute(
                        "INSERT INTO gold_v2 (section_id, law_name, text_md5, actor_label, position, holds, inferred, source, prompt_version) "
                        "VALUES (%s,%s,%s,%s,%s,%s,%s,'consensus',%s)", (sid, law, md5, label, pos, holds, inferred, PROMPT_VERSION))
                conn.execute(
                    "INSERT INTO gold_v2_provision (section_id, law_name, text_md5, relation, raw_type, source, prompt_version) "
                    "VALUES (%s,%s,%s,%s,%s,'consensus',%s) ON CONFLICT (section_id) DO UPDATE SET text_md5 = EXCLUDED.text_md5, "
                    "relation = EXCLUDED.relation, raw_type = EXCLUDED.raw_type, source = EXCLUDED.source, "
                    "prompt_version = EXCLUDED.prompt_version, created_at = now()",
                    (sid, law, md5, relation, raw_type, PROMPT_VERSION))
            conn.commit()
            with open(args.disputes, "w") as f:
                for d in disputes:
                    f.write(json.dumps(d) + "\n")
            print(f"wrote {len(consensus)} consensus provisions; {len(disputes)} disputes → {args.disputes}")


if __name__ == "__main__":
    main()
