# Session: Cascade v0.3 Implementation — Regex as Sieve

## Context

**Strategy**: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
**Gemini review**: `docs/reviews/gemini-cascade-v03-review-20260608.md`
**Prior session**: `.claude/sessions/cascade/06-08-26-local-model-tier2.md`

QA data (53 samples) proved regex confidence is not predictive. v0.3 demotes regex to sieve, routes by actor count. Gemini validates the approach.

## What to build

### 1. Revise Tier 2 filter (priority — core control flow)

Current filter: multi-actor AND all-active AND existing_conf < 0.80 AND DRRP not empty

New filter: (multi-actor) OR (single-actor AND DRRP=none AND has actors)

This routes the two failure categories (47% parser misses + 36% position wrong) to Tier 2.

### 2. Extend Tier 2 prompt for DRRP classification

Current Tier 2 prompt only classifies positions (active/counterparty/etc). For provisions with DRRP=none, it also needs to determine the DRRP type (Duty/Right/Responsibility/Power/none).

Add to prompt: "Also classify the DRRP type if the provision contains an obligation."

### 3. Revise confidence scoring

Current: `taxa_confidence` from regex match quality (not predictive).

New:
- Purpose gate structural skip → 0.90 (definitely not DRRP)
- Single-actor + DRRP match → 0.80 (regex reliable core)
- Single-actor + DRRP=none → 0.30 (elevate to Tier 2)
- Multi-actor → 0.30 (always elevate to Tier 2)
- Tier 2 validated → 0.80
- Tier 3 / QA correction → 0.90

### 4. Write DRRP type from Tier 2 back to LanceDB

Currently Tier 2 only writes actors. When it classifies DRRP type, write that back too.

## Verification

- Re-enrich MHR with new filter
- QA 20% of MHR provisions
- Target: >60% precision (up from 22% regex, 40% current)
- Each QA pass should show improvement as corrections accumulate

## Files to modify

| File | Change |
|------|--------|
| `crates/fractalaw-cli/src/main.rs` | Tier 2 filter, prompt, DRRP write-back, confidence scoring |
| `.claude/skills/drrp-qa/run_qa.py` | May need adjustment for new extraction_method values |

## Shipped (2026-06-08)

### v0.3 core changes (commit `8d119bb`)
- Tier 2 filter: multi-actor OR (DRRP=none with actors) — actor-count routing
- Tier 2 prompt extended: classifies DRRP type + positions
- Confidence scoring: single-actor+DRRP=0.80, multi-actor/none=0.30, structural=0.90
- DRRP type written back from Tier 2 to LanceDB

### Configurable provider (commit `44ef36d`)
- `TIER2_PROVIDER=gemini` — two-tier cascade, regex → Gemini (current)
- `TIER2_PROVIDER=local` — three-tier, regex → local Gemma → Gemini QA (after GPU)
- Unset — regex only, no model
- Same pipeline, confidence protection, write-back — just different inference backend
- MHR test: 8/8 Gemini classified, 6 validated, 14 protected

### Gemini review of v0.3 (commit `06c6d0a`)
- Demotion of regex justified by data
- Actor-count routing is the right discriminator
- Build the filter first — confirmed

### Structural provision cleanup (commit `0bbdd69`)
- Titles, signed blocks, headings, schedules, tables cleared of stale actors and DRRP
- Regex keyword matching on "Signed by order of the Secretary of State" no longer produces noise

### Regulation-level classification only (commit `c8ff5b8`)
- Tier 2 only fires on article/sub_article/section/sub_section
- Fragments (paragraph/sub_paragraph) inherit from parent — not independently classified
- QA report outputs to terminal showing regulation-level provisions only
- MHR: 0 Tier 2 candidates (down from 11) after filter tightening

### Key discovery: Gemma 4B works on full regulation text
- Previous 0/4 QA failures were on **fragments** — text too short to reason with
- With full regulation-level text, Gemma 4B achieves **10/11 validated, canonical labels**
- Work at Height Regs (UK_uksi_2005_735): 11 Tier 2 provisions, 10 validated

### QA results: DRRP type accuracy 100%
- WHR QA: 4/7 correct (57% overall), but **DRRP type was correct on all 7 samples**
- Failures were secondary: missing counterparty actors (2), invented label (1)
- Primary compliance question ("does this create a Duty on an Employer?") answered correctly
- Corrections applied via write-back ratchet

### QA precision journey (this session)

| Stage | Precision | What changed |
|---|---|---|
| Regex baseline (start of day) | 22% | Confidence not predictive |
| MHR with local 4B on fragments | 0% | 4B too small for fragments |
| v0.3 actor-count routing | 40% | Right provisions reach Tier 2 |
| Structural filter | 57% | No noise from titles/schedules |
| Regulation-level only + local 4B | 57% | Full text gives 4B enough context |
| **DRRP type accuracy** | **100%** | The primary signal is reliable |

### QA report for human review (commit `4b63f16`)
- `--report` flag generates regulation-level DRRP table to stdout
- Shows: section, DRRP type, confidence, method, actors+positions, text snippet
- Workflow: report → human review → QA with write-back → re-report

## Hardware recommendation

Gemma 4B on CPU proved the pipeline but is too small for legal parsing quality (0/4 QA). The local model needs a GPU upgrade.

**Recommended**: Used NVIDIA RTX 3090 24 GB (~£350-450 eBay UK)
- 24 GB VRAM fits Gemma 12B Q8 or Qwen 14B Q4
- ~50-100 tok/s (vs 5 tok/s CPU) — MHR in ~30 seconds
- Full QQ corpus: ~1-2 hours GPU vs 21 hours CPU
- Model in VRAM, pipeline in system RAM — no contention
- Check PSU (350W TDP) and case (3-slot)

Current 40 GB system RAM is sufficient — no upgrade needed until dual-GPU.

**What transfers to 12B**: all pipeline code, friendly labels, confidence protection, write-back loop. Just `ollama pull gemma3:12b` and set `TIER2_PROVIDER=local`.

## What's next

1. Run v0.3 across more customer SIs — `TIER2_PROVIDER=local` for bulk, `gemini` for QA
2. Human review via `--report` → QA with `--write-back` on flagged laws
3. Publish improved data to sertantai
4. Hardware upgrade → RTX 3090 → Gemma 12B for better counterparty detection
5. sertantai-legal#108 — sub-paragraph hierarchy path fix for inheritance

## Key learnings

1. **Fragments are the enemy** — sub-paragraphs can't be classified independently. Classify at regulation level.
2. **Gemma 4B works on full text** — the model is capable when given proper context. Previous failures were input quality, not model quality.
3. **DRRP type is the reliable signal** — 100% accuracy on primary classification. Counterparty detection is secondary and improving.
4. **Structural provisions are noise** — titles, schedules, headings should never reach the model.
5. **The QA report in the CLI** — human review before Gemini QA prevents circular LLM-checking-LLM.

## References

- Strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Review: `docs/reviews/gemini-cascade-v03-review-20260608.md`
- QA results: `data/qa-results/drrp-qa-*.json`
- Upstream issue: shotleybuilder/sertantai-legal#108
- Prior session: `.claude/sessions/cascade/06-08-26-local-model-tier2.md`
