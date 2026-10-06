#!/usr/bin/python3
"""Evidence pack for the gold v3 provisions (phase 0a): everything a justifier or reviewer needs per provision.

One JSONL row per selected provision:
  context   law title, Part/Chapter titles and cross-heading (via hierarchy_path), and the labelling context
            exactly as the models saw it (stem, referenced and applying provisions: drrp_prompt.user_prompt)
  tiers     the pipeline's own outputs: stored purposes and drrp_types, and per actor the regex / cls / slm /
            llm / final position and type (provision_actors)
  labels    the latest v1.3 label per model (Gemini, GPT-5.4-mini, GPT-5.5) and the Opus referee decision
  cue       the coarse purpose cue (coarse_purpose.py) and how it decided

Reads only; writes data/gold/v3/evidence.jsonl.

  evidence.py
"""

import argparse
import csv
import glob
import hashlib
import json
import os
import sys

import re

import duckdb
import psycopg2

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import ancestors, applying, applying_index, references, user_prompt  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from coarse_purpose import classify, coarse  # noqa: E402

# “term” means …, including the Welsh form “term” (“term in Welsh”) means …
DEFINES = re.compile(r"“([^”]{2,60})”(?:\s*\(“[^”]{1,60}”\))?\s+(?:means|includes|has the (?:same )?meaning)", re.I)

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DUCK = os.path.join(ROOT, "data/fractalaw.duckdb")
REFEREE = os.path.join(ROOT, "data/training/drrp-v1.1/referee")
# later files win: v1.3 re-rulings over batch 1, Opus decisions over the auto-settled rows
REFEREE_ORDER = ["batch1.jsonl", "batch1_v13.jsonl", "batch2_auto.jsonl", "batch2a.jsonl", "batch2b.jsonl"]
MODELS = {"gemini-3.8-flash": "gemini", "gpt-5.4-mini:low": "mini", "gpt-5.5:low": "gpt55"}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--selection", default=os.path.join(ROOT, "data/gold/v3/selection.csv"))
    ap.add_argument("--out", default=os.path.join(ROOT, "data/gold/v3/evidence.jsonl"))
    args = ap.parse_args()

    sel = {r["section_id"]: r for r in csv.DictReader(open(args.selection))}
    ids = sorted(sel)
    laws = sorted({r["law_name"] for r in sel.values()})
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT section_id, law_name, text, part, section_type, hierarchy_path, purposes, drrp_types
                   FROM legislation_text WHERE law_name = ANY(%s)""", (laws,))
    rows = {r[0]: r for r in cur.fetchall()}
    texts = {sid: r[2] for sid, r in rows.items()}
    parts = {sid: r[3] for sid, r in rows.items()}
    titles = {(r[1], r[5]): (r[4], r[2]) for r in rows.values() if r[4] in ("part", "chapter", "heading") and r[5]}
    app_idx = applying_index(texts, parts)
    # the law's own definitions ("the Authority" means …), so a justifier needn't guess defined terms
    defs: dict[str, dict] = {}
    for dsid, dr in rows.items():
        for m in DEFINES.finditer(dr[2] or ""):
            defs.setdefault(dr[1], {}).setdefault(m.group(1).strip(), (dsid, dr[2]))

    cur.execute("""SELECT section_id, actor_label, regex_position, regex_drrp, cls_position, cls_drrp, slm_position, slm_drrp,
                          llm_position, llm_drrp, inferred_position, inferred_drrp, adj_position, adj_drrp, position, drrp,
                          extraction_method
                   FROM provision_actors WHERE section_id = ANY(%s)""", (ids,))
    actors: dict[str, list] = {}
    for r in cur.fetchall():
        tiers = {t: [r[i], r[i + 1]] for t, i in
                 (("regex", 2), ("cls", 4), ("slm", 6), ("llm", 8), ("inferred", 10), ("adjudicated", 12), ("final", 14))
                 if r[i] or r[i + 1]}
        actors.setdefault(r[0], []).append({"label": r[1], **tiers, "method": r[16]})

    cur.execute("""SELECT DISTINCT ON (section_id, model) section_id, model, text_md5, response
                   FROM drrp_training_labels_raw
                   WHERE error IS NULL AND prompt_version LIKE 'drrp-v1.3%%' AND section_id = ANY(%s)
                   ORDER BY section_id, model, created_at DESC""", (ids,))
    labels: dict[str, dict] = {}
    for sid, model, md5, resp in cur.fetchall():
        labels.setdefault(sid, {})[MODELS.get(model, model)] = {**resp, "text_md5": md5}
    for name in REFEREE_ORDER:
        path = os.path.join(REFEREE, name)
        for line in open(path) if os.path.exists(path) else []:
            r = json.loads(line)
            if r["section_id"] in sel:
                labels.setdefault(r["section_id"], {})["referee"] = {**r["final"], "rationale": r.get("rationale"),
                                                                      "sided_with": r.get("sided_with"), "file": name}

    duck = duckdb.connect(DUCK, read_only=True)
    law_title = dict(duck.execute("SELECT name, title FROM legislation WHERE name IN (SELECT unnest(?))", [laws]).fetchall())

    n = 0
    with open(args.out, "w") as f:
        for sid in ids:
            r = rows.get(sid)
            if not r:
                print(f"  skip {sid}: no longer in legislation_text")
                continue
            law, text, path = r[1], r[2], r[5] or ""
            segs = path.split("/")
            heads = [{"type": titles[(law, p)][0], "title": titles[(law, p)][1]}
                     for p in ("/".join(segs[:i]) for i in range(1, len(segs))) if (law, p) in titles]
            stems = [(a, texts[a]) for a in ancestors(sid) if texts.get(a)]
            refs = references(sid, text, texts)
            # references in the stem too ("The measures required by paragraph (1) …" on a list item)
            seen = {sid, *(a for a, _ in stems), *(x for x, _ in refs)}
            for a, t in stems:
                for x, xt in references(a, t, texts):
                    if x not in seen:
                        refs.append((x, xt))
                        seen.add(x)
            body = " ".join([t for _, t in stems] + [text])
            used = sorted((term for term in defs.get(law, {})
                           if re.search(r"(?<!\w)" + re.escape(term) + r"(?!\w)", body, re.I)), key=len, reverse=True)[:8]
            definitions = [defs[law][term] for term in used]
            apps = applying(sid, parts, app_idx)
            md5 = hashlib.md5(text.encode()).hexdigest()
            lab = labels.get(sid, {})
            stale = sorted(m for m, v in lab.items() if v.get("text_md5") and v["text_md5"] != md5)
            f.write(json.dumps({
                "section_id": sid, "law_name": law, "law_title": law_title.get(law), "text_md5": md5,
                "selection": sel[sid]["part"], "stratum": sel[sid]["stratum"],
                "headings": heads, "text": text,
                "context": user_prompt(sid, text, stems, refs, apps).replace(
                    "\n\nPROVISION TO LABEL", "\n\nDEFINITIONS (same law; terms used above):\n"
                    + ("\n".join(f"[{d}] {t[:600]}" for d, t in definitions) or "(none)") + "\n\nPROVISION TO LABEL", 1),
                "definitions": [d for d, _ in definitions],
                "has_stem": bool(stems), "has_refs": bool(refs), "has_applying": bool(apps),
                "cue": dict(zip(("coarse_purpose", "how"), (lambda c: (coarse(c[0]), c[1]))(classify(sid, texts)))),
                "tiers": {"purposes": r[6], "drrp_types": r[7], "actors": actors.get(sid, [])},
                "labels": lab, "stale_labels": stale,
            }, ensure_ascii=False) + "\n")
            n += 1
    print(f"evidence: {n:,} provisions → {args.out}")
    print(f"  with labels: {sum(1 for s in ids if s in labels):,}; referee {sum('referee' in labels.get(s, {}) for s in ids)}; "
          f"pipeline actors {sum(1 for s in ids if s in actors):,}")


if __name__ == "__main__":
    main()
