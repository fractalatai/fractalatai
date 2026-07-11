---
session: Compliance Controls — LLM-Assisted Control Generation Design
status: closed
opened: 2026-07-10
closed: 2026-07-10
outcome: success

summary: >
  Designed a four-phase pipeline for generating L3 Controls from L1 Legal Obligations
  using an LLM. Iterated from v0.1 through two Gemini Pro critical reviews to reach v0.2,
  resolving all major architectural concerns. Introduced the policy predicate concept and
  the last_touched/staleness model for Expected Loss calculation.

decisions:
  - what: Controls written in indicative mood, not imperative
    why: >
      The ought-is brief shows that deontic verbs (must/shall) are structurally uncheckable.
      An indicative statement is simultaneously the standard and the test. The gap between
      the two readings is the compliance signal.
    result: Design constraints 1-4 encode this; Phase 2 validation catches deontic drift

  - what: Policy predicate replaces overarching control
    why: >
      Gemini proposed multi-theme controls per structural Part (5-7 for HSWA). Pushed back —
      most laws serve one domain. The overarching control is not an operational control but
      the law's "big idea" as a checkable proposition. Grounded in the Explanatory Note
      (written by the drafting lawyers themselves).
    result: One policy predicate per law, multi-domain regs flagged as exception

  - what: Four-phase pipeline (Generate → Validate → Consolidate → Review)
    why: >
      v0.1 single-step pipeline had no validation, no consolidation algorithm, and no
      human review workflow. Gemini critique identified these as critical gaps.
    result: Each phase has specified inputs, outputs, and algorithms

  - what: Canonical generation (once per corpus) not per-customer
    why: >
      ~2K laws in corpus. Generate canonical controls once, reuse across all customers.
      Reconciliation with customer's existing controls is a separate Mode 2, on request only.
    result: ~6-7K LLM calls for full corpus, amortised across all customers

  - what: LLM estimates Info_Distance, Blast_Radius, Expected_Touch_Frequency as defaults
    why: >
      Gemini said remove as "actively harmful false precision." Pushed back — obligation text
      strongly implies Info_Distance ("employer shall ensure" = Mediated vs "every person shall"
      = Direct). Framed as AI-suggested defaults the customer overrides. Gemini withdrew
      objection on v0.2 review.
    result: Three LLM-estimated fields clearly labelled as defaults, feed Expected Loss calc

  - what: last_touched is runtime (tracked), not LLM-estimated
    why: >
      last_touched is the last time anyone exercised the control — including the operator
      simply using it. A machine guard is "touched" every time the machine runs. This is
      a runtime property from L4 evidence flow, not something the LLM can estimate.
      expected_touch_frequency (LLM-estimated) sets the threshold; staleness is the gap.
    result: Three-concept model — expected_touch_frequency (estimated), last_touched (tracked), staleness (computed)

  - what: Consolidation algorithm specified as embed → HDBSCAN → LLM synthesis
    why: >
      v0.1 said "consolidate where the mechanism is the same" with no algorithm.
      Gemini demanded a concrete, automatable process. Uses existing 384-dim embedding
      infrastructure.
    result: HDBSCAN with min_cluster_size=2, fallback to cosine threshold for small sets

  - what: QA via empirical iteration, not golden dataset
    why: >
      Human compliance officers write in imperatives — you cannot benchmark the new
      indicative form against reference controls written in the old form.
    result: QA from Phase 2 constraint-check pass rates + Phase 4 customer edit rates

metrics:
  corpus_scale: { total_laws: ~2000, single_call: ~1900, chunked: ~100, total_phase1_calls: ~2150 }
  total_pipeline_calls: { estimate: "6000-7000", phases: "P1 Pro + P2 Flash + P3 Pro + predicate" }
  gemini_reviews: { count: 2, model: "gemini-2.5-pro", v01_concerns: 8, v02_concerns_remaining: 4 }

