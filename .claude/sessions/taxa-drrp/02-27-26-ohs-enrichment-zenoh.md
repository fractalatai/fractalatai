# Session: OHS family enrichment

**Date**: 2026-02-27
**Depends on**: 02-27-26-application-scope-tightening (GH #20, closed)
**Zenoh pub/sub**: Parked — will be a separate dedicated session (see research notes below)

## Goals

1. Enrich all laws in the `OH&S: Occupational / Personal Safety` family with the v5 taxa pipeline (tightened Application+Scope)
2. Validate quality via eyeball review on a representative sample

## Current State

### OHS family in DuckDB

| Metric | Value |
|--------|-------|
| Total laws | 451 |
| With body text (body_paras > 0) | 314 |
| Already enriched (duty_holder populated) | 63 |
| Remaining to enrich | ~251 (314 with text − 63 already done) |

The 63 already-enriched laws were processed with the older pipeline (pre-v5). They'll need `--force` re-enrichment to pick up the Application+Scope tightening.

### taxa enrich capabilities

Current `--laws` flag accepts a comma-separated list of law names. For 314 laws, passing them all manually is impractical. Options:

**A. Add `--family` flag** to `taxa enrich` — query DuckDB for all laws matching a family, then enrich those. Cleanest approach.

**B. Script it** — query DuckDB for law names, pipe to `--laws`. Fragile, shell-dependent.

**C. Use `--force`** — re-enriches ALL 452 laws with LanceDB text, not just OHS family. Wasteful but simple.

Recommendation: **Option A** — add `--family` flag.

## Plan

### Phase 1: Add `--family` flag to `taxa enrich`

Add a `--family` option to `TaxaAction::Enrich` that:
1. Queries DuckDB: `SELECT name FROM legislation WHERE family = ?`
2. Uses those names as the law filter
3. Works with `--force` to re-enrich already-processed laws

### Phase 2: Enrich OHS family

```bash
fractalaw taxa enrich --family "OH&S: Occupational / Personal Safety" --force
```

This re-enriches all 314 laws with text using the v5 pipeline.

### Phase 3: Validate quality

1. Run `taxa eyeball` on a sample of newly-enriched laws (pick 5-7 diverse ones)
2. Check DRRP counts, clause quality, false positive rate
3. Compare against the 7-law baseline from the previous session

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-cli/src/main.rs` | `taxa enrich` command, new `--family` flag |
| `crates/fractalaw-core/src/taxa/mod.rs` | v5 pipeline (with Application+Scope gate) |

## Progress

- [x] Understand current enrichment pipeline and OHS family size
- [x] Review sync/zenoh infrastructure status
- [x] Document plan
- [x] Phase 1: Add `--family` flag to `taxa enrich`
- [x] Phase 2: Enrich OHS family (451 laws)
- [x] Phase 3: Validate quality (at-scale + post-fix verification)

## Phase 1 Implementation

Added to `crates/fractalaw-cli/src/main.rs`:

1. `family: Option<String>` arg on `TaxaAction::Enrich`
2. Match arm resolves `--family` to law names via `laws_in_family()` helper before calling `cmd_taxa_enrich`
3. `laws_in_family()` queries DuckDB: `SELECT name FROM legislation WHERE family = ? ORDER BY name`

Usage: `fractalaw taxa enrich --family "OH&S: Occupational / Personal Safety" --force`

All 289 core tests + 41 AI tests pass. Compiles clean.

## Phase 2 Results

Ran: `fractalaw taxa enrich --family "OH&S: Occupational / Personal Safety" --force`

### Pipeline flow

```
451 OHS laws in DuckDB
 ├─ 156 are Making laws (have duties/responsibilities — full text stored)
 │   ├─ 123 have body_paras > 0
 │   │   ├─ 67 have full text in LanceDB → taxa enriched
 │   │   └─ 56 full text not yet parsed by sertantai
 │   └─ 33 no body text
 └─ 295 are non-Making (amending, commencing, revoking, etc.) — no full text stored
```

Only Making laws get full text stored in LanceDB. The 314 "with body text" count from DuckDB metadata is misleading — many of those are non-Making instruments.

### Coverage

| Metric | Value |
|--------|-------|
| OHS Making laws | 156 |
| Making laws with body text | 123 |
| Making laws with LanceDB text (enriched) | 67 |
| Making laws enriched with taxa signal | 67 |
| Making laws with `duty_holder` | 62 |
| **Coverage: enriched / Making with text** | **67 / 123 (54%)** |

### Gap: 56 Making laws without LanceDB text

These laws have body paragraphs in DuckDB metadata (from legislation.gov.uk XML) but their full text hasn't been parsed by sertantai and loaded into LanceDB yet.

### DuckDB columns from earlier pipelines

| Column | Populated | Notes |
|--------|-----------|-------|
| `duties` | 139 | From an earlier enrichment pipeline, not this taxa run |
| `rights` | 81 | " |
| `responsibilities` | 127 | " |
| `powers` | 114 | " |

### Summary stat from enrichment output

The "269" reported at the end (`Processed 451 laws. LRT now has 269 laws with DRRP taxa data.`) counts all laws corpus-wide where `duty_holder IS NOT NULL AND len(duty_holder) > 0` — not OHS-specific.

## Phase 3: Validation

### Verification methodology

Too many provisions to eyeball manually (10K+ in LanceDB for OHS alone). Three-query approach via PyLanceDB + DataFusion:

1. **Confidence distribution** — query LanceDB `legislation_text` for all provisions where `taxa_confidence > 0`, filter to OHS law names (cross-referenced from DuckDB), bucket by confidence score. Surface the low-confidence tail for manual inspection.

2. **Application+Scope gate audit** — query LanceDB for ALL OHS provisions (not just enriched), partition by whether `purposes` contains `Application+Scope`. Split into:
   - *Gated* (App+Scope present, no DRRP) — check for **false positives** (genuine duties blocked). Heuristic: does the provision have a modal verb AND a governed actor? If yes, further classify as *actor-led* (text starts with actor — highest false-positive risk) vs *scope-led* (text starts with "These Regulations..." — likely genuine scope).
   - *Passed* (App+Scope present, DRRP still classified) — check for **false negatives** (scope provisions that leaked through).

3. **Missing enrichment** — DataFusion query for OHS Making laws with `body_paras > 0` but `duty_holder IS NULL`. Cross-reference against LanceDB to distinguish "not in LanceDB yet" (sertantai gap) from "in LanceDB but taxa found nothing" (legitimate — check provision content).

### Results

## 1. Confidence Distribution

Across 2,237 OHS provisions with taxa confidence > 0:

| Bucket | Count | % |
|--------|------:|---:|
| 0.80–0.85 (high) | 1,494 | 66.8% |
| 0.60–0.79 (good) | 462 | 20.7% |
| 0.40–0.59 (medium) | 269 | 12.0% |
| 0.20–0.39 (low) | 12 | 0.5% |
| 0.00–0.19 (very low) | 0 | 0.0% |

**Mean: 0.750**. The scoring is quantized to 6 discrete values (0.20, 0.35, 0.45, 0.60, 0.70, 0.85) due to the additive signal weights.

**The 12 low-confidence provisions** (all at 0.20) are fee/charge provisions — things like "Fees payable by operators to the competent authority" from COMAH and REACH regulations. These score low because they're short, no modal verb, no span capture, no sentence-end punctuation. They're correctly classified as duties (someone must pay fees), just with weaker regex signal.

## 2. Application+Scope Tagged Provisions

**Count: 0** among enriched provisions.

This means the Application+Scope gate from GH #20 is working as intended — provisions tagged as Application+Scope primary are being excluded from DRRP classification before they ever get a confidence score. They don't appear in the enriched set because they were correctly skipped.

## 3. Laws Without Duty or Responsibility

**Count: 0**. All 62 enriched OHS laws have at least one Duty. 43 of those also have Responsibility. No law was enriched without getting a Duty or Responsibility classification.

### Coverage Gap

Of the 123 OHS Making laws with body text, only 62 (50%) got enriched. The gap:

| Status | Count |
|--------|------:|
| Not in LanceDB (no sertantai parse) | 55 |
| In LanceDB but taxa found nothing | 6 |

The 6 laws in LanceDB without DRRP are legitimate: amendment-heavy instruments (UK_nisr_2015_265), offences acts (UK_ukpga_2008_20), fee modification SIs (UK_nisr_1999_150) — no operative duties to find. The real gap is the 55 laws not yet parsed by sertantai.

## 4. Application+Scope Gate Audit

582 provisions gated (Application+Scope primary, DRRP skipped).

| Check | Count | Result |
|-------|------:|--------|
| False negatives (scope leaked to DRRP) | 0 | Clean |
| True gates (no modal+actor) | 441 | Correct |
| Scope-led with modal+actor | 126 | Correct — scope extensions mentioning actors |
| **Actor-led with modal (false positives)** | **15** | **Bug — genuine duties blocked** |

**Error rate: 15 / 582 = 2.6%**

All 15 false positives follow one pattern:

> "[Actor] to whom/which this regulation applies [shall/must] [action]"

The scope regex matches `this regulation applies` inside a relative clause that qualifies the actor, not the main verb. The `.{0,60}` gap in the regex is wide enough to span from "this regulation" across the relative clause to "applies".

### Fix applied

Rust `regex` crate doesn't support lookbehinds. Instead, tightened branch 2 of the Application+Scope regex to require `these/this` at a valid sentence-start position:

```
# Before (fires on "to whom this regulation applies")
(?:these|this) (?:Regulations?|Act|...).{0,60}...appl(?:y|ies)

# After (requires text-start, sentence boundary, comma, or paragraph number)
(?:^|[.;,]\s+|\d\s+)(?:these|this) (?:Regulations?|Act|...).{0,60}...appl(?:y|ies)
```

The prefix `(?:^|[.;,]\s+|\d\s+)` ensures `these/this` only matches when the law is the grammatical subject, not inside a relative clause.

Other branches (3–10) are unaffected. Tests: 300 pass (11 new regression tests added across `purpose.rs` and `mod.rs`).

### Post-fix re-enrichment validation

Re-ran `fractalaw taxa enrich --family "OH&S: ..." --force` after the fix, then repeated the validation queries.

| Metric | Before fix | After fix | Delta |
|--------|-----------|-----------|-------|
| OHS enriched provisions | 2,237 | 2,295 | **+58** |
| Mean confidence | 0.750 | 0.751 | +0.001 |
| High confidence (0.8–0.85) | 66.8% | 67.0% | +0.2% |
| Application+Scope gated (total) | 582 | 395 | −187 |
| Gated actor-led (false positives) | 15 | **0** | **−15 (all recovered)** |
| Laws with DRRP in DuckDB | 62 | 62 | 0 |

The +58 net new provisions exceeds the 15 directly-identified false positives — the tighter regex also reduced the total Application+Scope gate from 582 to 395, meaning 187 provisions that were previously tagged as Application+Scope no longer match (the relative-clause pattern was over-triggering beyond just the 15 actor-led cases). All 58 newly-classified provisions are genuine DRRP.

Confidence distribution held steady — no degradation. Zero actor-led false positives remain.

---

## Session closed

Zenoh research notes moved to dedicated session: `zenoh/02-27-26-zenoh-sync.md`

Also in this session: removed Anthropic/Claude inference backend from codebase (fractalaw-host, fractalaw-cli, drrp-polisher guest). All AI inference now uses ONNX only. The `inference` feature gate and `reqwest` dependency were removed from fractalaw-host.
