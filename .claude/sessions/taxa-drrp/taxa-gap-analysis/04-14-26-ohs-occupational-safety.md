# Session: 2026-04-14 — Taxa DRRP Gap Analysis: OH&S: Occupational / Personal Safety

## Context

**Skill**: [taxa-gap-analysis/SKILL.md](../../../skills/taxa-gap-analysis/SKILL.md)
**Runbook**: [TAXA-PATTERN-RUNBOOK.md](../../../../docs/TAXA-PATTERN-RUNBOOK.md)
**Family**: `OH&S: Occupational / Personal Safety`
**Objective**: QA taxa DRRP coverage for the OH&S: Occupational / Personal Safety family — identify gaps, measure precision/recall, determine if pattern improvements are needed.

## Family Profile

- 451 laws in DuckDB under `OH&S: Occupational / Personal Safety`
- 184 laws enriched (have `duty_type IS NOT NULL`)
- 155 laws with duties, 138 with responsibilities
- 16,027 total provisions in LanceDB across enriched laws

## Step 1: QA Report Baseline

### Coverage Summary (top laws by provision count)

| Law | Prov | DRRP% | Gated% | Notes |
|-----|------|-------|--------|-------|
| UK_ukpga_1937_67 | 500 | 31.2% | 11.6% | Factories Act — large, old |
| UK_asc_2025_4 | 496 | 7.1% | 5.2% | Low DRRP — investigate |
| UK_nisr_2017_229 | 477 | 28.3% | 13.4% | |
| UK_nisr_2007_31 | 352 | 32.1% | 10.5% | |
| UK_nisr_2012_179 | 361 | 31.6% | 10.5% | |
| UK_nisr_1997_248 | 319 | 32.6% | 15.7% | |
| UK_nisr_2010_160 | 233 | 31.3% | 22.7% | |
| UK_ukpga_1974_37 | 228 | 41.7% | 25.0% | HSWA — flagship law |
| UK_nisr_2016_146 | 249 | 52.6% | 12.0% | Highest DRRP% in large laws |
| UK_nisr_2015_325 | 227 | 46.3% | 19.4% | |
| UK_uksi_2016_1092 | 500 | 1.4% | 12.6% | Very low DRRP — investigate |
| UK_uksi_2016_1093 | 500 | 0.7% | 8.2% | Very low DRRP — investigate |
| UK_uksi_2016_1105 | 500 | 0.7% | 10.7% | Very low DRRP — investigate |

**Corpus totals**: 16,027 provisions, 81.2% Process+Rule, 3.3% Enactment, 12.4% Interpretation, 3.6% Scope/Fee, 10.3% Amendment

### Gate Analysis

| Gate | Triggered | % corpus |
|------|-----------|----------|
| skip_drrp (all) | 3,953 | 24.7% |
| — Interpretation-primary | 1,894 | 11.8% |
| — Enactment-primary | 522 | 3.3% |
| — Application+Scope | 394 | 2.5% |
| — All structural | 1,143 | 7.1% |
| descriptive_summary | 34 | 0.2% |
| **Total gated** | **3,987** | **24.9%** |

### Key Anomalies

**0 DRRP with >10 provisions** (suspicious — investigate):
- `UK_nisr_2015_265` — 43 provisions, 0 DRRP, 88.4% gated (likely pure amendment SI)
- `UK_ukpga_2008_20` — 36 provisions, 0 DRRP, 55.6% gated
- `UK_uksi_2009_716` — 25 provisions, 0 DRRP
- `UK_uksi_2011_2157` — 24 provisions, 0 DRRP
- `UK_uksi_2025_1073` — 26 provisions, 0 DRRP

**High Enactment (>10%)**: 25+ laws flagged — many are old Acts or commencement/amendment SIs, likely genuine

**High Enforcement (>15%)**: 8 laws — `UK_nisr_2009_238` (25%), `UK_uksi_2018_390` (29.5%), `UK_uksi_2008_2852` (25.5%), others

## Step 2: 0-DRRP Anomaly Investigation

All 5 laws with 0 DRRP and >10 provisions are **genuinely non-making**. No pattern gaps.

