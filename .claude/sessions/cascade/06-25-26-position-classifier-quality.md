# Session: Position Classifier Quality (CLOSED)

## Problem

The position classifier is confidently wrong on benchmark provisions. HSWA s.2(1) — "It shall be the duty of every employer to ensure..." — has `Org: Employer` classified as `mentioned` instead of `active`. The classifier predicts `other@0.99` overriding the correct regex signal `active@0.30`.

This is visible in sertantai's governed filter: only 1 duty came through for HSWA (out of hundreds) because most actors are misclassified as `mentioned`.

## Root cause chain

1. Regex correctly identifies `Org: Employer` as `active` and `Ind: Employee` as `counterparty` — but at low confidence (0.30)
2. Position classifier (v1) predicts `other` with high confidence (0.92-0.99)
3. The cascade picks the classifier result over regex because classifier tier > regex tier
4. `other` maps to `mentioned` in the final output
5. Sertantai's governed filter correctly excludes `mentioned` actors from duty display

## Immediate issue: benchmark laws

HSWA (`UK_ukpga_1974_37`) is a benchmark law with `is_benchmark = true`. It should have LLM-validated positions (agentic tier), not classifier-overridden ones. The `extraction_method` shows `agentic` but the reason trail shows the classifier was the last to touch positions.

**Question**: how did the classifier override agentic-tier positions on a benchmark law? The source-tier protection should prevent this. Need to investigate whether the benchmark data was corrupted by the accidental re-validation earlier today.

## Scope

### 1. Benchmark restoration
- Check if the 20 benchmark laws have correct actor positions or if they were all corrupted
- Restore from gold standard if needed
- Verify source-tier protection actually prevents position classifier from overriding agentic

### 2. Position classifier v1 quality
- Evaluate false-`other` rate across the corpus
- The classifier was trained on a small dataset — may need retraining with better position labels
- Known issue: `other` is a catch-all that absorbs ambiguous cases

### 3. Cascade logic for positions
- Currently: regex position (low confidence) → classifier position (high confidence) → LLM position (agentic)
- The classifier shouldn't override regex when regex is correct — confidence scores are miscalibrated
- Consider: should position even go through the classifier, or jump straight from regex to LLM for uncertain cases?

### 4. Impact on sertantai
- Sertantai filters on `position = 'active'` for governed duties
- Any law where active actors are misclassified as `mentioned` will show 0 duties
- This is a systemic quality issue, not a one-off

## Findings (2026-06-26)

### 1. Benchmark gold standard confirms the bug
HSWA s.2(1) gold: `Employer=active, Employee=counterparty`. Live Postgres: both `mentioned`. The benchmark data was correct — the live classification is wrong.

### 2. The cascade does NOT elevate position disagreements to LLM
In `cmd_taxa_classify` (taxa.rs:2858-2863):
```rust
// When classifier disagrees with "other", map to "mentioned"
let final_pos = if !agrees && cls_pos == "other" {
    "mentioned".to_string()
} else {
    regex_pos.clone()
};
```
When classifier says `other` and disagrees with regex, it **silently overrides to `mentioned`**. No LLM elevation. No flagging for review. Just overwrite.

### 3. The position classifier is systemically wrong — corpus-wide

| Metric | Count |
|--------|-------|
| Regex said `active`, classifier overrode to `mentioned` | **51,523** |
| Regex said `counterparty`, classifier overrode to `mentioned` | **18,073** |
| Total `mentioned` from classifier override | **72,187 / 72,312** (99.8%) |
| Correctly `active` (survived classifier) | 40,499 |

The position classifier v1 predicts `other` for nearly everything. It's worse than useless — it's actively destroying correct regex signals. Virtually every `mentioned` classification in the corpus is a false override.

### 4. Impact on sertantai
Sertantai filters duties by `position = 'active'` for governed actors. With 51K active actors misclassified as mentioned:
- HSWA shows 1 duty instead of hundreds
- The entire QQ corpus is underreporting duties by ~55%

### Fix options

**Option A: Remove the classifier override (immediate)**
Change the cascade logic: when classifier says `other`, keep the regex position instead of overriding to `mentioned`. This restores 51K+ correct positions immediately. No retraining needed.

