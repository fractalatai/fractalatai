# Session: Legal-BERT Embedding Upgrade (PENDING)

## Problem

The position classifier's 384-dim embedding (all-MiniLM-L6-v2) accounts for 79.9% of feature importance but is a general-purpose model. It doesn't understand legal terminology, duty/right distinctions, or legislative sentence structure.

## Proposed change

Replace all-MiniLM-L6-v2 with a domain-specific legal embedding model. Candidates:
- `nlpaueb/legal-bert-base-uncased` — pre-trained on EU/UK legal corpus
- `pile-of-law/legalbert-large-1.7M-2` — larger, trained on US+UK law
- Fine-tune MiniLM on our own legislation corpus (cheapest, most targeted)

## Expected impact

The embedding is 79.9% of the classifier's feature importance. A legal-specific embedding should improve semantic understanding of:
- "shall ensure" vs "shall be construed as" (duty vs interpretation)
- "employer" in a duty clause vs "employer" in a definition
- Modal verb patterns specific to UK legislation

## Relationship to other sessions

- Independent of dependency parsing (different feature, same classifier)
- Independent of correlative inference (inference doesn't use embeddings)
- Requires classifier retrain after swap
- Would also improve DRRP classifier (same embedding input)

## Work

1. ⬜ Evaluate candidate models (size, ONNX exportability, licence)
2. ⬜ Generate embeddings for benchmark provisions with Legal-BERT
3. ⬜ Retrain position classifier with new embeddings
4. ⬜ Compare accuracy: MiniLM vs Legal-BERT
5. ⬜ If significant: re-embed full corpus (183K provisions)
6. ⬜ Update ONNX model in fractalaw-ai for production inference

## Dependencies

- ✅ provision_actors table for per-tier benchmarking
- ✅ Classifier training script (scripts/train_position_classifier.py)
- Current embedding: models/all-MiniLM-L6-v2 (ONNX, 384-dim)