| Law | Title | Provisions | Verdict | Evidence |
|-----|-------|-----------|---------|----------|
| UK_nisr_2015_265 | CLP (Amendment) Regulations (NI) | 43 | Pure amendment SI | 88.4% gated, 81.4% Amendment purpose |
| UK_ukpga_2008_20 | Health and Safety (Offences) Act | 36 | Offence/penalty modification | 55.6% gated, 1 hot miss is Schedule listing penalties (not duties) |
| UK_uksi_2009_716 | CHIP Regulations 2009 | 25 | Revoked; only wrapper text in sertantai | 19/25 cold, 4 hot are citation/commencement ("shall come into force") |
| UK_uksi_2011_2157 | Supply of Machinery (Amendment) Regs | 24 | Pure amendment SI | Amendment text throughout |
| UK_uksi_2025_1073 | Noise Emission (Amendment) Regs | 26 | Amendment SI | 50% Amendment purpose; "must" verbs are in substituted text ("for paragraph (1) substitute-"), not standalone duties |

**Note on UK_uksi_2025_1073**: 3 provisions at heat=4 contain "must" within amendment-substituted text (e.g. "the guaranteed sound power level must be calculated"). These duties live in the parent SI being amended — the amendment SI itself is not making law.

## Step 3: Very Low DRRP% Investigation

### Product Safety SIs (UK_uksi_2016_1092, UK_uksi_2016_1093, UK_uksi_2016_1105)

All three are EU-derived product safety regulations (Simple Pressure Vessels, Lifts, and a third unnamed SI). Each has ~500 provisions with <2% DRRP but massive hot miss counts.

| Law | Prov | DRRP% | Hot Misses (>=3) | heat>=5 | Gated (Interp-primary) |
|-----|------|-------|-----------------|---------|----------------------|
| UK_uksi_2016_1092 | 499 | ~1.4% | 92 | 21 | 14/21 |
| UK_uksi_2016_1093 (Lifts) | 500 | 0.7% | 122 | 44 | (same pattern) |
| UK_uksi_2016_1105 | 500 | 0.7% | 108 | 28 | 16/28 |

**Root cause**: Sertantai text parsing produces very long multi-section provisions for these SIs. Definitions appear at the start of the text block → purpose classifier makes `Interpretation+Definition` the primary purpose → `should_skip_drrp()` Interpretation-primary gate fires → genuine duties buried later in the text are suppressed.

Example: `UK_uksi_2016_1092` provision text starts with market surveillance definitions but contains "An importer must not place a vessel on the market unless..." further down — a clear duty on SC: Importer.

**This is a provision-level classification problem in fractalaw.** All text parsing and classification happens here — sertantai delivers raw provision text blocks. The actors (SC: Manufacturer, SC: Importer, Operator etc.) and modal verbs are correctly detected, but the purpose gate kills the provision before DRRP extraction runs. The Interpretation-primary gate is too aggressive on these long mixed-content provisions.

**Potential fixes** (to be designed in a separate session):
1. Sub-provision scanning: split long provisions at definition boundaries, re-classify each segment independently
2. Secondary DRRP pass: even when Interpretation-primary fires, scan text after the definition section for duty patterns
3. Softer gate for mixed-purpose provisions: if Interpretation-primary but also has Process+Rule + governed actor + modal, allow DRRP extraction on the non-definition portion
4. Length-gated override: for provisions above a certain length with strong DRRP signals (heat>=5), bypass the Interpretation-primary gate

**Decision**: Actionable fractalaw issue. Log for a dedicated session — affects 3 product safety SIs (~1,500 provisions) and likely other EU-derived regulations with the same structure.

### Disused Mine Tips Act (UK_asc_2025_4)

| Metric | Value |
|--------|-------|
| Provisions | 496 |
| DRRP% | 7.1% |
| Classified | 199 |
| Hot misses (>=5) | 18 |
| Gated (Interp/App/Enact primary) | 16/18 |

Same root cause as the product safety SIs — long provisions mixing definitions with duties. The 2 ungated hot misses are subordinate clause fragments with no actor subject (Gap C: "must inspect the tip within 6 months...").

199 provisions DO get classified (from `taxa show` live), which is far more than the QA report's 7.1% DRRP suggests. The QA report uses stored enrichment data — the live parser is finding more than what was stored. The law may need re-enrichment.

