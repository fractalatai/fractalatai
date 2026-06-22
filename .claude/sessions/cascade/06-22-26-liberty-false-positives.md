# Session: Liberty False Positives (ACTIVE)

## Problem

Liberty recall was 64.1% after the Rule→Obligation remap — 128 gold=Liberty provisions being missed (68 as none, 60 as Obligation).

## Root cause found

Government v1/v2 keyword patterns (enforcement, direction, appointment, etc.) matched on semantic keywords without checking whether the modal was enabling ("may") or obligatory ("shall/must"). Example: "enforcing authority **may** serve a notice" hit `GOV_ENFORCEMENT_1` and returned `Enforcement → Obligation` instead of Liberty.

Traced via `parse_v2` integration test — `GOV_ENFORCEMENT_1` fired at Government v1 tier 2 before Governed v2 could try the Enabling pattern on the same text.

## Fix applied (commit `bc9a54c`)

Added `first_modal_is_enabling()` + `apply_modal_context()` wrapper to all specific government patterns in v1 and v2. If the first modal in the text is enabling (may/power to/entitled), the sub_type is overridden to `Enabling` → maps to Liberty. Patterns that already embed `\bshall\b` (GOV_EU_ENSURE, GOV_REG_MAKING_1) are unaffected since `first_modal_is_enabling` returns false when obligation modal comes first.

## Benchmark progression

| Stage | Accuracy | Liberty P | Liberty R | Liberty F1 |
|-------|----------|-----------|-----------|------------|
| Pre-fix (Rule in output) | 84.4% | 66.7% | 81.8% | 73.5% |
| Rule→Obligation remap | 84.0% | 81.8% | 64.1% | 71.9% |
| **Modal awareness** | **84.8%** | **81.8%** | **69.2%** | **75.0%** |

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Liberty | 81.8% | 69.2% | 75.0% | 357 |
| Obligation | 88.6% | 80.4% | 84.3% | 791 |
| none | 83.4% | 93.0% | 87.9% | 1102 |

**Changes from modal fix:**
- Liberty→Obligation misses: 60 → 42 (-18 fixed)
- Obligation precision: 86.1% → 88.6% (+2.5pp)
- none→Liberty: 33 → 36 (+3 slight regression)

## Remaining mismatches

- **68 Liberty→none**: regex finds no enabling modal at all. These are provisions where Liberty comes from immunity/entitlement context that the regex doesn't cover (e.g. "nothing in this regulation is taken to compel", "entitled to withhold production")
- **42 Liberty→Obligation**: obligation modal fires before enabling modal in mixed-modal text, or enabling context is too far from the actor keyword
- **36 none→Liberty**: regex over-triggering on procedural "may" (29 regex, 4 classifier)

## Post-classify benchmark (regex + classifier)

**85.5%** (1,923/2,250) after running both parse and classify on benchmark laws.

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Liberty | 67.1% | 85.2% | 75.1% | 357 |
| Obligation | 84.8% | 92.3% | 88.4% | 791 |
| none | 95.1% | 80.7% | 87.3% | 1102 |

The classifier filled 59/68 Liberty→none gaps. Liberty recall jumped from 69.2% → 85.2%. But Liberty precision dropped from 81.8% → 67.1% — the classifier is aggressively gap-filling none→Liberty (125 false positives).

### Remaining mismatches after classify

**9 Liberty→none**: Neither regex nor classifier finds Liberty. These need LLM.

