#!/usr/bin/python3
"""Lint compiled_applicability trees for the QQ-03 defects (L1-L8).

Mirrors the definitions in sertantai-legal QQ-03 so fractalaw can measure
before/after counts without waiting on `mix fitness.lint_trees`.

Usage:
    /usr/bin/python3 scripts/maintenance/lint_trees.py [--scope FILE] [--list L2] [--db data/fractalaw.duckdb]

--scope   file of law names (one per line) to restrict the lint to
--list    print the laws that fail the given check
--jsonl   overlay trees from `fractalaw fitness compile --out` output
          ({"name","tree"} per line; a null tree removes the law's tree)
"""

import argparse
import datetime
import json
from collections import Counter

import duckdb

GOV_ACTORS = {
    "secretary_of_state", "local_authority", "scottish_ministers", "welsh_ministers",
    "enforcement_authority", "public_authority",
}
GENERIC = {"building", "land", "licence", "body_corporate", "person", "offence", "application"}
NATIONS = {"england", "wales", "scotland", "northern_ireland"}
JURISDICTION = NATIONS | {"united_kingdom", "great_britain", "england_and_wales"}

OWN_BY_TYPE = {
    "asp": "scotland", "ssi": "scotland", "ssa": "scotland",
    "wsi": "wales", "anaw": "wales", "asc": "wales", "mwa": "wales",
    "nisr": "northern_ireland", "nia": "northern_ireland", "apni": "northern_ireland",
}
TITLE_NATION = {
    "(england)": "england", "(wales)": "wales", "(scotland)": "scotland",
    "(northern ireland)": "northern_ireland",
}


def own_jurisdiction(type_code, title):
    own = set()
    if type_code in OWN_BY_TYPE:
        own.add(OWN_BY_TYPE[type_code])
    t = (title or "").lower()
    for k, v in TITLE_NATION.items():
        if k in t:
            own.add(v)
    return own


STOPWORDS = {"of", "the", "and", "or", "for", "in", "to", "a", "an", "at", "on", "by", "with"}


def title_subject(code, title):
    """Mirror of applicability_compile::is_grounded against the law title."""
    import re
    if not title:
        return False
    words = [w for w in re.split(r"[_\s-]", code) if w and w not in STOPWORDS]
    if not words:
        return False
    head = words[0]
    pat = rf"\b{re.escape(head)}\b" if len(head) < 4 else rf"\b{re.escape(head[:6])}"
    return re.search(pat, title.lower()) is not None


def walk(node, in_not=False, depth=0, gating=False):
    """Yield (node, in_not, depth, gating) for every node.

    gating: the node is ANDed with at least one sibling condition.
    """
    yield node, in_not, depth, gating
    op = node.get("op")
    if op in ("And", "Or"):
        conds = [c for c in node["children"] if c.get("op") != "TimeWindow"]
        for c in node["children"]:
            yield from walk(c, in_not, depth + 1, op == "And" and len(conds) > 1)
    elif op == "Not":
        yield from walk(node["child"], True, depth + 1)
    elif op == "Conditional":
        yield from walk(node["condition"], in_not, depth + 1, True)
        yield from walk(node["then"], in_not, depth + 1, True)


def lint(tree, type_code, title, today):
    hits = set()
    own = own_jurisdiction(type_code, title)
    positive_non_territorial = False
    for node, in_not, _, gating in walk(tree):
        op = node.get("op")
        if op in ("And", "Or"):
            kids = [json.dumps(c, sort_keys=True) for c in node["children"]]
            if len(kids) != len(set(kids)) or len(kids) == 1:
                hits.add("L1")
        elif op == "TimeWindow":
            f, t = node.get("from"), node.get("to")
            if t and (t < today or (f and t < f)):
                hits.add("L2")
        elif op == "Not":
            hits.add("has_not")
        elif op == "Match":
            codes = set(node.get("codes", []))
            dim = node.get("dimension")
            if in_not and dim == "territorial" and codes & own:
                hits.add("L3")
            if in_not and any(title_subject(c, title) for c in codes):
                hits.add("L3_subject")
            if "construction" in codes:
                hits.add("L4")
                if in_not:
                    hits.add("L4_in_not")
            if codes & GOV_ACTORS:
                hits.add("L5_any")
                if gating and not in_not:
                    hits.add("L5")
            if codes & GENERIC and not in_not:
                hits.add("L6")
            if not in_not and dim != "territorial":
                positive_non_territorial = True
    # L7: root must carry a territorial jurisdiction gate as a direct And child
    root_kids = tree["children"] if tree.get("op") == "And" else [tree]
    if not any(
        k.get("op") == "Match" and k.get("dimension") == "territorial"
        and set(k.get("codes", [])) & JURISDICTION
        for k in root_kids
    ):
        hits.add("L7")
    if not positive_non_territorial:
        hits.add("L8")
    return hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", default="data/fractalaw.duckdb")
    ap.add_argument("--scope")
    ap.add_argument("--list")
    ap.add_argument("--jsonl")
    args = ap.parse_args()

    scope = None
    if args.scope:
        scope = {l.strip() for l in open(args.scope) if l.strip()}

    con = duckdb.connect(args.db, read_only=True)
    rows = con.execute(
        "SELECT name, type_code, title, compiled_applicability FROM legislation "
        "WHERE compiled_applicability IS NOT NULL"
    ).fetchall()
    today = datetime.date.today().isoformat()

    trees = {name: (type_code, title, json.loads(t)) for name, type_code, title, t in rows}
    if args.jsonl:
        meta = dict(
            (n, (tc, ti)) for n, tc, ti in con.execute("SELECT name, type_code, title FROM legislation").fetchall()
        )
        for line in open(args.jsonl):
            rec = json.loads(line)
            if rec["tree"] is None:
                trees.pop(rec["name"], None)
            else:
                tc, ti = meta.get(rec["name"], (None, None))
                trees[rec["name"]] = (tc, ti, rec["tree"])

    counts, failing, n = Counter(), {}, 0
    for name, (type_code, title, tree) in trees.items():
        if scope is not None and name not in scope:
            continue
        n += 1
        hits = lint(tree, type_code, title, today)
        counts.update(hits)
        for h in hits:
            failing.setdefault(h, []).append(name)

    print(f"trees linted: {n}")
    for key in ["L1", "L2", "L3", "L3_subject", "L4", "L4_in_not", "L5", "L5_any", "L6", "L7", "L8", "has_not"]:
        print(f"  {key:10} {counts.get(key, 0)}")
    if args.list:
        print("\n".join(sorted(failing.get(args.list, []))))


if __name__ == "__main__":
    main()
