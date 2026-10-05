#!/usr/bin/python3
"""Actor dictionary gaps from LLM output and from sertantai-legal (actor-drift skill, `llm` mode).

Jason (2026-10-05): the actor dictionary is a known gap; jump on any dictionary diff as soon as it surfaces.
Run after every LLM batch (training labels, gold v2, LLM tier) and after legal changes its regex library.

Sources:
  training   drrp_training_labels_raw: latest response per provision
  gold_v2    gold_v2_raw: model responses at the current prompt version (--all-versions for older runs)
  hub        provision_actors.actor_label values not in the dictionary (any tier)
  legal      legal's actor_definitions.ex labels we lack (after legal_label_map.json)

Reports OTHER: actors (minus data/training/drrp-v1.1/accepted_other.txt) and labels not in the dictionary,
with counts and example provisions. Writes data/audit/dictionary_gaps/<date>.json. Exit 1 when there are gaps,
so a script can stop on it.

  llm_gaps.py                     # all sources
  llm_gaps.py --source training   # one source (repeatable)
"""

import argparse
import collections
import datetime as dt
import json
import os
import re
import sys

import psycopg

ROOT = "/var/home/jason/fractalaw"
sys.path.insert(0, os.path.join(ROOT, "scripts"))
from drrp_prompt import PROMPT_VERSION, dictionary_entries  # noqa: E402

PG = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw"
LEGAL = "/var/home/jason/Desktop/sertantai-legal/backend/lib/sertantai_legal/legal/taxa/actor_definitions.ex"
LEGAL_MAP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "legal_label_map.json")
ACCEPTED = os.path.join(ROOT, "data/training/drrp-v1.1/accepted_other.txt")
SOURCES = ["training", "gold_v2", "hub", "legal"]


def accepted() -> set[str]:
    try:
        return {line.split("\t")[0].strip().lower() for line in open(ACCEPTED) if line.strip() and not line.startswith("#")}
    except OSError:
        return set()


