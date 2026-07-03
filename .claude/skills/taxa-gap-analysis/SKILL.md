# Skill: Taxa DRRP Gap Analysis

## When This Applies

When analysing a law family's taxa DRRP (Duty, Right, Responsibility, Power) coverage — measuring precision/recall, diagnosing gaps, improving patterns, and verifying fixes. This is the full lifecycle: baseline → diagnose → fix → verify.

**Trigger**: User asks to run a gap analysis on a family, or to improve DRRP coverage for a set of laws.

**Prerequisite**: Run the `lat-qa` skill first to validate upstream data quality. There is no value analysing DRRP coverage if the enricher is truncating provisions or the input data is malformed.

**First step**: Ask which **family** to analyse (e.g. `PUBLIC`, `OH&S: Offshore Safety`). Query available families with:
```bash
cargo run -p fractalaw-cli -- query "SELECT DISTINCT family FROM legislation ORDER BY family" 2>/dev/null
```

**Session log**: Create a new session in `.claude/sessions/taxa-drrp/taxa-gap-analysis/` following the naming convention `MM-DD-YY-<slug>.md`. Use previous sessions as format reference.

## Architecture — Two Parallel Actor Systems

The taxa pipeline (`crates/fractalaw-core/src/taxa/`) has two actor-detection mechanisms:

1. **`actors.rs`** — Comprehensive regex-based extraction with word-boundary matching. Returns structured labels like `Ind: Person`, `Org: Employer`, `SC: C: Contractor`. Supports **family-gated specialist defs** (e.g. `OFFSHORE_GOVERNED_DEFS` only run for `OH&S: Offshore` laws).

2. **`duty_patterns_v2.rs` `match_governed_v2()`** — Actor-anchored pattern matching. For each governed actor label extracted by `actors.rs`, builds anchored regexes requiring the actor keyword in subject position relative to a modal verb within a 200-char window.

**The gap**: `actors.rs` must extract the actor label first. If the keyword isn't in `GOVERNED_DEFS` (or a family-gated specialist list), the provision gets no governed DRRP classification.

## Gap Taxonomy

- **Gap A**: Actor extracted by `actors.rs` but v2 pattern matcher doesn't fire — actor is present but DRRP classification fails. Sub-categories: v2 window miss, subordinate clause rejection, epistemic "may" false positive.
- **Gap B**: Actor keyword exists in text but `actors.rs` boundary matching fails. See `actors-boundary-analysis` skill.
- **Gap C**: Truly actor-less obligations — passive voice ("equipment must be maintained"), application/fitness provisions. Beyond regex, AI frontier.

## Where the Data Lives

**CRITICAL**: Legislative text and taxa enrichment data are in **LanceDB**, NOT DuckDB/DataFusion.

- LanceDB table: `legislation_text` in `data/lancedb/`
- DuckDB table: `legislation` — law-level metadata only (accessed via `fractalaw query`)
- The CLI `query` command hits DataFusion which does NOT have `legislation_text`
- Use `fractalaw taxa show <LAW_NAME>` to see live taxa classifications (runs `parse_v2()` live)
- Use `fractalaw taxa enrich --laws <comma-separated>` to re-enrich after code changes

**Python**: Use `/usr/bin/python3` (system Python with lancedb installed), NOT bare `python3` (which may resolve to brew Python without deps).

Key LanceDB columns in `legislation_text`:
- `drrp_types` (list<string>) — stored DRRP classification, empty = no classification
- `governed_actors` (list<string>) — actor labels extracted by `actors.rs`
- `government_actors` (list<string>) — government actor labels
- `purposes` (list<string>) — purpose classification
- `taxa_confidence` (float) — clause confidence score
- `clause_refined` (string) — extracted clause text
- `text` (string) — full provision text

See `lancedb-validation` skill for detailed query patterns and pyarrow recipes.

## Pipeline Overview