**44 Liberty→Obligation**: Of these:
- **25 have both modals** → already flagged for LLM via `both_modals` check
- **2 enabling only** → edge case (regex shouldn't have found Obligation)
- **17 neither modal** → invisible to current elevation logic. No "may/shall" signal for the classifier or `both_modals` check to work with. These are provisions where Liberty comes from contextual entitlement/immunity language.

**125 none→Liberty false positives**: Classifier gap-fill threshold (0.7) is too aggressive. Tuning this up would trade Liberty recall for precision.

## Pipeline traceability analysis

### The problem

102+ decision branches across 4 phases (parse_v2 → enrich → classify → position), and most are invisible after the fact. The pipeline stores final results but not the reasoning:

- `drrp_history`: captures what each tier predicted, but not why
- `classification.family`: records which family (Governed/Government) but not which tier number (1-5) or sub-pattern
- `taxa_confidence`: a single float with no breakdown of contributing factors
- Purpose gate decisions: silently skip provisions with no record of which gate fired
- Classifier transitions: threshold decisions (0.7/0.9) applied but reasoning not persisted

### The optimisation framing

The pipeline is a **tier-promotion optimisation problem**: minimise LLM calls while maximising accuracy. Each tier's job is binary — accept or elevate. The current accept/elevate signals:

| Tier | Accept | Elevate | Traceable? |
|------|--------|---------|------------|
| Regex | Confident span match | No match, low conf, ambiguous modals | Partial (conf stored, not why) |
| Classifier | Gap fill ≥0.7 | Below threshold, both-modals, disagreement | Partial (prediction stored, not reasoning) |
| LLM | Terminal | — | N/A |

### What a transparent parsing journey would look like

A `decision_trail` per provision — a structured log of every gate, match, rejection, and promotion decision. Key elements:

1. **Gate reason**: which purpose gate fired (Amendment/Interpretation/DescriptiveSummary/etc.)
2. **Tier matched**: explicit tier number (1=governed_v2, 2=gov_v1, 3=gov_v2, 4=offence, 5=rule) + sub-pattern index
3. **Confidence breakdown**: base confidence, span bonus, window penalty, subordinate clause penalty — not just the final score
4. **Rejection log**: legal fiction detected, epistemic "may" rejected, subordinate clause rejected
5. **Classifier reasoning**: predicted class + confidence + transition rule applied ("gap_fill_accepted", "disagreement_pending_llm", "both_modals_flagged")

This could be a JSON string column in LanceDB (like `drrp_history`) or an opt-in log file.

### Simplification opportunities

The complexity lives mainly in `parse_v2` (75 branches). The tier cascade in `duty_type::classify` is 5 tiers × multiple sub-patterns = 31 branches. But the actual decision is simpler than the code suggests:

1. Is there an actor near a modal? → Which actor, which modal, which sub-pattern?
2. Is the modal enabling or obligatory?
3. Is the actor governed or government?

These 3 questions determine the DRRP type. The complexity comes from the many ways to detect each signal (v2 anchored, v1 keyword, extended window, special patterns). A refactor could separate **signal detection** (find all actor-modal pairs with positions) from **decision logic** (given signals, pick the best classification). Currently these are interleaved — each tier both detects and decides.

## Traceability investigation (2026-06-22, reopened)

Generated full trace (`data/benchmark_trace.json`, 18,382 provisions) using the new `--trace` flag from the pipeline traceability refactor.

### Liberty → none (68 regex-only) — trace reveals 3 root causes

| Decision reason | Count | Root cause |
|---|---|---|
| no_signals | 37 | No regex tier matches — no recognised actor near a modal |
| purpose_gated | 18 | Offence/Repeal/other gate fires, gold disagrees |
| legal_fiction | 13 | "Nothing in X shall..." / "shall be treated as" rejected |

### Legal fiction over-rejection — fix applied (`7b74179`)

13 provisions rejected as legal fiction were gold=Liberty. Three groups:
- **"Nothing in... taken to compel"** (2) — immunity from compulsion. Fixed with `IMMUNITY_RE` exemption.
- **"shall not affect... entitlement"** (1) — preservation of rights. Fixed with `IMMUNITY_RE`.
- **"shall be treated/deemed as"** (7) — beneficial deeming. Genuinely borderline: the language IS a legal fiction, but gold reads the benefit to the employee/person as Liberty. Not fixed — gold standard judgment call.
- **"shall not apply" / "shall be construed as"** (3) — varied patterns. Not fixed.

Impact: 84.8% → **84.9%** (regex-only benchmark, 2 provisions recovered).

### Liberty → Obligation (40) — trace reveals dominant patterns

| Winning tier | Count | Pattern |
|---|---|---|
| GovernmentV1 | 27 | Blunt gate: gov actor + obligation modal wins, but gold says Liberty |
| GovernedV2 | 13 | Actor-anchored obligation pattern matched before enabling |

| Sub-type | Count |
|---|---|
| Prescriptive | 31 | Generic "actor + shall/must" fallback — obligation modal fires first |
| Prohibitive | 6 |
| Enforcement/RegMaking/CodeApproval | 3 |

The dominant issue: **31/40 are Prescriptive** — the text has both obligation AND enabling language, but "shall/must" appears before "may" so the obligation pattern wins. The `apply_modal_context` fix helped Government patterns, but GovernedV2's sub-type ordering (Prescriptive at idx 6 before Enabling at idx 7) means obligation wins whenever both modals are present.

## Remaining issues

- **37 no_signals**: No regex signal at all — need classifier (handled) or LLM
- **18 purpose_gated**: Gold disagrees with the gate — enforcement/offence provisions that contain Liberty powers
- **31 Prescriptive wins over Enabling**: Both modals present, obligation fires first. Needs either sub-type reordering or a "both-modals → enabling" heuristic for provisions where the primary verb is "may"
- **Classifier threshold tuning**: 0.7 gap-fill is aggressive — causes 125 none→Liberty false positives

## Key files

- `fractalaw-core/src/taxa/mod.rs` — `is_legal_fiction()`, `IMMUNITY_RE`
- `fractalaw-core/src/taxa/duty_patterns.rs` — `apply_modal_context()`, `first_modal_is_enabling()`
- `fractalaw-core/src/taxa/duty_patterns_v2.rs` — governed actor-anchored patterns, SUB_TYPE_PATTERNS order
- `fractalaw-core/src/taxa/duty_type.rs` — integration tests
- `data/benchmark_trace.json` — full trace for 18,382 provisions
- `scripts/benchmark_report.py` — benchmark runner

## Key files

- `fractalaw-core/src/taxa/duty_patterns.rs` — `apply_modal_context()`, `first_modal_is_enabling()`
- `fractalaw-core/src/taxa/duty_patterns_v2.rs` — governed actor-anchored patterns
- `fractalaw-core/src/taxa/duty_type.rs` — tier cascade, integration tests
- `fractalaw-core/src/taxa/mod.rs:110-242` — parse_v2, purpose gates, actor positions
- `fractalaw-cli/src/main.rs:4863-5438` — classify pass, transition rules, thresholds
- `scripts/benchmark_report.py` — benchmark runner
