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

# PG is needed for both reading embeddings and writing classifier results

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

    # Write to PG (inference store — all tier signals live here)
    pg = psycopg2.connect(
        host="localhost", port=5433, dbname="fractalaw",
        user="fractalaw", password="fractalaw"
    )
    pg.autocommit = True
    cur = pg.cursor()

    updated = 0
    for sid, src, pred, conf in predictions:
        cur.execute("""
            UPDATE jsp_provisions
            SET cls_strength = %s, cls_confidence = %s, cls_classified_at = NOW()
            WHERE section_id = %s
        """, [pred, conf, sid])
        updated += 1

    cur.close()
    pg.close()
    print(f"\nUpdated {updated} rows in PG jsp_provisions")

    # Show agreement between regex and classifier
    # Regex labels are in DuckDB jsp_enrichment, classifier in PG
    duck = duckdb.connect(args.db, read_only=True)
    pg = psycopg2.connect(host="localhost", port=5433, dbname="fractalaw", user="fractalaw", password="fractalaw")
    pg_cur = pg.cursor()

    pg_cur.execute("SELECT section_id, cls_strength FROM jsp_provisions WHERE cls_strength IS NOT NULL")
    cls_map = {r[0]: r[1] for r in pg_cur.fetchall()}
    pg_cur.close()
    pg.close()

    regex_rows = duck.execute("SELECT section_id, obligation_strength FROM jsp_enrichment WHERE obligation_strength IS NOT NULL").fetchall()
    duck.close()

    agree = sum(1 for sid, regex_s in regex_rows if cls_map.get(sid) == regex_s)
    total_both = sum(1 for sid, _ in regex_rows if sid in cls_map)
    if total_both > 0:
        print(f"\nAgreement: {agree}/{total_both} ({100*agree/total_both:.1f}%)")
        from collections import Counter
        disagree = Counter()
        for sid, regex_s in regex_rows:
            cls_s = cls_map.get(sid)
            if cls_s and cls_s != regex_s:
                disagree[(regex_s, cls_s)] += 1
        if disagree:
            print("Disagreements:")
            for (regex, cls), n in disagree.most_common(10):
                print(f"  regex={regex} → cls={cls}: {n}")


if __name__ == "__main__":
    main()