**Option B: Flag disagreements for LLM review**
When regex and classifier disagree, flag as `pending_llm` instead of silently overriding. LLM adjudicates. More accurate but slower and costs API calls.

**Option C: Retrain the position classifier**
Fix the training data and retrain. The v1 model was trained on a small dataset with noisy labels. But this doesn't fix the 70K already-classified provisions.

**Recommended: A then C.** Remove the override now (fixes the corpus), retrain later (improves future classifications).

### 7. How the classifier is coded — root causes

The position classifier has three structural problems that make it fundamentally unable to do its job:

#### Problem 1: DRRP features use wrong labels (features are dead)

`position_classifier.rs` line 52:
```rust
const DRRP_TYPES: &[&str] = &["Duty", "Right", "Responsibility", "Power", "none"];
```

The pipeline stores DRRP types as `Obligation` and `Liberty` — the 3-class system we use everywhere. The one-hot encoding checks `drrp_types.iter().any(|d| d == drrp)` against "Duty", "Right" etc. — it never matches. **The 5 DRRP features are always zero.** The classifier has no information about whether the provision creates an Obligation or Liberty.

This is critical because position depends on DRRP type:
- Obligation active = bears the duty
- Liberty active = exercises the permission
- The actor who bears an Obligation is DIFFERENT from the actor who exercises a Liberty

Without knowing the DRRP type, the classifier can't distinguish these — it's guessing positions blind.

#### Problem 2: 3-class output conflates beneficiary and mentioned

```rust
pub enum PositionClass { Active, Counterparty, Other }
```

`Other` = beneficiary OR mentioned. These are legally distinct:
- Beneficiary has a genuine interest (public benefits from safety duties)
- Mentioned has NO legal relation (referenced in a definition)

Collapsing them into one class means the classifier never learns to distinguish them. And when `other` maps to `mentioned` in the cascade logic, all beneficiaries also become mentioned.

#### Problem 3: Training data was only 2,200 agentic provisions

From the original session: "3-class LogisticRegression, 68% overall accuracy, 74% precision on active". This was already marginal — and with dead DRRP features, the model was really only learning from embeddings + modals + category + offset. Not enough signal for a 3-class problem.

#### Problem 4: The original design said "detection only, don't auto-override"

From the session doc (Step 4): *"Don't auto-override position — regex position stays as source of truth. Detection only — counts disagreements, doesn't write back yet."*

But the implementation in `cmd_taxa_classify` (taxa.rs:2858-2863) DOES override:
```rust
let final_pos = if !agrees && cls_pos == "other" {
    "mentioned".to_string()
} else {
    regex_pos.clone()
};
```

The design was explicitly "don't override", but the code overrides. This is the proximate cause of the 51K false-mentioned — the implementation diverged from the design.

### Summary: three layers of failure

1. **Features broken** — DRRP labels mismatch, 5 features always zero
2. **Classes too coarse** — beneficiary conflated with mentioned
3. **Design violated** — "detection only" became "override to mentioned"

The classifier was set up to fail — and then wired to override rather than detect.

## The cure

### Goal

Regex + classifier agree on position for >80% of provisions, matching benchmarks. The remaining ~20% get elevated to LLM. This keeps LLM calls to the hard cases — ambiguous multi-actor provisions, structural provisions that look like duties, etc.

### Step 1: Fix the override logic (immediate, zero risk)

Change taxa.rs:2858-2863 — when classifier disagrees, **keep regex position** instead of overriding to `mentioned`. This restores the original "detection only" design. The regex was right 96.6% of the time for active actors (per the position confusion matrix: gold=active, pipeline=active for provisions without classifier override).

No retraining needed. This alone should bring legal relation accuracy from 31.8% to ~80%+ since DRRP types are already 100%.

### Step 2: Fix the DRRP feature encoding

Change `position_classifier.rs` line 52:
```rust
// Before (broken — never matches):
const DRRP_TYPES: &[&str] = &["Duty", "Right", "Responsibility", "Power", "none"];

// After (matches pipeline output):
const DRRP_TYPES: &[&str] = &["Obligation", "Liberty", "none"];
```

