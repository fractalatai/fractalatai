---
session: Fitness Applicability Regex
status: closed
opened: 2026-07-10
closed: 2026-07-10
outcome: success

summary: >
  Phase 1 of fitness strategy implemented: polarity detection separated from entity extraction,
  ungated from APPLICATION_SCOPE purpose, expanded with legislative self-reference filter and
  three new patterns (subject-to, activity-scope, commencement). Coverage doubled from 5,890
  to ~13,300 provisions. Strategy evolved to v0.3 with three-layer data model and two companion
  design documents (graph propagation, rules engine).

decisions:
  - what: Separate polarity detection from entity extraction in fitness.rs
    why: Polarity is a cheap finding signal that should run on ALL provisions. Entity extraction (P-dimension dictionaries) is expensive and only needed on APPLICATION_SCOPE provisions.
    result: detect_polarity() now public, called directly by pipeline on every provision. 6,140 new provisions detected that were previously gated out.

  - what: Legislative self-reference filter for polarity detection
    why: Without the APPLICATION_SCOPE gate, "apply/applies" in non-legislative contexts ("risks which apply", "employer shall apply") creates false positives. The law must be the subject of "apply".
    result: Dictionary of legislative nouns (Act, Part, section, regulation, etc.) rejects 495 false positives while keeping 6,140 genuine hits.

  - what: Three-layer data model replaces typed entities (v0.2 → v0.3)
    why: Both v0.1 (6 P-dimensions) and v0.2 (6 entity types) force single-type classification at extraction time. "Public authority affecting protected features of an MCZ" is simultaneously personal, material, and territorial scope. Separation of extraction from classification solves this.
    result: Mention (verbatim span) → Entity (canonical identity) → Classification (scope dimensions + domain facets). Reviewed and approved by Gemini Pro.

  - what: Compile/evaluate split between fractalaw and sertantai for rules engine
    why: Rule compilation (mentions → expression tree) is heavy enrichment-time work (Rust). Rule evaluation (tree walk against customer profile) is light query-time work (Elixir). Sertantai is the production app.
    result: Expression tree published as JSON in LRT payload via Zenoh. Evaluator is ~200 lines of Elixir pattern matching.

  - what: Four scope dimensions as grounding ontology
    why: Legal scholarship (ratione personae/materiae/loci/temporis) provides a universal, domain-agnostic classification validated across legal traditions. Domain-specific schemes (SIC, DEFRA, HSE) extend it.
    result: Replaces the ad-hoc P-dimension categories with a principled foundation referenced in Akoma Ntoso, LKIF, LegalRuleML.

metrics:
  polarity_detection:
    before: 5890
    after_core: 12030
    after_expanded: 13327
    false_positives_rejected: 495
    benchmark_key_provisions: { before: "1/8", after: "6/8" }
  new_patterns:
    subject_to: 231
    activity_scope: 26
    commencement: 1047
  tests: { total: 60, passed: 60, failed: 0 }
  strategy_versions: { v01: "reviewed", v02: "reviewed", v03: "reviewed + system review" }
  gemini_reviews: 2

lessons:
  - title: Polarity detection and entity extraction are different concerns with different cost profiles
    detail: Polarity is a cheap regex that should run everywhere. Entity extraction (dictionaries, NER) is expensive and should be gated. Bundling them in one function meant the cheap signal was gated by the expensive one's prerequisites.
    tag: architecture

  - title: False positive filtering via subject dictionary is more robust than tightening the verb pattern
    detail: The verb patterns ("shall apply", "does not apply") are correct — the false positives come from non-legislative subjects. A dictionary of legislative self-reference nouns (Act, Part, section, regulation...) cleanly separates "the law applying" from "an actor applying something".
    tag: methodology

  - title: Classification schemas applied at extraction time create lock-in
    detail: Both the 6-P model and the v0.2 entity types force you to decide the category before you've finished understanding the entity. Separating extraction (what does the text say?) from classification (what does it mean?) allows classification to evolve without re-extraction.
    tag: architecture

  - title: Gemini Pro reviews improve dramatically with companion design documents
    detail: v0.3 strategy alone got "conceptually sound but dangerously underspecified". Adding FITNESS-GRAPH.md and FITNESS-RULES-ENGINE.md addressing the specific gaps got "comprehensive and impressive overhaul, professional-grade strategy, ready for implementation". The review quality depends on the completeness of what you submit.
    tag: methodology

  - title: The rule compiler is the load-bearing component
    detail: Gemini identified that the gap between extracted mentions and compiled expression trees is the hardest unsolved problem. Inferring logical connectives (OR vs AND) between co-occurring mentions requires sentence structure analysis. Design spike needed before implementation.
    tag: architecture

  - title: "Any person" in legislation is a wildcard on personal scope
    detail: Criminal offence provisions using "any person who..." have universal personal scope — they match all customers unless negated by a DisappliesTo clause. This is not a special case; it's the most common applicability pattern.
    tag: data

artifacts:
  - crates/fractalaw-core/src/taxa/fitness.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - .claude/plans/fitness/FITNESS-STRATEGY.md
  - .claude/plans/fitness/FITNESS-GRAPH.md
  - .claude/plans/fitness/FITNESS-RULES-ENGINE.md
  - data/code-review/gemini-fitness-strategy-v03-review.md
  - data/code-review/gemini-fitness-strategy-v03-system-review.md

depends_on:
  - 07-09-26-nature-protection-fitness.md

enables:
  - Phase 2 implementation (three-layer extraction with NER staircase)
  - Phase 3 graph propagation implementation
  - Phase 4 rule compiler design spike
  - Sertantai rules engine evaluator (Phase 5)
  - Re-parse of full corpus to populate new polarity tags
