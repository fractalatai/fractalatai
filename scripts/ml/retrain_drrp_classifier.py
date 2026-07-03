#!/usr/bin/env /usr/bin/python3
"""Retrain the DRRP classifier (Obligation/Liberty/none) with benchmark data.

Combines existing agentic training data from LanceDB with golden benchmark
provisions from NAS. The benchmark data adds critical `none` examples that
the v6 classifier lacks, causing over-prediction of DRRP.

Usage:
    /usr/bin/python3 scripts/retrain_drrp_classifier.py
    /usr/bin/python3 scripts/retrain_drrp_classifier.py --output docs/drrp_classifier_v7.json
"""

import argparse
import glob
import json
import os
import re
import sys

import lancedb
import numpy as np
import pyarrow.parquet as pq

BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"

# Modal features — must match Rust drrp_classifier.rs modal_features()
MODAL_KEYWORDS = [
    "shall", "must", " may ", "requir", "ensur", "prohibit",
    ["duty", "duties"],
    ["right", "rights"],
    ["power", "powers"],
    "responsib", "penalt", "offence", "exempt",
]


def modal_features(text):
    t = text.lower()
    features = []
    for kw in MODAL_KEYWORDS:
        if isinstance(kw, list):
            features.append(1.0 if any(w in t for w in kw) else 0.0)
        else:
            features.append(1.0 if kw in t else 0.0)
    return features


def drrp_to_3class(drrp_type):
    """Map 5-class DRRP to 3-class hierarchy."""
    if drrp_type in ("Duty", "Responsibility"):
        return "Obligation"
    if drrp_type in ("Right", "Power"):
        return "Liberty"
    return "none"