Reduce from 5 to 3 features (413 → 411 dims). Requires retraining the classifier with the correct feature vector.

### Step 3: Retrain with 4 classes

Change output from `active/counterparty/other` to `active/counterparty/beneficiary/mentioned`. Train on the gold benchmark data (2,250 provisions with correct positions). The benchmarks are the cleanest training signal we have.

### Step 4: Wire disagreement → LLM elevation

When regex and retrained classifier disagree on position, flag as `pending_llm` — same pattern as DRRP disagreements. LLM adjudicates the ~20% hard cases. This completes the cascade: regex (fast, free) → classifier (fast, free) → LLM (slow, paid, accurate).

### Step 5: Re-run benchmark QA

Target: legal relation accuracy >80%. The benchmark script now tests the right thing.

### Revised architecture (v2): normalised per-actor signals

The v1 architecture (per-provision columns) was wrong because the unit of classification is **(provision, actor)**, not provision. A single provision can have multiple actors, each with a different DRRP type and position:

```
s.2(1) HSWA:
  Org: Employer  → Obligation, active (bears the duty)
  Ind: Employee  → Obligation, counterparty (holds the claim)
```

Flat columns on `legislation_text` (regex_drrp, cls_drrp, etc.) can't represent per-actor signals. JSONB actors columns require parsing for every comparison. Neither is queryable or benchmarkable per-actor.

#### New table: `provision_actors`

One row per (section_id, actor_label). Each tier writes to its own columns. No JSON parsing for benchmarking.

```sql
CREATE TABLE provision_actors (
    section_id      TEXT NOT NULL,
    actor_label     TEXT NOT NULL,    -- "Org: Employer"
    actor_category  TEXT,            -- "Org" (extracted from label prefix)

    -- Regex tier signals
    regex_drrp      TEXT,            -- "Obligation"
    regex_position  TEXT,            -- "active"

    -- Classifier tier signals
    cls_drrp        TEXT,            -- "Obligation"
    cls_position    TEXT,            -- "mentioned"
    cls_confidence  REAL,            -- 0.92

    -- LLM tier signals
    llm_drrp        TEXT,            -- "Obligation"
    llm_position    TEXT,            -- "active"

    -- Reconciled (final answer — what sertantai consumes)
    drrp            TEXT,            -- "Obligation"
    position        TEXT,            -- "active"
    extraction_method TEXT,          -- "agentic"

    PRIMARY KEY (section_id, actor_label)
);

CREATE INDEX idx_pa_section ON provision_actors (section_id);
```

#### Pipeline steps

1. `taxa parse` — regex → writes `regex_drrp`, `regex_position` per (section_id, actor)
2. `taxa embed` — embeddings (unchanged, stays on legislation_text)
3. `taxa classify` — classifier → writes `cls_drrp`, `cls_position`, `cls_confidence` per (section_id, actor)
4. `taxa reconcile` — reads all tier columns, writes `drrp`, `position`, `extraction_method`
5. `taxa validate` — LLM for disagreements → writes `llm_drrp`, `llm_position`
6. `taxa reconcile` — re-run to incorporate LLM

#### Benchmark QA (per-actor, per-tier)

```sql
-- Compare regex positions against gold
SELECT pa.section_id, pa.actor_label, pa.regex_position, g.gold_position
FROM provision_actors pa
JOIN gold_benchmarks g ON pa.section_id = g.section_id AND pa.actor_label = g.actor_label
WHERE pa.regex_position != g.gold_position;

-- Position accuracy per tier
SELECT 'regex' as tier,
  count(*) FILTER (WHERE regex_position = gold_position) as correct,
  count(*) as total
FROM provision_actors pa JOIN gold_benchmarks g USING (section_id, actor_label)
UNION ALL
SELECT 'classifier',
  count(*) FILTER (WHERE cls_position = gold_position),
  count(cls_position)
FROM provision_actors pa JOIN gold_benchmarks g USING (section_id, actor_label);
```

#### Benefits over v1 (JSONB columns)

