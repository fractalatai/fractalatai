# Session: DRRP QA Plan — Polishing Results, Models, Code, Testing

## Resume Point (2026-06-17)

**To resume**: read this session doc + the prior sessions in `.claude/sessions/cascade/`.

### What's done
- P1-P4 code fixes shipped and QQ corpus re-enriched (274 laws, 161K provisions)
- Golden benchmarks: 2,250 provisions across 16 families on NAS (`/mnt/nas/sertantai-data/data/fractalaw-benchmarks/`)
- Benchmark baseline: **67.1% DRRP accuracy, 37.1% position accuracy** vs Gemini gold standard
- NAS backup: 20260611 (DuckDB + LanceDB + classifiers)
- Actor dictionary: 105 entries, Zenoh dictionary stream working
- Position classifier: wired in with `reason` field disagreement signal (39% of HSWA)
- Source-tier protection: `source_tier()` replaces numeric confidence hierarchy
- Classifier disagreement analysis: 1,684 actor-position pairs, classifier 56.1% vs regex 46.8%
- Embedding backfill: 7,843 benchmark provision embeddings computed, 0% gap (was 39%)
- Benchmark QA skill created (`.claude/skills/benchmark-qa/`)
- Three mismatch patterns identified from text drill-down (see below)
- EU Directive fix (`a099135`): "member state" in GOVERNMENT_ACTORS + GOV_EU_ENSURE pattern
- Purpose gate softening (`a099135`): actor presence overrides Enactment/Interpretation/ALL-skip gates
- `--force` now bypasses source-tier protection for LanceDB re-enrichment
- **DRRP accuracy: 67.1% → 70.7%** — Responsibility recall 49% → 76.6%, Duty recall 43→45%
- PERSON_QUALIFIERS expanded (`2ce6ca2`): "the responsible person must", "an authorised person must", etc.
- Regex ceiling identified: ~200 provisions unreachable by regex (no actors, no modal, or structural mismatch)

### Benchmark progress (2026-06-17)

| DRRP Type | Recall | Target | Gap | Notes |
|-----------|--------|--------|-----|-------|
| Duty | 44.6% | 90% | -45.4pp | 155 classified as `none`, need classifier |
| Right | 30.2% | 90% | -59.8pp | 44 classified as `none`, weakest |
| Responsibility | 76.6% | 90% | -13.4pp | EU fix landed, close |
| Power | 52.4% | 90% | -37.6pp | 70 classified as `none` |
| none | 90.5% | 90% | at target | ✓ |
| **Overall** | **70.7%** | **90%** | **-19.3pp** | |

### Regex ceiling analysis

Of 1185 gold DRRP provisions, the pipeline misses 265. Root causes:

| Category | Count | Fix layer |
|----------|-------|-----------|
| No actor + no modal | ~24 | LLM only |
| No actor + modal | ~120 | Classifier (entity gap: NDA, Administrator, scheme administrator, compliance body, responsible undertaking, manufacturers) |
| Actor + no modal | ~42 | Classifier |
| Actor + modal but pattern miss | ~66 (partially fixed) | Some regex-fixable, rest classifier |
| Remaining gated | ~13 | Purpose gate or amendment |

**Key entity gaps**: NDA, Administrator, scheme administrator, compliance body, responsible undertaking, manufacturers, member of the Constabulary, Her Majesty — these are real duty-bearers not in the actor dictionary. Adding them would move ~20-30 provisions from "no actor" to "actor + modal" → regex-catchable.

### What's next: Regex → Classifier → LLM transition

**Strategy**: Regex handles the structural patterns it can (70.7% and ceiling ~75% with dictionary expansion). The DRRP classifier (Tier 2) handles provisions with embeddings but no regex match. LLM (Tier 3) handles the rest.

1. **Actor dictionary expansion** — add ~10 entity patterns (NDA, Administrator, etc.) to recover ~20-30 provisions for regex. Quick win.
2. **DRRP classifier retraining** — retrain with the 2,250 benchmark provisions as additional training data. The classifier currently only covers Obligation/Liberty/none — expand to 5-class (Duty/Right/Responsibility/Power/none).
3. **Classifier transition trigger** — when regex returns `drrp_types = []` AND the provision has an embedding AND has a modal verb → escalate to Tier 2 classifier.
4. **LLM escalation trigger** — when classifier confidence < 0.7 OR classifier disagrees with regex → escalate to Tier 3 LLM.
5. **Retry 6 failed benchmark laws** — Gemini rate-limited
6. **Position classifier P5 fix** — retrain with better features
7. **Full regression test suite** — codify learnings
8. **Publish QQ corpus to sertantai**

