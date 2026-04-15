# Gap C — Session 2: training repo (scoping / todo)

**Date**: 2026-04-15
**Status**: Not started — repo does not yet exist
**Orchestration**: [04-15-26-gap-c-orchestration.md](04-15-26-gap-c-orchestration.md)
**Spec**: [04-15-26-gap-c-ai-research.md](04-15-26-gap-c-ai-research.md)

## Purpose

Session 2 is the only session that runs *outside* `fractalaw`. Because the
training repo doesn't exist yet and the stack is unfamiliar, this doc is
a **scoping / todo list** rather than an execution plan. It sets out what
the new repo needs to look like, the language and library choices, and
the order of work, so that Session 2 can be kicked off without paralysis.

Once the training repo exists, its own session log (inside *that* repo)
will track execution. This doc stays in `fractalaw` as the reference for
what was specified.

## Language and stack decision

### Language — Python

Non-negotiable in practice:
- The Hugging Face ecosystem (`transformers`, `datasets`, `accelerate`,
  `peft`) is Python-native and has no serious competitor.
- The deprecated artefact at `models/deberta-v3-drrp/` (misnamed — it
  is DistilBERT, not DeBERTa) was produced with this stack. That
  artefact is ignored (`best_clause_acc: 0.0`, 200 training examples);
  we are not extending it. The stack itself is still the right choice
  for the new training.
- Teacher-model SDKs (Anthropic, OpenAI) are Python-first.

### Core libraries

| Purpose | Library | Notes |
|---------|---------|-------|
| Base model + training | `transformers` | ModernBERT-large (primary) and DeBERTa-v3-large (comparison baseline); new from-scratch training |
| Parameter-efficient fine-tuning | `peft` (LoRA/QLoRA) | Keeps GPU cost down |
| Dataset handling | `datasets` + `pyarrow` | Parquet interop with main repo |
| Training loop | `transformers.Trainer` | Or `accelerate` for custom loops |
| Tokenizer | `tokenizers` | Matches existing artefact |
| ONNX export | `optimum` (`optimum-onnxruntime`) | Used for the existing int8 export |
| Quantisation | `onnxruntime.quantization` | int8 dynamic quant |
| Eval metrics | `scikit-learn` | Confusion matrix, per-class P/R |
| Teacher-model labelling | `anthropic` SDK (Claude) or `openai` SDK | Claude recommended — matches domain tone and is available via the same CLI |
| Experiment tracking | `wandb` or `mlflow` (optional) | Useful for long runs, skip for MVP |
| Model hosting | `huggingface_hub` CLI + client | Private repo for artefacts |

Alternatives considered and rejected:
- **Unsloth / Axolotl** — good for quick Llama fine-tunes, but
  ModernBERT-large / DeBERTa-v3-large fine-tuning via `transformers`
  Trainer is the standard path and covered by `optimum` for ONNX export.
- **Raw PyTorch** — would force reimplementing trainer + eval loops for
  no benefit.
- **LoRA/QLoRA as default** — ruled out under §0 quality-first lens.
  Full fine-tune is affordable at this model size. LoRA is a fallback
  only if cost grows unexpectedly.

### Compute

- Training: **rented GPU** (RunPod, Lambda, or HF Spaces). Full
  fine-tune of ModernBERT-large or DeBERTa-v3-large (fp16) fits on a
  single A100 40 GB. LoRA/QLoRA would fit on 24 GB but we default to
  full fine-tune per §0 quality-first.
- Inference (after training): **remote serverless-GPU endpoint**
  (RunPod Serverless / Modal / HF Inference Endpoints) per research
  doc §0.1. Scale-to-zero when idle.
- Data prep + teacher labelling + eval: local CPU is fine. Teacher calls
  are I/O-bound, not compute-bound.

### Repository layout (proposed)

```
fractalaw-taxa-training/           # separate git repo, private
├── README.md
├── pyproject.toml                 # uv or poetry; pin all versions
├── .python-version                # 3.11 or 3.12
├── .env.example                   # ANTHROPIC_API_KEY, HF_TOKEN, etc.
├── data/
│   ├── raw/                       # parquet dropped from fractalaw (gitignored)
│   ├── labelled/                  # teacher-labelled parquet (gitignored)
│   └── gold/                      # human-reviewed subset (checked in, small)
├── src/
│   ├── label/
│   │   ├── teacher.py             # Claude/GPT call loop, prompt templates
│   │   ├── prompts.py             # Per-sub-type prompt variants
│   │   └── validate.py            # Label quality checks (no meta-labels!)
│   ├── train/
│   │   ├── dataset.py             # Parquet → HF Dataset, tokenisation
│   │   ├── model.py               # ModernBERT-large or DeBERTa-v3-large + detector + extraction heads
│   │   ├── train.py               # Trainer setup + launch
│   │   └── export.py              # ONNX + int8 quantisation + metadata.json
│   └── eval/
│       ├── metrics.py             # Per-sub-type P/R, confusion matrix
│       └── holdout.py             # Held-out law eval harness
├── notebooks/
│   └── (exploration only — not checked in long-term)
├── scripts/
│   ├── fetch_from_fractalaw.sh    # Rsync/copy the handoff parquet in
│   └── publish_to_hf.sh           # huggingface-cli upload
├── models/                        # local model outputs (gitignored)
└── sessions/                      # session logs for training work
    └── YYYY-MM-DD-*.md
```

