# Taxa Pattern Improvement Runbook

Repeatable workflow for improving DRRP classification patterns when new law families arrive, coverage is low, or anomalies are detected.

The taxa pipeline classifies legislative provisions into structured DRRP (Duties, Rights, Responsibilities, Powers) data using regex pattern matching across 5 classifiers. This runbook describes how to diagnose gaps and improve those patterns.

> **Note**: The full Claude Code skill (with Python snippets, confusion matrix methodology, and actor-gating details) is in `.claude/skills/taxa-gap-analysis/SKILL.md`. This runbook is a concise human-readable reference.

## Prerequisites

- LAT (provision text) populated in LanceDB for the target family
- LRT (law metadata with `family` column) populated in DuckDB
- Laws enriched at least once (`fractalaw taxa enrich --family "FAMILY"`)

## Pipeline Overview

```
Provision text (LanceDB)
  │
  ├─ 1. Purpose classifier (purpose.rs)         → what type of provision?
  │     Gate: skip DRRP if Interpretation/Amendment/Repeal-primary
  │
  ├─ 2. Actor extraction (actors.rs)             → who is mentioned?
  │     32 patterns: 16 governed + 16 government
  │
  ├─ 3. Duty patterns (duty_patterns*.rs)        → who must do what?
  │     Obligation/Prohibition/Enabling detection
  │     v2: actor-anchored pattern matching
  │     rule: thing-subject rules ("equipment must be...")
  │
  ├─ 4. POPIMAR classifier (popimar.rs)          → management category?
  │     Policy/Organising/Planning/Implementing/
  │     Measuring/Auditing/Reviewing
  │
  └─ 5. Fitness extraction (fitness.rs)          → who/what/where applies?
        See docs/FITNESS-DICTIONARY-RUNBOOK.md
```

## Workflow

### Step 1: Run QA report

```bash
fractalaw taxa qa --family "FAMILY_NAME"
```

This produces a 4-section report:

1. **Coverage Summary** — per-law: provisions, Purpose%, DRRP%, Gated%
2. **Purpose Distribution** — 15-column table with per-law rates + anomaly flags (>2x corpus average)
3. **Gate Analysis** — skip_drrp sub-gates (Interpretation-primary, Enactment-primary, Application+Scope, all-structural) + descriptive_summary counts
4. **Anomaly Detection** — flags laws with: Enactment >10%, Enforcement >15%, 0 DRRP with >10 provisions, any purpose >2x corpus average

```bash
# Single law deep-dive
fractalaw taxa qa --laws UK_uksi_2005_1541

# All enriched laws (corpus-wide baseline)
fractalaw taxa qa
```

### Step 2: Diagnose the gap

Use the QA report to identify which classifier needs improvement:

| Symptom | Likely cause | Classifier to fix |
|---------|-------------|-------------------|
| Low DRRP% (<30%) on a law with many provisions | Duty patterns not matching legislative language | `duty_patterns_v2.rs` |
| High gate% (>50%) | Purpose classifier over-matching (false Interpretation/Amendment) | `purpose.rs` |
| 0 DRRP with >10 provisions (anomaly) | Either gate bug or genuinely non-regulatory law | Check gate analysis first |
| Enactment >10% (anomaly) | Enactment pattern too broad | `purpose.rs` |
| Enforcement >15% (anomaly) | Enforcement pattern too broad | `purpose.rs` |
| Missing actors in DRRP output | Actor pattern doesn't recognise term | `actors.rs` |
| Wrong POPIMAR category | POPIMAR regex not matching keywords | `popimar.rs` |
| Low fitness Tagged% | Missing p-dimension dictionary terms | See FITNESS-DICTIONARY-RUNBOOK.md |

### Step 3: Inspect provisions

Use the `taxa eyeball` command to see per-provision DRRP output:

```bash
fractalaw taxa eyeball --laws UK_uksi_2005_1541
```

This shows each provision with its DRRP type, actor, clause, and confidence. Look for:

- **False positives** — provisions tagged with DRRP that shouldn't be (e.g., amendment text with "shall" detected as a duty)
- **False negatives** — provisions with obvious duties/powers that got no DRRP tag
- **Misclassification** — wrong DRRP type (e.g., Power classified as Responsibility)
- **Poor clause extraction** — clause text that's truncated or includes irrelevant preamble

### Step 4: Fix patterns

Each classifier has a different pattern structure:

