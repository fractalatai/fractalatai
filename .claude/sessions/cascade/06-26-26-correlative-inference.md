# Session: Correlative Actor Inference (CLOSED)

## Problem

~200 gold benchmark actors are implied, not stated in text. The LLM infers them from Hohfeldian correlatives — deterministic legal relationships between actors. These patterns are consistent enough to codify as rules.

## Patterns found

| When regex finds | Infer | Position | Coverage |
|-----------------|-------|----------|----------|
| Employee active (Obligation) | Employer | counterparty | 19/28 (68%) |
| Member State active (EU reg) | Responsible Undertaking | counterparty | 16/22 (73%) |
| Enforcement Authority active | Public | beneficiary | 7/11 (64%) |

## Architecture decision: Option B — `taxa infer` command

Gemini critical review (2026-06-26) unanimously recommends Option B: separate command.

**Why:** Correlative inference is a distinct logical step — not regex, not classifier, not LLM. It creates NEW actors based on relationships between existing actors. Must be:
- Re-runnable independently (rules will evolve)
- Auditable (legal pipeline needs provenance)
- Distinct from reconcile (reconcile picks winners, inference creates data)

**Own tier columns in provision_actors:**
- `inferred_drrp TEXT`
- `inferred_position TEXT`
- Distinct from regex/cls/llm — different provenance, different confidence

**False positive handling (30% rate):**
- Write to `inferred_*` columns as suggestions
- Reconcile flags inferred actors as `pending_llm` for LLM validation
- High-coverage rules (>80%) could bypass LLM in future

**Worth building for 200 actors?** Yes — 200 in benchmarks likely means thousands in full corpus. Rules are deterministic, explainable, and free vs LLM calls.

## Implementation

1. ✅ Add `inferred_drrp`, `inferred_position` columns to provision_actors
2. ✅ PgStore: extend upsert tier + query_provision_actors + clear_inferred_actors
3. ✅ Correlative rules: `docs/correlative-rules.yaml` + `fractalaw-core/src/taxa/correlatives.rs` (3 tests pass)
4. ✅ `cmd_taxa_infer` command in taxa.rs
5. ✅ CLI wiring: `TaxaAction::Infer` + dispatch
6. ✅ Verified: HSWA → 27 actors inferred (26 Public beneficiary, 1 Employer counterparty). Idempotent.
7. ✅ Run infer across all 20 benchmark laws: **615 actors inferred**
   - Ind: Public (beneficiary): 378
   - Org: Responsible Undertaking (counterparty): 223  
   - Org: Employer (counterparty): 14
   - Matched gold actors: 1,743 → 1,758 (+15 directly matched)
   - ✅ benchmark_report.py updated with inferred tier
   - **Inferred position accuracy: 86.7%** (13/15 matched gold actors correct)
   - Ind: 100%, Org: 71.4%

## Dependencies

- ✅ provision_actors table with regex signals
- ✅ Regex actor gaps session closed (1,743 matched actors)
