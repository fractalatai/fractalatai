# Gemini Review: Signal/Decision Separation for DRRP Pipeline

**Date:** 2026-06-22
**Model:** Gemini 2.5 Flash (or Pro)
**Status:** REVIEWED

## Review prompt

Review this refactoring plan for a legal text classification pipeline. The pipeline classifies UK legislation provisions into DRRP types (Obligation/Liberty/none) using a 5-tier regex cascade followed by an ML classifier. Current accuracy: 85.5% on 2,250 benchmark provisions.

The core problem: signal detection (finding actor-modal pairs in text) is interleaved with decision logic (choosing which classification to accept) across 102+ decision branches. This makes it hard to trace why a provision got a particular classification, and hard to tune tier-promotion thresholds for LLM optimisation.

Please assess:
1. Is the `SignalSet` / `PatternSignal` / `RejectedSignal` type design right for this problem?
2. Is the 5-stage incremental approach safe, or should stages be reordered/combined?
3. What risks does running all 5 tiers (instead of first-match-wins) introduce?
4. Is the `DecisionTrail` sufficient for the tracing/diagnostics use case, or should it carry more information?
5. Are there better patterns from NLP/information extraction for this kind of signal-then-decide architecture?
6. Any concerns about the backward-compatibility constraint (TaxaRecord unchanged)?

---

## Plan

### Context

The DRRP classification pipeline has 102+ decision branches across 5 regex tiers, with signal detection interleaved with decision logic. This makes it hard to trace why a provision got a particular classification, and hard to tune tier-promotion thresholds.

Discovered during Liberty false-positives investigation: 17 Liberty→Obligation provisions with no modal verbs are undetectable by any current tier because there's no tracing to show what the regex matched or why the classifier agreed.

Goal: Separate signal extraction from decision logic so every provision has a traceable "parsing journey" — what signals were detected, what was rejected, and why the winning classification was chosen.

### Current Architecture

Code in `crates/fractalaw-core/src/taxa/`. Five regex tiers in a first-match-wins cascade:

1. **Governed V2** (actor-anchored): For each governed actor keyword, builds anchored regexes `\b{keyword}\b.{0,window}{obligation_pattern}` with 8 sub-type patterns tried in specificity order. 120-char primary window, 200-char extended fallback. Rejection checks: subordinate clause detection, epistemic "may" filtering, definitional constructions.

2. **Government V1** (keyword-based): Specific patterns (enforcement, regulation-making, code approval) then blunt gate fallback (government actor + obligation/enabling modal). Recently added modal-context override (`apply_modal_context`) to detect enabling "may" vs obligatory "shall".

3. **Government V2** (extended): Direction, guidance, consultation, appointment, delegation, fees, parliamentary reporting patterns. Same modal-context override.

4. **Offence-as-duty**: Offence-creating language ("it is an offence for a person to...") as implicit prohibition. Penalty-dominant provisions excluded.

5. **Rule** (thing-subject): Thing keyword (equipment, routes, premises) near modal verb, with negative guard rejecting if person-actor is closer. Now maps to Obligation (was separate Rule class, recently remapped).

Each tier returns `DutyClassification { family: DutyFamily, sub_type: DutySubType, confidence: f32, span: Option<MatchSpan> }`.

The `parse_v2()` orchestrator runs: text cleaning → purpose classification → actor extraction → purpose gate → duty_type::classify() (5-tier cascade) → legal fiction rejection → POPIMAR → clause extraction → confidence scoring → actor position derivation. Returns `TaxaRecord`.

### Information lost in current design

- Which tier matched (family recorded but not tier number)
- Alternative matches from other tiers (first-match-wins discards all alternatives)
- Rejected candidates (subordinate clause, epistemic may, penalty exclusion)
- Confidence breakdown (flat values, no component scoring)
- Government patterns don't capture actor position (no anchor)

### Proposed Architecture

#### New types (`taxa/signals.rs`)

