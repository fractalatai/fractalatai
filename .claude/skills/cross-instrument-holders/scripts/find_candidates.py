#!/usr/bin/python3
"""Cross-instrument duty holders (#77): find holder-unknown duties in secondary legislation that
take their holder from a duty in another Act ("in accordance with the duty at section 6(1) of the Act").

Writes one JSONL row per candidate with the provision, its stem, and each cited Act provision
resolved via DuckDB legislation.enacted_by and the instrument's own definitions ("the 1990 Act" means …).
Nothing is written to the databases.

  find_candidates.py                  # candidates → data/audit/cross_instrument/<today>/candidates.jsonl
  find_candidates.py --gaps           # also list hub secondary laws with no enacted_by (→ lrt-sync)
  find_candidates.py --laws L1,L2     # restrict to these laws
"""

import argparse
import datetime as dt
import json
import os
import re
import sys

import duckdb
import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import ancestors  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
SECONDARY = {"uksi", "ssi", "wsi", "nisr"}

# "the duty at section 6(1) of the Act", "the requirement under section 91(1)(c) of the 2022 Act",
# "duties imposed by section 2 of the Health and Safety at Work etc. Act 1974"
DUTY_REF = re.compile(
    r"\b(?:duty|duties|requirements?|obligations?)\s+(?:at|in|under|imposed\s+by|of|contained\s+in|set\s+out\s+in)\s+"
    r"(?:section|article|regulation)s?\s+(?P<num>\d+[A-Z]*)(?P<sub>(?:\(\w{1,4}\))*)\s+of\s+(?:the\s+)?"
    r"(?:(?P<year>\d{4})\s+Act\b|Act\b|(?P<title>[A-Z][\w,.'() ]{2,120}?Act)\s+(?P<tyear>\d{4}))", re.I)
# '"the Act" means the Health Act 2006'; Welsh: '“the 1990 Act” (“Deddf 1990”) means the Environmental Protection Act 1990'
DEFINES = re.compile(r"[\"“‘']the\s+(?P<year>\d{4}\s+)?Act[\"”’'](?:\s*\([^)]{0,40}\))?\s+means\s+the\s+(?P<title>[A-Z][\w,.'() ]{2,120}?Act)\s+(?P<tyear>\d{4})", re.I)


def act_by_title(duck, title: str, year: str) -> str | None:
    rows = duck.execute(
        "SELECT name FROM legislation WHERE year = ? AND lower(title) = lower(?) ORDER BY name", [int(year), title.strip()]
    ).fetchall()
    return rows[0][0] if len(rows) == 1 else None