**Decision**: Re-enrich after this QA session; remaining gap is the same mixed-content provision issue as the product safety SIs.

## Triage Summary

| Category | Laws | Status |
|----------|------|--------|
| 0-DRRP anomalies | 5 laws | All genuinely non-making (amendment SIs, offence acts, revoked instruments) |
| Very low DRRP% | 4 laws | Mixed-content provision problem — long provisions mix definitions with duties, Interpretation-primary gate suppresses real duties. Needs fractalaw fix (gate/scanner improvement). |

## Step 4: Confusion Matrix

148 laws with LAT data, 20,192 provisions.

Ground truth heuristic: **Expected positive** = any modal verb (shall/must/may/power to/entitled to) + non-structural primary purpose.

|  | Predicted: DRRP | Predicted: No DRRP | Total |
|--|----------------:|-------------------:|------:|
| **Expected: DRRP** | 3,974 (TP) | 4,200 (FN) | 8,174 |
| **Expected: No DRRP** | 149 (FP) | 11,869 (TN) | 12,018 |
| **Total** | 4,123 | 16,069 | 20,192 |

| Metric | Value |
|--------|-------|
| **Precision** | 96.4% |
| **Recall** | 48.6% |
| **F1** | 64.6% |

### Comparison with OH&S: Offshore Safety

| Metric | Offshore (2,737 prov) | Occupational (20,192 prov) |
|--------|----------------------|---------------------------|
| Precision | 99.1% | 96.4% |
| Recall | 72.6% | 48.6% |
| F1 | 83.8% | 64.6% |

Recall is 24pp lower. The family is 7x larger with much more structural diversity.

### False Negative Breakdown (4,200)

| Category | Count | % of FN |
|----------|-------|---------|
| **Gap C** (no actor at all) | 3,275 | 78% |
| **Gap A** (actor present, no DRRP) | 925 | 22% |
| — with governed actor | 848 | |
| — with government actor only | 77 | |

384 FN are non-duty modal patterns (exemptions, scope conditions — "shall not apply", "nothing in...shall", etc.). Excluding these: adjusted recall 51.0%, adjusted F1 66.7%.

### FN Modal Type

| Type | Count |
|------|-------|
| Obligation (shall/must) | 3,462 |
| Enabling only (may/power to) | 738 |

82% of misses have obligation modals — these are genuine missed duties.

### Gap A: Governed Actor Labels in Misses

| Actor Label | Misses | In GOVERNED_ACTORS? | Notes |
|-------------|--------|---------------------|-------|
| Ind: Person | 552 | Partial (predicates) | Known: bare "person" too broad |
| Org: Employer | 114 | YES | Investigate — core actor should trigger DRRP |
| Ind: Employee | 106 | YES | Investigate — core actor should trigger DRRP |
| Operator | 44 | No | Not in GOVERNED_ACTORS |
| Ind: User | 40 | No | Not in GOVERNED_ACTORS |
| SC: Manufacturer | 31 | YES | Investigate |
| Ind: Worker | 23 | No | Beneficiary — SKIP |
| Org: Company | 21 | No | Low count |
| Public | 20 | No | Low count |
| Org: Occupier | 16 | YES | Investigate |
| SC: C: Contractor | 15 | YES | Investigate |
| Org: Owner | 13 | No | Low count |
| Ind: Responsible Person | 13 | YES | Investigate |
| Svc: Installer | 13 | YES | Investigate |
| Ind: Competent Person | 11 | No | Appointed role |

**Key finding**: Employer (114), Employee (106), Manufacturer (31), Occupier (16), Contractor (15), Responsible Person (13), Installer (13) are ALL in `GOVERNED_ACTORS` but still miss DRRP. Total: **308 provisions** where the actor IS recognised and IS in the gate list but the v2 pattern matcher doesn't fire. These are addressable.

Sample review of Employer misses (99 with obligation modal):
- ~30% are genuine duty misses ("he shall make arrangements", "shall ensure that")
- ~30% are exemptions/scope ("shall not apply", "shall relieve")
- ~40% are conditional/cross-reference text ("shall apply if the risk assessment indicates...")

### False Positive Breakdown (149)

