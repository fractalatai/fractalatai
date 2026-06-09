# Gemini Review: Sync Watch Enrichment Design

**Date:** 2026-06-09
**Model:** Gemini 2.5 Flash

## Summary

Recommends **Option 2C (Embed + Classify via ONNX)** as the best balance of effort, performance, and maintainability.

## Key Recommendations

### 1. Option 2 over Option 1
Embedding alone only partially solves the problem. Classification is needed for 85%+ coverage. The complexity of adding classification is manageable, and the payoff (production-quality data immediately) is significant.

### 2. Path 2C (ONNX) over 2A (Python) and 2B (port)
- **2A rejected**: process spawn overhead, Python runtime dependency, two LanceDB writers
- **2B viable but higher effort**: manual weight porting, re-validation needed
- **2C recommended**: leverages existing `ort` dependency, no runtime Python, single process, `skl2onnx` automates conversion

### 3. skl2onnx gotchas
- **Feature order**: must exactly match training order (embedding dims then modal features)
- **Preprocessing**: if a `StandardScaler` or pipeline exists, export the entire pipeline
- **Output format**: verify output shape (probabilities vs log-odds)
- **Test thoroughly**: compare ONNX predictions against original scikit-learn on a test set before deploying

### 4. Always embed, skip classification only
- Embeddings are valuable for many downstream tasks beyond classification
- One-time cost per provision, simplifies logic
- Confidence protection applies at the classification step: skip classifier for `taxa_confidence >= 0.90` or `extraction_method == "agentic"`

### 5. Single merge_insert for embedding + taxa
- More atomic (no half-written state if crash occurs)
- Less write amplification (one version per row, not two)
- Batch the constructed records and perform a single `merge_insert`

### 6. Long-running ONNX sessions are fine
- `onnxruntime` is designed for this
- Monitor RSS over time for leaks
- Keep `ort` updated
- Periodic restart (24-48h) as pragmatic fallback if leaks appear

### 7. Missing from design
1. **Error handling & retries** — exponential backoff for LanceDB writes
2. **Concurrency** — `Arc<OnnxEmbedder>` if parallel processing needed (sequential is fine for now)
3. **Model versioning** — how to deploy new models without restarting watch
4. **Observability** — latency metrics for embedding, classification, writes
5. **Backfill** — existing regex-only laws still need batch embed+classify
6. **GPU config** — CUDA execution provider setup for future RTX 3090
