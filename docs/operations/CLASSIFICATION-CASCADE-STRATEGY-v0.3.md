# Classification Cascade Strategy v0.3

## What Changed from v0.2

QA data (53 samples) revealed that the regex confidence score is **not predictive of correctness**. INCORRECT provisions have higher average confidence (0.76) than CORRECT ones (0.68). The confidence-based gating in v0.2 was fundamentally flawed.

### The evidence

| Category | Correct | Total | Precision |
|---|---|---|---|
| Regex overall | 10 | 46 | 22% |
| Regex single-actor + DRRP found | ~8 | ~10 | ~80% |
| Regex single-actor + DRRP=none | ~1 | ~20 | ~5% |
| Regex multi-actor | 2 | 17 | 12% |
| Regex confidence >= 0.7 | 4 | 13 | 31% |
| Regex confidence >= 0.85 | 4 | 13 | 31% |

Confidence doesn't discriminate. But actor count + DRRP presence does.

### Failure categories (36 INCORRECT regex provisions)

| Category | Count | % | Root cause |
|---|---|---|---|
| DRRP=none wrong | 17 | 47% | Parser missed the duty/power/right |
| Position wrong | 13 | 36% | DRRP type correct, actors in wrong positions |
| Fragment | 4 | 11% | Text too short to classify meaningfully |
| Other | 2 | 6% | DRRP type wrong (e.g., Power classified as Responsibility) |

## New Architecture: Regex as Sieve, Not Classifier

Regex is demoted from "classifier" to "sieve". It answers three questions reliably and nothing more:

### What regex CAN do reliably

1. **Purpose classification** — "is this a definitions section, an enactment block, a scope clause?" These are structural and don't need semantic understanding. The purpose gate (`should_skip_drrp`) is reliable.

2. **DRRP smell test** — "does this text contain obligation language?" The presence of actors + modals (shall/must/is required to) is a reliable signal that DRRP exists. This is the making/not-making gate.

3. **Single-actor DRRP extraction** — when exactly one actor is found and the regex matches a clear "actor + modal" pattern, the classification is ~80% reliable. Single-actor provisions are unambiguous: the one actor found is active.

### What regex CANNOT do reliably

1. **Multi-actor position classification** — which actor is active vs counterparty. The span heuristic is wrong 36% of the time.

2. **DRRP type for complex text** — schedule fragments, passive voice, thing-subject obligations, narrative duty references. The parser misses ~47% of these.

3. **Confidence scoring** — the current `taxa_confidence` does not predict correctness.

## Revised Pipeline

```
                    ┌─────────────┐
                    │  Provision  │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  Purpose    │
                    │  Gate       │  Regex (FREE)
                    │  (skip/     │
                    │   keep)     │
                    └──────┬──────┘
                           │ keep
                    ┌──────▼──────┐
                    │  Actor +    │
                    │  Modal      │  Regex (FREE)
                    │  Detection  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        no actors    1 actor +     2+ actors
        no modal     modal found    or no modal
              │            │            │
              ▼            ▼            ▼
         ┌────────┐  ┌────────┐  ┌────────┐
         │ DONE   │  │ Regex  │  │ Tier 2 │
         │ none   │  │ DRRP   │  │ Local  │  4B CPU
         │ conf=  │  │ conf=  │  │ Gemma  │  (~20s)
         │ 0.90   │  │ 0.80   │  │        │
         └────────┘  └────────┘  └────────┘
                                      │
                               ┌──────▼──────┐
                          high │ Confidence   │ low (customer laws)
                          conf │  check       │
                               └──────┬──────┘
                                      │ low
                               ┌──────▼──────┐
                               │  Tier 3     │
                               │  Gemini     │  API ($)
                               │  (QA +      │
                               │   correct)  │
                               └─────────────┘
```

### Key changes from v0.2

1. **No confidence-based routing between Tier 1 and Tier 2** — actor count is the router, not confidence
2. **Single-actor = regex definitive** — only case where regex is trusted as classifier
3. **Multi-actor = always Tier 2** — local model classifies positions for all multi-actor provisions
4. **DRRP=none with actors = Tier 2** — the 47% failure category, let the model decide
5. **Correct "none" (no actors, no modals) = done at 0.90** — structural provisions are definitively non-DRRP

### Confidence values (revised)

| Classification | Confidence | Meaning |
|---|---|---|
| Purpose gate: structural (no actors, no modals) | 0.90 | Definitely not DRRP |
| Regex: single-actor + DRRP match | 0.80 | Reliable, single-actor unambiguous |
| Regex: single-actor + DRRP=none | 0.30 | Parser miss candidate, needs Tier 2 |
| Local model (Tier 2) validated | 0.80 | Position classified by Gemma |
| Local model (Tier 2) unvalidated | 0.60 | Invented labels |
| Gemini (Tier 3) validated | 0.90 | Highest confidence |
| Gemini QA correction | 0.90 | Write-back from QA loop |

### What this means for the code

**Tier 1 changes (regex):**
- Purpose gate stays as-is (reliable)
- Actor + modal detection stays as-is (reliable for presence, not classification)
- Single-actor: keep full regex classification (DRRP type + position=active)
- Multi-actor: extract actors and DRRP type hint, but mark for Tier 2 elevation
- DRRP=none with actors: mark for Tier 2 elevation
- New confidence scoring based on the routing decision, not match quality

**Tier 2 changes (local model):**
- Receives: provision text + actors + DRRP type hint from regex
- Classifies: DRRP type (confirm/override regex) + actor positions
- Fires on: all multi-actor provisions AND single-actor DRRP=none provisions
- No longer gated by confidence — gated by actor count and DRRP presence

**Tier 3 stays as-is:**
- QA + correction write-back on customer laws
- Confidence protection prevents overwrite

## Estimated Impact

Based on MHR (61 provisions):
- 12 structural (no actors/modals) → done at purpose gate (0.90)
- ~20 single-actor with DRRP → regex definitive (0.80)
- ~20 multi-actor → Tier 2 local model
- ~9 single-actor DRRP=none → Tier 2 for DRRP classification

Tier 2 load: ~29 provisions per law (~20s each = ~10 min per law on CPU). For the QQ corpus of 274 laws, that's ~130 making laws × 29 provisions × 20s = ~21 hours total CPU. One-time cost, stamped as done.

## Hierarchy Path Issue (Upstream)

Sub-paragraph hierarchy paths are flattened in the sertantai LAT parser (shotleybuilder/sertantai-legal#108). This breaks Tier 1 inheritance for clause fragments like reg.4(1)(b)(ii). Until fixed upstream, these provisions correctly get DRRP=none and elevate to Tier 2.

## What to Build

1. **Revise Tier 2 filter** — fire on multi-actor OR (single-actor + DRRP=none with actors)
2. **Revise confidence scoring** — based on routing decision, not regex match quality
3. **Extend Tier 2 prompt** — classify DRRP type as well as positions
4. **Remove confidence-based escalation** — replace with actor-count routing

## References

- QA data: `data/qa-results/drrp-qa-*.json` (53 samples across 6 runs)
- v0.2 strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY.md`
- Gemini cascade review: `docs/reviews/gemini-cascade-strategy-review-20260607.md`
- Hierarchy issue: shotleybuilder/sertantai-legal#108
- Session: `.claude/sessions/cascade/06-08-26-local-model-tier2.md`