def legal_labels() -> dict[str, list[str]]:
    """label → patterns from legal's @government_patterns_raw / @governed_patterns_raw keyword lists."""
    out, cur, inside = {}, None, False
    for line in open(LEGAL):
        if re.search(r"@(?:government|governed)_patterns_raw \[", line):
            inside = True
            continue
        if inside and re.match(r"\s*\]\s*$", line):
            inside = False
            continue
        if not inside or line.strip().startswith("#"):
            continue
        m = re.match(r'\s*(?:"([^"]+)"|([A-Za-z][\w ]*)):\s*(.*)$', line)
        if m:
            cur = m.group(1) or m.group(2)
            out[cur] = re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(3))
        elif cur and line.strip().startswith('"'):
            out[cur] += re.findall(r'"((?:[^"\\]|\\.)*)"', line)
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--source", action="append", choices=SOURCES)
    ap.add_argument("--all-versions", action="store_true", help="gold_v2: all prompt versions, not just the current one")
    args = ap.parse_args()
    sources = args.source or SOURCES
    entries = dictionary_entries()
    labels = set(entries)
    # an OTHER actor whose words now match a label's triggers is covered (e.g. "producer" since SC: Producer)
    covered_by = {t.lower(): lab for lab, e in entries.items() for t in e["triggers"]}
    covered_by.update({lab.split(": ")[-1].lower(): lab for lab in labels})
    acc = accepted()
    covered = collections.defaultdict(int)
    # labels renamed in the dictionary (renamed_from) still appear in raw responses made before the rename
    import yaml
    renamed = {old: e["label"] for e in yaml.safe_load(open(os.path.join(ROOT, "crates/fractalaw-core/data/actor-dictionary.yaml")))
               for old in e.get("renamed_from", [])}
    other = collections.defaultdict(lambda: collections.defaultdict(list))    # OTHER key → source → sids
    unknown = collections.defaultdict(lambda: collections.defaultdict(list))  # label → source → sids

    def take(source: str, sid: str, label: str) -> None:
        if label.startswith("OTHER"):
            key = re.sub(r"^OTHER:\s*", "", label).strip().lower()
            hit = covered_by.get(key) or next((covered_by[t] for t in sorted(covered_by, key=len, reverse=True)
                                               if len(t) >= 4 and re.search(r"\b" + re.escape(t) + r"\b", key)), None)
            if hit:
                covered[f"{key} → {hit}"] += 1
            elif key not in acc:
                other[key][source].append(sid)
        elif label in renamed:
            covered[f"{label} → {renamed[label]} (renamed)"] += 1
        elif label not in labels:
            unknown[label][source].append(sid)

    with psycopg.connect(PG) as conn:
        if "training" in sources:
            for sid, resp in conn.execute(
                    "SELECT DISTINCT ON (section_id) section_id, response FROM drrp_training_labels_raw "
                    "WHERE response IS NOT NULL ORDER BY section_id, created_at DESC"):
                for a in resp.get("actors", []):
                    take("training", sid, a["label"])
        if "gold_v2" in sources:
            for sid, resp in conn.execute("SELECT section_id, response FROM gold_v2_raw WHERE response IS NOT NULL "
                                          "AND (%s OR prompt_version = %s)", (args.all_versions, PROMPT_VERSION)):
                for a in resp.get("actors", []):
                    take("gold_v2", sid, a["label"])
        hub_free = collections.Counter()
        if "hub" in sources:
            # Free-text labels (not dictionary labels) in the hub: summarised by tier, listed only if repeated
            for label, method, n, sid in conn.execute(
                    "SELECT actor_label, coalesce(extraction_method, '?'), count(*), min(section_id) FROM provision_actors GROUP BY 1, 2"):
                if label not in labels and not label.startswith("OTHER"):
                    hub_free[method] += n
                    if n > 2:
                        unknown[label]["hub"] += [sid] + [""] * (n - 1)

    legal_missing = {}
    if "legal" in sources and os.path.exists(LEGAL):
        lmap = {k: v for k, v in json.load(open(LEGAL_MAP)).items() if not k.startswith("_")}
        for lab, pats in legal_labels().items():
            if lab in labels or lab in lmap:
                continue
            legal_missing[lab] = pats
        mapped_missing = {lab: tgt for lab, tgt in lmap.items() if tgt and tgt not in labels}
    else:
        mapped_missing = {}

    def n(d):
        return sum(len(v) for v in d.values())

    print(f"dictionary: {len(labels)} labels; accepted unlabelled: {len(acc)}")
    print(f"\nOTHER actors ({len(other)}):")
    for k, d in sorted(other.items(), key=lambda kv: -n(kv[1])):
        ex = [s for v in d.values() for s in v if s][:3]
        print(f"  {k!r} ×{n(d)} [{', '.join(f'{s} {len(v)}' for s, v in d.items())}] e.g. {', '.join(ex)}")
    print(f"\nLabels not in the dictionary ({len(unknown)}):")
    for k, d in sorted(unknown.items(), key=lambda kv: -n(kv[1])):
        ex = [s for v in d.values() for s in v if s][:3]
        print(f"  {k!r} ×{n(d)} [{', '.join(f'{s} {len(v)}' for s, v in d.items())}] e.g. {', '.join(ex)}")
    if covered:
        print(f"\nOTHER actors now covered by the dictionary ({len(covered)}): "
              + ", ".join(f"{k} ×{v}" for k, v in sorted(covered.items(), key=lambda kv: -kv[1])[:12]))
    if hub_free:
        print(f"\nHub rows with free-text (non-dictionary) labels: {sum(hub_free.values())} by tier {dict(hub_free)}"
              " (listed above only when a label repeats >2×)")
    print(f"\nLegal labels we lack, unmapped ({len(legal_missing)}):")
    for k, p in sorted(legal_missing.items()):
        print(f"  {k!r} {p[:2]}")
    if mapped_missing:
        print(f"\nlegal_label_map targets missing from our dictionary: {mapped_missing}")

    out = os.path.join(ROOT, "data/audit/dictionary_gaps", f"{dt.date.today().isoformat()}.json")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    json.dump({"other": {k: dict(d) for k, d in other.items()}, "unknown": {k: {s: len(v) for s, v in d.items()} for k, d in unknown.items()},
               "legal_missing": legal_missing, "mapped_missing": mapped_missing,
               "covered": covered, "hub_free_text_rows": dict(hub_free)}, open(out, "w"), indent=1)
    gaps = len(other) + len(unknown) + len(legal_missing) + len(mapped_missing)
    print(f"\n{gaps} gaps → {out}")
    if gaps:
        print("Fix the same day (actor-drift skill, 'LLM gaps' loop): add the label, check for regex clashes "
              "(pattern + mask + test), measure the hub rows it affects and schedule the repair, tell legal.")
    sys.exit(1 if gaps else 0)


if __name__ == "__main__":
    main()
