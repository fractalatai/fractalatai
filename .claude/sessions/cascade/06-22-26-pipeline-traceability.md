# Session: Pipeline Traceability & Refactor (PENDING)

## Motivation

The DRRP parsing pipeline has 102+ decision branches across 4 phases (parse_v2 → enrich → classify → position). Most decisions are invisible after the fact — the pipeline stores *what* each tier decided but not *why*. This makes threshold tuning, debugging, and LLM optimisation harder than they need to be.

Discovered during Liberty false-positives investigation (06-22-26-liberty-false-positives.md, CLOSED): 17 Liberty→Obligation provisions with no modal verbs are undetectable by any current tier because there's no tracing to show what the regex matched or why the classifier agreed.

## Two problems, one root cause

### 1. No parsing journey log

Currently stored per provision:
- `drrp_types`: final DRRP label
- `extraction_method`: which tier wrote it (regex/classifier/pending_llm/agentic)
- `taxa_confidence`: single float, no breakdown
- `drrp_history`: JSON array of `{tier, drrp, confidence, timestamp}` — what each tier predicted

Not stored:
- Which regex tier matched (1=governed_v2, 2=gov_v1, 3=gov_v2, 4=offence, 5=rule)
- Which sub-pattern fired (Prohibitive, SFAIRP, Enforcement, Enabling, etc.)
- Why purpose gate skipped DRRP (which gate condition fired)
- Confidence breakdown (span bonus, window penalty, subordinate clause, epistemic may)
- Classifier transition rule applied (gap_fill_accepted, disagreement_pending_llm, both_modals)
- Legal fiction / descriptive summary rejections
- Actor position reasoning chain

### 2. Signal detection interleaved with decision logic

`parse_v2` → `duty_type::classify` → 5 tiers, each both *detects* signals (actor-modal pairs) and *decides* (returns immediately on first match). This means:
- Later tiers never see the text if an earlier tier matches
- No way to compare what different tiers would have said
- Tuning one tier's patterns affects all downstream tiers unpredictably

The 3 core questions are simple:
1. Is there an actor near a modal?
2. Is the modal enabling or obligatory?
3. Is the actor governed or government?

But the code answers them through 31 interleaved branches across 5 tiers.

## Design direction

### Decision trail (tracing)

A structured `decision_trail` per provision — opt-in, not stored by default. Either:
- A JSON string column in LanceDB (like `drrp_history`)
- A log file written during `--verbose` or `--trace` parse runs
- An in-memory struct returned alongside TaxaRecord for QA tooling

Key nodes to log:
1. **Gate**: purpose gate result + which condition fired + actor override
2. **Tier match**: tier number + sub-pattern index + confidence + span positions
3. **Rejections**: legal fiction, epistemic may, subordinate clause
4. **Classifier**: predicted class + confidence + transition rule + threshold applied
5. **Position**: regex position vs classifier position + agreement

### Signal/decision separation (refactor)

Phase 1: Extract all actor-modal pairs from text with positions and types, returning a `Vec<Signal>`. Phase 2: Decision logic picks the best classification from the signal set. This would:
- Make tracing trivial (log the signal set)
- Allow "what-if" analysis (what would tier 3 have said if tier 1 hadn't matched?)
- Simplify threshold tuning (tune the decision logic, not the detection)

This is a larger refactor. The tracing work can start without it.

## Benchmark context

Current pipeline accuracy: **85.5%** (regex + classifier, 16 benchmark laws, 2,250 provisions)

| Class | Precision | Recall | F1 |
|-------|-----------|--------|-----|
| Liberty | 67.1% | 85.2% | 75.1% |
| Obligation | 84.8% | 92.3% | 88.4% |
| none | 95.1% | 80.7% | 87.3% |

Hard cases that tracing would help diagnose:
- 17 Liberty→Obligation with no modals (invisible to current tiers)
- 125 none→Liberty classifier false positives (threshold tuning needed)
- 9 Liberty→none missed by all tiers

## Key files

- `fractalaw-core/src/taxa/mod.rs:110-242` — parse_v2 (75 decision branches)
- `fractalaw-core/src/taxa/duty_type.rs:72-104` — tier cascade (31 branches)
- `fractalaw-core/src/taxa/duty_patterns.rs` — government patterns + modal context
- `fractalaw-core/src/taxa/duty_patterns_v2.rs` — governed actor-anchored patterns
- `fractalaw-cli/src/main.rs:4863-5438` — classify pass, transition rules

## Prior sessions

- `06-22-26-liberty-false-positives.md` (CLOSED) — discovered the tracing gap
- `06-22-26-rule-class-cleanup.md` (CLOSED) — Rule→Obligation remap
- `06-18-26-benchmark-post-restructure.md` (CLOSED) — benchmark baseline