### Key files
- Session doc: `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md` (this file)
- Benchmark generator: `scripts/generate_benchmarks.py` (+ `_batch.py`)
- Benchmark report: `scripts/benchmark_report.py`
- Code review results: `data/code-review/` (5 files, gitignored)
- Gemini review cache script: `scripts/gemini_code_review.py`
- Benchmarks on NAS: `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet`

### Key metrics to track
- DRRP accuracy vs benchmark: currently 67.1% (target: 80%+)
- Position accuracy: currently 37.1% (target: 60%+)
- Duty recall: 47% (biggest gap — pipeline misses half the duties)
- Right recall: 37% (pipeline misses most rights)

## Context

**Prior sessions**: actor-labels, sync-watch-enrichment, position-classifier
**Trigger**: The heavy lifting is done. Two workflows operational:
- **Development**: regex + Tier 1 inheritance + Tier 2/3 LLM (Gemini) — `--gap-c --laws`
- **Production**: regex + embed + DRRP classifier + position classifier — `--pending`

Now we shift from building to polishing. The goal is production-quality DRRP data across the QQ corpus.

## QA Goals

### 1. Code Review with Gemini

Deep review of the dev and production DRRP parse code paths. Not a surface lint — a domain-aware review asking:

- Is the regex DRRP matching missing common UK/EU legislative patterns?
- Are the purpose gates (skip-DRRP) too aggressive or too lenient?
- Is the confidence protection logic correct at every boundary?
- Are there edge cases in the actor position heuristic?
- Is the classifier feature engineering sound (modal features, category encoding)?
- Are there race conditions in the merge_insert write patterns?

**Files to review:**
- `fractalaw-core/src/taxa/mod.rs` — parse_v2 pipeline, position heuristic
- `fractalaw-core/src/taxa/duty_type.rs` — DRRP classification
- `fractalaw-core/src/taxa/duty_patterns_v2.rs` — actor-anchored regex patterns
- `fractalaw-core/src/taxa/purpose.rs` — purpose gate
- `fractalaw-cli/src/main.rs` — enrichment loop, Tier 2/3, embed+classify pipeline
- `fractalaw-ai/src/drrp_classifier.rs` — DRRP classifier
- `fractalaw-ai/src/position_classifier.rs` — position classifier

### 2. Golden Benchmark Set — Per-Family Gold Standard

For each family in the QQ corpus, select one challenging Act and one challenging SI. Parse them fully with Gemini (agentic, highest quality) to create a ground-truth benchmark.

**Purpose:**
- Regression testing: re-run the production pipeline and compare against Gemini gold standard
- Precision/recall per family, per DRRP type, per actor position
- Catch regressions when code changes — the benchmark is the safety net

**Selection criteria for "challenging":**
- Multi-actor provisions (employer + employee + inspector)
- Mixed DRRP types within a single law (duties, rights, powers)
- Cross-reference heavy (schedule provisions, amendment clauses)
- Non-standard language ("It shall be the duty of..." vs "The employer shall ensure...")

**Deliverable**: a `data/golden-benchmarks/` directory with per-family Gemini-parsed provisions, used by a `taxa qa --benchmark` command.

### 3. Ad-Hoc Human Drill-Through

Manual QA through Baserow, checking provisions the customer will see. Focus on:

- Do the DRRP types make sense for the provision text?
- Is the active actor correct? (The duty-bearer, not a mentioned actor)
- Are counterparties reasonable?
- Do repealed/empty provisions correctly have no DRRP?
- Are cross-reference provisions correctly skipped?

**Process**: pick 5-10 provisions per family, check in Baserow, flag issues, trace back to the pipeline stage that caused them.

### 4. Surfacing Classifier Disagreements

The position classifier wrote `reason = "classifier:active@0.82"` for 346/899 HSWA provisions. Scale this across the QQ corpus:

- Run `enrich --pending` on the full QQ corpus to populate position classifier reasons
- Query all provisions with `reason LIKE 'classifier:%'` — aggregate by:
  - Actor category (Org/Ind/Gvt — which categories disagree most?)
  - DRRP type (are Duty provisions more contested than Power?)
  - Family (are some families systematically wrong?)
  - Confidence (high-confidence disagreements are the most interesting)
