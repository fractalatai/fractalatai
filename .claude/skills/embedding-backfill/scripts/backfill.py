#!/usr/bin/env /usr/bin/python3
"""Backfill missing embeddings for LanceDB provisions.

Computes 384-dim embeddings using the local ONNX model (all-MiniLM-L6-v2)
for provisions that have been enriched but are missing embedding vectors.

Usage:
    /usr/bin/python3 .claude/skills/embedding-backfill/backfill.py
    /usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --method agentic
    /usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --laws UK_uksi_1992_2793
    /usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --dry-run
"""

import argparse
import time

import lancedb
import numpy as np
import pyarrow as pa


def load_model(model_dir: str = "models/all-MiniLM-L6-v2"):
    """Load ONNX model and tokenizer."""
    import onnxruntime as ort
    from tokenizers import Tokenizer

    tokenizer = Tokenizer.from_file(f"{model_dir}/tokenizer.json")
    tokenizer.enable_padding(pad_id=0, pad_token="[PAD]", length=128)
    tokenizer.enable_truncation(max_length=128)

    session = ort.InferenceSession(f"{model_dir}/model.onnx")
    return tokenizer, session


def embed_text(tokenizer, session, text: str) -> list:
    """Compute a 384-dim L2-normalised embedding for text."""
    encoded = tokenizer.encode(text)
    input_ids = np.array([encoded.ids], dtype=np.int64)
    attention_mask = np.array([encoded.attention_mask], dtype=np.int64)
    token_type_ids = np.zeros_like(input_ids, dtype=np.int64)

    outputs = session.run(
        None,
        {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "token_type_ids": token_type_ids,
        },
    )

    token_embeddings = outputs[0]
    mask = attention_mask[..., np.newaxis].astype(np.float32)
    pooled = (token_embeddings * mask).sum(axis=1) / mask.sum(axis=1)
    norm = np.linalg.norm(pooled, axis=1, keepdims=True)
    pooled = pooled / np.maximum(norm, 1e-12)
    return pooled[0].tolist()


def main():
    parser = argparse.ArgumentParser(description="Backfill missing embeddings")
    parser.add_argument(
        "--method",
        type=str,
        default="agentic",
        help="Extraction method to backfill (default: agentic)",
    )
    parser.add_argument(
        "--laws",
        type=str,
        default=None,
        help="Comma-separated law names to backfill",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Count provisions needing backfill without computing",
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default="data",
        help="Data directory (default: data)",
    )
    args = parser.parse_args()

    db = lancedb.connect(f"{args.data_dir}/lancedb")
    tbl = db.open_table("legislation_text")

    # Build filter
    if args.laws:
        law_list = [l.strip() for l in args.laws.split(",")]
        law_filter = " OR ".join([f"law_name = '{l}'" for l in law_list])
        where = f"({law_filter})"
    else:
        where = f"extraction_method = '{args.method}'"

    results = (
        tbl.search()
        .where(where, prefilter=True)
        .select(["section_id", "text", "embedding"])
        .limit(100000)
        .to_arrow()
    )

    # Find provisions needing embeddings
    need_backfill = []
    for i in range(len(results)):
        emb = results.column("embedding")[i].as_py()
        if emb is None or len(emb) == 0:
            sid = results.column("section_id")[i].as_py()
            text = results.column("text")[i].as_py() or ""
            if text.strip():
                need_backfill.append((sid, text))

    has_embedding = len(results) - len(need_backfill)
    print(f"Provisions: {len(results)} total, {has_embedding} with embeddings, {len(need_backfill)} need backfill")

    if args.dry_run or not need_backfill:
        return

    # Load model
    tokenizer, session = load_model()

    start = time.time()
    batch_size = 50
    updated = 0

    for batch_start in range(0, len(need_backfill), batch_size):
        batch = need_backfill[batch_start : batch_start + batch_size]
        sids = []
        embeddings = []

        for sid, text in batch:
            emb = embed_text(tokenizer, session, text[:512])
            sids.append(sid)
            embeddings.append(emb)

        update_table = pa.table(
            {
                "section_id": sids,
                "embedding": pa.array(
                    embeddings, type=pa.list_(pa.float32(), 384)
                ),
            }
        )

        try:
            tbl.merge_insert("section_id").when_matched_update_all().execute(
                update_table
            )
            updated += len(batch)
        except Exception as e:
            print(f"Error at batch {batch_start}: {e}")
            break

        if (batch_start + batch_size) % 200 == 0 or batch_start + batch_size >= len(
            need_backfill
        ):
            elapsed = time.time() - start
            rate = updated / elapsed if elapsed > 0 else 0
            print(f"  {updated}/{len(need_backfill)} ({rate:.1f}/s)")

    elapsed = time.time() - start
    print(f"Done: {updated} embeddings in {elapsed:.1f}s ({updated / elapsed:.1f}/s)")


if __name__ == "__main__":
    main()