## Todo list — what Session 2 must do

### Phase A — repo bootstrap

- [ ] Create new private GitHub repo `fractalaw-taxa-training`.
- [ ] Initialise Python project (`uv init` or `poetry new`). Pin Python
  3.11 or 3.12.
- [ ] Add dependencies: `transformers`, `peft`, `datasets`, `accelerate`,
  `optimum[onnxruntime]`, `anthropic`, `huggingface_hub`, `scikit-learn`,
  `pyarrow`, `pandas`.
- [ ] `.env.example` for API keys; `.gitignore` for `data/raw/`,
  `data/labelled/`, `models/`, `.env`, `wandb/`.
- [ ] Copy in the Session 1 hand-off package: Gap C parquet,
  `gap-c-context-format.md`, and the **reconciled, pinned**
  `holder_labels.json` from Session 1 S1b. Note: this file is the
  **model vocabulary** (training output space), which is a cleaned
  superset of the previous shipped file and deliberately excludes
  regex-era placeholders like `": He"`. See research doc §4.1's
  two-vocabulary section.
- [ ] Write the training repo's CLAUDE.md (tailored for the Python stack)
  and first session log.

### Phasing — C2 pilot first, then expand

Per research doc §5 (stages S4–S6 revised) and resolving critical review
item #9, training is phased:

- **Phase 1 (pilot): C2 only.** Sub-clause inheritance is the
  highest-frequency Gap C sub-type, has the simplest context
  (just parent paragraph), and is the most structurally detectable.
  A working C2-only model proves the full pipeline (labelling →
  training → serverless deploy → fractalaw integration) before
  broader investment.
- **Phase 2: expand to C4 and C6** via head-swap retrain per §8.
  C4 (cross-section actor) needs full section-definitions context;
  C6 (truly actor-less passive) needs act-general-duty context.
  Both exercise the full context-retrieval design.

Phase 1 deliverables are the full Phase A–E set below, scoped to C2
only. Phase 2 re-runs Phase B–D with C4+C6 examples added.

### Phase B — teacher labelling (S4)

- [ ] Write `src/label/prompts.py` — one prompt per sub-type (C1–C6),
  each constrained to output one of the concrete roles from the pinned
  `holder_labels.json` (N classes, where N grows as taxonomy evolves —
  see research doc §8), or `unresolved`. No meta-labels, no regex-era
  placeholders (`": He"` must never appear in output). Enforced by a
  parser that rejects anything else.
- [ ] Write `src/label/teacher.py` — call loop with rate limiting, retry,
  response caching to parquet (so re-runs are cheap).
- [ ] Write `src/label/validate.py` — fails loudly on:
  - Holder not in pinned `holder_labels.json`
  - Holder containing "passive", "inherited", "he", or similar
    meta-language / regex-era placeholders
  - Holder with trailing colon (catches any regression of the
    `"Gvt: Ministry:"` bug)
  - Missing `inferred_from` citation for non-explicit holders
- [ ] Run over the Session 1 parquet. Cache all responses. Produce
  `data/labelled/gap_c_teacher.parquet`.
- [ ] Sanity report: distribution of `holder_class`, `inferred_from`,
  `unresolved` rate per sub-type.

### Phase C — human review (S5)

- [ ] Stratified sample: ~100–200 examples from C2 (parent-inherited) and
  C4 (cross-section) — the sub-types teacher models hallucinate on.
- [ ] Simple review UI (could be a CSV in a spreadsheet — don't
  over-engineer). Reviewer (domain expert) confirms or corrects the
  holder + `inferred_from`.