```rust
pub enum SignalTier { GovernedV2, GovernmentV1, GovernmentV2, OffenceAsDuty, Rule }

pub struct PatternSignal {
    pub tier: SignalTier,
    pub family: DutyFamily,
    pub sub_type: DutySubType,
    pub confidence: f32,
    pub span: Option<MatchSpan>,
    pub actor_keyword: Option<String>,
    pub actor_label: Option<String>,
}

pub struct RejectedSignal {
    pub tier: SignalTier,
    pub reason: RejectionReason,  // SubordinateClause, EpistemicMay, PenaltyProvision, etc.
    pub actor_keyword: Option<String>,
    pub span: Option<MatchSpan>,
}

pub struct SignalSet {
    pub matches: Vec<PatternSignal>,     // ALL positive hits across all tiers
    pub rejected: Vec<RejectedSignal>,   // Candidates rejected with reasons
    pub governed_actors: Vec<ActorMatch>,
    pub government_actors: Vec<ActorMatch>,
    pub purposes: Vec<&'static str>,
    pub is_legal_fiction: bool,
    pub is_descriptive_summary: bool,
    pub purpose_gated: bool,
}
```

#### Decision engine (`taxa/decision.rs`)

```rust
pub struct DecisionTrail {
    pub winner: Option<PatternSignal>,
    pub reason: &'static str,       // "tier_priority_then_confidence", "purpose_gated", etc.
    pub candidates_count: usize,
    pub rejections_count: usize,
}

/// Pure function: given signals, pick best classification.
/// Default strategy replicates current first-match-wins cascade.
pub fn decide(signals: &SignalSet) -> (ClassificationResult, DecisionTrail)
```

#### Public API addition

```rust
// Backward compatible — TaxaRecord struct unchanged
pub fn parse_v2_with_trail(raw_text: &str, family: Option<&str>) -> (TaxaRecord, DecisionTrail)
```

### Staging Plan

**Stage 1: Introduce types (no behaviour change)**
- New `signals.rs`, `decision.rs`. `extract_all()` stub wraps existing `classify()` into single-entry SignalSet. Wire through `parse_v2` — output byte-identical.

**Stage 2: Extract signals from Governed V2 (Tier 1)**
- Add `extract_governed_v2_signals()` — collects ALL matches and rejections instead of first/best. `find_valid_match` pushes RejectedSignal for subordinate/epistemic failures.

**Stage 3: Extract signals from Tiers 2-5**
- Add `extract_*_signals()` to each tier. Government patterns: push instead of early return. Modal override recorded. Offence/Rule: rejections captured.

**Stage 4: Wire parse_v2 through signals/decision**
- Replace `classify()` call with `extract_all()` + `decide()`. Shadow-mode test verifies identical output. Benchmark regression check.

**Stage 5: Expose trail to CLI diagnostics**
- `cmd_taxa_show`, `cmd_taxa_eyeball` use `parse_v2_with_trail()`. Optional `--signals` flag for JSON dump.

### Performance estimate

Running all 5 tiers adds ~15 regex tests per provision. Government/offence/rule are simple compiled regexes (<1us each). Estimated <5% overhead on 20K-provision enrichment pass.

### Downstream consumers (unchanged)

- `enrich_single_law` in CLI: maps TaxaRecord → Arrow columns
- `cmd_taxa_qa/show/eyeball`: re-run parse_v2 for live analysis
- `DutyClassification` types are internal to taxa/ — not used outside the crate
- `classification.span` used only for clause extraction, then discarded
- `clause_structure` computed but never persisted

---

## Gemini Review

See full review: `data/code-review/signal-decision-separation.md`

### Key feedback

1. **Type design approved** — `SignalSet`/`PatternSignal`/`RejectedSignal` "spot-on" for the problem. Suggested enhancements: signal IDs for cross-referencing, granular rejection context.

2. **Staging order is safe** — current order optimal. Shadow-mode test in Stage 4 is "absolutely critical" and the primary regression defence.

3. **Running all tiers — risks manageable** — performance overhead likely <5% as estimated. Main risk is ensuring `decide()` exactly replicates implicit tie-breaking rules from original cascade. Memory unlikely to be an issue.

4. **DecisionTrail enhancements** — start as-is, but consider:
   - More specific `reason` enum (e.g. `TierPriority(GovernedV2)`)
   - Top N alternatives with "why they lost" annotations
   - Rejection reason histogram (`HashMap<RejectionReason, usize>`)

5. **Architecture pattern validated** — signal-then-decide is an industry-standard NLP pattern (matches UIMA/spaCy annotation pipelines). Refactor sets up future ML integration: `SignalSet` → feature vector → trained classifier could replace hand-tuned rules.

6. **Backward compatibility** — the biggest risk. Must verify byte-identical output including f32 precision. Tie-breaking, order of operations, and implicit defaults need careful replication.
