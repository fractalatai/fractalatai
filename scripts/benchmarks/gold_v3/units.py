#!/usr/bin/python3
"""Sentence units (meta-plan phase A): join a stem with its items and closing words into one legal sentence.

A **stem** is a row with list-item children (paragraphs, sub-paragraphs) whose text holds a dash ("X shall—", "shall consist of— and the provision of …")
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
DASH = re.compile(r"\s*[—–]+\s*")
COLON = re.compile(r":\s*")
QUOTE_START = re.compile(r"^[“\"‘']")
INTERP = re.compile(r"^In (?:this|these|the|subsections?|paragraphs?)\b", re.I)
DASH_END = re.compile(r"[—–]\s*$")
DEF_SPLIT = re.compile(r"(?<=;)\s*(?=[“\"‘'])")
PLACEHOLDER = re.compile(r"[\s.…]+")  # repealed rows are dots
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
        # legal's parse puts a subsection's definitions on the section row ("5.—(1) In these Regulations—" with
        # "“the 1990 Act” means …" on reg.5): they go back to the one child that ends in a dash and has no items
        self.defs: dict[str, str] = {}     # child → section row holding its definitions
        self.adopted: dict[str, str] = {}  # section row → child
        for sid, kids in self.kids.items():
            if self.rows[sid][3] in ("section", "article") and QUOTE_START.match(self.text(sid)) and not self.is_stem(sid):
                open_ = [k for k in kids if not self.kids.get(k) and DASH_END.search(self.text(k))]
                if len(open_) > 1:  # "In this section—" over "… there is inserted—"
                    open_ = [k for k in open_ if INTERP.match(self.text(k))]
                if len(open_) == 1:
                    self.defs[open_[0]], self.adopted[sid] = sid, open_[0]

    def text(self, sid: str) -> str:
        return (self.rows.get(sid, (None, ""))[1] or "").strip()

    def is_stem(self, sid: str) -> bool:
        t = self.text(sid)
        # a stem introduces list items; a section whose children are all numbered subsections isn't one, even when
        # its own text has a dash (Water Act 2003 s.3 holds the s.3(12) definitions after legal's 2026-10-07 repair)
        # list items always continue their parent's words, so a lead-in that lost its dash in the source ("the
        # diving project plan shall;", "In Scotland") is still a stem; the dash only marks where closing words start
        items = any(self.rows[k][3] in ("paragraph", "sub_paragraph") for k in self.kids.get(sid, []))
        return items and bool(PLACEHOLDER.sub("", t))

    def root(self, sid: str) -> str:
        if sid in self.adopted:
            return self.root(self.adopted[sid])
        # the outermost stem above a row; a stem's unit is its whole subtree (members), so an item row under a
        # non-stem item ((b) "that is to say" with no dash, then (b)(i)) still belongs to the stem's sentence
        r = sid
        for a in ancestors(sid):
            if a in self.rows and self.is_stem(a):
                r = a
        return r

    def members(self, root: str) -> list[str]:
        if not self.is_stem(root):
            return [root] + ([self.defs[root]] if root in self.defs else [])
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
            if sid in self.defs:  # its definitions, one per line
                lines += [pad + "    " + d.strip() for d in DEF_SPLIT.split(self.text(self.defs[sid])) if d.strip()]
            # an item without a dash still carries its sub-items ((b) "that is to say" (i) (ii)); a root that isn't
            # a stem is one row, so its subsections (their own units) aren't printed under it
            for k in self.kids.get(sid, []) if depth else []:
                lines += self.render(k, depth + 1)
            return lines
        parts = DASH.split(t)
        series = self._series(self.kids[sid])
        if len(parts) - 1 != len(series):
            # one list: the head up to the first dash, the items, then everything after it
            m = DASH.search(t) or COLON.search(t)  # no dash: a colon opens the list ("direct in writing that: shall …")
            parts = [t[:m.start()], t[m.end():]] if m else [t, ""]
            series = [self.kids[sid]]
        # "X— (a) (b) then Y— (i) (ii) Z": each dash opens the next run of items (Civil Aviation Act 1982 s.94(2))
        mark = "—" if DASH.search(t) else ":" if COLON.search(t) else ""
        lines = [pad + label + parts[0].strip() + mark]
        for i, run in enumerate(series):
            for k in run:
                lines += self.render(k, depth + 1)
            nxt = parts[i + 1].strip()
            if nxt:
                lines.append(pad + nxt + ("—" if i + 1 < len(series) else ""))
        return lines

    def _series(self, kids: list[str]) -> list[list[str]]:
        """Split a stem's items into runs where the numbering restarts ((a) (b) then (i) (ii), or (a) again)."""
        runs: list[list[str]] = []
        prev = None
        for k in kids:
            lab = k[k.rfind("("):]
            restart = prev is not None and (lab == "(a)" or (lab == "(i)" and prev != "(h)"))
            if not runs or restart:
                runs.append([])
            runs[-1].append(k)
            prev = lab
        return runs

    def unit(self, root: str) -> dict:
        mem = self.members(root)
        text = "\n".join(self.render(root))
        items = mem[1:]
        flags = []
        if len(mem) > 1:
            m = DASH.search(self.text(root)) or COLON.search(self.text(root))
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
            flags.append("proviso_row")  # a sibling "But …" qualifying the row before it: not joined (yet)
        if self.rows[root][3] in ("section", "article") and self.kids.get(root) and not self.is_stem(root):
            flags.append("section_text")
        if root in self.defs:
            flags.append("definitions_moved")  # definitions taken back from the section row (source fault)  # text on a section row with subsections: often a stray fragment of one
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