- Prioritise high-confidence disagreements for human review
- Feed human-validated corrections back into position classifier training data

**Deliverable**: a disagreement report and a batch of human-validated corrections.

### 5. AI-Suggested QA Methods

Approaches that Claude or Gemini can bring to the table:

**a. Cross-provision consistency checks:**
- Within a law, do parent and child provisions have consistent DRRP? (e.g., if s.3 is Duty, are s.3(1), s.3(2) also Duty or consistent sub-types?)
- Do actors propagated by Tier 1 inheritance make sense for the child provision's text?

**b. Statistical anomaly detection:**
- Laws where the DRRP distribution is unusual (e.g., 100% Duty and no Rights — suspicious)
- Provisions where the classifier confidence is very low (<0.5) — uncertain, likely wrong
- Actors that appear as active in >90% of provisions — might be a regex bias

**c. Embedding space analysis:**
- Cluster provisions by embedding similarity — do DRRP types cluster coherently?
- Outlier provisions that are semantically similar to a group but have different DRRP

**d. Gemini spot-check sampling:**
- Random-sample 50-100 provisions from the classifier pipeline, send to Gemini for independent verification
- Compare Gemini's DRRP + position against classifier's — disagreements are the highest-value QA items
- This is the existing `drrp-qa` skill but targeted at classifier output specifically

**e. Regression test suite:**
- Codify known-good provisions as unit tests
- After any code change, verify these provisions still parse correctly
- Start with the s.3(1) case (employer = active, person = counterparty)

**f. Coverage gap analysis:**
- Which provision types (section_type) have the lowest DRRP coverage?
- Which families have the most null DRRP provisions?
- Are there systematic patterns in what the regex misses vs what the classifier catches?

## Gemini Review (2026-06-11)

Full review: `docs/reviews/gemini-drrp-qa-plan-review-20260611.md`

Key additions from Gemini:
- Create initial regression tests alongside golden benchmarks, not after
- Run coverage gap analysis early — informs benchmark selection
- Add model explainability (SHAP) for debugging high-confidence disagreements
- 39% position disagreement rate is concerning but expected for v1 — iterate fast
- **Single most impactful first action**: code review of `duty_patterns_v2.rs` + `purpose.rs`

## Code Review Complete — Findings by Priority

Gemini reviewed 52K tokens of pipeline code against a 6-hour cache (`cachedContents/gr8fjs0ls3kt5htpmtaszat4vhkng1n0cga586em`). Five targeted reviews produced actionable findings. Full results in `data/code-review/`.

### P1 CRITICAL: Position heuristic — government patterns have no span — FIXED (`3acbdaf`)

Government patterns (`match_government_v1/v2`) don't populate the `MatchSpan` field. All Responsibility and Power provisions classified via government patterns get **no actor positions at all** — the `actor_positions` HashMap stays empty, so everyone defaults to "active". This affects every government-sourced DRRP provision in the corpus.

**Fix**: propagate span from government pattern matches, same as governed patterns.

### P2 CRITICAL: Confidence protection — hierarchy not fully implemented — FIXED (`0e6c150`)

The `taxa_confidence` written to LanceDB reflects regex routing heuristics (0.30/0.80/0.90), not the classifier or LLM confidence. The protection mechanism compares these routing scores against each other, which doesn't correctly implement the intended agentic > classifier > regex hierarchy. A regex provision scored at 0.90 (structural/no-actors) blocks classifier updates at 0.85.

**Fix**: separate routing confidence from classification confidence, or ensure the confidence values correctly reflect the cascade hierarchy.

### P3 ~~CRITICAL~~ LOW: Purpose gate — softened for actor override — FIXED (`2f21ebf`)

Pure INTERPRETATION provisions with governed actors and modal verbs are skipped entirely. Duties embedded in definitions are missed. ENACTMENT and APPLICATION_SCOPE provisions are unconditionally skipped even when they contain substantive obligations.

**Fix**: refine the gate to check for modal verbs before skipping, not just purpose classification.

### P4 HIGH: Regex patterns — penalty false positives + missing rights — FIXED (`ad7694a`)

`PERSON_QUALIFIERS` regex matches "any person" in penalty/offence provisions, misclassifying offence provisions as duties. Missing "has a right to" / "is entitled to" patterns for right-type provisions.

**Fix**: add penalty/offence rejection in `classify_after_modal`, add right-entitlement patterns.

### P5 MEDIUM: Classifier features — text offset unreliable (`data/code-review/classifier_features.md`)