**Purpose classifier** (`purpose.rs`):
- Regex patterns that tag provisions by structural purpose
- Key concern: precision, not recall — false positives cause gate skips
- The gate uses ALL strategy: all purposes must be skip-purposes to trigger

**Actor patterns** (`actors.rs`):
- Keyword lists for governed actors (employer, worker, etc.) and government actors (Secretary of State, HSE, etc.)
- Blacklist to prevent false positives (e.g., "agency worker" shouldn't match "government agency")

**Duty patterns** (`duty_patterns_v2.rs`):
- Actor-anchored: finds actor mention → scans for modal verb → extracts clause
- Primary window (200 chars before modal) and extended window (with reduced confidence)
- Passive reverse patterns ("must be prepared by [actor]")

**Duty patterns — rules** (`duty_patterns_rule.rs`):
- Thing-subject rules: "equipment must be suitable", "premises shall be ventilated"
- Matches when subject is a thing/place, not a person

**POPIMAR** (`popimar.rs`):
- Keyword-based classification into 7 management categories
- Default "Risk control" for provisions with certain duty types

### Step 5: Add tests

Add tests alongside the pattern changes. Tests live in `#[cfg(test)] mod tests` at the bottom of each file. Pattern:

```rust
#[test]
fn new_pattern_matches() {
    let text = "Real provision text from the gap analysis";
    let record = parse_v2(text, Some("FAMILY"));
    assert!(!record.duty_types.is_empty());
    // or: assert specific DRRP type, actor, etc.
}
```

For purpose classifier changes, verify both positive and negative cases:
```rust
#[test]
fn not_false_positive_amendment() {
    let text = "The employer shall ensure that amendments are notified.";
    let purposes = purpose::classify(text);
    // Should NOT be classified as Amendment (it's a genuine duty)
    assert!(!purposes.contains(&purpose::AMENDMENT));
}
```

### Step 6: Validate

```bash
# Run tests
cargo test -p fractalaw-core

# Re-run QA to measure improvement
fractalaw taxa qa --family "FAMILY_NAME"

# Compare DRRP%, Gated%, and anomalies against Step 1 baseline
```

### Step 7: Re-enrich and publish

```bash
# Re-enrich the target family to persist improved classifications
fractalaw taxa enrich --family "FAMILY_NAME" --force

# Verify with QA (should match Step 6 results)
fractalaw taxa qa --family "FAMILY_NAME"

# Publish changed laws to sertantai
fractalaw sync publish --changed
```

## Anomaly Thresholds

| Anomaly | Threshold | Rationale |
|---------|-----------|-----------|
| Enactment >10% | Enactment provisions are rare (1-3 per law) — high rates suggest pattern over-matching |
| Enforcement >15% | Enforcement provisions exist but shouldn't dominate — high rates suggest false positives |
| 0 DRRP, >10 provisions | A law with 10+ provisions and zero DRRP hits is suspicious unless it's purely administrative |
| Any purpose >2x corpus avg | Individual law significantly above corpus baseline suggests classification error |

## Known Patterns of Genuinely Low DRRP

Not all low DRRP% is a bug. These law types legitimately have low or zero DRRP:

- **Commencement orders** — bring other Acts into force, no standalone duties
- **Pure amendment SIs** — modify other legislation, no own duties
- **Safety zone orders** — designate zones, 2-3 paragraphs
- **Revocation orders** — remove provisions
- **Climate/budget orders** — set numerical targets

The gate analysis in the QA report helps distinguish these from genuine pattern gaps.

## Key files

| File | Contains |
|------|----------|
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classification patterns, `classify()` |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction, governed/government keyword lists |
| `crates/fractalaw-core/src/taxa/duty_patterns_v2.rs` | Actor-anchored DRRP pattern matching |
| `crates/fractalaw-core/src/taxa/duty_patterns_rule.rs` | Thing-subject rule detection |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | Legacy duty type classifier (Government/Governed) |
| `crates/fractalaw-core/src/taxa/popimar.rs` | POPIMAR management category classifier |
| `crates/fractalaw-core/src/taxa/clause_refiner.rs` | Clause text extraction (modal verb window) |
| `crates/fractalaw-core/src/taxa/clause_structure.rs` | Clause structure decomposition |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse_v2()`, `should_skip_drrp()`, pipeline orchestration |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_qa()`, `cmd_taxa_eyeball()`, `enrich_single_law()` |