```
Provision text (LanceDB)
  │
  ├─ 1. Purpose classifier (purpose.rs)         → what type of provision?
  │     Gate: skip DRRP if Interpretation/Amendment/Repeal-primary
  │     Override: gate bypassed if governed actor present (Interpretation-primary only)
  │
  ├─ 2. Actor extraction (actors.rs)             → who is mentioned?
  │     Core GOVERNED_DEFS + family-gated specialists + GOVERNMENT_DEFS
  │
  ├─ 3. Duty patterns (duty_patterns*.rs)        → who must do what?
  │     Tier order: governed v2 → gov v1 → gov v2 → rule → empty
  │     v2: actor-anchored pattern matching (governed actors)
  │     v1/v2: keyword-based (government actors)
  │     rule: thing-subject rules ("equipment must be...")
  │
  ├─ 4. POPIMAR classifier (popimar.rs)          → management category
  │
  └─ 5. Fitness extraction (fitness.rs)          → who/what/where applies
```

## Workflow

### Phase 1: Baseline

#### Step 1.1: Family Profile

```bash
# Summary counts
cargo run -p fractalaw-cli -- query "
SELECT
  count(*)::BIGINT AS total,
  count(CASE WHEN is_making = true THEN 1 END)::BIGINT AS making,
  count(CASE WHEN is_making = true AND body_paras > 0 THEN 1 END)::BIGINT AS making_with_text,
  count(CASE WHEN duty_holder IS NOT NULL AND array_length(duty_holder) > 0 THEN 1 END)::BIGINT AS with_drrp
FROM legislation
WHERE family = '<FAMILY>'" 2>/dev/null

# List all laws
cargo run -p fractalaw-cli -- query "
SELECT name, title, is_making, body_paras
FROM legislation WHERE family = '<FAMILY>' ORDER BY name" 2>/dev/null
```

DuckDB gotchas:
- DRRP columns (`duty_holder`, `duty_type`, `role`) are **arrays** — use `array_length()`, `array_has()`, `array_to_string()`, not `LIKE` or `length()`
- Column is `is_making` not `making`
- Cast counts with `::BIGINT` for clean display

#### Step 1.2: QA Report

```bash
cargo run -p fractalaw-cli -- taxa qa --family "<FAMILY>"
```

Produces 4 sections: Coverage Summary, Purpose Distribution (with anomaly flags), Gate Analysis, Anomaly Detection.

#### Step 1.3: Confusion Matrix

Build a 2x2 matrix from LanceDB stored enrichment data:

```python
#!/usr/bin/env /usr/bin/python3
import lancedb, pyarrow as pa, pyarrow.compute as pc, re
from collections import Counter

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")

# Get law names from Step 1.1 — only include laws with LAT data
laws = [...]  # fill from QA report (laws with >0 provisions)

ANY_MODAL_RE = re.compile(
    r'\b(shall|must|is required to|has a duty|may|power to|entitled to)\b',
    re.IGNORECASE
)
STRUCTURAL = {"Interpretation+Definition", "Amendment", "Repeal", "Enactment+Commencement"}

tp = fp = tn = fn_ = fn_gap_a = fn_gap_c = 0
fn_obligation = fn_enabling = 0
gap_a_actors = Counter()
gap_a_gov_actors = Counter()

for law in laws:
    data = tbl.search().where(f"law_name = '{law}'").select(
        ["law_name", "text", "drrp_types", "governed_actors", "government_actors", "purposes"]
    ).limit(5000).to_arrow()

    for i in range(data.num_rows):
        text = data.column("text")[i].as_py() or ""
        drrp = data.column("drrp_types")[i].as_py() or []
        gov = data.column("governed_actors")[i].as_py() or []
        gvt = data.column("government_actors")[i].as_py() or []
        purposes = data.column("purposes")[i].as_py() or []

        has_modal = bool(ANY_MODAL_RE.search(text))
        is_structural = bool(purposes and purposes[0] in STRUCTURAL)
        has_drrp = bool(drrp)

        expected = has_modal and not is_structural

        if has_drrp and expected:     tp += 1
        elif has_drrp and not expected: fp += 1
        elif not has_drrp and not expected: tn += 1
        else:
            fn_ += 1
            if gov or gvt:
                fn_gap_a += 1
                for a in gov: gap_a_actors[a] += 1
                for a in gvt: gap_a_gov_actors[a] += 1
            else:
                fn_gap_c += 1
            obl = re.search(r'\b(shall|must|is required to|has a duty)\b', text, re.I)
            if obl: fn_obligation += 1
            else: fn_enabling += 1

precision = tp / (tp + fp) if (tp + fp) else 0
recall = tp / (tp + fn_) if (tp + fn_) else 0
f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
```

