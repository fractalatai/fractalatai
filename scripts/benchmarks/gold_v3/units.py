#!/usr/bin/python3
"""Sentence units (meta-plan phase A): join a stem with its items and closing words into one legal sentence.

A **stem** is a row with child rows whose text holds a dash ("X shall—", "shall consist of— and the provision of …")
or ends in a colon. The source puts a stem's closing words after the dash on the stem row; here they go after the
items, where the law reads them. A unit's **root** is the outermost row in an unbroken chain of stems above a row
(nested stems (2)— (a)— (i) are one sentence). A row with no stem above it is its own unit.

  units.py                                   # the 61 test laws of the gold selection → data/gold/v3/units.jsonl
  units.py --show UK_uksi_1998_2306:reg.11(2)(a)   # print the unit a row belongs to
  units.py --stats                           # counts and edge-case flags only

Each unit: unit_id (the root row's id), law_name, members (row ids in document order), text (assembled, items
indented), n_rows, chars, flags. The selection is re-keyed in data/gold/v3/selection_units.csv (section_id → unit_id).
"""

import argparse
import collections
import csv
import json
import os
import re
import sys

import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import ancestors  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DASH = re.compile(r"\s*[—–]\s*")
COLON_END = re.compile(r":\s*$")
MODAL = re.compile(r"\b(?:shall|must|may|is to|are to)\b", re.I)
# an item that opens with its own subject before a modal ("the Secretary of State shall …")
OWN_SUBJECT = re.compile(r"^(?:the|a|an|any|every|each|no|such|that|this|where|if)\b[^;—]{0,120}?\b(?:shall|must|may)\b", re.I)
PROVISO = re.compile(r"^\s*(?:provided that|but\b|except that)", re.I)
LONG = 4000


class Units:
    def __init__(self, rows: dict[str, tuple]):
        self.rows = rows  # sid → (law, text, position, section_type)
        self.kids: dict[str, list[str]] = collections.defaultdict(list)
        for sid in rows:
            anc = ancestors(sid)
            if anc and anc[0] in rows:
                self.kids[anc[0]].append(sid)
        for k in self.kids.values():
            k.sort(key=lambda s: self.rows[s][2])

    def text(self, sid: str) -> str:
        return (self.rows.get(sid, (None, ""))[1] or "").strip()

    def is_stem(self, sid: str) -> bool:
        t = self.text(sid)
        return bool(self.kids.get(sid)) and bool(t) and (bool(DASH.search(t)) or bool(COLON_END.search(t)))

    def root(self, sid: str) -> str:
        r = sid
        for a in ancestors(sid):
            if a in self.rows and self.is_stem(a):
                r = a
            else:
                break
        return r

    def members(self, root: str) -> list[str]:
        if not self.is_stem(root):
            return [root]
        out = [root]
        for k in self.kids[root]:
            out += self.members(k) if self.is_stem(k) else self._subtree(k)
        return out

    def _subtree(self, sid: str) -> list[str]:
        out = [sid]
        for k in self.kids.get(sid, []):
            out += self._subtree(k)
        return out

    def render(self, sid: str, depth: int = 0) -> list[str]:
        """The sentence as the law reads it: stem head, items indented, then the stem's closing words."""
        t = self.text(sid)
        label = "" if depth == 0 else sid[sid.rfind("("):] + " "
        pad = "    " * depth
        if not self.is_stem(sid):
            lines = [pad + label + t] if t else []
            for k in self.kids.get(sid, []):
                lines += self.render(k, depth + 1)
            return lines
        m = DASH.search(t)
        head, tail = (t[:m.start()] + "—", t[m.end():].strip()) if m else (t, "")
        lines = [pad + label + head]
        for k in self.kids[sid]:
            lines += self.render(k, depth + 1)
        if tail:
            lines.append(pad + tail)
        return lines

    def unit(self, root: str) -> dict:
        mem = self.members(root)
        text = "\n".join(self.render(root))
        items = mem[1:]
        flags = []
        if len(mem) > 1:
            m = DASH.search(self.text(root))
            if m and self.text(root)[m.end():].strip():
                flags.append("closing_words")
            if any(self.is_stem(s) for s in items):
                flags.append("nested")
            if any(OWN_SUBJECT.search(self.text(s)) for s in items):
                flags.append("full_sentence_item")
            if any(PROVISO.search(self.text(s)) for s in items):
                flags.append("proviso_item")
        if len(text) > LONG:
            flags.append("long")
        if PROVISO.search(self.text(root)):
            flags.append("proviso_row")
        if "sch" in root.split(":", 1)[1][:4]:
            flags.append("schedule")
        return {"unit_id": root, "law_name": self.rows[root][0], "members": mem, "n_rows": len(mem),
                "chars": len(text), "flags": flags, "text": text}


def load(laws: list[str]) -> dict[str, tuple]:
    cur = psycopg2.connect(PG).cursor()
    cur.execute("SELECT section_id, law_name, text, position, section_type FROM legislation_text WHERE law_name = ANY(%s)",
                (laws,))
    return {r[0]: (r[1], r[2], r[3] or 0, r[4]) for r in cur.fetchall()}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--selection", default=os.path.join(ROOT, "data/gold/v3/selection.csv"))
    ap.add_argument("--out", default=os.path.join(ROOT, "data/gold/v3/units.jsonl"))
    ap.add_argument("--show", help="print the unit containing this row id")
    ap.add_argument("--stats", action="store_true", help="counts only; write nothing")
    args = ap.parse_args()

    sel = list(csv.DictReader(open(args.selection)))
    laws = sorted({r["law_name"] for r in sel})
    if args.show:
        laws = [args.show.split(":", 1)[0]]
    U = Units(load(laws))
    if args.show:
        u = U.unit(U.root(args.show))
        print(f"{u['unit_id']}  rows {u['n_rows']}  chars {u['chars']}  flags {u['flags']}\n")
        print(u["text"])
        return

    # rows that carry text (empty containers such as "reg.11" with no words aren't units)
    roots = sorted({U.root(s) for s in U.rows if U.text(s)}, key=lambda s: (U.rows[s][0], U.rows[s][2]))
    units = [U.unit(r) for r in roots]
    multi = [u for u in units if u["n_rows"] > 1]
    print(f"{len(laws)} laws: {sum(1 for s in U.rows if U.text(s)):,} text rows → {len(units):,} units "
          f"({len(multi):,} multi-row)")
    fl = collections.Counter(f for u in units for f in u["flags"])
    for f, n in fl.most_common():
        print(f"  {f:20s} {n:,}")
    sel_units = {r["section_id"]: U.root(r["section_id"]) for r in sel if r["section_id"] in U.rows}
    su = collections.Counter(sel_units.values())
    print(f"selection: {len(sel_units):,} provisions → {len(su):,} units; "
          f"{sum(1 for r, u in sel_units.items() if r != u)} are items inside a sentence")
    chars = sorted(u["chars"] for u in units if u["unit_id"] in su)
    print(f"  selection unit chars: median {chars[len(chars) // 2]}, p90 {chars[int(len(chars) * .9)]}, max {chars[-1]}")
    if args.stats:
        return
    with open(args.out, "w") as f:
        for u in units:
            f.write(json.dumps(u, ensure_ascii=False) + "\n")
    with open(os.path.join(os.path.dirname(args.out), "selection_units.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["section_id", "unit_id"])
        for sid, uid in sorted(sel_units.items()):
            w.writerow([sid, uid])
    print(f"wrote {args.out} and selection_units.csv")


if __name__ == "__main__":
    main()