def resolve(duck, m, enacted_by: list[str], defs: dict[str, str]) -> tuple[list[str], str]:
    """Candidate parent laws for one DUTY_REF match, and how they were found."""
    if m.group("title"):
        hit = act_by_title(duck, m.group("title"), m.group("tyear"))
        return ([hit], "named") if hit else ([], f"unresolved title: {m.group('title')} {m.group('tyear')}")
    key = (m.group("year") or "").strip()
    if key in defs:
        return [defs[key]], f'defined: "the {key + " " if key else ""}Act"'
    if key:
        same_year = [e for e in enacted_by if e.split("_")[2] == key]
        return (same_year, "enacted_by (year)") if same_year else ([], f"unresolved: the {key} Act")
    return (enacted_by, "enacted_by") if enacted_by else ([], "unresolved: the Act (no enacted_by)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=os.path.join(ROOT, "data/audit/cross_instrument", dt.date.today().isoformat()))
    ap.add_argument("--laws", help="comma-separated law names (default: all hub secondary laws)")
    ap.add_argument("--gaps", action="store_true", help="list hub secondary laws with no enacted_by")
    args = ap.parse_args()

    duck = duckdb.connect(DUCK, read_only=True)
    pg = psycopg2.connect(PG)
    cur = pg.cursor()

    cur.execute("SELECT DISTINCT law_name FROM legislation_text")
    hub_laws = [r[0] for r in cur.fetchall()]
    secondary = [law for law in hub_laws if law.split("_")[1] in SECONDARY]
    if args.laws:
        wanted = {x.strip() for x in args.laws.split(",") if x.strip()}
        secondary = [law for law in secondary if law in wanted]
    cur.execute("SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks")
    bench = {r[0] for r in cur.fetchall()}
    bench |= {r[0] for r in duck.execute("SELECT name FROM legislation WHERE is_benchmark").fetchall()}
    enacted = {name: [e["name"] for e in (eb or []) if e.get("name")]
               for name, eb in duck.execute("SELECT name, enacted_by FROM legislation WHERE name IN (SELECT unnest(?))", [secondary]).fetchall()}

    os.makedirs(args.out, exist_ok=True)
    if args.gaps:
        gaps = sorted(law for law in secondary if not enacted.get(law))
        with open(os.path.join(args.out, "enacted_by_gaps.txt"), "w") as f:
            f.write("\n".join(gaps) + "\n")
        print(f"enacted_by gaps: {len(gaps)} of {len(secondary)} hub secondary laws → {args.out}/enacted_by_gaps.txt")

    # Holder-unknown duties: live, substantive Obligation provisions with no active actor and no adjudicated row
    cur.execute(
        """SELECT t.law_name, t.section_id, t.text FROM legislation_text t
           WHERE t.law_name = ANY(%s) AND 'Obligation' = ANY(t.drrp_types) AND t.scope = 'substantive'
             AND coalesce(t.status, '') <> 'repealed'
             AND NOT EXISTS (SELECT 1 FROM provision_actors a WHERE a.section_id = t.section_id
                             AND (a.position = 'active' OR a.adj_position IS NOT NULL))""",
        ([law for law in secondary if law not in bench],),
    )
    duties = [(law, sid, text) for law, sid, text in cur.fetchall() if text and DUTY_REF.search(text)]

    laws = sorted({law for law, _, _ in duties})
    cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)", (laws,))
    own = dict(cur.fetchall())
    defs: dict[str, dict[str, str]] = {}
    for sid, text in own.items():
        for d in DEFINES.finditer(text or ""):
            hit = act_by_title(duck, d.group("title"), d.group("tyear"))
            if hit:
                defs.setdefault(sid.split(":", 1)[0], {})[(d.group("year") or "").strip()] = hit

    rows, parents_needed = [], set()
    for law, sid, text in duties:
        cites = []
        for m in DUTY_REF.finditer(text):
            parents, how = resolve(duck, m, enacted.get(law, []), defs.get(law, {}))
            for p in parents:
                parents_needed.add(p)
            cites.append({"ref": m.group(0), "section": m.group("num") + m.group("sub"), "parents": parents, "resolved_by": how})
        rows.append({"law": law, "section_id": sid, "text": text, "enacted_by": enacted.get(law, []),
                     "stem": [{"section_id": a, "text": own.get(a)} for a in reversed(ancestors(sid)) if own.get(a)],
                     "cites": cites})

    cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s)", (sorted(parents_needed),))
    parent_text = dict(cur.fetchall())
    for r in rows:
        for c in r["cites"]:
            c["parent_provisions"] = []
            for p in c["parents"]:
                sid = f"{p}:s.{c['section']}"
                if sid not in parent_text:
                    sid = f"{p}:s.{re.match(r'\d+[A-Z]*', c['section']).group()}"  # fall back to the whole section
                stems = [{"section_id": a, "text": parent_text[a]} for a in reversed(ancestors(sid)) if parent_text.get(a)]
                # a whole section has no text of its own: give its subsections
                items = [] if parent_text.get(sid) else [
                    {"section_id": k, "text": parent_text[k]} for k in sorted(parent_text)
                    if k.startswith(sid + "(") and k.count("(") == sid.count("(") + 1 and parent_text[k]][:10]
                c["parent_provisions"].append({"section_id": sid, "text": parent_text.get(sid), "stem": stems,
                                               "items": items, "in_hub": sid in parent_text})

    path = os.path.join(args.out, "candidates.jsonl")
    with open(path, "w") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    unresolved = sum(1 for r in rows for c in r["cites"] if not c["parents"])
    missing = sum(1 for r in rows for c in r["cites"] for p in c["parent_provisions"] if not p["in_hub"])
    print(f"candidates: {len(rows)} provisions in {len(laws)} laws (benchmarks excluded: {len(bench & set(secondary))}); "
          f"unresolved cites: {unresolved}; parent provisions not in hub: {missing} → {path}")


if __name__ == "__main__":
    main()
