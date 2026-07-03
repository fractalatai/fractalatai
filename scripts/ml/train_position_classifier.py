#!/usr/bin/env /usr/bin/python3
"""Train position classifier v2: 4-class with correct DRRP features.

Uses golden benchmark data from NAS as training set.
Output: docs/position_classifier_v2.json

Features (411 dims):
  embedding(384) + modal(13) + drrp(3) + category(10) + offset(1)

Classes: active / counterparty / beneficiary / mentioned

Usage:
    /usr/bin/python3 scripts/train_position_classifier.py
    /usr/bin/python3 scripts/ml/train_position_classifier.py --output crates/fractalaw-cli/config/position_classifier_v4.json
"""

import argparse
import glob
import json
import os
import sys

import numpy as np
import pyarrow.parquet as pq

BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"

# Must match Rust position_classifier.rs
MODAL_KEYWORDS = [
    "shall", "must", " may ", "requir", "ensur", "prohibit",
    ["duty", "duties"],
    ["right", "rights"],
    ["power", "powers"],
    "responsib", "penalt", "offence", "exempt",
]

# Must match Rust position_classifier.rs DRRP_TYPES
DRRP_TYPES = ["Obligation", "Liberty", "none"]

# Must match Rust position_classifier.rs CATEGORIES
CATEGORIES = ["Org", "Ind", "Gvt", "SC", "Spc", "EU", "Svc", "Public", "Offshore", "other"]

# Map natural-language gold labels to canonical form
LABEL_ALIASES = {
    "employer": "Org: Employer",
    "employee": "Ind: Employee",
    "person": "Ind: Person",
    "any person": "Ind: Person",
    "responsible person": "Ind: Responsible Person",
    "inspector": "Spc: Inspector",
    "hse": "Gvt: Agency: Health and Safety Executive",
    "secretary of state": "Gvt: Minister",
    "local authority": "Gvt: Authority: Local",
    "enforcing authority": "Gvt: Authority: Enforcement",
    "self-employed": "Ind: Self-Employed",
    "occupier": "Ind: Occupier",
    "manufacturer": "Org: Manufacturer",
    "supplier": "Org: Supplier",
    "designer": "Org: Designer",
    "importer": "Org: Importer",
    "installer": "Org: Installer",
    "contractor": "Org: Contractor",
    "owner": "Ind: Owner",
    "duty holder": "Org: Duty Holder",
}


def normalise_label(label):
    low = label.strip().lower()
    return LABEL_ALIASES.get(low, label.strip())


def modal_features(text):
    t = text.lower()
    features = []
    for kw in MODAL_KEYWORDS:
        if isinstance(kw, list):
            features.append(1.0 if any(w in t for w in kw) else 0.0)
        else:
            features.append(1.0 if kw in t else 0.0)
    return features


def fix_parquet(path):
    """Fix NAS block-padding."""
    with open(path, "rb") as f:
        data = f.read()
    idx = data.rfind(b"PAR1")
    if idx == -1:
        return None
    if idx + 4 < len(data):
        import tempfile
        tmp = tempfile.NamedTemporaryFile(suffix=".parquet", delete=False)
        tmp.write(data[: idx + 4])
        tmp.close()
        return tmp.name
    return path