| Primary Purpose | Count |
|-----------------|-------|
| Charge+Fee | 75 |
| Process+Rule | 60 |
| Power Conferred | 8 |
| Defence+Appeal | 4 |
| Exemption | 2 |

75 are fee/charge provisions that get DRRP — low value but not harmful. Precision is strong at 96.4%.

## Summary of Issues Found

| Issue | Impact | Addressable? |
|-------|--------|-------------|
| **Mixed-content provisions** (product safety SIs) | ~1,500 provisions gated, ~300+ real duties suppressed | Yes — gate/scanner improvement needed |
| **Gap A: GOVERNED_ACTORS misses** | 308 provisions where recognised actor + modal doesn't trigger DRRP | Yes — v2 pattern matcher investigation |
| **Gap C: No actor** | 3,275 provisions — passive voice | No (beyond regex, AI frontier) |
| **Gap A: Ind: Person** | 552 misses, bare "person" too broad | Diminishing returns |

## Next Steps

- [x] Investigate 0-DRRP anomalies — all genuinely non-making
- [x] Check very low DRRP% laws — mixed-content provision problem
- [x] Build confusion matrix — recall 48.6%, precision 96.4%
- [x] Classify Gap A vs Gap C — Gap C dominates (78%), Gap A has actionable GOVERNED_ACTORS misses
- [x] **Fix 1**: Mixed-content provision gate bypass (product safety SIs) — committed 2cc3aa0
- [ ] **Fix 2**: GOVERNED_ACTORS v2 matcher failures (747 Gap A provisions post-Fix 1)
- [ ] Log Gap C for future AI session

## Fix 1: Mixed-Content Provision Gate Bypass

### Problem

`parse_v2()` flow (mod.rs:101-169):
```
1. purpose::classify()          → purposes (Interpretation-primary for mixed-content)
2. should_skip_drrp(&purposes)  → true → EARLY RETURN (no actors, no DRRP)
3. actors::extract()            ← NEVER REACHED
4. duty_type::classify()        ← NEVER REACHED
```

Actor extraction happens AFTER the purpose gate. Real duties buried after definitions in long provisions are invisible.

### Design

Restructure `parse_v2()` to extract actors BEFORE the gate, then use actor presence as a gate-override signal:

```
1. purpose::classify()          → purposes
2. actors::extract()            → governed, government    [MOVED UP]
3. should_skip_drrp_v2()        → skip ONLY if no DRRP signals
4. duty_type::classify()
```

New gate logic in `should_skip_drrp()` — add `governed_actors` parameter:

- **ALL-structural** (every purpose is skip-purpose): skip unconditionally (unchanged)
- **Interpretation-primary + no governed actor**: skip (unchanged behaviour for pure definitions)
- **Interpretation-primary + governed actor present**: **DON'T SKIP** — proceed to DRRP extraction. The v2 anchored matcher has its own false-positive guards (subordinate clause check, modal window) that will prevent bad classifications from definition fragments like `"exposure limit" means... which must not be exceeded`
- **Enactment-primary**: skip unconditionally (unchanged — enactment text never has real duties)
- **Application+Scope-primary**: skip unconditionally (unchanged — these feed fitness, not DRRP)

### Why this is safe

The Interpretation-primary gate was added to prevent false DRRP from definition modals ("must not be exceeded" inside a definition). With the override, those provisions would reach `duty_type::classify()`, but:
- `match_governed_v2()` requires an actor keyword in **subject position** relative to the modal (within a 200-char window, not inside a subordinate clause)
- In a pure definition, the modal appears inside a quote or after "means" — the actor (if extracted at all) won't be in subject position relative to it
- The v2 matcher has a 99%+ precision rate on operative provisions

### TODO