lessons:
  - title: Gemini multi-theme proposal was over-engineering driven by a bad example
    detail: >
      Gemini argued HSWA needs 5-7 theme controls because it's a "framework act" covering
      multiple domains. Wrong — HSWA is one domain (occupational safety), and the governed-only
      filter already excludes the administrative machinery (ss.9-82). The key sections (ss.2-8)
      all serve the same goal. The critique was driven by looking at HSWA's total structure
      rather than its governed provisions. Domain expertise trumped the reviewer's pattern matching.
    tag: methodology

  - title: Explanatory Notes are a first-class LLM input for policy predicates
    detail: >
      UK SI Explanatory Notes are written by the drafting lawyers to explain the instrument's
      purpose in plain language. They typically open with "These Regulations impose requirements
      with respect to..." — a direct statement of legislative intent. This is exactly what the
      LLM needs to produce the policy predicate. Sertantai will scrape these into DuckDB before
      implementation.
    tag: data

  - title: LLMs may outperform humans on indicative-mood control writing
    detail: >
      The worked Confined Spaces example produced controls that are indicative, checkable,
      honest about judgement limits, and hold the full regulatory context simultaneously.
      A human compliance officer would need to fight their training to avoid "the employer
      must ensure..." — the industry default is imperative. An LLM given the right constraints
      can adopt the new form more consistently than a human steeped in the old one.
    tag: methodology

  - title: Push back on LLM reviewer recommendations with domain evidence
    detail: >
      Gemini's v0.1 review recommended removing LLM-estimated Info_Distance as "actively
      harmful." On v0.2, after seeing the argument that obligation text encodes organisational
      distance ("employer shall ensure" vs "every person shall"), Gemini withdrew the objection
      entirely. Reviewers (human or AI) default to conservative "remove it" when they don't
      see the signal. Domain-specific evidence changes the assessment.
    tag: methodology

  - title: last_touched is exercising the control, not verifying it
    detail: >
      Critical distinction. A machine operator using a guarded machine IS touching the guard
      control. A person entering a confined space through a permit system IS touching the
      permit control. last_touched is a runtime property from L4 evidence flow, not a formal
      2nd-line verification event. This feeds staleness, which feeds Expected Loss. The
      implementation challenge (how L4 artefacts automatically update L3 last_touched) is
      flagged as the biggest remaining ambiguity.
    tag: architecture

artifacts:
  - .claude/plans/compliance-controls/COMPLIANCE-CONTROLS.md
  - data/code-review/compliance-controls-design-review.md
  - data/code-review/compliance-controls-v02-review.md

depends_on: []

enables:
  - Explanatory Note scraping in sertantai (prerequisite for policy predicate quality)
  - L3 Controls implementation in fractalaw-cli (controls generate command)
  - L4 Evidence last_touched mechanism design
  - Mode 2 Reconciliation prompt design
---

# Session: Compliance Controls — LLM-Assisted Control Generation Design (CLOSED)

## Goal

Design a system for creating L3 Controls from L1 Legal Obligations using an LLM, following the indicative-mood design philosophy from the ought-is brief and the control ontology from the compliance architecture.

## Work Completed

- ✅ Read and synthesised key documents: compliance 7-layer architecture, Baserow layer status, L1-L6 QQ briefs, controls design doc, ought-is brief, legible vs load-bearing, definition of evidence
- ✅ Explored fractalaw data structures (DuckDB LRT, Postgres provisions, DRRP types, actors, fitness dimensions) to determine LLM prompt inputs
- ✅ Wrote v0.1 design: single-step pipeline, 7 design constraints, worked Confined Spaces example
- ✅ Gemini 2.5 Pro review of v0.1: 8 architectural concerns, 6 open question responses
- ✅ Revised to v0.2 incorporating Gemini feedback: four-phase pipeline, validation, consolidation algorithm, three-way merge, feedback capture
- ✅ Introduced policy predicate concept (replacing multi-theme overarching controls)
- ✅ Introduced last_touched / expected_touch_frequency / staleness model
- ✅ Gemini 2.5 Pro review of v0.2: all original concerns resolved, 4 second-order concerns remaining
- ✅ Updated batch strategy to canonical generation model (~2K laws, ~6-7K calls)
- ✅ Updated QA approach (empirical, not golden dataset)
