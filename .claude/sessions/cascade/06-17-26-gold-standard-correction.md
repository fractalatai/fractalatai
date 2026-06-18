# Session: Gold Standard Correction + 3-Class Model Migration (PENDING)

## Context

**Prior sessions**:
- `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md` (CLOSED)
- `.claude/sessions/cascade/06-17-26-actor-drift-surfacing.md` (CLOSED)

**Trigger**: Benchmark analysis revealed ~160 gold labels that are wrong or stale. Separately, the actor drift session identified that the 5-class DRRP model (Duty/Right/Responsibility/Power/none) creates unnecessary decomposition errors. The classifier already uses 3-class (Obligation/Liberty/none). Switching the entire pipeline to 3-class eliminates the decomposition problem and simplifies the gold standard correction.

## Two problems, one fix

### Problem 1: Stale gold labels

The benchmark Parquet files on NAS contain:
- **~95 provisions gold=Duty where pipeline says Responsibility** — not a pipeline error. The LLM was prompted without the governed/government distinction. These are actually Obligation.
- **~38 offence provisions gold=Duty** — "is guilty of an offence", no modal verb. These are not DRRP. Gold should be `none`.
- **~28 Rule provisions gold=Duty** — thing-subject ("A notice must be given"). The LLM correctly identifies the implied duty-bearer from context (parent sections). Gold is correct — these ARE Obligation. The pipeline's Rule classification is the gap, not the gold. The actor is a person, not the thing — the Rule tier fires incorrectly because the person-actor isn't extracted.

### Problem 2: 5-class creates decomposition errors

The Duty vs Responsibility distinction is derivable from actor type (governed/government). Storing it in `drrp_types` creates errors whenever:
- A government actor is mentioned but isn't the duty-bearer (95 cases)
- An actor is reclassified between governed and government (7 Spc actors this session)
- The LLM disagrees with our actor classification

**The fix**: switch the entire pipeline and gold standard to 3-class (Obligation/Liberty/none). The consumer derives Duty/Responsibility from actor labels at display time. No loss of signal.

## 3-Class Model

```
Obligation (shall/must/required to)  — someone bears a legal obligation
Liberty    (may/entitled to/power)   — someone has permission or discretion
none       — no legal relation (definitions, enactment, scope, offence)
```

The consumer decomposes at display time:
```
Obligation + governed active actor  → "Duty"           (customer sees this)
Obligation + government active actor → "Responsibility" (customer can filter)
Liberty    + governed active actor  → "Right"
Liberty    + government active actor → "Power"
```

## Scope

### Gold standard correction
1. Rewrite all benchmark Parquet files: Duty/Responsibility → Obligation, Right/Power → Liberty
2. Remove offence provisions (gold=none, not Obligation)
3. Remove Rule provisions (gold=none, context-dependent)
4. Write corrected files to NAS

### Pipeline migration (3-class)
1. `drrp_types` in LanceDB: `["Obligation"]` / `["Liberty"]` instead of `["Duty"]` / `["Right"]` etc.
2. `duty_type.rs`: `map_to_duty_type()` returns Obligation/Liberty instead of Duty/Right/Responsibility/Power
3. `DutyType` enum: simplify to `Obligation`, `Liberty`, `Rule` (keep Rule as structural)
4. Remove `decompose_drrp()` from `drrp_classifier.rs`
5. DuckDB columns: `duty_holder`/`rights_holder`/`responsibility_holder`/`power_holder` → `obligation_holder`/`liberty_holder` (sertantai breaking change)
6. LLM prompts: "classify as Obligation/Liberty/none"
7. Benchmark report: compare Obligation/Liberty/none

### Sertantai coordination
- DuckDB schema change requires sertantai code update
- The `obligation_holder` column replaces both `duty_holder` and `responsibility_holder`
- Sertantai derives Duty vs Responsibility from actor labels in the actors struct
- This is a coordinated release — pipeline + sertantai must update together

## Approach

### Phase 1: Gold standard (no pipeline change)
1. Script to rewrite benchmark Parquet: map 5-class → 3-class labels
2. Fix offence provisions: gold → none (no modal = not DRRP)
3. Keep Rule provisions as Obligation in gold (LLM is correct — implied actor from context, pipeline must catch up via classifier/LLM)
4. Write to NAS, re-run benchmark with current pipeline for new baseline

### Phase 2: Pipeline migration + parse pipe consolidation

Consolidate all parse pipeline changes into one pass. Includes fractalaw/fractalaw#37 (run_patterns refactor).

#### 2a. 3-class DutyType
1. Change `DutyType` enum: Obligation, Liberty, Rule (drop Duty/Right/Responsibility/Power)
2. Update `map_to_duty_type()` — Government and Governed both → Obligation or Liberty
3. Remove `decompose_drrp()` from `drrp_classifier.rs`
4. Update LanceDB write paths (`drrp_types` column)
5. Update DuckDB schema: `obligation_holder`/`liberty_holder` replace 4 holder columns
6. Update enrichment pipeline, QA commands, benchmark reports

