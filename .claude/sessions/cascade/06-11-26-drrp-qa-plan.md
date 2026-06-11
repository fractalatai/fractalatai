# Session: DRRP QA Plan — Polishing Results, Models, Code, Testing

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

## Execution Order (Refined)

1. **Code review** (Gemini) — `duty_patterns_v2.rs` + `purpose.rs` first, then full pipeline
2. **Coverage gap analysis** — which families/section_types have lowest DRRP coverage?
3. **Golden benchmarks + initial regression tests** — one Act + one SI per family, Gemini-parsed
4. **Classifier disagreements** — run position classifier across QQ, analyse by category/family
5. **Gemini spot-check** — 50-100 random classifier provisions verified by Gemini
6. **Ad-hoc human drill-through** — Baserow validation, 5-10 provisions per family
7. **Statistical anomalies** — distribution checks, low-confidence flagging, actor bias detection
8. **Full regression test suite** — codify all learnings