**Ground truth heuristic**: "Expected positive" = any modal verb (shall/must/may/power to) + operative purpose (not interpretation/amendment/repeal/enactment).

**Reading the matrix**:

|  | Predicted: DRRP | Predicted: No DRRP |
|--|----------------:|-------------------:|
| **Expected: DRRP** | TP | FN |
| **Expected: No DRRP** | FP | TN |

- **Precision** = TP / (TP + FP) — target >95%
- **Recall** = TP / (TP + FN) — improves as we add actors/patterns
- **FN Gap A** (actor present, no DRRP) — addressable via regex
- **FN Gap C** (no actor, no DRRP) — passive voice, AI frontier

### Phase 2: Diagnose

#### Step 2.1: Per-Law FN Breakdown

Run per-law TP/FN/recall to identify which laws drive the most misses. Look for:
- Laws with very low recall (<20%) — investigate provision structure
- Laws where one law dominates FN — may be a special case (e.g. Online Safety Act)
- Laws with high FN-A — most addressable

#### Step 2.2: Classify Gap A vs Gap C

From the confusion matrix actor counters:
- **Gap A governed**: Tabulate actor labels. Check each against GOVERNED_DEFS — is the actor already known?
  - If YES: v2 matcher is failing (subordinate clause, epistemic may, text cleaner artefacts)
  - If NO: candidate for adding to GOVERNED_DEFS or family-gated specialist
- **Gap A government**: Expected — gov v1/v2 patterns are keyword-based with structural limitations
- **Gap C**: Count and note — these are the AI frontier, not addressable via regex

#### Step 2.3: Domain-Specific Actor Survey

Search FN provisions for domain-specific keywords not in current patterns:

```python
DOMAIN_ACTORS = re.compile(
    r'\b(keyword1|keyword2|...)\b', re.I
)
# Count each keyword across FN provisions
# Sample 2-3 provisions per keyword to verify they're duty-holders
```

For each candidate keyword, determine:
1. Is it a **duty-holder** (subject of obligation) or **object/beneficiary**?
2. Is it domain-specific to this family or cross-cutting?
3. How many FN provisions would it address?

#### Step 2.4: 0-DRRP Anomaly Investigation

For any law flagged with 0 DRRP and >10 provisions, verify it's genuinely non-making:
- Check if it's an amendment SI, commencement order, revocation order
- Check gate analysis — if 80%+ gated, likely legitimate
- Use `taxa show --misses <LAW>` to inspect hot misses

#### Step 2.5: Purpose Gate Audit

Check both directions of the purpose gate:

**False positives** (gate wrongly blocked a genuine duty):
```python
# Among gated provisions with modal+actor, classify:
# - Actor-led (text starts with actor) — highest false-positive risk
# - Scope-led (text starts with "These Regulations...") — likely genuine scope
```

**False negatives** (gate let through a scope/interpretation provision):
- Check provisions with Interpretation purpose that DO have DRRP
- Verify these are the gate-bypass cases (governed actor present)

#### Step 2.6: Government Actor Coverage

Survey `government_actors` labels across the family. Look for:
- High counts on generic labels (`Gvt: Authority`, `Gvt: Agency`) — specific bodies captured generically
- Domain-specific bodies not yet in `GOVERNMENT_DEFS`
- Add specific patterns BEFORE generic catch-alls (list is ordered by specificity)

### Phase 3: Fix

#### Step 3.1: Determine Fix Strategy

| Symptom | Fix Location |
|---------|-------------|
| Domain-specific actor not extracted | `actors.rs` — family-gated specialist defs |
| Cross-cutting actor not extracted | `actors.rs` — core GOVERNED_DEFS (careful: false-positive risk) |
| Actor extracted but no DRRP | `duty_patterns_v2.rs` — v2 matcher investigation |
| Low DRRP% from gate over-matching | `purpose.rs` or `mod.rs` `should_skip_drrp()` |
| Wrong DRRP type | `duty_type.rs` — mapping logic |
| Wrong POPIMAR category | `popimar.rs` — keyword patterns |
| Missing government body specificity | `actors.rs` — GOVERNMENT_DEFS (add specific before generic) |