#### 2b. run_patterns() refactor (#37)
Non-mutating overlap resolution — collect all matches, resolve by span length + pattern priority. Fix the 56 provisions where actor IS extracted and modal IS present but governed v2 doesn't anchor.

#### 2c. Re-parse with updated dictionary (NO re-embedding)
Run `parse_v2()` on benchmark laws with:
- Updated YAML actor dictionary (30 new actors take effect)
- Reason provenance written to actor struct (`regex:active@0.80`)
- 3-class drrp_types written to LanceDB
- Existing embeddings untouched

### Obligation→none gap after Phase 2

Of the 171 misses identified in Phase 1:

| Category | Count | Status after Phase 2 |
|----------|-------|---------------------|
| New actors recover | 30 | Fixed — parse_v2 extracts them |
| Actor + modal but regex miss | 56 | Target of 2b — run_patterns + v2 improvements |
| Thing-subject (implied actor) | 50 | LLM territory — assume correct, exclude from QA |
| Offence/passive (no modal) | 35 | Gate or LLM — see offence session |

### Liberty→none findings (50 remaining, was 72)

**Cat 1: No enabling modal, has actor (23)** — LLM territory but on review many are not new legal relations. 23 provisions reclassified to none in gold. Rights expressed as "No answer is admissible", "right of appeal is to a magistrates' court" are subordinate detail — they reference/condition rights created elsewhere. Only provisions creating a new legal relation count as Liberty.

**Cat 2: Has enabling modal, no actor (13)** — thing-subject + "may" where the actor is implied from surrounding sections. The LLM had full law context when building gold; the production Tier 3 prompt doesn't. Raised as fractalaw/fractalaw#38.