- Each signal is a plain TEXT column — queryable, indexable, no JSON parsing
- Benchmark is a simple JOIN, not Python JSONB extraction
- Re-run any tier independently — writes to its own columns only
- Per-actor granularity matches the gold standard format
- `legislation_text` stays clean — provision-level data only (text, embeddings)
- `provision_actors` has the classification signals — one row per actor

#### What stays on legislation_text

- All LAT columns (text, section_type, hierarchy_path, etc.)
- Embeddings (vector)
- The existing `drrp_types` / `actors` / `extraction_method` stay for backward compat with sertantai publish — updated from `provision_actors` during reconcile

#### Gemini harsh review (2026-06-26)

Key feedback:
1. **PK (section_id, actor_label) is fine** — checked gold benchmarks: 1/1,711 has a dupe (data quality issue, not schema). Safe for our data.
2. **Migration is a full re-run** — can't reconstruct per-actor DRRP from old per-provision drrp_types. Must re-parse all provisions. This is OK since regex is fast.
3. **Actor identity** — string matching is fragile. Already have actor dictionary for normalisation. FK to dictionary is a future improvement.
4. **Case/enum validation** — already normalise in code but should enforce. Defer.
5. **Versioning/audit** — not needed now. drrp_history JSON already captures tier progression.
6. **500K rows is trivial** for Postgres. Not a concern.

#### Implementation order

1. ✅ Create `provision_actors` table
2. ✅ Populate from existing actors JSONB (124K actors, 72K with regex_drrp)
3. ✅ Modify `taxa parse` to write regex_drrp + regex_position to provision_actors (tested: HSWA 914 actors)
4. ✅ Modify `taxa classify` to write cls_drrp + cls_position to provision_actors (tested: HSWA 914 actors)
5. Build `taxa reconcile` reading from provision_actors
6. Run benchmark QA per-tier
7. Backfill legislation_text drrp_types/actors from provision_actors (sertantai compat)

### Order of work

1. Add per-tier columns to Postgres schema
2. Refactor `taxa parse` to write `regex_drrp` + `regex_actors` (not `drrp_types`/`actors`)
3. Refactor `taxa classify` to write `cls_drrp` + `cls_actors` (not overwrite)
4. Build `taxa reconcile` command
5. Run benchmark QA at each stage
6. Wire LLM elevation into reconcile for disagreements

### Gemini review feedback (2026-06-26)

- **Fix data/logic first** — LR with 1,242 params may be sufficient once features work and classes are correct. Don't jump to GBT/MLP prematurely.
- **Non-embedding features matter** — modal, DRRP, category, offset will contribute once DRRP encoding is fixed. Don't strip them.
- **Fine-tuned local LLM (gemma3:4b)** — valid future direction but larger undertaking. Fix current classifier first.
- **2,200 provisions adequate** for LR with 411 features + embeddings. Watch `beneficiary` class imbalance.
- **Confidence thresholding for LLM escalation** — don't just escalate on disagreement. If classifier confidence < 0.7, escalate even when regex agrees. This catches uncertain-but-agreeing cases.
- **GBT (XGBoost/LightGBM) as fallback** — if fixed LR still < 80%, gradient boosted trees handle mixed features well and provide feature importance for debugging.

### 5. Position determines whether DRRP applies to an actor

The DRRP type (Obligation/Liberty) and actor position (active/counterparty/mentioned) are coupled — not independent:

| DRRP Type | Position | Legal meaning | Example |
|-----------|----------|---------------|---------|
| Obligation | **active** | Bears the duty | Employer *shall ensure* safety |
| Obligation | **counterparty** | Holds a claim against the duty | Employee *is owed* the duty |
| Liberty | **active** | Exercises the liberty/permission | Inspector *may enter* premises |
| Liberty | **counterparty** | No-duty: cannot prevent the liberty | Occupier *has no right to prevent* entry |
| — | **beneficiary** | Benefits without a direct legal relation | Public benefits from safety duty |
| — | **mentioned** | **No legal relation — referenced only** | "as defined in the Act" |

Every Obligation has an antecedent — the counterparty who holds a claim. "Occupier must permit entry" is itself an Obligation on the occupier (active), not a Liberty counterparty. The Liberty is the inspector's permission to enter; the occupier's duty to permit is a *separate* correlative Obligation.