def load_training_data():
    """Load benchmark data and build (features, labels) from Postgres embeddings."""
    import psycopg2

    # Load benchmarks
    files = sorted(glob.glob(os.path.join(BENCHMARK_DIR, "tier2-*.parquet")))
    if not files:
        print(f"No benchmark files in {BENCHMARK_DIR}")
        sys.exit(1)

    # Collect gold (section_id → {drrp, actors})
    gold = {}
    for f in files:
        fixed = fix_parquet(f)
        if not fixed:
            continue
        t = pq.read_table(fixed)
        for i in range(t.num_rows):
            sid = t.column("section_id")[i].as_py()
            drrp = t.column("gold_drrp_types")[i].as_py() or []
            actors_raw = t.column("gold_actors")[i].as_py() or "[]"
            actors = json.loads(actors_raw) if isinstance(actors_raw, str) else actors_raw
            if actors:
                gold[sid] = {"drrp": drrp, "actors": actors}

    print(f"Loaded {len(gold)} provisions with actors from benchmarks")

    # Get embeddings from Postgres
    conn = psycopg2.connect(
        host="localhost", port=5433, dbname="fractalaw",
        user="fractalaw", password="fractalaw"
    )
    cur = conn.cursor()

    sids = list(gold.keys())
    embeddings = {}
    texts = {}
    for i in range(0, len(sids), 500):
        chunk = sids[i:i+500]
        placeholders = ",".join(["%s"] * len(chunk))
        cur.execute(
            f"SELECT section_id, embedding, text FROM legislation_text "
            f"WHERE section_id IN ({placeholders}) AND embedding IS NOT NULL",
            chunk,
        )
        for row in cur.fetchall():
            sid, emb, text = row
            if emb is not None:
                # pgvector returns "[0.1,0.2,...]" string via psycopg2
                if isinstance(emb, str):
                    emb = [float(x) for x in emb.strip("[]").split(",")]
                else:
                    emb = list(emb)
                embeddings[sid] = emb
                texts[sid] = text or ""

    cur.close()
    conn.close()
    print(f"Got embeddings for {len(embeddings)} provisions from Postgres")

    # Build feature vectors
    X = []
    y = []
    skipped_no_emb = 0
    class_counts = {"active": 0, "counterparty": 0, "beneficiary": 0, "mentioned": 0}

    for sid, g in gold.items():
        if sid not in embeddings:
            skipped_no_emb += 1
            continue

        emb = embeddings[sid]
        text = texts.get(sid, "")
        drrp = g["drrp"]
        drrp_type = drrp[0] if drrp else "none"

        # Modal features (13)
        modals = modal_features(text)

        # DRRP one-hot (3)
        drrp_feat = [1.0 if dt == drrp_type else 0.0 for dt in DRRP_TYPES]

        for actor in g["actors"]:
            label = normalise_label(actor["label"])
            position = actor["position"].lower()

            if position not in class_counts:
                continue

            # Category one-hot (10)
            cat = label.split(":")[0].strip() if ":" in label else "other"
            cat_feat = [1.0 if c == cat else 0.0 for c in CATEGORIES]

            # Relative offset (1)
            label_lower = label.lower().split(":")[-1].strip()
            text_lower = text.lower()
            offset = text_lower.find(label_lower)
            rel_offset = offset / max(len(text), 1) if offset >= 0 else 0.5

            # Build 411-dim vector
            features = emb + modals + drrp_feat + cat_feat + [rel_offset]
            assert len(features) == 411, f"Expected 411, got {len(features)}"

            X.append(features)
            y.append(position)
            class_counts[position] += 1

    if skipped_no_emb:
        print(f"Skipped {skipped_no_emb} provisions without embeddings")

    print(f"\nTraining data: {len(X)} samples")
    for cls, count in sorted(class_counts.items()):
        print(f"  {cls}: {count}")

    return np.array(X, dtype=np.float32), np.array(y)


def train(X, y, output_path):
    """Train logistic regression and export weights."""
    from sklearn.linear_model import LogisticRegression
    from sklearn.model_selection import cross_val_score, StratifiedKFold
    from sklearn.metrics import classification_report

    print(f"\nTraining 4-class LogisticRegression on {X.shape[0]} samples, {X.shape[1]} features...")

    # Stratified 5-fold cross-validation
    cv = StratifiedKFold(n_splits=5, shuffle=True, random_state=42)
    model = LogisticRegression(
        max_iter=1000,
        class_weight="balanced",
        random_state=42,
        solver="lbfgs",
    )

    scores = cross_val_score(model, X, y, cv=cv, scoring="accuracy")
    print(f"Cross-validation accuracy: {scores.mean():.3f} (+/- {scores.std():.3f})")

    # Train final model on all data
    model.fit(X, y)

    # Report on cross-validated predictions (not training data)
    from sklearn.model_selection import cross_val_predict
    y_cv_pred = cross_val_predict(model, X, y, cv=cv)
    print(f"\nCross-validated classification report:")
    print(classification_report(y, y_cv_pred, digits=3))

    # Export weights
    weights = {
        "classes": model.classes_.tolist(),
        "coef": model.coef_.tolist(),
        "intercept": model.intercept_.tolist(),
        "features": X.shape[1],
        "training_samples": X.shape[0],
        "cv_accuracy": float(scores.mean()),
        "version": 2,
    }

    with open(output_path, "w") as f:
        json.dump(weights, f, indent=2)

    print(f"\nSaved weights to {output_path}")
    print(f"Classes: {weights['classes']}")
    print(f"Intercepts: {[f'{b:.3f}' for b in weights['intercept']]}")


def main():
    parser = argparse.ArgumentParser(description="Train position classifier v2")
    parser.add_argument("--output", default="crates/fractalaw-cli/config/position_classifier_v3.json")
    args = parser.parse_args()

    X, y = load_training_data()
    if len(X) < 100:
        print("Too few training samples, aborting")
        sys.exit(1)

    train(X, y, args.output)


if __name__ == "__main__":
    main()
