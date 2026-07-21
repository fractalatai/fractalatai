#!/usr/bin/env python3
"""Train an obligation strength classifier for JSP provisions.

Logistic regression on 384-dim sentence embeddings + modal indicator features.
Classes: Mandatory / Recommended / Permissive / None.

Reads JSP provisions from DuckDB (text + regex-labelled strength),
computes embeddings via sentence-transformers, trains classifier,
exports JSON weights.

Usage:
    /usr/bin/python3 scripts/ml/train_strength_classifier.py
    /usr/bin/python3 scripts/ml/train_strength_classifier.py --output crates/fractalaw-cli/config/strength_classifier_v1.json
"""

import argparse
import json
import sys
from collections import Counter

import duckdb
import numpy as np

# JSP-specific modal keywords — includes "will" and "is to" as mandatory indicators
MODAL_KEYWORDS = [
    "shall", "must", " may ", "should",
    " will ",           # JSP mandatory (NOT future tense)
    "is to", "are to",  # JSP mandatory
    "is required", "is responsible",
    "requir", "ensur", "prohibit",
    "shall not", "must not",
    "competent",        # competence signal
]


def modal_features(text):
    """Extract binary modal indicator features from text."""
    t = text.lower()
    return [1.0 if kw in t else 0.0 for kw in MODAL_KEYWORDS]


def strength_label(strength):
    """Normalise regex strength to classifier label."""
    if strength in ("Mandatory",):
        return "Mandatory"
    elif strength in ("Recommended",):
        return "Recommended"
    elif strength in ("Permissive",):
        return "Permissive"
    else:
        return "None"


def main():
    parser = argparse.ArgumentParser(description="Train JSP strength classifier")
    parser.add_argument(
        "--output",
        default="crates/fractalaw-cli/config/strength_classifier_v1.json",
        help="Output weights path",
    )
    parser.add_argument(
        "--db",
        default="data/fractalaw.duckdb",
        help="DuckDB path",
    )
    parser.add_argument(
        "--model",
        default="all-MiniLM-L6-v2",
        help="Sentence transformer model name",
    )
    args = parser.parse_args()

    # ── Load provisions with embeddings from PG + labels from DuckDB ─

    print("Loading embeddings from fractalaw PG (port 5433)...")
    import psycopg2
    pg = psycopg2.connect(
        host="localhost", port=5433, dbname="fractalaw",
        user="fractalaw", password="fractalaw"
    )
    pg_cur = pg.cursor()
    pg_cur.execute("""
        SELECT section_id, text, embedding
        FROM jsp_provisions
        WHERE embedding IS NOT NULL AND text IS NOT NULL
    """)
    def parse_vector(v):
        """Parse pgvector string '[0.1,0.2,...]' to numpy array."""
        if isinstance(v, (list, np.ndarray)):
            return np.array(v, dtype=np.float32)
        return np.array(json.loads(v.replace("(", "[").replace(")", "]")), dtype=np.float32)

    pg_rows = {r[0]: (r[1], parse_vector(r[2])) for r in pg_cur.fetchall()}
    pg_cur.close()
    pg.close()
    print(f"  {len(pg_rows)} provisions with embeddings")

    print("Loading regex labels from DuckDB...")
    conn = duckdb.connect(args.db, read_only=True)

    # Provisions with regex enrichment labels
    labelled = conn.execute("""
        SELECT section_id, obligation_strength
        FROM jsp_enrichment
    """).fetchall()
    label_map = {r[0]: r[1] for r in labelled}
    conn.close()
    print(f"  {len(label_map)} provisions with regex labels")

    # Build dataset: embedding + text + label
    rows = []
    for sid, (text, emb) in pg_rows.items():
        strength = label_map.get(sid)  # None if no enrichment
        rows.append((sid, text, emb, strength))

    labels = [strength_label(r[3]) for r in rows]
    dist = Counter(labels)
    print(f"\nDataset: {len(rows)} provisions")
    print("Class distribution:")
    for cls, count in dist.most_common():
        print(f"  {cls}: {count} ({100*count/len(labels):.1f}%)")

    # ── Build feature matrix ─────────────────────────────────────────

    print("Building features...")
    n_modal = len(MODAL_KEYWORDS)
    n_features = 384 + n_modal

    X = np.zeros((len(rows), n_features), dtype=np.float32)
    y = np.array(labels)

    for i, (sid, text, emb, strength) in enumerate(rows):
        X[i, :384] = emb
        X[i, 384:] = modal_features(text)

    print(f"  Feature matrix: {X.shape}")

    # ── Train / test split ───────────────────────────────────────────

    from sklearn.model_selection import train_test_split
    from sklearn.linear_model import LogisticRegression
    from sklearn.metrics import classification_report

    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y
    )
    print(f"\nTrain: {len(X_train)}, Test: {len(X_test)}")

    # ── Train classifier ─────────────────────────────────────────────

    print("Training logistic regression...")
    clf = LogisticRegression(
        max_iter=1000,
        C=1.0,
        class_weight="balanced",
        random_state=42,
    )
    clf.fit(X_train, y_train)

    # ── Evaluate ─────────────────────────────────────────────────────

    y_pred = clf.predict(X_test)
    accuracy = (y_pred == y_test).mean()
    print(f"\nAccuracy: {accuracy:.1%}")
    print(classification_report(y_test, y_pred))

    # ── Export weights ───────────────────────────────────────────────

    weights = {
        "classes": clf.classes_.tolist(),
        "coef": clf.coef_.tolist(),
        "intercept": clf.intercept_.tolist(),
        "feature_dim": int(clf.coef_.shape[1]),
        "features": f"embedding(384)+modal({n_modal})={n_features}",
        "modal_keywords": MODAL_KEYWORDS,
        "training_examples": len(X_train),
        "test_accuracy": float(accuracy),
        "version": "v1",
        "description": "JSP obligation strength classifier (Mandatory/Recommended/Permissive/None)",
    }

    with open(args.output, "w") as f:
        json.dump(weights, f, indent=2)
    print(f"\nSaved weights to {args.output}")
    print(f"  Classes: {weights['classes']}")
    print(f"  Feature dim: {weights['feature_dim']}")
    print(f"  Test accuracy: {accuracy:.1%}")


if __name__ == "__main__":
    main()
