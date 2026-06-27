# Session: Local LLM Tier — gemma3:4b via Ollama (PENDING)

## Problem

The pipeline's LLM tier uses Gemini 2.5 Flash (cloud, paid, slow). For the ~20% of provisions where regex+classifier disagree, we need LLM adjudication. A local SLM (small language model) could handle many of these cases faster and free.

## Proposed approach

Use Ollama with gemma3:4b as a Tier 2.5 — between classifier (Tier 2) and Gemini (Tier 3):
- Regex (Tier 1) → Classifier (Tier 2) → **Local LLM (Tier 2.5)** → Gemini (Tier 3)
- Local LLM handles disagreements where classifier confidence < 0.7
- Gemini reserved for cases where local LLM is also uncertain

## Options

1. **Prompt-based**: send provision text + actors to gemma3:4b with structured output prompt (same as Gemini but local)
2. **Fine-tuned**: fine-tune gemma3:4b on the gold benchmark data for position classification. Eliminates prompt engineering, faster inference.
3. **LoRA adapter**: lightweight fine-tune on top of gemma3:4b, keeps base model intact

## Current state

- Ollama installed, gemma3:4b available (`http://localhost:11434`)
- Tier 2 LLM already wired in pipeline (LLM_PROVIDER=local uses Ollama)
- Gold benchmarks available for fine-tuning data (4,062 actor-position pairs)

## Deferred because

Gemini review (2026-06-26) said: "feature quality is the bottleneck, not model." Position classifier at 60% with LR and GBT identical — a local LLM won't help if the features feeding it are weak. Fix features first (dependency parsing, Legal-BERT), then evaluate whether SLM adds value on top.

## Work (when unblocked)

1. ⬜ Evaluate gemma3:4b on benchmark provisions with prompt-based approach
2. ⬜ Compare accuracy vs Gemini on the same provisions
3. ⬜ If competitive: wire as Tier 2.5 in pipeline
4. ⬜ If not: consider fine-tuning or LoRA on gold data
5. ⬜ Latency/cost comparison: local vs Gemini

## Dependencies

- Dependency parsing features (for better classifier, reducing LLM load)
- Legal-BERT embedding (same reason)
- Ollama running with gemma3:4b model