- [ ] Corrections written back to `data/gold/gap_c_reviewed.parquet`
  (this one *is* checked in — it's small and precious).
- [ ] Report inter-annotator disagreement stats if multiple reviewers.

### Phase D — detector training (S6)

- [ ] `src/train/dataset.py` — loads parquet (teacher + gold), formats
  input as `[CLS] query [SEP] source [SEP] context [SEP]` matching
  `gap-c-context-format.md` from main. Context built from the same
  retrieval rules as S3.
- [ ] `src/train/model.py` — two training configs (dispatched via CLI
  arg), both from scratch, no heads reused from the deprecated artefact:
  - `modernbert-large` (primary) — 8192 context, full fine-tune fp16
  - `deberta-v3-large` (comparison) — 512 context, full fine-tune fp16
  - Heads on both: `is_drrp` binary + `drrp_type` 5-class +
    `holder_class` N-class (N per pinned `holder_labels.json`) +
    `clause_span` + `qualifier_span`
- [ ] `src/train/train.py` — full fine-tune per §0 quality-first lens
  (LoRA only as fallback). Trainer with per-head losses (weighted).
  Eval on held-out **laws** (not provisions) to measure generalisation.
- [ ] `src/train/export.py` — export to ONNX (fp16 default; int8 only
  if throughput needs it and quality holds), bundle with `metadata.json`
  (including `model_version` and `min_fractalaw_ai_version`), write
  pinned `holder_labels.json`.
- [ ] `src/eval/metrics.py` — per-sub-type precision/recall, held-out
  confusion matrix. No prior-polisher baseline exists (the deprecated
  artefact's `best_clause_acc: 0.0` is not a meaningful baseline);
  compare ModernBERT-large vs. DeBERTa-v3-large and pick the winner.
- [ ] Publish winner to HF Hub private repo. Tag the revision.
- [ ] Deploy winner to RunPod Serverless / Modal / HF Inference Endpoint
  behind a stable URL that fractalaw's remote-inference provider calls.
- [ ] Write the hand-off-back note (see orchestration doc).

### Phase E — hand-off back to main

Deliverables per the orchestration doc:
- [ ] HF Hub revision pin (commit hash / tag)
- [ ] `model_version`
- [ ] `min_fractalaw_ai_version`
- [ ] Model card (per-sub-type P/R, held-out law list, known failure
  modes, training data provenance)
- [ ] Label-vocabulary delta (any new concrete roles added, with
  rationale for each)

## Decisions to defer until training repo exists

- ModernBERT-large vs. DeBERTa-v3-large — train both, compare on
  held-out laws, ship the winner. Both runs together are cheap.
- **Single multi-head model vs. multiple specialist models behind the
  endpoint** — server-side implementation detail (research doc §4.1
  resolves critical review #7). The HTTP contract is fixed; fractalaw
  doesn't see the internal architecture. Decide based on training
  stability and endpoint latency, not fractalaw-integration concerns.
- Whether to use `wandb` for tracking — skip for MVP, add if multiple
  training runs happen.
- Which teacher model to use first — Claude Opus vs. GPT-4o. Try Claude
  first because it's already integrated via this CLI and tends to handle
  UK legal text well; fall back to GPT-4o on specific sub-types if
  needed.
- Serverless provider — RunPod Serverless (lean cost, most flexible)
  vs. Modal (best DX) vs. HF Inference Endpoints (simplest, direct
  from HF Hub artefact). Decide at deploy time.

## Phase F — ongoing retraining (long-lived; persists after bootstrap)

Per research doc §8, training is not one-off. The repo persists as a
long-lived asset with:

- [ ] **Retrain recipe** (head-swap fine-tune): script that takes the
  previous model's weights, a new `holder_labels.json`, and the
  incremental training data, and produces a new model version in
  ~10× less time than a full retrain.
- [ ] **Model-version series** with per-version change notes published
  alongside the HF Hub artefact.
- [ ] **QA → taxonomy-change-log → retrain workflow** documented so
  that new actors surfaced by fractalaw's QA command can be turned
  into training data and then a new model version without reinventing
  the process each time.
- [ ] **Endpoint-update script** that pushes a new artefact to the
  serverless provider and rolls over traffic (or just updates "latest"
  if the provider does that transparently).

## Risks specific to Session 2

- **Teacher hallucinations in C2/C4** — mitigated by human review phase.
- **Training dataset too small** — the OHS Gap C parquet is ~3,275
  provisions. After stratification and holdout, training set may be
  under 2k. Consider multi-law expansion before training if results are
  weak. Synthetic augmentation is risky in legal text — avoid unless
  desperate.
- **GPU cost overrun** — full fine-tune of ModernBERT-large on A100 40GB
  should be ~2–4 GPU-hours; DeBERTa-v3-large similar. If it balloons,
  something is wrong —
  re-check the data pipeline rather than throwing more compute at it.
- **Drift from the `gap-c-context-format.md` contract** — any deviation
  in how training builds context vs. how main's `fetch_context` returns
  it will silently ruin inference. Write a test that asserts exact
  byte-equivalence on at least one example.

## Open questions (for Session 2 itself to answer)

- Where to host the training repo — same GitHub org as `fractalaw`, or
  separate? Affects secrets and access.
- HF Hub account — personal or project-owned? Affects artefact URLs.
- Budget ceiling for teacher labelling (Anthropic/OpenAI API spend) —
  estimate ~$50–150 for a 3k-provision one-off pass with Claude/GPT-4o
  at 2026 rates; confirm before kicking off.
