#!/usr/bin/python3
"""Score each model's raw labels against finished gold v2 (consensus + referee) for the given laws.

Provision exact = same relation, raw_type and actor set with identical (position, holds);
actor accuracy is over gold actors; token usage and cost-relevant counts are shown where recorded.

  /usr/bin/python3 scripts/benchmarks/gold_v2/eval_models.py --laws UK_asp_2005_13
"""

import argparse
import sys
from collections import defaultdict

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from common import connect  # noqa: E402
from compare import actors_of  # noqa: E402


# Labels renamed by the dictionary fixes after prompt .2 (b52b62e): score them as equal
ALIAS = {
    "Gvt: Devolved Admin: Scottish Ministers": "Gvt: Devolved Admin: Scottish Parliament",
    "Ind: Person in Control": "Ind: Manager",
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--laws", required=True)
    args = ap.parse_args()
    laws = [x.strip() for x in args.laws.split(",") if x.strip()]
    with connect() as conn:
        gold_p = {sid: (rel, raw) for sid, rel, raw in conn.execute(
            "SELECT section_id, relation, raw_type FROM gold_v2_provision WHERE law_name = ANY(%s)", (laws,))}
        gold_a = defaultdict(dict)
        for sid, label, pos, holds in conn.execute(
                "SELECT section_id, actor_label, position, holds FROM gold_v2 WHERE law_name = ANY(%s)", (laws,)):
            gold_a[sid][label] = (pos, holds)
        raw = conn.execute(
            "SELECT model, prompt_version, section_id, response, usage FROM gold_v2_raw "
            "WHERE law_name = ANY(%s) AND error IS NULL", (laws,)).fetchall()
    stats = defaultdict(lambda: defaultdict(int))
    for model, pv, sid, resp, usage in raw:
        if sid not in gold_p:
            continue
        s = stats[(model, pv)]
        s["n"] += 1
        rel_ok = (resp["relation"], resp.get("raw_type")) == gold_p[sid]
        mine = {ALIAS.get(k, k): (p, h) for k, (p, h, _) in actors_of(resp).items()}
        gold = {ALIAS.get(k, k): v for k, v in gold_a.get(sid, {}).items()}
        s["relation_ok"] += rel_ok
        s["exact"] += rel_ok and mine == gold
        s["gold_actors"] += len(gold)
        s["actor_ok"] += sum(mine.get(k) == v for k, v in gold.items())
        s["active_gold"] += sum(v[0] == "active" for v in gold.values())
        s["active_ok"] += sum(v[0] == "active" and mine.get(k) == v for k, v in gold.items())
        if usage:
            s["with_usage"] += 1
            s["in"] += usage.get("input_tokens") or usage.get("promptTokenCount") or 0
            s["out"] += (usage.get("output_tokens") or 0) + (usage.get("candidatesTokenCount") or 0) + (usage.get("thoughtsTokenCount") or 0)
    print(f"{'model':22} {'prompt':8} {'n':>4} {'relation':>9} {'exact':>6} {'actors':>7} {'active':>7} {'in tok/call':>12} {'out tok/call':>13}")
    for (model, pv), s in sorted(stats.items()):
        pct = lambda a, b: f"{100 * a / b:.0f}%" if b else "-"
        u = s["with_usage"]
        print(f"{model:22} {pv[-2:]:8} {s['n']:>4} {pct(s['relation_ok'], s['n']):>9} {pct(s['exact'], s['n']):>6} "
              f"{pct(s['actor_ok'], s['gold_actors']):>7} {pct(s['active_ok'], s['active_gold']):>7} "
              f"{(s['in'] // u if u else '-'):>12} {(s['out'] // u if u else '-'):>13}")


if __name__ == "__main__":
    main()