#### Step 3.2: Family-Gated Specialist Actors

For domain-specific actors, follow the offshore pattern in `actors.rs`:

```rust
const <FAMILY>_GOVERNED_DEFS: &[(&str, &str)] = &[
    actor!("<Family>: <Actor>", r"(?:[\s[:punct:]])pattern(?:[\s[:punct:]])"),
];
```

Update `specialist_governed_for()` to route by family prefix.

**Critical**: Do NOT add domain-specific actors to core `GOVERNED_DEFS` — they cause false positives across other families.

#### Step 3.3: False-Positive Risk Assessment

Before adding any actor keyword:
1. Count provisions mentioning the keyword across all sample laws
2. Count how many already have DRRP (true positives — working)
3. Count how many have modal + no DRRP (would be affected)
4. **Read the affected provision texts** — is the keyword the duty-holder (subject) or just mentioned (object/beneficiary)?

Known high-risk actors:
- **"worker"** — almost always the object/beneficiary, NOT the duty-holder
- **"competent person"** — usually an appointed role, not the duty-holder
- **"person"** — too broad; existing predicates ("person who", "every person", "no person") are intentionally specific

#### Step 3.4: Test-Driven Implementation

```
1.  Write true-negative regression tests BEFORE changing anything —
    provisions that correctly get NO DRRP today
2.  Write failing true-positive tests using real provision text
3.  Implement minimal change
4.  Full suite: cargo test -p fractalaw-core --lib taxa
5.  Re-enrich: cargo run -p fractalaw-cli -- taxa enrich --family "<FAMILY>" --force
6.  Measure improvement (re-run confusion matrix)
```

**Where tests go**:
- `duty_patterns.rs mod tests` — unit tests for pattern matchers. True-negative tests.
- `mod.rs mod tests` — full-pipeline tests using `taxa::parse_v2()`. Both true-negative and true-positive.

**Test naming convention**:
```rust
// True-negative (duty_patterns.rs):
fn <keyword>_heading_no_match()
fn <keyword>_cross_reference_no_match()

// True-negative (mod.rs — full pipeline):
fn <keyword>_definition_no_drrp()

// True-positive (mod.rs — full pipeline):
fn <keyword>_duty_<description>()
fn <keyword>_prohibition()
```

### Phase 4: Verify

#### Step 4.1: Re-enrich and Measure

```bash
cargo run -p fractalaw-cli -- taxa enrich --family "<FAMILY>" --force
```

Re-run the confusion matrix from Step 1.3 and compare:

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| Recall | | | |
| Precision | | | |
| F1 | | | |
| Gap A | | | |
| Gap C | | | |

#### Step 4.2: Confidence Distribution

```python
# Check confidence scores for newly classified provisions
data = tbl.search().where(f"law_name = '{law}' AND taxa_confidence > 0").select(
    ["law_name", "taxa_confidence", "purposes", "clause_refined", "drrp_types"]
).limit(100000).to_arrow()

# Bucket distribution
bins = [(0, 0.2, "very low"), (0.2, 0.4, "low"), (0.4, 0.6, "medium"),
        (0.6, 0.8, "good"), (0.8, 0.86, "high")]
```

Confidence values: 0.85 = best (span + clean end + length + modal), 0.60 = good, 0.45 = medium, 0.20 = low.

#### Step 4.3: Eyeball Review

```bash
# Generate per-provision review for specific laws
cargo run -p fractalaw-cli -- taxa eyeball --laws <LAW_1>,<LAW_2>
```

Look for:
- **False positives** — provisions tagged with DRRP that shouldn't be
- **Misclassification** — wrong DRRP type
- **Poor clause extraction** — truncated or includes preamble
- Provisions ending without `.` or `;` get a `**[BAD END]**` marker

#### Step 4.4: Before/After Comparison

```bash
cargo run -p fractalaw-cli -- taxa show --clauses <LAW> > /tmp/before.txt
# ... make changes, re-enrich ...
cargo run -p fractalaw-cli -- taxa show --clauses <LAW> > /tmp/after.txt
diff /tmp/before.txt /tmp/after.txt
```

## Diminishing Returns

