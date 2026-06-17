#!/usr/bin/env /usr/bin/python3
"""Position classifier disagreement analysis against golden benchmarks.

Runs the position classifier on benchmark provisions and compares three views:
  - Regex position (what the pipeline currently ships)
  - Classifier prediction (logistic regression on embeddings)
  - Gemini gold standard (ground truth)

Reports which source is more accurate, aggregated by family, actor category,
and DRRP type.

Usage:
    /usr/bin/python3 scripts/benchmark_classifier_disagreements.py
    /usr/bin/python3 scripts/benchmark_classifier_disagreements.py --family "OH&S"
    /usr/bin/python3 scripts/benchmark_classifier_disagreements.py --mismatches 30
"""

import argparse
import glob
import json
import os
import re
import sys
from collections import Counter, defaultdict

import lancedb
import numpy as np
import pyarrow.parquet as pq

BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"
WEIGHTS_PATH = "docs/position_classifier_v1.json"

# Must match Rust: position_classifier.rs CATEGORIES
CATEGORIES = ["Org", "Ind", "Gvt", "SC", "Spc", "EU", "Svc", "Public", "Offshore", "other"]
DRRP_TYPES = ["Duty", "Right", "Responsibility", "Power", "none"]

# Modal keywords — must match drrp_classifier.rs modal_features()
MODAL_KEYWORDS = [
    "shall", "must", " may ", "requir", "ensur", "prohibit",
    ["duty", "duties"],     # word_match
    ["right", "rights"],    # word_match
    ["power", "powers"],    # word_match
    "responsib", "penalt", "offence", "exempt",
]


def modal_features(text):
    """Extract 13 modal binary features from text (matches Rust implementation)."""
    t = text.lower()
    features = []
    for kw in MODAL_KEYWORDS:
        if isinstance(kw, list):
            features.append(1.0 if any(w in t for w in kw) else 0.0)
        else:
            features.append(1.0 if kw in t else 0.0)
    return features


def actor_category(label):
    """Extract category prefix from actor label (e.g. 'Org: Employer' → 'Org')."""
    if ":" in label:
        return label.split(":")[0].strip()
    return "other"


def build_features(embedding, text, drrp_types, label, text_offset):
    """Build 413-dim feature vector (matches Rust build_position_features)."""
    features = list(embedding)  # 384

    # Modal features (13)
    features.extend(modal_features(text))

    # DRRP one-hot (5)
    for dt in DRRP_TYPES:
        features.append(1.0 if dt in (drrp_types or []) else 0.0)

    # Category one-hot (10)
    cat = actor_category(label)
    for c in CATEGORIES:
        features.append(1.0 if c == cat else 0.0)

    # Text offset (1)
    features.append(text_offset)

    return features


class PositionClassifier:
    def __init__(self, weights_path):
        with open(weights_path) as f:
            data = json.load(f)
        self.classes = data["classes"]
        self.coef = np.array(data["coef"], dtype=np.float32)
        self.intercept = np.array(data["intercept"], dtype=np.float32)

    def predict(self, features):
        x = np.array(features, dtype=np.float32)
        logits = self.coef @ x + self.intercept
        # softmax
        logits -= logits.max()
        exp = np.exp(logits)
        probs = exp / exp.sum()
        idx = int(np.argmax(probs))
        return self.classes[idx], float(probs[idx])


def load_benchmarks(family_filter=None):
    """Load benchmark Parquet files from NAS."""
    pattern = os.path.join(BENCHMARK_DIR, "tier2-*.parquet")
    files = sorted(glob.glob(pattern))
    if not files:
        print(f"No benchmark files found in {BENCHMARK_DIR}")
        sys.exit(1)

    tables = []
    for f in files:
        t = pq.read_table(f)
        if family_filter:
            slug = family_filter.lower().replace(" ", "")
            fname = os.path.basename(f).lower()
            if slug not in fname:
                continue
        tables.append(t)
        print(f"  Loaded {os.path.basename(f)}: {t.num_rows} provisions")

    if not tables:
        print(f"No benchmarks matched filter '{family_filter}'")
        sys.exit(1)

    import pyarrow as pa
    return pa.concat_tables(tables)


