# Priority Reviews

Tracking how issue priorities shift as the project evolves.

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