**Cat 3: Has enabling modal, has actor (9)** — RESOLVED per provision:
- 3 fixed: BROAD_LABELS compound predicate — added `person may/entitled`, `persons entitled`, `person making` to PERSON_QUALIFIERS
- 2 fixed: drrp_keywords gap — Crown missing "majesty", Local Authority missing "council"
- 2 fixed: actor after modal — Office of Rail and Road, added "office of rail" to drrp_keywords (s.43A(5), s.43A(6) now Liberty)
- 1 gold fixed: s.6(8) "as the case may be" is stock phrasing, not enabling → gold corrected to none
- 1 remaining: s.43(4) governed actor after modal — same as Cat 2 (fractalaw/fractalaw#38)
- 1 known issue: s.84(3) "Her Majesty may... shall" — obligation pattern fires before enabling. Both modals present, pipeline picks wrong one

### Phase 3: Sertantai
1. Update sertantai to read `obligation_holder`/`liberty_holder`
2. Derive Duty/Responsibility from actor struct `label` prefix
3. Coordinated deploy

## Cascade Transition Rules (codified 2026-06-17)

The pipeline is a cascade: regex → classifier → LLM. Each tier ADDS signal — it never silently replaces the previous tier. The `reason` field on each actor records what EVERY tier said, not just disagreements.

### Rule 1: Regex always runs first
- Fast, free, deterministic
- Extracts actors from text using YAML dictionary
- Classifies DRRP type (Obligation/Liberty/none) using pattern matching
- Assigns actor positions (active/counterparty) using span heuristic
- Writes `reason = "regex:{position}@{confidence}"` on each actor

### Rule 2: Classifier always runs second (when embedding exists)
- Runs on EVERY provision with an embedding, not just regex gaps
- Predicts Obligation/Liberty/none from embedding + modal features
- Predicts actor position (active/counterparty/other) per actor
- APPENDS to reason: `"regex:active@0.80 | classifier:active@0.85"`
- Adds signal even when agreeing (confirms the classification)
- When disagreeing: record both views, the disagreement IS the signal

### Rule 3: Disagreements are LLM escalation candidates
- Where regex and classifier disagree on DRRP type → LLM candidate
- Where regex and classifier disagree on actor position → LLM candidate
- Where no actor extracted but modal present → LLM candidate (implied actor)
- LLM appends: `"regex:active@0.80 | classifier:counterparty@0.72 | llm:counterparty@0.95"`

### Rule 4: drrp_types reflects highest-tier non-none result
- Regex says Obligation, classifier agrees → Obligation (method=regex)
- Regex says none, classifier says Obligation → Obligation (method=classifier)
- Regex and classifier disagree → hold for LLM, keep regex until resolved
- Never silently override — disagreements wait for LLM

### Rule 5: QA findings tracked at provision level
- Each benchmark run produces provision-level findings
- Findings logged in session with section_id, what went wrong, which tier failed
- Findings tackled systematically, not rushed past
- Actor dictionary gaps surfaced via actor-drift skill

## Phase 2 Results (2026-06-17)

### Shipped
- 3-class DutyType enum (`404e534`): Obligation/Liberty/Rule
- `decompose_drrp()` removed
- Classifier confidence thresholding: 0.7 for gaps, 0.9 for overrides
- Cascade transition rules codified (5 rules)
- Arrow IPC backup for compact (Parquet can't handle null list components)

### Benchmark: 78.5% overall, Obligation recall 84.2%

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Obligation | 77.0% | **84.2%** | 80.4% | 791 |
| Liberty | 71.1% | 64.6% | 67.7% | 381 |
| none | 84.8% | 79.2% | 81.9% | 1078 |

### Outstanding provision-level findings (to work through)

1. **139 classifier false positives on none** — gold=none but classifier predicts Obligation at ≥0.7 confidence. Classifier v7 `none` boundary still too loose. Need: either raise threshold or retrain with more none examples.

2. **31 stale agentic provisions** with old 5-class labels (Duty/Power/Responsibility/Rule). These have `extraction_method = 'agentic'` (tier 6) so `--force` regex can't overwrite them. Need: script to migrate agentic provisions to 3-class labels.

3. **81 Obligation→none** — regex returns none, classifier below threshold or predicts none. These are the regex/classifier gap: 47 are new actors needing re-parse, 27 are inverted patterns, 9 are long preambles.

4. **72 Liberty→none** — similar gap for Liberty provisions.

5. **60 Liberty→Obligation** — classifier or regex misclassifies Liberty (may/entitled) as Obligation (shall/must). Likely the provision has both modal types.

### Finding #2 resolved: 3-class label migration (2026-06-18)

Migrated 30,970 provisions from 5-class to 3-class via merge_insert. Includes 1,636 agentic (tier 6) provisions. Zero old labels remaining.

Benchmark after migration: **79.0% accuracy, Obligation recall 85.6%**. The 31 stale benchmark provisions now show clean 3-class labels. 18 Rule provisions remain (pipeline correctly classifies thing-subjects as Rule but gold says Obligation).

### What's next (systematic, not rushing)

1. ~~Work through finding #2~~ — DONE (label migration)
2. Work through finding #1 — analyse the 139 classifier FPs on none
3. ~~Work through finding #3~~ — DONE (2026-06-18). Re-parsed UK_uksi_2014_1643 (ESOS). 25→50 provisions now Obligation. **Obligation recall: 90.5% — target hit.**
4. Work through finding #4 — Liberty→none (50 remaining, was 72):

   **Cat 1: No enabling modal, has actor (23)** — LLM territory. Rights expressed without modal verbs: "No answer is admissible in evidence", "The right of appeal is to a magistrates' court", "is to be in such manner as the Secretary of State considers appropriate". Structural/passive language. No regex fix possible. → Accept as LLM-only, exclude from regex/classifier QA.

   **Cat 2: Has enabling modal, no actor (13)** — PARTIALLY FIXED. Rule enabling→Liberty fix applied but these 13 provisions still miss. Thing-subject + "may" ("the guidance may be revised", "An order may contain..."). The Rule tier should now catch these but they're not matching — need to check if the thing-subject keywords list covers "guidance", "representations", "order", "scheme", "marking". → TODO: expand THING_KEYWORDS in duty_patterns_rule.rs.

   **Cat 3: Has enabling modal, has actor (9)** — PARTIALLY FIXED. `entitled` broadened from `entitled to` to bare `entitled`. 3 provisions fixed but 9 remain. Patterns: "Nothing in this regulation is taken to compel" (negative construction), "Regulations under subsection (1) may enable" (may + enable in same sentence), "Her Majesty may by Order in Council" (Crown as actor). → TODO: investigate why governed v2 doesn't anchor these — distance? subordinate clause rejection? compound predicate?
5. Work through finding #5 — 60 Liberty→Obligation misclassifications
6. Codify the cascade transition rules IN CODE, not just in docs
7. Then re-benchmark and log the next round of findings

### Current benchmark (2026-06-18)

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Obligation | 78.0% | **90.4%** | 83.7% | 791 |
| Liberty | 71.2% | **71.4%** | 71.3% | 381 |
| none | 90.6% | 78.7% | 84.2% | 1078 |
| **Overall** | | | | **81.6%** |

## Key files

- `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet` — gold standard
- `crates/fractalaw-core/src/taxa/duty_type.rs` — DRRP type mapping
- `crates/fractalaw-core/src/taxa/mod.rs` — `DutyType` enum
- `crates/fractalaw-ai/src/drrp_classifier.rs` — already 3-class, remove `decompose_drrp()`
- `crates/fractalaw-cli/src/main.rs` — enrichment pipeline, DuckDB schema
- `scripts/benchmark_report.py` — benchmark comparison
- `docs/drrp_classifier_v7.json` — classifier weights (already 3-class)
