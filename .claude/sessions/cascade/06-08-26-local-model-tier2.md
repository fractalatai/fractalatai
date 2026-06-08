# Session: Local Model Installation — Tier 2 Classifier

## Context

**Prior session**: `.claude/sessions/cascade/06-07-26-drrp-qa-bugs.md`
**Strategy doc**: `docs/CLASSIFICATION-CASCADE-STRATEGY.md` v0.2
**Problem**: Regex parser (Tier 1) handles ~70% of provisions. The remaining ~30% needs semantic understanding. Gemini API (Tier 3) works but is too expensive for the full corpus. We need a local Tier 2 model.

## Hardware

- **CPU**: Intel CoffeeLake-S (no discrete GPU)
- **GPU**: Intel UHD 630 (integrated — not usable for LLM inference)
- **RAM**: 40 GB (30 GB available)
- **OS**: Fedora Bluefin DX (atomic/immutable Linux)
- **Disk**: ~11 GB free on /var/home

**Constraint**: CPU-only inference. No CUDA. Model must fit in ~16 GB RAM to leave room for LanceDB and the enrichment pipeline.

## Model Selection

Gemini's recommendation: Gemma 3 or 4 12B. With CPU-only and 40 GB RAM:

| Model | Quantization | RAM needed | Speed (CPU) | Quality |
|-------|-------------|-----------|-------------|---------|
| Gemma 3 12B Q4_K_M | 4-bit | ~8 GB | ~5-10 tok/s | Good for extraction |
| Gemma 4 12B Q4_K_M | 4-bit | ~8 GB | ~5-10 tok/s | Better reasoning |
| Gemma 3 4B Q8_0 | 8-bit | ~5 GB | ~15-25 tok/s | Faster, less capable |

**Decision**: Start with Gemma 3 12B Q4_K_M via Ollama. If too slow, fall back to 4B.

## Task Profile

The model needs to do ONE thing well: classify actor positions in legal provisions.

Input: provision text (~100-500 chars) + actor list + DRRP type
Output: JSON with position per actor (active/counterparty/beneficiary/mentioned)

This is constrained extraction, not open-ended generation. Temperature 0.0, structured JSON output. The task is well-suited to a smaller quantized model.

## Installation Plan

### 1. Install Ollama

Ollama is the simplest local model runner. On Fedora Bluefin (immutable):

```bash
# Option A: Homebrew (already configured for C++ toolchain)
brew install ollama

# Option B: Direct install script
curl -fsSL https://ollama.com/install.sh | sh
```

### 2. Pull model

```bash
ollama pull gemma3:12b-it-q4_K_M
# or if disk is tight:
ollama pull gemma3:4b
```

### 3. Test locally

```bash
ollama run gemma3:12b-it-q4_K_M "Extract the duty holder: 'The employer shall ensure the health and safety of employees.'"
```

### 4. Wire into enrichment pipeline

Ollama exposes a REST API at `http://localhost:11434/api/generate`. Replace the Gemini API call path in Tier 2 with a local Ollama call — same reqwest client, different URL.

### 5. Benchmark

- Accuracy: run DRRP QA on OH&S with local model vs Gemini
- Speed: tokens/second on CPU, total time for OH&S corpus
- Coverage: how many of the Tier 1 "none" provisions does Tier 2 correctly classify?

## Integration with Cascade

```
Tier 1: Regex (parse_v2) — FREE, ~70% coverage
    ↓ low confidence
Tier 2: Local Gemma 12B — CHEAP (CPU time only), target ~20% coverage
    ↓ low confidence
Tier 3: Gemini API — EXPENSIVE, ~10% coverage (customer laws only)
```

The local model runs on every enrichment — no API cost. It processes the ~30% that regex can't handle. Only genuinely ambiguous provisions on customer-registered laws escalate to Gemini.

## Gemini's Guidance (from briefing)

Key points from the Gemini analysis:
- Anchor extraction to modals of obligation (shall, must, is required to)
- Isolate the "true entity" vs qualifications
- Enforce strict JSON output
- Temperature 0.0 for deterministic extraction
- The model excels at complex syntax parsing (passive voice, nested clauses, delayed subjects)

## Risks

- **Speed**: CPU inference at ~5-10 tok/s means ~2-5 seconds per provision. OH&S corpus (~14K provisions) = ~8-20 hours. May need batching strategy or selective application.
- **Disk**: Ollama + model weights = ~8 GB. With 11 GB free, tight. May need to sweep build artifacts first.
- **Quality**: 4-bit quantization may degrade legal parsing accuracy vs full precision. Need to benchmark against Gemini.

## References

- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY.md`
- Gemini cascade review: `docs/reviews/gemini-cascade-strategy-review-20260607.md`
- Prior QA results: `data/qa-results/drrp-qa-all-20260607-*.json`
- DRRP QA skill: `.claude/skills/drrp-qa/`
- Enrichment skill: `.claude/skills/enrich-and-publish/`
