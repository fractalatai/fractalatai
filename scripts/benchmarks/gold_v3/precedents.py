#!/usr/bin/python3
"""Precedent store (meta-plan phase B): reviewed gold sentences as retrievable examples, not cited rules.

Each precedent is a gold-v4-sentence unit with its final labels (Jason's decisions), his comments and changes, the
catalogue-v2 principles it exercises (old rule IDs mapped through the v2 crosswalk), and precedent-pattern tags
(catalogue v2 "Precedent patterns", matched by cue or carried in rule_ids). Retrieval order for a new sentence:
1. shared pattern tag, 2. same law and section, 3. similarity: the cosine of the mean member-row
embeddings (legislation_text.embedding, 384-d), tf-idf when a sentence has none; law and Jason's notes break ties.

  precedents.py                         # build data/gold/v4/precedents.jsonl and report counts
  precedents.py --query UNIT_ID [-k 5]  # show the precedents retrieved for a unit (any unit of the test laws)

`retrieve(text, unit_id, k)` is imported by unit_evidence.py for phase C evidence packs.
"""

import argparse
import collections
import json
import math
import os
import re
import sys

import psycopg2

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
V4 = os.path.join(ROOT, "data/gold/v4")
CATALOGUE = os.path.join(ROOT, "docs/architecture/DRRP-RULE-CATALOGUE.md")
OUT = os.path.join(V4, "precedents.jsonl")

# catalogue v2 precedent patterns, recognised by their cue words (a tag already in rule_ids also counts)
PATTERNS = {
    "functions-list": re.compile(r"\bfunctions of (?:the|a|an)\b.{0,80}?\b(?:shall|are|is) (?:be )?(?:to|as follows)|\bfunctions of\b[^.]{0,60}—", re.I),
    "implied-access-right": re.compile(r"\b(?:available|open) (?:for|to) (?:public )?inspection\b|\bmay inspect\b[^.]{0,40}\bregister", re.I),
    "no-person-shall-be-engaged": re.compile(r"\bno (?:person|employee) shall be (?:engaged|employed|permitted)\b", re.I),
    "participation-right": re.compile(r"\bshall take part in\b|\bshall be consulted\b", re.I),
    "deemed-holder": re.compile(r"\bshall be (?:treated|regarded|deemed) as (?:the |an? )?(?:employer|occupier|operator|person responsible|employment by)\b|\bare to be treated as references to\b", re.I),
}
TOKEN = re.compile(r"[a-z]{3,}")
STOP = set("the and for any such that this with shall which under from have has been are not may its their there other"
           " these those must where who whom into upon all each person regulation regulations section subsection".split())


def crosswalk() -> dict[str, list[str]]:
    """v1 rule ID → the v2 principle IDs it now lives in (principles map to themselves)."""
    text = open(CATALOGUE).read()
    principles = set(re.findall(r"^\| ([A-Z]{3,4}-\d+) \| (?!merged|retired|precedent|dictionary)", text, re.M))
    out = {p: [p] for p in principles}
    for rid, now in re.findall(r"^\| ([A-Z]{3,4}-\d+) \| ((?:merged|retired|precedent|dictionary)[^|]*) \|", text, re.M):
        out[rid] = [t for t in re.findall(r"[A-Z]{3,4}-\d+", now) if t in principles]
    return out


def tokens(text: str) -> list[str]:
    return [t for t in TOKEN.findall(text.lower()) if t not in STOP]


def embed(members: list[str]) -> list[float] | None:
    """Mean of the member rows' stored embeddings, unit-normalised; None when none is embedded yet."""
    cur = psycopg2.connect(PG).cursor()
    cur.execute("SELECT embedding::text FROM legislation_text WHERE section_id = ANY(%s) AND embedding IS NOT NULL", (members,))
    vs = [json.loads(r[0]) for r in cur.fetchall()]
    if not vs:
        return None
    m = [sum(c) / len(vs) for c in zip(*vs)]
    n = math.sqrt(sum(x * x for x in m)) or 1.0
    return [round(x / n, 5) for x in m]


def tags_of(text: str, rule_ids: set) -> list[str]:
    return sorted({p for p, rx in PATTERNS.items() if rx.search(text)} | (set(PATTERNS) & rule_ids))