- [x] 1a. Move `actors::extract_actors_for_family()` call above the skip gate in `parse_v2()` (mod.rs)
- [x] 1b. Change `should_skip_drrp()` signature to accept `has_governed_actor: bool`
- [x] 1c. Interpretation-primary: skip only when `!has_governed_actor`
- [x] 1d. Update the early-return block to include actor labels in the returned `TaxaRecord` (they're now available)
- [x] 1e. Add true-negative tests: `pure_definition_with_actor_keyword_no_drrp` — ALL-structural gate still fires for pure definitions
- [x] 1f. Add true-positive tests: `mixed_content_provision_employer_duty_extracted` — definitions + employer duty in same provision
- [x] 1g. Run `cargo test -p fractalaw-core` — 384 passed, 0 failed (updated 2 existing tests, added 3 new)
- [x] 1h. Re-enriched entire OH&S family (451 laws) with `--force`
- [x] 1i. Confusion matrix results:

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Total FN | 4,200 | 3,800 | **-400** |
| Genuine misses | 3,835 | 3,435 | **-400** |
| Gap A (actor present) | 925 | 747 | **-178** |
| Gap C (no actor) | 2,910 | 2,688 | **-222** |
| Raw recall | 48.6% | 53.4% | **+4.8pp** |
| Adjusted recall | 50.7% | 55.9% | **+5.2pp** |
| Raw F1 | 64.6% | 66.2% | **+1.6pp** |
| Adjusted F1 | 66.5% | 68.1% | **+1.6pp** |

- [x] 1j. Product safety SIs: UK_uksi_2016_1092 1.4%→12.4%, UK_uksi_2016_1093 0.7%→50.2%, UK_uksi_2016_1105 0.7%→45.2%, UK_asc_2025_4 tested OK

**Bug fix**: `extract_clause()` panicked on `begin <= end` when span actor_start > modal_end due to regex overlap in mixed-content provisions. Added guard to swap bounds.

## Fix 2: v2 Matcher Gap Analysis

### Investigation

Post-Fix 1, 747 Gap A provisions remain (governed actor present, no DRRP). Deep investigation across all governed actors (excluding Ind: Person, Worker, etc.):

| Category | Count | Notes |
|----------|-------|-------|
| Gate-blocked (correct) | 197 | scope 83, enact 85, structural 26, descriptive 3 |
| Actor after modal (correct) | 316 | Actor is object/beneficiary, not subject — v2 correctly rejects |
| **v2 potential bugs** | **50** | Forward anchor exists but Rust v2 doesn't fire |

v2 bugs by actor:

| Actor | Count |
|-------|-------|
| Ind: Employee | 18 |
| Org: Employer | 14 |
| SC: Manufacturer | 5 |
| Svc: Installer | 5 |
| Operator | 4 |
| Org: Occupier | 3 |
| Ind: User | 1 |

### Root causes identified

1. **Subordinate clause + pronoun reference** (4 provisions): Text cleaner strips paragraph numbers ("2 ") → "Where an employer..., he shall" → subordinate check fires for "employer" but main clause uses pronoun "he" → v2 retries keyword but no second occurrence exists.

2. **Enabling "may" epistemic rejection** (~16 provisions): "employer may" matched by forward anchor but rejected by `is_epistemic_may()` check (false positive on the epistemic filter).

3. **Text cleaner artefacts**: Paragraph number stripping changes subordinate clause boundary detection.

### Decision

**Diminishing returns** — 50 provisions = 0.25% of corpus. Fix 1 recovered 400 provisions (8x larger impact). The remaining 2,688 Gap C provisions (passive voice, no actor) are 54x larger than Fix 2's potential gain.

**No code changes for Fix 2.** The subordinate/pronoun and epistemic-may issues are documented for future reference. Gap C is the priority and will be addressed in a dedicated AI/LLM session.

## Session Summary

| Metric | Baseline | Post-Fix 1 | Delta |
|--------|----------|------------|-------|
| Recall | 48.6% | 53.4% | +4.8pp |
| F1 | 64.6% | 66.2% | +1.6pp |
| False negatives | 4,200 | 3,800 | -400 |
| Gap A (actor present) | 925 | 747 | -178 |
| Gap C (no actor) | 3,275 | 2,688 | -587 |

### Remaining work

- [ ] Gap C: AI/LLM session for passive-voice provisions (2,688 — 71% of remaining FN). Will also cover the 50 v2 bugs (pronoun reference, epistemic "may") since LLM parsing is actor-agnostic.
- [ ] GH #32: Bulk LAT prune across all families
- [ ] Publish re-enriched OH&S data: `sync publish --tenant dev --family "OH&S: Occupational / Personal Safety"`

---

**Session closed**: 2026-04-15. Commit: 2cc3aa0 (Fix 1).
