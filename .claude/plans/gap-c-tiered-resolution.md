# Gap C Tiered Resolution — Session Coordination

**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (v0.4)
**Created**: 2026-06-05
**Status**: Active — ready to begin Phase 1A

## Goal

Resolve ~3,200 Gap C provisions (implicit duty holders) via a four-tier pipeline: regex → deterministic inheritance → deterministic cross-ref → LLM reasoning. Each phase is an independent session with clear exit criteria and hand-off artifacts.

## Session Sequence

```
Phase 1A ──► Milestone 1 ──► Phase 1B ──► Phase 1C
                                              │
                                              ▼
                                          Phase 2A ──► Phase 2B
                                                           │
                                                           ▼
                                                       Phase 3 ──► Phase 4
```

## Sessions

### Phase 1A — Deterministic parent inheritance (Tier 1) — COMPLETE

**Status**: Done (2026-06-05, commit `39076f6`)

**Results** (299-law customer corpus):

| Metric | Value |
|--------|-------|
| Regex-extracted | 63,260 provisions |
| Tier 1 inherited | 8,648 provisions |
| Total with DRRP | 71,908 provisions |
| Tier 1 uplift | +13.7% more DRRP provisions |
| Laws with inheritance | 141 of 274 making laws |

Ancestor distance distribution:
- Distance 1 (immediate parent): 74.1%
- Distance 2: 4.1%
- Distance 3-5: 17.3%
- Distance 0 (same level): 4.5%

Published to sertantai: 88 law-level + 14,593 provision-level.

**Hand-off artifacts**:
- Parent-chain resolver function (reused by all subsequent phases)
- Tier 1 metrics (informs Milestone 1 decision)
- Schema columns landed in LanceDB + DuckDB

---

### Milestone 1 — Deterministic baseline assessment — PASSED

8,648 inherited provisions is far beyond the 55% threshold (original Gap C estimate was ~3,275 — Tier 1 alone found 2.6x that). The higher number reflects that the original estimate only counted OHS corpus; the full customer corpus has more deeply-nested SIs.

**Decisions**:
- Tier 3 token budget: freeze at Level 2 max context (Tier 1 handles the bulk)
- Ancestor distance: 74.1% at distance 1, confirming the nearest-parent assumption
- Distance 0 entries (388): investigate in Phase 1B — may be siblings inheriting from each other
- Precision: not yet formally measured (needs manual QA sample) — defer to Phase 2A

**Parked**: Re-parse laws whose LAT was pruned (making classifier may now catch them with expanded actors/APPLICATION_SCOPE). Corpus refresh task, not pipeline work.

---

### Phase 1B — Cross-reference resolver (Tier 2)

**Scope**: Citation resolver (intra-document only), fetch cited provisions, propagate actors.

**Depends on**: Phase 1A (shares context assembly infrastructure).

**Work**:
- Citation resolver: regex citation → section_id mapping within same law
- Always fetch Interpretation section as context (addresses the "Purpose Gate Leak")
- One-hop recursive resolution: follow one citation chain, then bail to Tier 3
- Range expansion: "sections 2 to 4" → `["s.2", "s.3", "s.4"]` before query
- `conflicting_actors` column for Tier 1/2 disagreements
- Conflict resolution precedence: cross-ref > parent inheritance

**Exit criteria**:
- >80% precision on C4 provisions with intra-document citations
- Measured: cross-ref resolved / unresolved counts
- External citation failures classified as `unresolved_external_reference`

**Hand-off artifacts**:
- Citation resolver function (reused by Phase 2B for cross-document)
- Unresolved provision list (input for Phase 1C)

---

### Phase 1C — LLM reasoning proof of concept (Tier 3)

**Scope**: Anthropic API integration, structured prompt, externally-derived confidence. Test on provisions that Tiers 1-2 could not resolve.

**Depends on**: Phase 1B (needs unresolved provisions to test against).

**Work**:
- Anthropic API client in fractalaw (Claude Sonnet / Haiku)
- Structured prompt with: structural tags, valid actor enum (from actors.rs), evidence_sections requirement
- Model routing: Haiku for Level 1-2 context (<800 tokens), Sonnet for Level 3-4 (≥800)
- Externally-derived confidence (signal strength, not self-reported)
- Tiered context expansion: Level 1 (parent only) → Level 4 (+ general duty)
- Test on 50 hand-picked C6 provisions

**Exit criteria**:
- >70% precision on C6 provisions
- Understood failure modes — what does the LLM get wrong?
- Cost per provision measured against budget

**Hand-off artifacts**:
- Working Tier 3 integration
- Prompt template (tuned)
- Precision/cost metrics

---

### Phase 2A — Single-law integration

**Scope**: Wire all three tiers into production enrichment for one law.

**Depends on**: Phases 1A + 1B + 1C proven individually.

**Work**:
- Full integration in `enrich_single_law()`: regex → Tier 1 → Tier 2 → Tier 3
- DuckDB aggregate columns: `inherited_count`, `cross_ref_count`, `agentic_count`
- QA report shows per-tier breakdown
- Full enrichment of HSWA with all tiers

**Exit criteria**:
- HSWA DRRP rate improves by >15 percentage points
- No regressions on existing regex-enriched provisions
- Tier-level metrics visible in QA output

---

### Phase 2B — Cross-document citations + EU support

**Scope**: Cross-document citation resolver, EU Directive dual extraction.

**Depends on**: Phase 2A (stable integration point).

**Work**:
- Explicit external citations: "section 2 of the Health and Safety at Work etc. Act 1974"
- Interpretation section → external law name mapping
- EU dual extraction: two records per provision (primary + delegated), `obligation_layer` field
- Test on REACH cross-references and Framework Directive delegated duties
- Implicit external ("section 2 of the Act") — assess volume, defer if <3% of Gap C

**Exit criteria**:
- EU corpus Gap C resolved at comparable rates to UK
- External citation resolved / unresolved metrics

---

### Phase 3 — Corpus-wide rollout

**Scope**: Process all remaining Gap C provisions, publish to sertantai.

**Depends on**: Phase 2B.

**Work**:
- Batch process ~3,275 UK + ~2,000 EU Gap C provisions
- Tune confidence thresholds
- Promote `--gap-c` to default
- Publish enriched provisions (law-level + provision-level) to sertantai

**Exit criteria**:
- Corpus-wide QA report, false-positive rate <5%
- All tier metrics published

---

### Phase 4 — Automation

**Scope**: Wire Gap C resolution into `sync watch` automated pipeline.

**Depends on**: Phase 3.

**Work**:
- Gap C resolution runs automatically after regex enrichment, before publish
- Rate limiting and error handling for Tier 3 API calls
- Cost monitoring and alerting
- Tiered context expansion (minimal first, expand if unresolved)

**Exit criteria**:
- New laws automatically get Gap C resolution
- Monthly API cost within budget (~$0.50/month)

## Key Decisions Log

| Decision | Status | Phase |
|----------|--------|-------|
| Four-tier pipeline (not classifier) | Approved | Design |
| Precision > recall for Tier 1 | Approved | Design |
| Externally-derived confidence | Approved | Design |
| EU dual extraction (Option A, two records) | Approved | Design |
| obligation_layer field | Approved | Design |
| Cross-ref > parent inheritance precedence | Approved | Design |
| Deepest-first parent walk | Approved | Design |
| Haiku for Level 1-2, Sonnet for Level 3-4 | Approved | Design |
| Implicit external citations ("the Act") | Deferred | Phase 3+ |
| Structured provenance (List<Struct>) | Deferred | Post Phase 1A |
