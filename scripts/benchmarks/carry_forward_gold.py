#!/usr/bin/python3
"""Carry benchmark gold labels forward across a LAT sync (2026-09-30).

For each gold label in the snapshot table whose provision changed in the sync:
  - same section_id, text changed cosmetically  -> carry (adjudicated tier)
  - row renumbered (old id removed, one current row with the same
    normalised text)                            -> carry onto the new id
  - text changed substantively                  -> review queue
  - old text gone                               -> archived (snapshot only)
Labels on unchanged rows are left alone (still keyed in gold_benchmarks), and
labels whose section_id never matched the hub stay in the snapshot.

Carried labels are written to provision_actors.adj_drrp/adj_position/adj_note,
the adjudicated tier, which reconcile never overrides. gold_benchmarks is never
modified. Dry run unless --apply.

Usage:
  /usr/bin/python3 scripts/benchmarks/carry_forward_gold.py --laws UK_ukpga_1990_10 [--apply]
"""

import argparse
import csv
import difflib
import re
import unicodedata
from collections import defaultdict

import psycopg

PG = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw"
SNAPSHOT = "benchmark_gold_snapshot_20260930"
REVIEW = "benchmark_gold_review_20260930"
NOTE = f"carried from gold 2026-09-30 ({SNAPSHOT})"
THRESHOLD = 0.97


def normalise(text: str | None) -> str:
    """Ignore whitespace, quote/dash style, amendment markers, numbering and punctuation."""
    if not text:
        return ""
    t = unicodedata.normalize("NFKC", text).lower()
    t = re.sub(r"[‘’“”\"'`]", "", t)
    t = re.sub(r"[‐-―\-]", " ", t)
    t = re.sub(r"\[?\bf\d+\b", " ", t)  # amendment markers [F12
    t = re.sub(r"\(\s*[0-9a-z]{1,5}\s*\)", " ", t)  # (1) (a) (iv)
    t = re.sub(r"\b\d+[a-z]{0,2}\b", " ", t)  # section/cross-ref numbers
    t = re.sub(r"[^\w\s]", " ", t)
    return re.sub(r"\s+", " ", t).strip()


def similarity(a: str, b: str) -> float:
    if a == b:
        return 1.0
    # A section heading prefixed to (or dropped from) the same body is cosmetic:
    # "35 application of act to isles of scilly in relation to land …"
    short, long = sorted((a, b), key=len)
    if len(short) >= 30 and long.endswith(short) and len(long) - len(short) <= 100:
        return 1.0
    return difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--laws", required=True)
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--threshold", type=float, default=THRESHOLD)
    args = ap.parse_args()
    laws = [law.strip() for law in args.laws.split(",") if law.strip()]

    with psycopg.connect(PG) as conn, conn.cursor() as cur:
        cur.execute(
            f"CREATE TABLE IF NOT EXISTS {REVIEW} (law_name text, section_id text, new_section_id text, "
            "actor_label text, gold_drrp text, gold_position text, old_text text, new_text text, "
            "similarity real, status text, reviewed_at timestamptz, decision text)"
        )
        report = []
        for law in laws:
            cur.execute(
                f"SELECT section_id, actor_label, gold_drrp, gold_position, text_at_snapshot "
                f"FROM {SNAPSHOT} WHERE law_name = %s",
                (law,),
            )
            labels = cur.fetchall()
            cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = %s", (law,))
            current = dict(cur.fetchall())
            by_norm = defaultdict(list)
            for sid, text in current.items():
                by_norm[normalise(text)].append(sid)
            cur.execute("SELECT actor_label, max(actor_category) FROM provision_actors GROUP BY 1")
            category = dict(cur.fetchall())

            counts = defaultdict(int)
            for sid, label, gdrrp, gpos, old_text in labels:
                if old_text is None:
                    counts["unmatched_id"] += 1
                    continue
                old_n = normalise(old_text)
                target, sim = None, 0.0
                if sid in current:
                    new_text = current[sid]
                    if new_text == old_text:
                        counts["unchanged"] += 1
                        continue
                    target, sim = sid, similarity(old_n, normalise(new_text))
                else:
                    candidates = by_norm.get(old_n, [])
                    if len(candidates) == 1:
                        target, sim = candidates[0], 1.0
                    else:
                        counts["archived"] += 1
                        continue
                new_text = current[target]
                if sim >= args.threshold:
                    status = "carried" if target == sid else "carried_renumbered"
                    counts[status] += 1
                    if args.apply:
                        cur.execute(
                            "INSERT INTO provision_actors (section_id, actor_label, actor_category, adj_drrp, adj_position, adj_note) "
                            "VALUES (%s, %s, %s, %s, %s, %s) ON CONFLICT (section_id, actor_label) DO UPDATE SET "
                            "adj_drrp = EXCLUDED.adj_drrp, adj_position = EXCLUDED.adj_position, adj_note = EXCLUDED.adj_note",
                            (target, label, category.get(label, "Unknown"), gdrrp, gpos,
                             NOTE if target == sid else f"{NOTE}; renumbered from {sid}"),
                        )
                else:
                    status = "review"
                    counts["review"] += 1
                    if args.apply:
                        cur.execute(
                            f"INSERT INTO {REVIEW} (law_name, section_id, new_section_id, actor_label, gold_drrp, "
                            "gold_position, old_text, new_text, similarity, status) VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,'pending')",
                            (law, sid, target, label, gdrrp, gpos, old_text, new_text, sim),
                        )
            report.append((law, len(labels), dict(counts)))
        if args.apply:
            conn.commit()
        else:
            conn.rollback()

    keys = ["unchanged", "carried", "carried_renumbered", "review", "archived", "unmatched_id"]
    print(("APPLIED" if args.apply else "DRY RUN") + f" (threshold {args.threshold})")
    print("law\tlabels\t" + "\t".join(keys))
    for law, n, c in report:
        print(f"{law}\t{n}\t" + "\t".join(str(c.get(k, 0)) for k in keys))
    if args.apply:
        with open(f"data/audit/benchmark_gold_carry_forward_20260930.tsv", "a", newline="") as f:
            w = csv.writer(f, delimiter="\t")
            for law, n, c in report:
                w.writerow([law, n] + [c.get(k, 0) for k in keys])


if __name__ == "__main__":
    main()