The position classifier's text offset feature fails for passive voice and complex clauses. Missing negative modal features (`shall not`, `must not`). The DRRP classifier lacks full obligation/enabling phrases.

**Fix**: add negative modal features, consider replacing text offset with a more robust signal.

## Execution Order (Revised — Fix-Driven)

1. ~~**P1 fix**: Government pattern span propagation~~ — DONE (`3acbdaf`). Added `find_government_span()`, 450/450 tests pass.
2. ~~**P3 fix**: Purpose gate~~ — DONE (`2f21ebf`). Downgraded CRITICAL→LOW (3/161K affected). Softened INTERPRETATION gate with actor override.
3. ~~**P4 fix**: Penalty clause rejection + right patterns~~ — DONE (`ad7694a`). 538 penalty FPs rejected, 311 rights recoverable. 454/454 tests.
4. ~~**P2 fix**: Confidence hierarchy~~ — DONE (`0e6c150`). Replaced numeric confidence with source_tier() based on extraction_method.
5. **P5 fix**: Classifier feature improvements — retrain with better features (deferred, MEDIUM)
6. ~~**Re-enrich QQ corpus**~~ — DONE. 274 laws re-enriched with P1-P4 fixes. NAS backup 20260611.
7. ~~**Golden benchmarks**~~ — DONE (`8fc62b0`). **2,250 provisions across 16 families** with Gemini gold standard on NAS. Baseline: **67.1% DRRP accuracy, 37.1% position accuracy**. 6 laws need retry (Gemini rate limits). Key gaps: Duty recall 47%, Right recall 37%. Scripts: `generate_benchmarks.py`, `generate_benchmarks_batch.py`, `benchmark_report.py`.
8. ~~**Classifier disagreements**~~ — DONE (2026-06-17). Position classifier vs regex vs Gemini gold on 2,250 benchmark provisions. See analysis below.
9. **Gemini spot-check** — 50-100 provisions verified (partially covered by benchmarks — 2,250 already verified)
10. **Ad-hoc human drill-through** — Baserow validation
11. **Full regression test suite** — codify all learnings
12. **Publish QQ corpus** — propagate P1-P4 fixes to sertantai (held pending sertantai code review)

## Golden Benchmark Plan

### Purpose

A per-family set of provisions with Gemini-verified DRRP types + actor positions, stored as Parquet on NAS. Used to:
- Measure precision/recall of the production pipeline against ground truth
- Catch regressions after code changes
- Track improvement over time as the pipeline evolves

### Three tiers

**Tier 1: Regression anchors (free, code-driven)**
Provisions already debugged during development — s.3(1) employer active, s.33 penalty clause, s.21 inspector power, government regulation-making. Already exist as unit tests across 454 tests. Tier 1 is about organising them into a runnable benchmark, not creating new data.

**Tier 2: Per-family Gemini gold standard (API cost, one-time)**
For each family in the QQ corpus, select one challenging Act and one challenging SI. Send 50-100 provisions from each to Gemini with a structured prompt:

```
For this provision, classify:
1. DRRP type: Duty / Right / Responsibility / Power / none
2. For each actor, their Hohfeldian position: active / counterparty / beneficiary / mentioned
3. Briefly explain your reasoning

Provision: {section_id}
Text: {text}
Actors found: {actor_labels}
```

Store Gemini's response as ground truth in Parquet: `section_id, gold_drrp_types, gold_actors (JSON with positions), gemini_reasoning`.

**Selection criteria for "challenging" laws:**
- Multi-actor provisions (employer + employee + inspector)
- Mixed DRRP types within a single law (duties, rights, powers in the same Act)
- Penalty/offence provisions (tests the P4 rejection fix)
- Government powers (tests the P1 span fix)
- Non-standard language ("It shall be the duty of..." inverted pattern)

**Tier 3: Human-validated disagreements (highest quality, ongoing)**
The position classifier writes `reason = "classifier:active@0.82"` for disagreements. Once a human reviews in Baserow and corrects, the corrected data enters the benchmark. This grows over time.

### Storage

```
/mnt/nas/sertantai-data/data/fractalaw-benchmarks/
  tier1-regression-anchors.parquet      # unit test provisions
  tier2-ohs-occupational.parquet        # OH&S family gold standard
  tier2-fire.parquet                    # Fire family gold standard
  tier2-environmental-protection.parquet # etc.
  tier3-human-validated.parquet         # growing from QA
```

