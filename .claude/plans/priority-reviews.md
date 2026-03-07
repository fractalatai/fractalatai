# Priority Reviews

Tracking how issue priorities shift as the project evolves.

## 2026-03-07 — Post #8 Commencement Status Denormalization

Context: #8 implemented in two phases. Phase 1 (5b93360): backfill annotation totals — LEFT JOIN `annotation_totals.parquet` into `export_legislation.sql`, 128 laws now have `total_text_amendments`/`total_modifications`/`total_commencements`/`total_extents`. Phase 2 (e6b7579): derive `commencement_status` from `law_edges` commencement edges — 1,855 fully_commenced, 329 not_commenced, 83 partially_commenced. SCHEMA.md updated to v0.8 (6ab39cd). Filed #29 (commencement_date follow-up).

### What shifted

- **#8 substantially complete** — both annotation totals and semantic commencement status are on the LRT. Remaining work (#29 commencement_date) is incremental.
- **#25 rises to P1** — Zenoh WAN sync is the next infrastructure target. All LRT enrichment work (#7 fitness, #8 commencement, #16 Rule) is now done.
- **#29 (NEW)** — commencement_date column, filed as follow-up to #8. Low priority.

### Priority order (post #8)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#25** — Zenoh WAN sync | Medium | Production infrastructure; all LRT enrichment complete |
| — | #29 | Low | Commencement date extraction (follow-up to #8) |
| — | #27 | Ongoing | Vocabulary gaps tracker (12 gaps remaining) |
| — | #28 | Medium | Intra-law cross-ref resolution (66 cross-refs) |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#8** — Commencement status denormalization (5b93360 + e6b7579, 2,267 laws with status)
- **#16** — Rule classification + actor back-linking (b429f17)

---

## 2026-03-07 — Post #16 Rule Classification + Actor Back-Linking

Context: #16 implemented and closed (b429f17). Phase 1: Rule detection via thing-subject + modal matcher (45 keywords, 80-char window, person negative guard). Phase 2: actor back-linking infers duty holder from dominant governed actor. Phase 3: LanceDB fitness column migration (7 `List<Utf8>` columns added via `add_columns()` API). CDM 2015: 26 Rule provisions detected, DRRP provisions 63→89 (+41%). 361 tests pass.

### What shifted

- **#16 closed** — Rule classification captures ~8% of obligation-bearing provisions that were invisible. Actor back-linking provides useful duty holder inference. End-to-end enrichment verified.
- **DRRP pipeline is now feature-complete** — all 5 classification types (Duty, Right, Responsibility, Power, Rule) implemented with actor extraction.
- **#8 rises to P1** — commencement status denormalization is the next LRT enrichment target. However, the Feb 19 comment on #8 notes it's blocked on LAT schema cleanup (section_id collision, annotation ID uniqueness).
- **#25 unchanged** — Zenoh WAN sync remains production infrastructure work.

### Priority order (post #16)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#8** — Denormalize commencement status onto LRT | Medium | Next LRT enrichment; blocked on LAT schema cleanup |
| 2 | **#25** — Zenoh WAN sync | Medium | Production infrastructure, not urgent for dev |
| — | #27 | Ongoing | Vocabulary gaps tracker (12 gaps remaining) |
| — | #28 | Medium | Intra-law cross-ref resolution (66 cross-refs) |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#16** — Rule classification + actor back-linking (b429f17, 26 Rule provisions in CDM 2015, +41% DRRP coverage)

---

## 2026-03-07 — Post #26 APPLICATION_SCOPE Priority Bug Fix

Context: #26 fixed (e5e141c). Both `parse_v2()` code paths now use `purposes.contains()` instead of `purposes.first()`. 95 provisions that were silently skipped now get fitness extraction. Polarity% 81.4→99.0%, Tagged% 58.5→79.6%, no-polarity 99→4. Also fixed per-provision audit counting bug.

### What shifted

