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
Tier 2: Local Gemma 4B — CHEAP (CPU time only), target ~20% coverage
    ↓ low confidence
Tier 3: Gemini API — EXPENSIVE, ~10% coverage (customer laws only)
```

The local model runs on every enrichment — no API cost. It processes the ~30% that regex can't handle. Only genuinely ambiguous provisions on customer-registered laws escalate to Gemini.

## Shipped (2026-06-08)

### Infrastructure
- NAS backup taken (20260608) — DuckDB 202 MB + LanceDB 375 MB (compacted)
- LanceDB compacted: 8.4 GB → 375 MB
- Old local Parquet backups removed (NAS has copies) — recovered 900 MB
- Build artifacts swept — recovered 2.67 GB
- Disk: 9.3 GB free (from 2.6 GB)

### Ollama + Gemma 3 4B installed
- Brew formula missing llama-server → downloaded full tarball from GitHub releases
- Installed to `~/.local/ollama/` (1.29 GB tarball, extracts to bin + lib)
- Model: `gemma3:4b` (3.3 GB, Q4 quantization)
- Server: `nohup ~/.local/ollama/bin/ollama serve`
- API: `http://localhost:11434/api/generate`

### First test result
- Input: "The employer shall ensure the health and safety of employees."
- Output: Employer=ACTIVE, Employee=COUNTERPARTY (correct)
- Speed: 5.6 tok/s, 19.9 seconds for 112 tokens
- Issues: wraps response in markdown code fences, uses bare labels not prefixed (same parsing issues as Gemini, already have solutions)

### Hardware note
- No discrete GPU — Intel UHD 630 only
- CPU inference: ~5.6 tok/s (Gemma 4B Q4)
- 27.5 GB available for inference
- **Recommendation**: add a second M.2 NVMe SSD (500 GB, ~£30-40) for model storage and build overflow. Or reclaim the 120 GB Windows partition (never used).

## What's next

1. Wire Ollama into the enrichment pipeline as Tier 2
2. Update prompt to enforce exact label format (reuse existing parsing functions)
3. Benchmark on OH&S: accuracy vs Gemini, speed for targeted provisions
4. Implement confidence-based routing: Tier 1 low-confidence → Tier 2
5. Consider upgrading to Gemma 12B when disk space allows (12B better for legal parsing but needs ~8 GB)

## Starting Ollama

```bash
# Start server (run once per session)
nohup ~/.local/ollama/bin/ollama serve > /tmp/ollama.log 2>&1 &

# Verify
curl -s http://localhost:11434/api/tags | /usr/bin/python3 -c "import sys,json; [print(m['name']) for m in json.load(sys.stdin)['models']]"
```

## References

- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY.md`
- Gemini cascade review: `docs/reviews/gemini-cascade-strategy-review-20260607.md`
- Prior QA results: `data/qa-results/drrp-qa-all-20260607-*.json`
- DRRP QA skill: `.claude/skills/drrp-qa/`
- Enrichment skill: `.claude/skills/enrich-and-publish/`