Schema per Parquet:
```
section_id: Utf8
law_name: Utf8
family: Utf8
text: Utf8
gold_drrp_types: List<Utf8>
gold_actors: Utf8  (JSON: [{label, position}])
gold_source: Utf8  ("gemini" | "human" | "unit_test")
created_at: Timestamp
```

### CLI command: `taxa qa --benchmark`

New QA mode that:
1. Loads benchmark Parquet from NAS (or local copy)
2. Queries LanceDB for the same section_ids
3. Compares: DRRP type match, actor position match
4. Reports per-family precision/recall + overall
5. Lists specific mismatches for investigation

### Implementation steps

1. **Select laws**: query DuckDB for one Act + one SI per QQ family, picking laws with the most diverse DRRP distribution
2. **Extract provisions**: query LanceDB for 50-100 regulation-level provisions per law
3. **Send to Gemini**: structured prompt, cache the law text, classify each provision
4. **Store as Parquet**: write to NAS benchmark directory
5. **Build `taxa qa --benchmark`**: load Parquet, diff against LanceDB, report

### Cost estimate

- ~20 QQ families × 2 laws × 75 provisions = ~3,000 Gemini calls
- At Gemini 2.5 Flash pricing with caching: ~$1-2 total
- One-time cost, benchmark is reusable indefinitely

### Gemini Review Feedback (2026-06-11)

**Endorsed.** Three-tier structure is excellent. Key refinements:

1. **Merge for usage**: `taxa qa --benchmark` loads all tiers together for overall metrics, but reports Tier 1 separately as critical regression anchors
2. **Random sampling**: within a selected law, randomly sample the 50-100 provisions — don't cherry-pick
3. **Prompt design**: add role persona ("expert legal analyst"), explicit JSON format, definitions of DRRP types and positions. Do NOT provide regex output — Gemini must classify independently
4. **Schema**: consider `List<Struct>` for gold_actors instead of JSON string (easier to query). Add provision uniqueness check (`section_id` is already globally unique in this codebase)
5. **Metrics**: add F1-score, confusion matrix (DRRP and position separately), per-type P/R/F1, micro+macro averages, coverage percentage
6. **Biggest risk**: Gemini as ground truth is LLM-checking-LLM. Mitigate by prioritising human review (Tier 3) for any provision where Gemini disagrees with the pipeline — those are the highest-value cases. The benchmark is a regression tool, not the source of truth.

### What this does NOT do

- Does not replace human QA — Gemini is LLM-checking-LLM, useful for scale but not authoritative
- Does not cover every provision — sampling, not exhaustive
- Does not test the sync watch pipeline — only the regex+classifier path
- Does not test sertantai's consumption — only fractalaw's output

## Classifier Disagreement Analysis (2026-06-17)

Ran the position classifier against 2,250 golden benchmark provisions. Script: `scripts/benchmark_classifier_disagreements.py`. Skill: `.claude/skills/benchmark-qa/`.

### Coverage

- 2,250 benchmark provisions → 1,211 analysable (with embedding + non-agentic)
- 879 skipped (no embedding — 39% of benchmarks lack embeddings)
- 160 skipped (agentic — already gold standard)
- 558 skipped (no actor label matches between gold and pipeline)
- 1,040 actor-position pairs evaluated

### Overall: Classifier beats regex 57.1% vs 43.9%

| Outcome | Count | % |
|---------|-------|---|
| Both correct | 254 | 24.4% |
| Classifier only correct | 340 | 32.7% |
| Regex only correct | 203 | 19.5% |
| Both wrong | 243 | 23.4% |
| **Total disagreements** | **681** | **65.5%** |

### Key finding: classifier advantage is entirely from non-DRRP provisions

| DRRP | Regex% | Classifier% | Winner |
|------|--------|------------|--------|
| Duty | 65.7% | 53.9% | **Regex** |
| Responsibility | 67.4% | 53.3% | **Regex** |
| Power | 52.9% | 47.1% | **Regex** |
| Right | 47.5% | 22.0% | **Regex** |
| none | 6.2% | 74.2% | **Classifier** |

**Interpretation**: Regex correctly identifies duty-bearers when there IS a duty, but wrongly assigns "active" to every actor in non-DRRP provisions (definitions, interpretations, penalties). The classifier correctly demotes those to "other" but over-corrects on real duty provisions, demoting actual duty-bearers.

### By actor category