Rights and Powers are not separate DRRP types — they emerge from the combination of actor type + position:
- Government actor + active Liberty = **Power** (e.g. HSE *may* prosecute)
- Non-government actor + counterparty Obligation = **Right** (claim against the duty-holder)

A `mentioned` actor has **no DRRP relationship**. When the position classifier overrides `active→mentioned`, it **nullifies the entire DRRP classification for that actor**. An Obligation with no active actor is an orphan duty with no holder.

The DRRP benchmark was passing on a technicality: the type field said `Obligation`, but the actor's position was set to `mentioned` — so the obligation has no holder. **The benchmark measured the DRRP type label, not the complete legal relation (type + actor + position).**

Sertantai correctly filters on `position = 'active'` because only active actors bear duties. The 51,523 overridden actors represent 51,523 duties that exist in name only — no holder assigned.

### 6. Why did benchmarks show >80% accuracy?

The benchmark report (`scripts/benchmark_report.py`) measures **DRRP type** accuracy (Obligation/Liberty/none) — NOT position accuracy. The >80% metric was for DRRP classification, which IS working. Position errors (active→mentioned) don't affect the DRRP metric at all. The benchmark never tested position quality.

The benchmark does compute a position confusion matrix (`pos_confusion`), but it was never the headline metric. If we had looked at the position confusion matrix, we'd have seen the massive active→mentioned drift.

### 6. What rules elevate to LLM?

DRRP classifier elevation rules (taxa.rs:2599-2641):
- **Gap fill** (regex=none, classifier=DRRP, confidence ≥ 0.7): classifier wins → `extraction_method = "classifier"`
- **Weak gap** (regex=none, classifier=DRRP, confidence < 0.7): flag → `pending_llm`
- **DRRP disagreement** (regex=X, classifier=Y, confidence ≥ 0.75): flag → `pending_llm`
- **Both modals** (obligation + enabling modal words): flag → `pending_llm`

**Position classifier has NO elevation rules.** When the position classifier disagrees with regex, it silently overrides to `mentioned` (taxa.rs:2858-2863). There's no `pending_llm` flagging, no LLM adjudication, no disagreement tracking for positions. This is the fundamental gap — DRRP has a principled disagreement→LLM path, positions don't.

## Outstanding in this session

### Cleanup ✅
- ✅ Dropped 8 per-tier columns from legislation_text (superseded by provision_actors)
- ✅ Removed snapshot_regex/classifier/llm_signals and write_cls_actors from ProvisionStore + PgStore
- ✅ Archived position_classifier_v1.json → v1.json.archived

### Corpus-wide (carried to daughter sessions)
- ⬜ 51K false-mentioned actors in non-benchmark corpus — NOT FIXED. Full corpus needs re-parse + re-classify to populate provision_actors. Carried to reconciliation session.
- ⬜ DRRP classifier agreement recording — currently writes to legislation_text (cls_drrp on agree). This is now redundant since provision_actors captures it. The taxa.rs agreement code can be simplified in a future cleanup. Low priority.

## Daughter sessions created

- `cascade/06-26-26-reconciliation.md` — reconcile engine, LLM elevation, sertantai backfill
- `cascade/06-26-26-benchmark-qa.md` — per-tier benchmarking from provision_actors
- `cascade/06-26-26-classifier-training.md` — improve position classifier, GBT fallback, local LLM

## Key files

- `crates/fractalaw-cli/src/commands/taxa.rs` — classify + position classifier
- `crates/fractalaw-cli/src/commands/pipeline.rs` — parse → provision_actors write
- `crates/fractalaw-ai/src/position_classifier.rs` — 4-class, 411 features
- `crates/fractalaw-store/src/pg.rs` — upsert_provision_actors
- `crates/fractalaw-store/src/provision_store.rs` — trait with new methods
- `scripts/train_position_classifier.py` — v2 training script
- `scripts/benchmark_report.py` — rewritten but needs provision_actors update
- `docs/position_classifier_v2.json` — active weights
- `docs/position_classifier_v1.json` — deprecated, archive
- `.claude/sessions/cascade/06-11-26-position-classifier.md` — original training session