- **#26 closed** — the single biggest fitness coverage improvement: 95 provisions unlocked. Remaining 4 no-polarity provisions are genuinely unparseable (no applicability vocabulary).
- **Fitness pipeline is now mature** — 99% polarity detection, 79.6% tagged coverage for OH&S. Remaining work is incremental (#27 vocabulary gaps, #28 intra-law resolution).
- **#16, #8, #25 unchanged** — no new evidence to reprioritize

### Priority order (post #26)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| 2 | **#8** — Denormalize commencement status onto LRT | Medium | Useful metadata, no blockers |
| 3 | **#25** — Zenoh WAN sync | Medium | Production infrastructure, not urgent for dev |
| — | #27 | Ongoing | Vocabulary gaps tracker (12 gaps remaining) |
| — | #28 | Medium | Intra-law cross-ref resolution (66 cross-refs) |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#26** — APPLICATION_SCOPE priority bug (e5e141c, Polarity% 81.4→99.0%)

---

## 2026-03-07 — Post #22 Cross-Reference Resolution (Phases 1 & 2)

Context: #22 Phase 1 (cross-reference detection, e443556) and Phase 2 (dictionary expansion + plural fixes, b6f05cf) implemented. Gaps reduced 94→10 (Phase 1 separated 63 cross-refs; Phase 2 resolved 21 vocab gaps via dictionary expansion). Tagged% 52.3→58.5%. Filed #26 (APPLICATION_SCOPE priority bug) as side finding.

### What shifted

- **#22 Phase 1+2 complete** — cross-reference detection separates cross-ref provisions from vocab gaps; dictionary expansion resolves most remaining vocab gaps. 10 residual gaps are genuinely hard edge cases (abstract legal concepts, misclassified family). Phase 3 (intra-law resolution) deferred.
- **#26 (NEW: APPLICATION_SCOPE priority bug)** — 15 provisions have "shall (not) apply" but APPLICATION_SCOPE is not their first purpose, so `parse_v2()` skips fitness extraction. Fix requires priority-aware purpose routing in mod.rs.
- **#16, #8, #25 unchanged** — no new evidence to reprioritize

### Priority order (post #22)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#26** — APPLICATION_SCOPE priority bug | Low-Med | 15 provisions with valid "shall apply" get zero fitness_rules; fix in mod.rs |
| 2 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| 3 | **#8** — Denormalize commencement status onto LRT | Medium | Useful metadata, no blockers |
| 4 | **#25** — Zenoh WAN sync | Medium | Production infrastructure, not urgent for dev |
| — | #22 Phase 3 | Medium | Intra-law cross-ref resolution; deferred, 57 cross-refs addressable later |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#22 Phase 1** — Cross-reference detection and audit separation (e443556)
- **#22 Phase 2** — Dictionary expansion, plural fixes, gaps 31→10 (b6f05cf)
- **#26 filed** — APPLICATION_SCOPE priority bug (side finding from #22 investigation)

---

## 2026-03-07 — Post #24 Purpose Classifier Investigation

Context: #24 investigated and closed as "not planned". The APPLICATION_SCOPE regex in `purpose.rs` is universal — not OH&S-biased. Non-OH&S families (FOOD, TRANSPORT: Maritime Safety) returned zero APPLICATION_SCOPE provisions because their LAT text hasn't been synced to LanceDB from sertantai, not because the classifier fails on them. CLIMATE CHANGE (non-OH&S, with LAT data) shows 150 APPLICATION_SCOPE provisions with 90% polarity, proving the classifier works cross-family.

### What shifted

- **#24 closed as "not planned"** — the purpose classifier is universal; the real blocker for non-OH&S fitness is LAT population (syncing full-text law data from sertantai)
- **LAT population is the true bottleneck** — families like FOOD and Maritime have DuckDB metadata but no provision text in LanceDB. Once sertantai syncs their text, the existing classifier + dictionary architecture will work immediately.
- **#22, #16, #8, #25 unchanged** — no new evidence to reprioritize

### Priority order (post #24)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#22** — Cross-reference resolution | Medium | Valid but lower impact at 83% coverage |
| 2 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| 3 | **#8** — Denormalize commencement status onto LRT | Medium | Useful metadata, no blockers |
| 4 | **#25** — Zenoh WAN sync | Medium | Production infrastructure, not urgent for dev |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#24** — Purpose classifier investigation (closed as "not planned" — classifier is universal, blocker is LAT population)

---

## 2026-03-07 — Post #23 P-Dimension Dictionary Expansion

Context: #23 implemented and closed (967adfd + 66cadcd). Two-tier dictionary architecture (Option A: runtime family lookup) with OH&S specialist dicts (19 terms). Gaps reduced 118→94 (tagged% 46.2→52.3%). Audit tooling (`taxa audit-fitness`) and runbook (`docs/FITNESS-DICTIONARY-RUNBOOK.md`) in place.

### What shifted

- **#23 complete** — dictionary architecture is extensible; adding a new family specialist is ~30 lines + a branch in `specialist_dicts_for()`
- **#24 (NEW: purpose classifier for non-OH&S)** — discovered that non-OH&S families get zero APPLICATION_SCOPE classifications, blocking fitness expansion beyond OH&S. This is the next prerequisite before dictionary work can help other families.
- **#25 (NEW: Zenoh WAN sync)** — current sync is LAN-only; production deployment needs WAN connectivity with auth/TLS
- **#8 (commencement status) unchanged** — still valid, no new evidence
- **#22 and #16 unchanged** — cross-ref and Rule classification remain medium priority

### Priority order (post #23)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#24** — Extend purpose classifier beyond OH&S | Medium | Unblocks fitness for all families; #23 architecture is ready |
| 2 | **#22** — Cross-reference resolution | Medium | Valid but lower impact at 83% coverage |
| 3 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| 4 | **#8** — Denormalize commencement status onto LRT | Medium | Useful metadata, no blockers |
| 5 | **#25** — Zenoh WAN sync | Medium | Production infrastructure, not urgent for dev |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#23** — P-dimension dictionary expansion (967adfd + 66cadcd, all 4 phases complete)

---

## 2026-03-05 — Post #17 Gap Investigation

Context: #21 (taxa hash) and #17 (enrichment gap) both closed. The 193-law gap was mostly bugs (panic in clause_structure, duty_holder-only check), not pattern coverage. True enrichment coverage is 83% (384/464), not 60%.

### What shifted

- **#22 (cross-ref resolution) and #23 (p-dimension dictionaries) dropped in urgency** — the main gap was bugs, not pattern coverage
- **#15 (Taxa QA report) rose** — the session proved that ad-hoc validation catches real bugs; needs to be a first-class CLI command
- **#7 (denormalize fitness onto LRT) became actionable** — fitness extraction is implemented in core, ready to ship to sertantai

### Priority order (post #17)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#15** — Taxa QA report | Medium | Validation tooling before further pattern work |
| 2 | **#7** — Denormalize fitness/scope onto LRT | Medium | Ships fitness work to hot path |
| 3 | **#23** — Expand p-dimension dictionaries | Low-Med | Improves fitness quality |
| 4 | **#22** — Cross-reference resolution | Medium | Valid but lower impact at 83% coverage |
| 5 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#21** — Taxa hash change tracking (implemented 2026-03-01, closed 2026-03-05)
- **#17** — Enrichment gap investigation (2 bugs fixed, gap resolved from 193 → 33 genuine + 47 missing LRT)

---

## 2026-03-05 — Post #15 Taxa QA Report

Context: #15 implemented and closed. `fractalaw taxa qa` now provides live re-classification with 4-section validation report (Coverage Summary, Purpose Distribution, Gate Analysis, Anomalies). Filters by `--laws` or `--family`.

### What shifted

- **#15 complete** — validation tooling is now first-class; future pattern changes can be QA'd immediately
- **#7 (denormalize fitness onto LRT) rises to #1** — fitness extraction is in core, ready to ship to sertantai via the publish pipeline
- **#23 and #22 unchanged** — no new evidence to reprioritize

### Priority order (post #15)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#7** — Denormalize fitness/scope onto LRT | Medium | Ships fitness work to hot path; QA tooling now available to validate |
| 2 | **#23** — Expand p-dimension dictionaries | Low-Med | Improves fitness quality |
| 3 | **#22** — Cross-reference resolution | Medium | Valid but lower impact at 83% coverage |
| 4 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#15** — Taxa QA report (b658113, closed 2026-03-05)

---

## 2026-03-05 — Post #7 Fitness Denormalization

Context: #7 implemented and pushed (9eda38e). Fitness/applicability data now flows end-to-end: LAT (7 per-provision columns) → LRT (6 tag + 1 detail column) → taxa hash → sync publish (12→19 columns). SCHEMA.md updated to v0.7. Sertantai schema extension tracked in [sertantai-legal#39](https://github.com/shotleybuilder/sertantai-legal/issues/39).

### What shifted

- **#7 complete** — fitness data is on the hot path and publishable; sertantai needs schema migration (sertantai-legal#39) before the data lands
- **#23 (expand p-dimension dictionaries) rises to #1** — now that fitness flows end-to-end, improving dictionary coverage directly improves published data quality
- **#22 and #16 unchanged** — no new evidence to reprioritize

### Priority order (post #7)

| Priority | Issue | Effort | Rationale |
|----------|-------|--------|-----------|
| 1 | **#23** — Expand p-dimension dictionaries | Low-Med | Directly improves fitness quality now that it ships end-to-end |
| 2 | **#22** — Cross-reference resolution | Medium | Valid but lower impact at 83% coverage |
| 3 | **#16** — Add 'Rule' classification | Medium | Informed by #17 findings |
| — | #18, #19 | High | Blocked on Phase C architecture |
| — | #14, #12, #10, #6, #5, #4, #2, #1 | High | Future / large scope |

### Closed this session

- **#7** — Denormalize fitness/scope onto LRT hot path (9eda38e, sertantai-legal#39 for downstream schema)
