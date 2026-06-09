#!/usr/bin/env /usr/bin/python3
"""Production Tier 2: DRRP classifier on LanceDB provisions.

Runs the trained logistic regression (embedding + modal features) to
classify Obligation/Liberty/none, then decomposes to DRRP sub-type
using actor label prefix.

Usage:
    /usr/bin/python3 .claude/skills/classifier-enrich/scripts/classify.py --laws UK_uksi_1992_2793
    /usr/bin/python3 .claude/skills/classifier-enrich/scripts/classify.py --laws $(cat data/qq-applicable-laws.csv)
    /usr/bin/python3 .claude/skills/classifier-enrich/scripts/classify.py --laws UK_uksi_1992_2793 --dry-run
"""

import argparse
import pickle
import re
import sys
import time

import lancedb
import numpy as np
import onnxruntime as ort
import pyarrow as pa
from tokenizers import Tokenizer


# ── DRRP decomposition ──────────────────────────────────────────────

def decompose_drrp(classifier_label: str, actors: list) -> str:
    """Map Obligation/Liberty to DRRP sub-type using actor label prefix."""
    if classifier_label == "none":
        return "none"

    has_gvt = any(
        a.get("label", "").startswith("Gvt:") or a.get("label", "").startswith("EU:")
        for a in (actors or [])
    )

    if classifier_label == "Obligation":
        return "Responsibility" if has_gvt else "Duty"
    elif classifier_label == "Liberty":
        return "Power" if has_gvt else "Right"
    return classifier_label


# ── Feature computation ─────────────────────────────────────────────

def load_embedding_model(model_dir="models/all-MiniLM-L6-v2"):
    tokenizer = Tokenizer.from_file(f"{model_dir}/tokenizer.json")
    tokenizer.enable_padding(pad_id=0, pad_token="[PAD]", length=128)
    tokenizer.enable_truncation(max_length=128)
    session = ort.InferenceSession(f"{model_dir}/model.onnx")
    return tokenizer, session


def embed_text(tokenizer, session, text: str) -> np.ndarray:
    encoded = tokenizer.encode(text)
    input_ids = np.array([encoded.ids], dtype=np.int64)
    attention_mask = np.array([encoded.attention_mask], dtype=np.int64)
    token_type_ids = np.zeros_like(input_ids, dtype=np.int64)
    outputs = session.run(None, {
        "input_ids": input_ids,
        "attention_mask": attention_mask,
        "token_type_ids": token_type_ids,
    })
    token_embeddings = outputs[0]
    mask = attention_mask[..., np.newaxis].astype(np.float32)
    pooled = (token_embeddings * mask).sum(axis=1) / mask.sum(axis=1)
    norm = np.linalg.norm(pooled, axis=1, keepdims=True)
    pooled = pooled / np.maximum(norm, 1e-12)
    return pooled[0]


def modal_features(text: str, actors: list) -> list:
    t = (text or "").lower()
    return [
        1.0 if re.search(r"\bshall\b", t) else 0.0,
        1.0 if re.search(r"\bmust\b", t) else 0.0,
        1.0 if re.search(r"\bmay\b", t) else 0.0,
        1.0 if re.search(r"\bshall not\b", t) else 0.0,
        1.0 if re.search(r"\bmust not\b", t) else 0.0,
        1.0 if re.search(r"\bmay not\b", t) else 0.0,
        1.0 if re.search(r"\bentitled\b", t) else 0.0,
        1.0 if re.search(r"\bpower to\b", t) else 0.0,
        1.0 if re.search(r"\brequired to\b", t) else 0.0,
        1.0 if re.search(r"\bhas a duty\b", t) else 0.0,
        1.0 if re.search(r"\bright to\b", t) else 0.0,
        float(len(actors)) if actors else 0.0,
        1.0 if any(
            a.get("label", "").startswith("Gvt:") or a.get("label", "").startswith("EU:")
            for a in (actors or [])
        ) else 0.0,
    ]


