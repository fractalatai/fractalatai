#!/usr/bin/env python3
"""Batch classify JSP provisions with the strength classifier.

Reads embeddings from fractalaw PG (port 5433), runs the logistic
regression classifier, writes cls_strength + cls_confidence to DuckDB
alongside existing regex obligation_strength.

Usage:
    /usr/bin/python3 scripts/ml/classify_jsp_strength.py
    /usr/bin/python3 scripts/ml/classify_jsp_strength.py --source-id JSP-375-CH23
"""

import argparse
import json

import duckdb
import numpy as np
import psycopg2

CLASSIFIER_PATH = "crates/fractalaw-cli/config/strength_classifier_v1.json"

# Must match training script
MODAL_KEYWORDS = [
    "shall", "must", " may ", "should",
    " will ", "is to", "are to",
    "is required", "is responsible",
    "requir", "ensur", "prohibit",
    "shall not", "must not",
    "competent",
]


def modal_features(text):
    t = text.lower()
    return [1.0 if kw in t else 0.0 for kw in MODAL_KEYWORDS]


def parse_vector(v):
    if isinstance(v, (list, np.ndarray)):
        return np.array(v, dtype=np.float32)
    return np.array(json.loads(v.replace("(", "[").replace(")", "]")), dtype=np.float32)


def main():
    parser = argparse.ArgumentParser(description="Classify JSP provisions with strength classifier")
    parser.add_argument("--source-id", help="Classify a specific source only")
    parser.add_argument("--db", default="data/fractalaw.duckdb")
    args = parser.parse_args()

    # Load classifier weights
    with open(CLASSIFIER_PATH) as f:
        weights = json.load(f)

    classes = np.array(weights["classes"])
    coef = np.array(weights["coef"], dtype=np.float32)
    intercept = np.array(weights["intercept"], dtype=np.float32)
    print(f"Loaded classifier: {len(classes)} classes, {coef.shape[1]} features")

    # Load embeddings from PG
    pg = psycopg2.connect(
        host="localhost", port=5433, dbname="fractalaw",
        user="fractalaw", password="fractalaw"
    )
    cur = pg.cursor()

    where = ""
    if args.source_id:
        where = f"WHERE source_id = '{args.source_id}'"

    cur.execute(f"""
        SELECT section_id, source_id, text, embedding
        FROM jsp_provisions
        {where}
        ORDER BY source_id, position
    """)
    rows = cur.fetchall()
    cur.close()
    pg.close()

    provisions = []
    for sid, src, text, emb in rows:
        if emb is None or text is None:
            continue
        provisions.append((sid, src, text, parse_vector(emb)))

    print(f"Loaded {len(provisions)} provisions with embeddings")

    # Classify
    n_modal = len(MODAL_KEYWORDS)
    predictions = []

    for sid, src, text, emb in provisions:
        features = np.concatenate([emb, np.array(modal_features(text), dtype=np.float32)])
        # Logistic regression: softmax(X @ coef.T + intercept)
        logits = features @ coef.T + intercept
        exp_logits = np.exp(logits - logits.max())
        probs = exp_logits / exp_logits.sum()

        pred_idx = probs.argmax()
        pred_class = classes[pred_idx]
        confidence = float(probs[pred_idx])

        predictions.append((sid, src, pred_class, confidence))

    print(f"Classified {len(predictions)} provisions")

    # Distribution
    from collections import Counter
    dist = Counter(p[2] for p in predictions)
    for cls, count in dist.most_common():
        print(f"  {cls}: {count}")

    # Write to DuckDB
    duck = duckdb.connect(args.db)

    # Add classifier columns if missing
    cols = duck.execute("SELECT column_name FROM information_schema.columns WHERE table_name = 'jsp_enrichment'").fetchall()
    col_names = [c[0] for c in cols]
    if "cls_strength" not in col_names:
        duck.execute("ALTER TABLE jsp_enrichment ADD COLUMN cls_strength TEXT")
        duck.execute("ALTER TABLE jsp_enrichment ADD COLUMN cls_confidence REAL")
        print("Added cls_strength + cls_confidence columns to jsp_enrichment")

    updated = 0
    for sid, src, pred, conf in predictions:
        duck.execute("""
            UPDATE jsp_enrichment
            SET cls_strength = ?, cls_confidence = ?
            WHERE section_id = ?
        """, [pred, conf, sid])
        updated += 1

    duck.close()
    print(f"\nUpdated {updated} rows in jsp_enrichment")

    # Show agreement between regex and classifier
    duck = duckdb.connect(args.db, read_only=True)
    agree = duck.execute("""
        SELECT count(*) FROM jsp_enrichment
        WHERE obligation_strength = cls_strength
          AND cls_strength IS NOT NULL
    """).fetchone()[0]
    total_both = duck.execute("""
        SELECT count(*) FROM jsp_enrichment
        WHERE obligation_strength IS NOT NULL AND cls_strength IS NOT NULL
    """).fetchone()[0]
    disagree = duck.execute("""
        SELECT obligation_strength as regex, cls_strength as classifier, count(*) as n
        FROM jsp_enrichment
        WHERE obligation_strength != cls_strength
          AND obligation_strength IS NOT NULL AND cls_strength IS NOT NULL
        GROUP BY obligation_strength, cls_strength
        ORDER BY n DESC
        LIMIT 10
    """).fetchall()
    duck.close()

    if total_both > 0:
        print(f"\nAgreement: {agree}/{total_both} ({100*agree/total_both:.1f}%)")
        if disagree:
            print("Disagreements:")
            for regex, cls, n in disagree:
                print(f"  regex={regex} → cls={cls}: {n}")


if __name__ == "__main__":
    main()