| Category | Pairs | Regex% | Cls% | Disagreement% |
|----------|-------|--------|------|----------------|
| Org | 90 | 51.1% | 81.1% | 55.6% |
| Spc | 49 | 44.9% | 77.6% | 40.8% |
| EU | 35 | 8.6% | 42.9% | 62.9% |
| other | 67 | 40.3% | 64.2% | 61.2% |
| SC | 76 | 64.5% | 71.1% | 46.1% |
| Gvt | 354 | 45.8% | 52.8% | 65.8% |
| Ind | 369 | 40.1% | 49.9% | 75.9% |

Classifier is strongest on Org (81%) and Spc (78%) — categories with clear role semantics in legislation.

### Actionable insights

1. **Ensemble approach**: trust regex positions for DRRP provisions (Duty/Right/Responsibility/Power), trust classifier for non-DRRP provisions (none). This would combine the best of both: ~66% regex accuracy on DRRP + ~74% classifier accuracy on none.

2. **Embedding gap**: 39% of benchmark provisions lack embeddings — these are invisible to the classifier. Backfilling embeddings would increase coverage.

3. **Ind category is hardest**: 75.9% disagreement rate on Individual actors (Person, Employee). These are the most ambiguous — "any person" appears in both duty provisions (active) and penalty clauses (mentioned).

4. **High-confidence classifier errors**: The classifier predicts "other" with >0.90 confidence for many actual duty-bearers (s.5(1), s.6(8A) HSWA). This is the P5 issue — the text offset feature is unreliable for passive voice.

5. **Right recall is critical**: 22% classifier accuracy on Rights — the classifier almost never predicts active for right-holders. Rights use different modal language ("entitled to", "right to") that the classifier hasn't learned.

### Embedding backfill (2026-06-17)

Backfilled 7,843 embeddings for the 13 benchmark laws with gaps. Coverage: 0% missing (was 39%). Analysable pairs increased from 1,040 → 1,684 (+62%). Updated metrics in the table above reflect post-backfill state.

### Text pattern drill-down (2026-06-17)

Added `--text` flag to `benchmark_classifier_disagreements.py` and examined provision text for all three mismatch categories. Three clear linguistic patterns emerge:

#### Pattern 1: EU Directive subordinate clauses (affects Nuclear, Climate Change, Energy)

**Text**: "Member States shall ensure that [workers/persons] are given adequate training..."

**Problem**: The duty-bearer is "Member State" but the regex finds "Worker" in the text, sees "shall", and marks Worker as active. The Worker is actually the *object* of protection — a counterparty or beneficiary.

**Scale**: 215 Nuclear pairs at 90.2% disagreement; 196 EU-category actors at 81.6% disagreement. This single pattern drives the majority of Nuclear/EU mismatches.

**Fix approach**: Detect "Member States shall ensure that..." pattern in regex. When the main clause actor is a Member State and the subordinate clause contains another actor, the subordinate actor should NOT be marked active.

#### Pattern 2: Inverted duty phrasing (affects OH&S, Energy, Fire)

**Text**: "It shall be the duty of the person having control..." / "Nothing in ... shall relieve any person from any duty"

**Problem**: Classifier predicts "other" at 0.98+ confidence for these actors. The training data is dominated by "X shall [verb]" patterns; the inverted "It shall be the duty of X" pattern is underrepresented.

**Scale**: Visible in top regex-wins — s.5(1), s.6(8A), s.18(2) HSWA/Energy. High confidence errors (>0.95) that affect core OH&S provisions.

**Fix approach**: Add inverted duty phrases as training examples for the position classifier. Also consider adding regex patterns for "It shall be the duty of [actor]" → mark actor as active.

#### Pattern 3: Counterparty detection (the "both wrong" problem)

**Text**: "shall ensure that workers are given..." / "shall not be disclosed without the consent of the person" / "may serve on the responsible person a notice"

**Problem**: Gold says counterparty, regex says active, classifier says other. Neither system has a signal for "this actor is the target/object of the obligation, not the bearer." Counterparty detection requires understanding clause structure — who does the verb apply *to*?

**Scale**: Almost all "both wrong" cases (197) are gold=counterparty. This is the hardest category because it requires parsing the relationship between two actors within a clause.

**Fix approach**: Heuristic patterns for common counterparty signals: "serve on [actor] a notice", "consent of [actor]", "ensure that [actor] is/are [given/provided]". Longer-term: clause structure parsing to identify subject vs object actors.
