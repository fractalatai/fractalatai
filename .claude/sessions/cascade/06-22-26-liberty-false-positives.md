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

## Next steps

- Remaining Liberty→none (68) and Liberty→Obligation (42) are harder — likely need classifier or new regex patterns for immunity/entitlement language
- Consider whether these warrant further regex work or are better left for the LLM tier

## Key files

- `fractalaw-core/src/taxa/duty_patterns.rs` — `apply_modal_context()`, `first_modal_is_enabling()`
- `fractalaw-core/src/taxa/duty_patterns_v2.rs` — governed actor-anchored patterns (already modal-aware via SUB_TYPE_PATTERNS order)
- `fractalaw-core/src/taxa/duty_type.rs` — integration tests
- `scripts/benchmark_report.py` — benchmark runner
