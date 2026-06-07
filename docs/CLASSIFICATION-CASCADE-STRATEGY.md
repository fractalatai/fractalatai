# Classification Cascade Strategy

## Problem

The DRRP parser uses regex patterns to classify ~162K provisions across ~500 making laws. The regex handles the clear cases well (actor in subject position + modal verb) but misses ~30% of provisions — schedule fragments, passive voice, thing-subject obligations, narrative duty references. These require semantic understanding that regex can't provide.

LLM classification (Gemini 2.5 Flash) handles these cases accurately but is too expensive to run on the full corpus (~$X per run, plus latency). We need a strategy that maximises classification coverage while minimising cost.

## Architecture: Confidence-Based Cascade

Based on the [3-Tier Routing Cascade](https://blog.meganova.ai/the-3-tier-routing-cascade-rule-based-semantic-llm/) and [Gatekeeper](https://arxiv.org/html/2502.19335v3) patterns from production ML systems.

```
                    ┌─────────────┐
                    │  Provision  │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  Tier 1:    │
                    │  Regex      │  FREE
                    │  (parse_v2) │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
              high  │ Confidence  │  low
           ┌────────┤  threshold  ├────────┐
           │        └─────────────┘        │
           │                               │
    ┌──────▼──────┐                 ┌──────▼──────┐
    │   DONE      │                 │  Tier 2:    │
    │  stamp &    │                 │  Own model  │  CHEAP
    │  persist    │                 │  (LanceDB)  │
    └─────────────┘                 └──────┬──────┘
                                           │
                                    ┌──────▼──────┐
                              high  │ Confidence  │  low
                           ┌────────┤  threshold  ├────────┐
                           │        └─────────────┘        │
                           │                               │
                    ┌──────▼──────┐                 ┌──────▼──────┐
                    │   DONE      │                 │  Tier 3:    │
                    │  stamp &    │                 │  LLM        │  EXPENSIVE
                    │  persist    │                 │  (Gemini)   │
                    └─────────────┘                 └──────┬──────┘
                                                           │
                                                    ┌──────▼──────┐
                                                    │   DONE      │
                                                    │  stamp &    │
                                                    │  persist    │
                                                    └─────────────┘
```

### Tier 1: Regex (parse_v2) — FREE

What we have now. Actor-anchored patterns with match spans.

**Produces**: DRRP type, actor positions (via span heuristic), confidence score.

**High confidence signals** (stamp as done):
- v2 match span captured (actor + modal in expected positions)
- `taxa_confidence > 0.7`
- Single DRRP type (not ambiguous)
- Actors clearly in subject/object positions

**Low confidence signals** (escalate):
- No DRRP match despite actors present
- `taxa_confidence < 0.5`
- Multiple actors but no span (can't determine positions)
- Schedule/paragraph fragments (short text, high depth)

### Tier 2: Own Model — CHEAP (future)

A lightweight classifier trained on Tier 1 high-confidence provisions and Tier 3 LLM-labelled provisions. Runs locally on LanceDB embeddings — no API cost.

**Training data sources**:
- Tier 1 high-confidence provisions (~60K) as positive examples
- Tier 3 LLM-classified provisions (~500) as labelled examples for the hard cases
- Active learning: selectively send uncertain Tier 2 predictions to Tier 3 for labelling

**Model approach options**:
- Fine-tuned classifier on 384-dim embeddings (already in LanceDB)
- Few-shot retrieval: find nearest high-confidence provision, transfer its classification
- Simple neural net: embedding → DRRP type + position

**This tier doesn't exist yet.** The cascade currently goes Tier 1 → Tier 3 directly.

### Tier 3: LLM (Gemini 2.5 Flash) — EXPENSIVE

Current implementation. REST API via reqwest, `thinkingBudget: 256`.

**When it fires**:
- Inherited provisions with multiple actors (position ambiguity)
- Tier 1/2 low-confidence provisions on customer-registered laws (priority)
- Active learning samples (building Tier 2 training set)

**Never fires on**:
- Provisions already stamped as done
- Laws not on any customer register (accept Tier 1 classification)
- Single-actor provisions (position is unambiguous)

## Persistence: "Done" Stamping

Once a provision is classified with high confidence by ANY tier, it is stamped as done and never re-parsed. This is the key cost control mechanism.

### Implementation

Add a `classification_confidence` field per provision (already exists as `taxa_confidence`) and a `classification_tier` field:

```
classification_tier: Utf8   -- "regex" | "model" | "llm"
taxa_confidence: Float32    -- 0.0 to 1.0
```

Re-enrichment with `--force` currently re-parses everything. Add `--force-low-confidence` that only re-parses provisions below the confidence threshold. The `--skip-recent` flag already prevents re-processing within 24 hours; `--force-low-confidence` extends this to a permanent skip for high-confidence provisions.

### Thresholds

| Threshold | Meaning |
|-----------|---------|
| > 0.7 | **Done** — high confidence, skip on re-enrichment |
| 0.5 - 0.7 | **Acceptable** — use as-is but eligible for Tier 2/3 refinement |
| < 0.5 | **Low confidence** — prioritise for escalation |
| 0.0 | **No match** — DRRP type = none, must escalate if actors present |

## Customer Prioritisation

Not all laws need the same classification depth. The cascade depth depends on whether the law is on a customer register.

| Law status | Cascade depth | Rationale |
|-----------|---------------|-----------|
| On customer register | Full cascade (Tier 1 → 2 → 3) | Customer sees this data in the UI |
| In customer's applicable family | Tier 1 + selective Tier 2 | May become relevant |
| Not on any register | Tier 1 only | Accept regex confidence |

This means the expensive tiers only run on ~274 laws (QQ applicable) rather than ~500 making laws or ~19K total. The cost reduction is ~95%.

### Implementation

The customer's applicable laws file (`data/qq-applicable-laws.csv`) already exists. Add a `--priority` flag to enrichment:

```bash
# Full cascade on customer laws
fractalaw taxa enrich --gap-c --force-low-confidence --priority customer

# Regex only for everything else
fractalaw taxa enrich --gap-c
```

## Active Learning Loop

The cascade generates training data for Tier 2 as a byproduct:

1. Tier 3 LLM classifies a provision → labelled example
2. Provision's 384-dim embedding is already in LanceDB
3. Periodically: train Tier 2 model on accumulated labels
4. Tier 2 handles more cases → fewer Tier 3 calls → cost decreases over time

The [active learning](https://lilianweng.github.io/posts/2022-02-20-active-learning/) strategy is **uncertainty sampling**: pick the provisions where Tier 2 is least confident and send those to Tier 3. This maximises the information gain per LLM call.

## Estimated Coverage

Based on QA results (OH&S family, 10 samples each across 3 QA runs):

| Category | Provisions | Tier needed | Current status |
|----------|-----------|-------------|----------------|
| Clear duty (actor + modal) | ~60% | Tier 1 (regex) | Working |
| Position classification | ~15% | Tier 1 (span heuristic) | Working |
| Participial/separated patterns | ~5% | Tier 1 (regex fix) | Fixed this session |
| Schedule fragments, passive voice | ~10% | Tier 2 (model) | Not built |
| Narrative duty, complex clauses | ~5% | Tier 3 (LLM) | Working but expensive |
| Definitional/conditional (not DRRP) | ~5% | Correct "none" | No action needed |

## What to Build Next

1. **`--force-low-confidence` flag** — only re-parse provisions below confidence threshold
2. **`classification_tier` column** — track which tier classified each provision
3. **Tier 2 prototype** — nearest-neighbour on embeddings, transfer classification from similar high-confidence provision
4. **Active learning harness** — select uncertain Tier 2 provisions, send to Tier 3, accumulate labels
5. **Customer priority routing** — cascade depth based on register membership

## References

- [3-Tier Routing Cascade: Rule-Based → Semantic → LLM](https://blog.meganova.ai/the-3-tier-routing-cascade-rule-based-semantic-llm/)
- [Gatekeeper: Improving Model Cascades Through Confidence Tuning](https://arxiv.org/html/2502.19335v3)
- [LLM Routing and Model Cascades: Cut Costs Without Sacrificing Quality](https://tianpan.co/blog/2025-11-03-llm-routing-model-cascades)
- [Confident Thresholding](https://iterate.ai/ai-glossary/confident-thresholding)
- [Active Learning](https://lilianweng.github.io/posts/2022-02-20-active-learning/)
- [CARGO: Confidence-Aware Routing for LLMs](https://arxiv.org/pdf/2509.14899)
- Fractalaw design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md`
- Hohfeldian position model: `docs/reviews/gemini-actors-struct-review-20260607.md`