def load_pipeline(section_ids):
    """Load pipeline data for benchmark provisions from LanceDB."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    pipeline = {}
    for i in range(0, len(section_ids), 500):
        chunk = section_ids[i:i + 500]
        filter_expr = " OR ".join([f"section_id = '{sid}'" for sid in chunk])
        data = (
            tbl.search()
            .where(filter_expr)
            .select([
                "section_id", "text", "embedding", "drrp_types",
                "actors", "extraction_method",
            ])
            .limit(len(chunk) + 10)
            .to_arrow()
        )
        for j in range(data.num_rows):
            sid = data.column("section_id")[j].as_py()
            emb = data.column("embedding")[j].as_py()
            actors_raw = data.column("actors")[j].as_py() or []
            pipeline[sid] = {
                "text": data.column("text")[j].as_py() or "",
                "embedding": emb,
                "drrp": data.column("drrp_types")[j].as_py() or [],
                "actors": [
                    {"label": a.get("label", ""), "position": a.get("position", "")}
                    for a in actors_raw if isinstance(a, dict)
                ],
                "method": data.column("extraction_method")[j].as_py() or "",
            }
    return pipeline


def normalise_position(pos):
    """Map gold/pipeline positions to classifier's 3-class scheme."""
    pos = (pos or "").lower().strip()
    if pos in ("active",):
        return "active"
    if pos in ("counterparty",):
        return "counterparty"
    return "other"  # beneficiary, mentioned, etc.


def normalise_label(label):
    """Normalise gold actor labels for matching against pipeline labels."""
    label = label.strip()
    # Gold labels may be bare ("Employer") while pipeline uses "Org: Employer"
    # Try to match on the suffix
    return label


def match_gold_to_pipeline(gold_actors, pipeline_actors):
    """Match gold actors to pipeline actors by label similarity.

    Returns list of (gold_label, gold_pos, pipeline_label, pipeline_pos) tuples.
    """
    matches = []
    used_pipeline = set()

    for ga in gold_actors:
        g_label = ga.get("label", "")
        g_pos = ga.get("position", "")
        g_label_lower = g_label.lower()

        best_match = None
        best_score = 0

        for idx, pa_item in enumerate(pipeline_actors):
            if idx in used_pipeline:
                continue
            p_label = pa_item.get("label", "")
            p_label_lower = p_label.lower()

            # Exact match (ignoring category prefix)
            p_suffix = p_label.split(":")[-1].strip().lower() if ":" in p_label else p_label_lower
            g_suffix = g_label.split(":")[-1].strip().lower() if ":" in g_label else g_label_lower

            if g_suffix == p_suffix or g_label_lower == p_label_lower:
                score = 3
            elif g_suffix in p_suffix or p_suffix in g_suffix:
                score = 2
            elif any(w in p_label_lower for w in g_label_lower.split() if len(w) > 3):
                score = 1
            else:
                score = 0

            if score > best_score:
                best_score = score
                best_match = (idx, pa_item)

        if best_match and best_score >= 1:
            idx, pa_item = best_match
            used_pipeline.add(idx)
            matches.append((g_label, g_pos, pa_item["label"], pa_item["position"]))

    return matches