---

# Session: Fitness Applicability Regex (CLOSED)

## Problem

Phase 1 of FITNESS-STRATEGY.md: improve applicability provision detection via regex across the full corpus. The P-dimension dictionaries only cover OH&S and FIRE domains — every other family gets zero fitness tags from the dictionaries. Polarity detection is strong (96%) but the content extraction is OH&S-centric.

The goal is to find applicability patterns in the full LAT (136K+ provisions across 428+ laws) — structural sections (headings containing "application", "scope", "applicability") and context-specific phrases within provisions. The 4 nature protection laws are the benchmark to measure improvement, not the exclusive source of patterns.

Benchmark laws (for measuring improvement):
- **UK_ukpga_1981_69** — Wildlife and Countryside Act 1981
- **UK_ukpga_2009_23** — Marine and Coastal Access Act 2009
- **UK_uksi_2017_1012** — Conservation of Habitats and Species Regs 2017
- **UK_ukpga_2006_16** — NERC Act 2006

## Baseline

Full corpus: 150,393 substantive provisions, 5,968 APPLICATION_SCOPE, 5,890 with polarity (98.7%).
Only 2,223 have P-dim tags (37.3%). OH&S/FIRE: 71-73% P-dim coverage, everything else: 7-33%.
Offence (5,526) and Obligation (43,945) provisions never reach polarity detection — gated by APPLICATION_SCOPE purpose.

## Work

1. ✅ Mine full corpus LAT for applicability patterns — baseline stats across all families
2. ✅ Refactor fitness.rs: separate `detect_polarity()` (public, `Vec<RulePolarity>`) from `extract()` (entity extraction)
3. ✅ Decouple pipeline: `pipeline.rs` calls `detect_polarity()` directly on cleaned text for ALL provisions, not gated by APPLICATION_SCOPE
4. ✅ Add legislative self-reference filter to `detect_polarity()` — "this Act", "these Regulations", "subsection (3)" etc. Rejects false positives where "apply" has a non-legislative subject ("risks which apply", "employer shall apply")
5. ✅ Measure: 5,890 existing → 12,030 total (+6,140 new). 495 false positives rejected by self-ref filter. Benchmark laws roughly double coverage.
6. ✅ Expand polarity patterns: added SUBJECT_TO_RE ("Subject to the provisions of this Part") and ACTIVITY_SCOPE_RE ("it is a licensable marine activity"). 12,030 → 12,280 total. Catches 6/8 key benchmark provisions (was 1/8). Two still missed (reg.43, s.40) are implicit subject-matter — Phase 2 NER territory.
7. ✅ Temporal applicability: added COMMENCEMENT_RE ("comes into force", "shall come into force"). 1,047 commencement provisions were missing polarity. "Ceases to have effect" (sunset) already covered by DISAPPLIES_RE.
8. ✅ Strategy v0.3: three-layer model (mention → entity → classification), scope dimensions as grounding ontology, reviewed by Gemini Pro.
9. ✅ Created FITNESS-RULES-ENGINE.md — two-stage architecture (hierarchical index coarse filter + boolean expression tree evaluation per law), compiled from extracted mentions not hand-authored
10. ✅ Created FITNESS-GRAPH.md — nodes are scope units (law/Part/Chapter/section), edges are structural inheritance + cross-reference overrides + commencement propagation
11. ⏸️ Run full parse on benchmark laws to verify new polarity tags land in Postgres (deferred — next session)
12. ⏸️ Final corpus measurement + commit (deferred — next session)

## Gemini Review Feedback (2026-07-10)

FITNESS-STRATEGY.md v0.3 reviewed by Gemini Pro. Full review: `data/code-review/gemini-fitness-strategy-v03-review.md`.

**Validated:**
- Three-layer separation is correct architecture, not over-engineering — "minimum viable architecture for this problem"
- Mention layer is "non-negotiable" as immutable audit artifact — but warns about one-to-many (multiple mentions per provision) storage model
- Scope dimensions are the foundational ontology, not just an "exemplar" — should be framed as the base schema with domain-specific extensions on top

**Actionable concerns:**
1. **Mention storage**: single provision can have multiple mentions (AppliesTo + DisappliesTo + ExtendsTo). JSON blob on LAT row is suspect — consider a `mentions` table. Already the pattern used for `provision_actors`.
2. **Facets need governance**: who owns the schemas, how are they versioned, what's the process for adding terms? Without this, "flexible facets" becomes tag soup.
3. **Customer matching is naive**: facet intersection won't handle boolean logic (OR conditions), hierarchical matching (SIC 08.11 vs SIC section B), or conditional applicability ("if they handle asbestos"). Needs a rules engine, not set intersection.
4. **Entity resolution is underspecified**: canonical entity URIs, curation, drift over time — replaced a dictionary with an ontology management problem.
5. **Temporal dimension ignored**: listed but never addressed. Commencement/sunset clauses are core compliance.
6. **Phase 3 propagation hand-waved**: "graph traversal later" is not viable — law IS a graph of cross-references.
7. **No feedback loop**: no mechanism for analyst corrections to flow back to model retraining.

**Assessment**: v0.3 is "conceptually sound but dangerously underspecified" — the right layers but the hard problems (propagation, matching, entity governance) remain unaddressed. Good starting point for conversation, not yet a blueprint.

## Dependencies

- ✅ FITNESS-STRATEGY.md v0.2 reviewed and approved (07-09-26-nature-protection-fitness session)
- ✅ 4 benchmark laws fully enriched in Postgres with LAT text
- ✅ fitness.rs module exists with polarity + P-dimension extraction
- ✅ Nature Protection session closed — 428 laws published