After adding domain-specific actors for a family, Gap A shrinks rapidly. When remaining Gap A is dominated by:
- "Ind: Person" (broad, high false-positive risk)
- Actor labels that are beneficiaries not duty-holders
- v2 matcher structural failures (subordinate clause + pronoun, epistemic "may")

...you've hit diminishing returns on regex improvements.

When Gap A is near zero, remaining recall gap is Gap C (passive voice, no actor). This is the AI polisher frontier — fundamentally different approach needed.

## Anomaly Thresholds

| Anomaly | Threshold | Rationale |
|---------|-----------|-----------|
| Enactment >10% | Enactment provisions are rare — high rates suggest pattern over-matching |
| Enforcement >15% | Enforcement provisions exist but shouldn't dominate |
| 0 DRRP, >10 provisions | Suspicious unless purely administrative |
| Any purpose >2x corpus avg | Individual law significantly above baseline |

### Known Low-DRRP Patterns (Not Bugs)

- **Commencement orders** — bring other Acts into force
- **Pure amendment SIs** — modify other legislation
- **Safety zone / revocation orders** — designate zones, 2-3 paragraphs
- **Climate/budget orders** — set numerical targets
- **Offence-heavy criminal law** — duties expressed as "a person who does X commits an offence" (Gap C — no governed actor in subject position)

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse_v2()`, `should_skip_drrp()`, full-pipeline tests |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction, GOVERNED_DEFS, GOVERNMENT_DEFS, family-gated specialists |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | Orchestrates tiers: governed v2 → gov_v1 → gov_v2 → rule → empty |
| `crates/fractalaw-core/src/taxa/duty_patterns_v2.rs` | Actor-anchored DRRP pattern matching |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | Government v1/v2 patterns, `has_government_actor()` |
| `crates/fractalaw-core/src/taxa/duty_patterns_rule.rs` | Thing-subject rule detection |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classification, `classify()` |
| `crates/fractalaw-core/src/taxa/confidence.rs` | Clause confidence scorer |
| `crates/fractalaw-core/src/taxa/popimar.rs` | POPIMAR management category classifier |
| `crates/fractalaw-core/src/taxa/clause_refiner.rs` | Clause text extraction (modal verb window) |
| `crates/fractalaw-core/src/taxa/clause_structure.rs` | Clause structure decomposition |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_qa()`, `cmd_taxa_eyeball()`, `cmd_taxa_show()`, `enrich_single_law()` |
| `crates/fractalaw-store/src/lance.rs` | `query_legislation_text()`, `update_taxa()` |
| `.claude/skills/lancedb-validation/SKILL.md` | LanceDB query patterns and pyarrow recipes |
| `.claude/skills/actors-boundary-analysis/SKILL.md` | Gap B: actor boundary matching failures |

## Fitness Dictionary Expansion

When `taxa audit-fitness` shows low tagged% or vocabulary gaps, expand the p-dimension dictionaries:

### Audit

```bash
fractalaw taxa audit-fitness --family "FAMILY_NAME"
fractalaw taxa audit-fitness --family "FAMILY_NAME" --limit 0  # show all gaps
```

Reports: coverage by family, gap provisions (polarity but zero tags), candidate terms (n-gram frequency), dictionary utilisation.

### Adding entries

**Core dictionaries** (always applied) — edit the relevant `*_DICT` static in `crates/fractalaw-core/src/taxa/fitness.rs`:

```rust
static PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("new\\s+term\\s+pattern", "canonical term"),
    ])
});
```

**Family specialist dictionaries** — add to existing family dict or create new:

```rust
static FOOD_PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("food\\s+business\\s+operator", "food business operator"),
    ])
});
```

Register in `specialist_dicts_for()`:

```rust
fn specialist_dicts_for(family: &str) -> Vec<(PDimension, &'static [DictEntry])> {
    if family.starts_with("OH&S") {
        vec![/* ... */]
    } else if family.starts_with("FOOD") {
        vec![(PDimension::Person, &FOOD_PERSON_DICT)]
    } else {
        vec![]
    }
}
```

### Regex tips

- Use `\b` word boundaries, `\s+` for spaces, `(?:s)?` for plurals
- Compound terms first (longer before shorter)
- Test with family: `extract(text, Some("FAMILY"))` and without: `extract(text, None)`
