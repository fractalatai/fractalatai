---
session: Tier 0 — Base Case Definition
status: closed
opened: 2026-06-29
closed: 2026-06-29
outcome: success

summary: >
  Defined the pipeline base case: three-category provision scope (OUT/STRUCTURAL/SUBSTANTIVE).
  Implemented as shared filter function, wired into pipeline, cleaned 429 orphan actors.
  QA passing on both QQ corpus and benchmarks.

decisions:
  - what: Three-category scope model (OUT / STRUCTURAL / SUBSTANTIVE)
    why: Binary in/out is too coarse — structural provisions (definitions, amendments) should create actors tagged "mentioned" for discovery but not consume classifier/SLM resources
    result: Gemini validated as correct granularity

  - what: Two-pass filter — section_type first, purpose second
    why: section_type is free metadata from LAT ingest. Purpose requires regex analysis. Filter as early as possible.
    result: Pass 1 removes 16.9% of provisions before any text analysis

  - what: Modal override includes rights + powers, not just obligations
    why: Gemini identified that "entitled to", "may", "has the power to" are DRRP but were missing from the duty-only modal list
    result: has_drrp_modal() covers obligations, rights, and powers

metrics:
  qq_corpus: { total: 174666, out_section_type: 20170, out_short: 9338, in_scope: 145158 }
  out_percentage: 16.9%
  orphan_actors_cleaned: 429
  qa_pass: true

lessons:
  - title: Define the base case before building pipeline stats
    detail: "\"42% classifier coverage\" was meaningless because the denominator included provisions that should never have been classified. Base case defines the denominator. All coverage stats are relative to it."
    tag: methodology

  - title: Scattered filtering is invisible filtering
    detail: The pipeline had skip logic for headings in parse_provisions, purpose gating in should_default_to_mentioned, and text length checks in the embedder — all doing the same job independently. Unifying into provision_scope() makes the decision visible and testable.
    tag: architecture

  - title: LEGAL_FICTION_RE already handles definitional "shall"
    detail: Gemini flagged "'vehicle' shall mean..." as a false positive for modal override. Already handled by existing regex in taxa/mod.rs. Check existing code before adding new rules.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/mod.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - scripts/corpus_stats.py
  - .claude/skills/corpus-stats/SKILL.md

depends_on: []

enables:
  - 06-29-26-tier1-regex
  - 06-29-26-tier2-classifier
  - Meaningful pipeline coverage statistics
---

# Session: Tier 0 — Base Case Definition (CLOSED)

## Problem

The pipeline has no defined base case. Regex parses every provision including schedules, definitions, headings, and cross-references that will never progress past regex tier. This inflates actor counts and makes coverage stats meaningless — "42% classifier coverage" includes actors that should never have been created.

## Scope rules

The question per provision: **can this provision create a legal obligation?**

Two-pass filter using data available at different pipeline stages:

### Pass 1: section_type (available at LAT ingest, no analysis needed)

| section_type | Scope | Rationale |
|-------------|-------|-----------|
| `heading` | **OUT** | Structural label, no legal content |
| `part` | **OUT** | Container title ("Part I General Duties") |
| `chapter` | **OUT** | Container title |
| `schedule` (title only) | **OUT** | Schedule heading, not content |
| `signed` | **OUT** | Signatory block |
| Text < 20 chars | **OUT** | Cross-ref stubs, empty fragments |
| `section`, `sub_section` | **IN** | UK primary legislation — substantive |
| `article`, `sub_article` | **IN** | UK secondary / EU — substantive |
| `regulation`, `sub_regulation` | **IN** | SI regulations — substantive |
| `paragraph`, `sub_paragraph` | **IN** | Could be substantive or schedule content |
| `schedule_part`, `schedule_paragraph` | **IN (conditional)** | Schedule content — may contain duties (e.g. Schedule 1 to HSWA) |

### Pass 2: purpose (requires regex analysis of text)

For provisions that passed section_type filter:

| purpose | Scope | Rationale |
|---------|-------|-----------|
| `Regulatory` | **SUBSTANTIVE** | Creates duties, rights, powers — full pipeline |
| `Unclassified` | **SUBSTANTIVE** | Default — may contain obligations, needs classification |
| `Definitional` | **STRUCTURAL** | Defines terms — actors default to "mentioned" unless obligation modal detected |
| `Amending` | **STRUCTURAL** | Amends another law — actors default to "mentioned" |
| `Commencement` | **STRUCTURAL** | Brings provisions into force — no obligations |
| `Transitional` | **STRUCTURAL** | Transitional arrangements — actors default to "mentioned" |
| `Procedural` | **STRUCTURAL** | Describes process — actors default to "mentioned" unless obligation modal detected |

### Modal override for STRUCTURAL provisions

A provision classified as STRUCTURAL is promoted to SUBSTANTIVE if it contains obligation modals ("shall", "must", "is required to"). This catches cases like definition sections that also impose duties.

### Three categories

| Category | Enters pipeline | Gets actors | Gets embedding | Gets classifier |
|----------|----------------|-------------|---------------|----------------|
| **OUT** (section_type filter) | No | No | No | No |
| **STRUCTURAL** (purpose, no modal) | Yes (discovery only) | Yes, all "mentioned" | No | No |
| **SUBSTANTIVE** (regulatory, or structural + modal) | Yes (full pipeline) | Yes, classified | Yes | Yes |

### Gemini review feedback (2026-06-29)

1. **Modal override should include rights + powers** — "entitled to", "may", "has the power to" are DRRP but not covered by duty-only modals. Add enabling modals to the override.
2. **Definitional "shall" false positive** — "'vehicle' shall mean..." would trigger modal override. **Already handled** — `LEGAL_FICTION_RE` in `taxa/mod.rs` catches "shall be treated/deemed/construed/read" patterns.
3. **Missing purposes: Repealing, Savings** — subsume under existing Amending/Transitional for now.
4. **Three-category model validated** as correct granularity.

Action: expand modal override to include enabling modals (rights + powers), not just obligation modals.

## Work

1. ✅ Define which provisions are "in scope" — three categories (OUT/STRUCTURAL/SUBSTANTIVE), reviewed with Gemini
2. ✅ Implement `provision_scope()` in `fractalaw-core/src/taxa/mod.rs` — two-pass filter (section_type then purpose)
3. ✅ Implement `scripts/corpus_stats.py` — reports per-tier coverage with QA checks
4. ✅ Descriptive stats implemented in `scripts/corpus_stats.py` — Tier 0 reports OUT/IN split + orphan actor QA
5. ✅ Cleaned 429 orphan actors from OUT provisions
6. ✅ Wired `provision_scope()` into `parse_provisions` in pipeline.rs — OUT provisions now skipped before regex parse
7. ✅ QA PASS on both QQ corpus and benchmarks — zero actors on OUT provisions
8. ✅ Created corpus-stats skill (stub, Tier 0 only — expand per tier session)

## QA checks (close signal)

- Every provision is tagged OUT / STRUCTURAL / SUBSTANTIVE
- No OUT provision has actors in provision_actors
- STRUCTURAL provisions have actors all set to "mentioned" (unless modal override)
- SUBSTANTIVE count + STRUCTURAL count + OUT count = total provisions
- Counts are stable across re-runs (idempotent)