def load_agentic_training_data():
    """Load existing agentic training data from LanceDB."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    data = (
        tbl.search()
        .where("extraction_method = 'agentic'")
        .select(["section_id", "text", "embedding", "drrp_types"])
        .limit(10000)
        .to_arrow()
    )

    examples = []
    for i in range(data.num_rows):
        emb = data.column("embedding")[i].as_py()
        if emb is None or len(emb) != 384:
            continue
        text = data.column("text")[i].as_py() or ""
        drrp = data.column("drrp_types")[i].as_py() or []
        sid = data.column("section_id")[i].as_py()

        label = drrp_to_3class(drrp[0]) if drrp else "none"
        features = list(emb) + modal_features(text)
        examples.append({"sid": sid, "features": features, "label": label})

    return examples


def load_benchmark_training_data():
    """Load benchmark provisions with gold labels + embeddings."""
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    # Load gold labels from NAS
    gold = {}
    for f in sorted(glob.glob(os.path.join(BENCHMARK_DIR, "tier2-*.parquet"))):
        t = pq.read_table(f)
        for i in range(t.num_rows):
            sid = t.column("section_id")[i].as_py()
            drrp = t.column("gold_drrp_types")[i].as_py() or []
            gold[sid] = drrp[0] if drrp else "none"

    # Load embeddings + text from LanceDB
    examples = []
    sids = list(gold.keys())
    for batch_start in range(0, len(sids), 500):
        chunk = sids[batch_start : batch_start + 500]
        filt = " OR ".join([f"section_id = '{s}'" for s in chunk])
        data = (
            tbl.search()
            .where(filt)
            .select(["section_id", "text", "embedding"])
            .limit(len(chunk) + 10)
            .to_arrow()
        )
        for i in range(data.num_rows):
            sid = data.column("section_id")[i].as_py()
            emb = data.column("embedding")[i].as_py()
            if emb is None or len(emb) != 384:
                continue
            text = data.column("text")[i].as_py() or ""
            label = drrp_to_3class(gold[sid])
            features = list(emb) + modal_features(text)
            examples.append({"sid": sid, "features": features, "label": label})

    return examples


def main():
    parser = argparse.ArgumentParser(description="Retrain DRRP classifier")
    parser.add_argument(
        "--output",
        default="docs/drrp_classifier_v7.json",
        help="Output weights path",
    )
    args = parser.parse_args()

    print("Loading agentic training data from LanceDB...")
    agentic = load_agentic_training_data()
    print(f"  {len(agentic)} agentic examples")

    print("Loading benchmark training data from NAS...")
    benchmark = load_benchmark_training_data()
    print(f"  {len(benchmark)} benchmark examples")

    # Combine, deduplicate by section_id (benchmark takes precedence)
    by_sid = {}
    for ex in agentic:
        by_sid[ex["sid"]] = ex
    for ex in benchmark:
        by_sid[ex["sid"]] = ex  # benchmark overwrites agentic
    combined = list(by_sid.values())
    print(f"\nCombined: {len(combined)} unique examples")

    # Class distribution
    from collections import Counter
    dist = Counter(ex["label"] for ex in combined)
    for cls, count in dist.most_common():
        print(f"  {cls}: {count} ({100*count/len(combined):.1f}%)")

    # Split: 80% train, 20% test (stratified)
    from sklearn.model_selection import train_test_split

    X = np.array([ex["features"] for ex in combined], dtype=np.float32)
    y = np.array([ex["label"] for ex in combined])

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y
    )
    print(f"\nTrain: {len(X_train)}, Test: {len(X_test)}")

    # Train
    from sklearn.linear_model import LogisticRegression
    from sklearn.metrics import classification_report

    model = LogisticRegression(
        max_iter=1000,
        C=1.0,
        class_weight="balanced",
        random_state=42,
    )
    model.fit(X_train, y_train)

    # Evaluate
    y_pred = model.predict(X_test)
    accuracy = (y_pred == y_test).mean()
    print(f"\nAccuracy: {accuracy:.1%}")
    print(classification_report(y_test, y_pred))

    # Export weights
    weights = {
        "classes": model.classes_.tolist(),
        "coef": model.coef_.tolist(),
        "intercept": model.intercept_.tolist(),
        "feature_dim": int(model.coef_.shape[1]),
        "features": "embedding(384)+modal(13)=397",
        "training_examples": len(X_train),
        "test_accuracy": float(accuracy),
        "version": "v7",
    }
    with open(args.output, "w") as f:
        json.dump(weights, f, indent=2)
    print(f"\nSaved weights to {args.output}")

    # Also run the combined regex+classifier simulation
    print("\n=== Simulated regex + v7 classifier benchmark ===")
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")

    import yaml
    with open("crates/fractalaw-core/data/actor-dictionary.yaml") as f:
        lines = [l for l in f if not l.startswith("#")]
        actors_yaml = yaml.safe_load("".join(lines))
    govt_labels = set(a["label"] for a in actors_yaml if a.get("type") == "government")

    HAS_MODAL = re.compile(r"(?i)\bshall\b|\bmust\b|\bmay\b")

    gold = {}
    for f_path in sorted(glob.glob(os.path.join(BENCHMARK_DIR, "tier2-*.parquet"))):
        t = pq.read_table(f_path)
        for i in range(t.num_rows):
            sid = t.column("section_id")[i].as_py()
            drrp = t.column("gold_drrp_types")[i].as_py() or []
            gold[sid] = drrp[0] if drrp else "none"

    coef = np.array(weights["coef"], dtype=np.float32)
    intercept = np.array(weights["intercept"], dtype=np.float32)
    cls_classes = weights["classes"]

    def predict_v7(embedding, text):
        t = text.lower()
        modals = np.array([float(fn(t)) for fn in [
            lambda t: "shall" in t, lambda t: "must" in t, lambda t: " may " in t,
            lambda t: "requir" in t, lambda t: "ensur" in t, lambda t: "prohibit" in t,
            lambda t: any(w in t for w in ["duty", "duties"]),
            lambda t: any(w in t for w in ["right", "rights"]),
            lambda t: any(w in t for w in ["power", "powers"]),
            lambda t: "responsib" in t, lambda t: "penalt" in t, lambda t: "offence" in t, lambda t: "exempt" in t,
        ]], dtype=np.float32)
        features = np.concatenate([np.array(embedding, dtype=np.float32), modals])
        logits = coef @ features + intercept
        logits -= logits.max()
        probs = np.exp(logits) / np.exp(logits).sum()
        idx = int(np.argmax(probs))
        return cls_classes[idx], float(probs[idx])

    def decompose(cls_name, has_govt):
        if cls_name == "Obligation":
            return "Responsibility" if has_govt else "Duty"
        elif cls_name == "Liberty":
            return "Power" if has_govt else "Right"
        return "none"

    for label, criteria in [
        ("regex only", lambda text, actors: False),
        ("+ v7 (modal + actor)", lambda text, actors: bool(HAS_MODAL.search(text.lower())) and bool(actors)),
        ("+ v7 (modal)", lambda text, actors: bool(HAS_MODAL.search(text.lower()))),
        ("+ v7 (all)", lambda text, actors: True),
    ]:
        correct = 0
        total = 0
        for sid, gold_drrp in gold.items():
            data = tbl.search().where(f"section_id = '{sid}'").select([
                "drrp_types", "embedding", "text", "governed_actors", "government_actors"
            ]).limit(1).to_arrow()
            if data.num_rows == 0:
                continue
            total += 1
            pipe_drrp = data.column("drrp_types")[0].as_py() or []

            if pipe_drrp:
                pred = pipe_drrp[0]
            else:
                emb = data.column("embedding")[0].as_py()
                text = data.column("text")[0].as_py() or ""
                govt = data.column("government_actors")[0].as_py() or []
                governed = data.column("governed_actors")[0].as_py() or []
                all_actors = governed + govt

                if emb and len(emb) == 384 and criteria(text, all_actors):
                    cls_name, conf = predict_v7(emb, text)
                    if cls_name != "none":
                        has_govt = any(g in govt_labels for g in govt)
                        pred = decompose(cls_name, has_govt)
                    else:
                        pred = "none"
                else:
                    pred = "none"

            if gold_drrp == pred:
                correct += 1

        print(f"  {label:>30s}: {correct}/{total} ({100*correct/total:.1f}%)")


if __name__ == "__main__":
    main()