# ── Main ────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Production Tier 2 DRRP classifier")
    parser.add_argument("--laws", type=str, required=True,
                        help="Comma-separated law names")
    parser.add_argument("--model", type=str, default="data/drrp_classifier_v6.pkl",
                        help="Path to classifier pickle")
    parser.add_argument("--confidence", type=float, default=0.85,
                        help="Confidence to assign classified provisions (default: 0.85)")
    parser.add_argument("--protect-threshold", type=float, default=0.85,
                        help="Skip provisions with existing confidence >= this (default: 0.85)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Show what would be classified without writing")
    parser.add_argument("--data-dir", type=str, default="data")
    args = parser.parse_args()

    REG_TYPES = {"article", "sub_article", "section", "sub_section"}

    # Load classifier
    with open(args.model, "rb") as f:
        model_data = pickle.load(f)
    clf = model_data["model"]
    print(f"Loaded classifier: {args.model}")

    # Load embedding model
    tokenizer, onnx_session = load_embedding_model()

    # Connect to LanceDB
    db = lancedb.connect(f"{args.data_dir}/lancedb")
    tbl = db.open_table("legislation_text")

    law_list = [l.strip() for l in args.laws.split(",")]
    filter_expr = " OR ".join([f"law_name = '{l}'" for l in law_list])

    results = (
        tbl.search()
        .where(f"({filter_expr})", prefilter=True)
        .select([
            "section_id", "section_type", "text", "drrp_types",
            "extraction_method", "taxa_confidence", "actors", "embedding",
        ])
        .limit(200000)
        .to_arrow()
    )

    # Find candidates
    candidates = []
    skipped_structural = 0
    skipped_protected = 0

    for i in range(len(results)):
        st = results.column("section_type")[i].as_py()
        if st not in REG_TYPES:
            skipped_structural += 1
            continue

        conf = results.column("taxa_confidence")[i].as_py() or 0.0
        if conf >= args.protect_threshold:
            skipped_protected += 1
            continue

        candidates.append(i)

    print(f"Laws: {len(law_list)}, Provisions: {len(results)}")
    print(f"Candidates: {len(candidates)} (skipped {skipped_structural} structural, {skipped_protected} protected)")

    if args.dry_run:
        print("[DRY RUN] Would classify {len(candidates)} provisions")
        return

    if not candidates:
        print("Nothing to classify")
        return

    # Classify
    start = time.time()
    batch_sids = []
    batch_drrps = []
    classified = 0

    for idx in candidates:
        sid = results.column("section_id")[idx].as_py()
        text = results.column("text")[idx].as_py() or ""
        actors = results.column("actors")[idx].as_py() or []

        # Get or compute embedding
        emb = results.column("embedding")[idx].as_py()
        if emb is None or len(emb) == 0:
            emb = embed_text(tokenizer, onnx_session, text[:512])
        else:
            emb = np.array(emb, dtype=np.float32)

        # Build feature vector
        features = np.concatenate([emb, modal_features(text, actors)])
        features = features.reshape(1, -1)

        # Predict
        pred = clf.predict(features)[0]  # Obligation/Liberty/none
        drrp_type = decompose_drrp(pred, actors)

        batch_sids.append(sid)
        batch_drrps.append([drrp_type] if drrp_type != "none" else [])

        # Write in batches of 100
        if len(batch_sids) >= 100:
            update = pa.table({
                "section_id": batch_sids,
                "drrp_types": pa.array(batch_drrps, type=pa.list_(pa.string())),
                "extraction_method": ["classifier"] * len(batch_sids),
                "taxa_confidence": pa.array(
                    [args.confidence] * len(batch_sids), type=pa.float32()
                ),
            })
            tbl.merge_insert("section_id").when_matched_update_all().execute(update)
            classified += len(batch_sids)
            batch_sids = []
            batch_drrps = []

    # Final batch
    if batch_sids:
        update = pa.table({
            "section_id": batch_sids,
            "drrp_types": pa.array(batch_drrps, type=pa.list_(pa.string())),
            "extraction_method": ["classifier"] * len(batch_sids),
            "taxa_confidence": pa.array(
                [args.confidence] * len(batch_sids), type=pa.float32()
            ),
        })
        tbl.merge_insert("section_id").when_matched_update_all().execute(update)
        classified += len(batch_sids)

    elapsed = time.time() - start
    rate = classified / elapsed if elapsed > 0 else 0
    print(f"Classified: {classified} provisions in {elapsed:.1f}s ({rate:.0f}/s)")

    # Compact
    print("Compacting...")
    arrow = tbl.to_arrow()
    db.drop_table("legislation_text")
    db.create_table("legislation_text", data=arrow)
    print(f"Compacted: {arrow.num_rows:,} rows")


if __name__ == "__main__":
    main()