def build() -> list[dict]:
    ev = {json.loads(l)["unit_id"]: json.loads(l) for l in open(os.path.join(V4, "evidence.jsonl"))}
    cw = crosswalk()
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT section_id, field, actor_label, actor_position, proposed, decision, decided, comment, rule_ids
                   FROM drrp_gold WHERE gold_version = 'gold-v4-sentence' AND decision IS NOT NULL""")
    by = collections.defaultdict(list)
    for sid, field, actor, pos, proposed, decision, decided, comment, rule_ids in cur.fetchall():
        by[sid].append((field, actor, pos, decided if decision == "change" else proposed, decision, comment, rule_ids or []))
    # carried sentences cite CARRIED: take their citations from the row-level gold of their member rows
    cur.execute("""SELECT section_id, rule_ids FROM drrp_gold WHERE gold_version = 'gold-v3-draft' AND decision IS NOT NULL""")
    v3 = collections.defaultdict(set)
    for sid, rule_ids in cur.fetchall():
        v3[sid] |= set(rule_ids or [])
    out = []
    for uid, rows in by.items():
        if uid not in ev:
            continue
        e = ev[uid]
        rids = {r for row in rows for r in row[6]}
        # a consistency change cites its v2 principle in the comment ("v2 consistency (…): REL-33: …")
        rids |= {m for row in rows if row[5] for m in re.findall(r"v2 consistency[^:]*: ([A-Z]{3,4}-\d+):", row[5])}
        if "CARRIED" in rids:
            rids |= {r for m in e["members"] for r in v3.get(m, ())}
        labels = {"actors": []}
        for field, actor, pos, value, *_ in rows:
            if field == "actor":
                if not (isinstance(value, dict) and value.get("listed") is False):
                    labels["actors"].append({"label": actor, **(value or {})})
            else:
                labels[field] = value
        notes = [{"field": f, "actor": a, "decision": d, "comment": c} for f, a, _, _, d, c, _ in rows
                 if c and not c.startswith("Carried") and (d in ("change", "query") or not c.startswith("Ruled"))]
        out.append({
            "unit_id": uid, "law_name": e["law_name"], "law_title": e.get("law_title"), "text": e["text"],
            "patterns": tags_of(e["text"], rids),
            "principles": sorted({p for r in rids for p in cw.get(r, [])}),
            "labels": labels, "jason": notes,
            "changed": any(d == "change" for _, _, _, _, d, _, _ in rows),
            "embedding": embed(e["members"]),
        })
    return out


class Store:
    def __init__(self, items: list[dict]):
        self.items = items
        self.df = collections.Counter(t for it in items for t in set(tokens(it["text"])))
        self.n = len(items)
        self.vecs = [self._vec(it["text"]) for it in items]

    def _vec(self, text: str) -> dict[str, float]:
        tf = collections.Counter(tokens(text))
        v = {t: c * math.log((self.n + 1) / (self.df.get(t, 0) + 1)) for t, c in tf.items()}
        norm = math.sqrt(sum(x * x for x in v.values())) or 1.0
        return {t: x / norm for t, x in v.items()}

    def retrieve(self, text: str, unit_id: str, k: int = 5, members: list[str] | None = None) -> list[dict]:
        """Precedents for a sentence: pattern tag, then same law and section, then similarity; never itself."""
        tags = set(tags_of(text, set()))
        qe = embed(members or [unit_id])
        law, _, sec = unit_id.partition(":")
        section = re.match(r"[a-z]+\.[^()]*", sec)
        q = self._vec(text)
        scored = []
        for it, v in zip(self.items, self.vecs):
            if it["unit_id"] == unit_id:
                continue
            if qe and it.get("embedding"):
                sim = sum(a * b for a, b in zip(qe, it["embedding"]))
            else:
                sim = sum(x * v.get(t, 0.0) for t, x in q.items())
            same_sec = section and it["unit_id"].startswith(f"{law}:{section.group(0)}")
            # pattern and section decide; similarity ranks the rest; law and Jason's notes only break near-ties
            score = (3.0 * bool(tags & set(it["patterns"])) + 1.0 * bool(same_sec) + sim
                     + 0.03 * (it["law_name"] == law) + 0.03 * (it["changed"] or bool(it["jason"])))
            scored.append((score, sim, it))
        scored.sort(key=lambda s: -s[0])
        return [{**{k_: v_ for k_, v_ in it.items() if k_ != "embedding"}, "why": ("pattern " + ",".join(sorted(tags & set(it["patterns"]))) if tags & set(it["patterns"])
                               else "same section" if section and it["unit_id"].startswith(f"{law}:{section.group(0)}")
                               else f"similar ({sim:.2f})")} for _, sim, it in scored[:k]]


_STORE = None


def retrieve(text: str, unit_id: str, k: int = 5, members: list[str] | None = None) -> list[dict]:
    global _STORE
    if _STORE is None:
        _STORE = Store([json.loads(l) for l in open(OUT)])
    return _STORE.retrieve(text, unit_id, k, members)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--query")
    ap.add_argument("-k", type=int, default=5)
    args = ap.parse_args()
    if args.query:
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from units import Units, load  # noqa: E402
        U = Units(load([args.query.split(":", 1)[0]]))
        u = U.unit(U.root(args.query))
        text = u["text"]
        print(text[:400], "\n")
        for p in retrieve(text, u["unit_id"], args.k, u["members"]):
            print(f"- {p['unit_id']}  [{p['why']}]  patterns={p['patterns']}  principles={p['principles'][:6]}")
            for n in p["jason"][:2]:
                print(f"    Jason ({n['decision']}): {n['comment'][:140]}")
        return
    items = build()
    with open(OUT, "w") as f:
        for it in items:
            f.write(json.dumps(it, ensure_ascii=False) + "\n")
    pc = collections.Counter(p for it in items for p in it["patterns"])
    prc = collections.Counter(p for it in items for p in it["principles"])
    print(f"{len(items)} precedents → {OUT}; with Jason's notes {sum(bool(i['jason']) for i in items)}, "
          f"changed {sum(i['changed'] for i in items)}")
    print("patterns:", dict(pc))
    print(f"principles exercised: {len(prc)} of {len(set(p for v in crosswalk().values() for p in v))}; "
          "least: " + ", ".join(f"{p} {n}" for p, n in sorted(prc.items(), key=lambda x: x[1])[:8]))


if __name__ == "__main__":
    main()