def main():
    parser = argparse.ArgumentParser(description="Position classifier disagreement analysis")
    parser.add_argument("--family", help="Filter benchmarks by family")
    parser.add_argument("--mismatches", type=int, default=20, help="Detailed mismatches to show")
    parser.add_argument("--text", action="store_true", help="Show provision text alongside mismatches")
    args = parser.parse_args()

    print("=== Position Classifier Disagreement Analysis ===\n")

    # Load classifier
    cls = PositionClassifier(WEIGHTS_PATH)
    print(f"Loaded position classifier: {len(cls.classes)} classes, {cls.coef.shape[1]} features\n")

    # Load benchmarks
    print("Loading golden benchmarks...")
    bench = load_benchmarks(args.family)

    # Build gold dict
    gold = {}
    for i in range(bench.num_rows):
        sid = bench.column("section_id")[i].as_py()
        gold[sid] = {
            "drrp": bench.column("gold_drrp_types")[i].as_py() or [],
            "actors": json.loads(bench.column("gold_actors")[i].as_py() or "[]"),
            "family": bench.column("family")[i].as_py() or "",
            "text": bench.column("text")[i].as_py() or "",
        }

    # Load pipeline data
    print(f"\nLoading pipeline data for {len(gold)} provisions...")
    pipeline = load_pipeline(list(gold.keys()))
    print(f"  Matched {len(pipeline)} in LanceDB")

    # Track results
    stats = {
        "total_provisions": 0,
        "provisions_with_embedding": 0,
        "total_actor_pairs": 0,
        "regex_correct": 0,
        "classifier_correct": 0,
        "both_correct": 0,
        "both_wrong": 0,
        "regex_only_correct": 0,
        "classifier_only_correct": 0,
    }
    by_family = defaultdict(lambda: {"total": 0, "regex_ok": 0, "cls_ok": 0, "disagree": 0})
    by_category = defaultdict(lambda: {"total": 0, "regex_ok": 0, "cls_ok": 0, "disagree": 0})
    by_drrp = defaultdict(lambda: {"total": 0, "regex_ok": 0, "cls_ok": 0, "disagree": 0})
    detailed_mismatches = []  # where classifier disagrees with regex

    no_embedding = 0
    no_actors = 0
    skipped_agentic = 0

    for sid, g in gold.items():
        if sid not in pipeline:
            continue
        p = pipeline[sid]
        stats["total_provisions"] += 1

        # Skip agentic — already gold standard
        if p["method"] in ("agentic", "agentic_unvalidated"):
            skipped_agentic += 1
            continue

        # Need embedding for classifier
        emb = p.get("embedding")
        if emb is None or len(emb) != 384:
            no_embedding += 1
            continue
        stats["provisions_with_embedding"] += 1

        # Match gold actors to pipeline actors
        matches = match_gold_to_pipeline(g["actors"], p["actors"])
        if not matches:
            no_actors += 1
            continue

        family = g["family"]
        drrp_type = g["drrp"][0] if g["drrp"] else "none"
        text = p["text"] or g["text"]

        for g_label, g_pos, p_label, p_pos in matches:
            gold_pos = normalise_position(g_pos)
            regex_pos = normalise_position(p_pos)

            # Run classifier
            cat = actor_category(p_label)
            text_lower = text.lower()
            # Compute text offset: where does the actor label first appear?
            label_suffix = p_label.split(":")[-1].strip().lower() if ":" in p_label else p_label.lower()
            idx = text_lower.find(label_suffix)
            text_offset = idx / max(len(text), 1) if idx >= 0 else 0.5

            features = build_features(emb, text, p["drrp"], p_label, text_offset)
            cls_pos, cls_conf = cls.predict(features)

            stats["total_actor_pairs"] += 1
            regex_ok = regex_pos == gold_pos
            cls_ok = cls_pos == gold_pos

            if regex_ok:
                stats["regex_correct"] += 1
            if cls_ok:
                stats["classifier_correct"] += 1
            if regex_ok and cls_ok:
                stats["both_correct"] += 1
            elif regex_ok and not cls_ok:
                stats["regex_only_correct"] += 1
            elif cls_ok and not regex_ok:
                stats["classifier_only_correct"] += 1
            else:
                stats["both_wrong"] += 1

            # Track by dimensions
            for bucket, key in [(by_family, family), (by_category, cat), (by_drrp, drrp_type)]:
                bucket[key]["total"] += 1
                if regex_ok:
                    bucket[key]["regex_ok"] += 1
                if cls_ok:
                    bucket[key]["cls_ok"] += 1
                if regex_pos != cls_pos:
                    bucket[key]["disagree"] += 1

            # Record detailed mismatch where classifier disagrees with regex
            if regex_pos != cls_pos:
                detailed_mismatches.append({
                    "section_id": sid,
                    "actor": p_label,
                    "gold": gold_pos,
                    "regex": regex_pos,
                    "classifier": cls_pos,
                    "cls_conf": cls_conf,
                    "family": family,
                    "drrp": drrp_type,
                    "regex_correct": regex_ok,
                    "cls_correct": cls_ok,
                    "text": text,
                })

    # === Report ===
    total = stats["total_actor_pairs"]
    if total == 0:
        print("\nNo actor pairs to analyze.")
        return

    print(f"\n{'=' * 70}")
    print(f"OVERVIEW")
    print(f"{'=' * 70}")
    print(f"Total provisions in benchmarks:    {stats['total_provisions']}")
    print(f"  Skipped (agentic):               {skipped_agentic}")
    print(f"  Skipped (no embedding):          {no_embedding}")
    print(f"  Skipped (no actor matches):      {no_actors}")
    print(f"  Analysed (with embedding):       {stats['provisions_with_embedding']}")
    print(f"Total actor-position pairs:        {total}")
    print(f"")
    print(f"Regex correct:                     {stats['regex_correct']}/{total} ({100*stats['regex_correct']/total:.1f}%)")
    print(f"Classifier correct:                {stats['classifier_correct']}/{total} ({100*stats['classifier_correct']/total:.1f}%)")
    print(f"Both correct:                      {stats['both_correct']}/{total} ({100*stats['both_correct']/total:.1f}%)")
    print(f"Both wrong:                        {stats['both_wrong']}/{total} ({100*stats['both_wrong']/total:.1f}%)")
    print(f"Regex only correct:                {stats['regex_only_correct']}/{total} ({100*stats['regex_only_correct']/total:.1f}%)")
    print(f"Classifier only correct:           {stats['classifier_only_correct']}/{total} ({100*stats['classifier_only_correct']/total:.1f}%)")
    disagree_total = sum(1 for m in detailed_mismatches)
    print(f"Total disagreements (regex≠cls):   {disagree_total}/{total} ({100*disagree_total/total:.1f}%)")

    # By family
    print(f"\n{'=' * 70}")
    print(f"BY FAMILY")
    print(f"{'=' * 70}")
    print(f"{'Family':<45s} {'Pairs':>5s} {'Regex%':>7s} {'Cls%':>7s} {'Dis%':>7s}")
    for fam in sorted(by_family, key=lambda f: -by_family[f]["total"]):
        d = by_family[fam]
        if d["total"] == 0:
            continue
        print(f"{fam[:44]:<45s} {d['total']:>5d} {100*d['regex_ok']/d['total']:>6.1f}% {100*d['cls_ok']/d['total']:>6.1f}% {100*d['disagree']/d['total']:>6.1f}%")

    # By category
    print(f"\n{'=' * 70}")
    print(f"BY ACTOR CATEGORY")
    print(f"{'=' * 70}")
    print(f"{'Category':<20s} {'Pairs':>5s} {'Regex%':>7s} {'Cls%':>7s} {'Dis%':>7s}")
    for cat in sorted(by_category, key=lambda c: -by_category[c]["total"]):
        d = by_category[cat]
        if d["total"] == 0:
            continue
        print(f"{cat:<20s} {d['total']:>5d} {100*d['regex_ok']/d['total']:>6.1f}% {100*d['cls_ok']/d['total']:>6.1f}% {100*d['disagree']/d['total']:>6.1f}%")

    # By DRRP type
    print(f"\n{'=' * 70}")
    print(f"BY DRRP TYPE")
    print(f"{'=' * 70}")
    print(f"{'DRRP':<20s} {'Pairs':>5s} {'Regex%':>7s} {'Cls%':>7s} {'Dis%':>7s}")
    for dt in sorted(by_drrp, key=lambda d: -by_drrp[d]["total"]):
        d = by_drrp[dt]
        if d["total"] == 0:
            continue
        print(f"{dt:<20s} {d['total']:>5d} {100*d['regex_ok']/d['total']:>6.1f}% {100*d['cls_ok']/d['total']:>6.1f}% {100*d['disagree']/d['total']:>6.1f}%")

    # High-value disagreements: classifier correct, regex wrong
    cls_wins = [m for m in detailed_mismatches if m["cls_correct"] and not m["regex_correct"]]
    regex_wins = [m for m in detailed_mismatches if m["regex_correct"] and not m["cls_correct"]]
    both_wrong_dis = [m for m in detailed_mismatches if not m["regex_correct"] and not m["cls_correct"]]

    print(f"\n{'=' * 70}")
    print(f"DISAGREEMENT BREAKDOWN ({disagree_total} total)")
    print(f"{'=' * 70}")
    print(f"Classifier wins (cls correct, regex wrong):  {len(cls_wins)}")
    print(f"Regex wins (regex correct, cls wrong):       {len(regex_wins)}")
    print(f"Both wrong (disagree but neither correct):   {len(both_wrong_dis)}")

    def print_mismatch(m, show_text):
        print(f"  {m['section_id']}")
        print(f"    {m['actor']}: gold={m['gold']} regex={m['regex']} cls={m['classifier']}@{m['cls_conf']:.2f}  [{m['family']}, {m['drrp']}]")
        if show_text:
            txt = m.get("text", "")
            # Truncate long text but show enough to see the pattern
            if len(txt) > 300:
                txt = txt[:300] + "..."
            print(f"    TEXT: {txt}")
            print()

    if args.mismatches > 0 and cls_wins:
        n = min(args.mismatches, len(cls_wins))
        print(f"\n--- Classifier wins (top {n} by confidence) ---")
        for m in sorted(cls_wins, key=lambda x: -x["cls_conf"])[:n]:
            print_mismatch(m, args.text)

    if args.mismatches > 0 and regex_wins:
        n = min(args.mismatches, len(regex_wins))
        print(f"\n--- Regex wins (top {n} by classifier confidence — high-confidence errors) ---")
        for m in sorted(regex_wins, key=lambda x: -x["cls_conf"])[:n]:
            print_mismatch(m, args.text)

    if args.text and args.mismatches > 0 and both_wrong_dis:
        n = min(args.mismatches, len(both_wrong_dis))
        print(f"\n--- Both wrong (top {n} by classifier confidence) ---")
        for m in sorted(both_wrong_dis, key=lambda x: -x["cls_conf"])[:n]:
            print_mismatch(m, args.text)


if __name__ == "__main__":
    main()
