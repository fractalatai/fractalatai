# Skill: Taxa DRRP Gap Analysis

## When This Applies

When iteratively improving the taxa regex patterns to reduce DRRP (Duty, Right, Responsibility, Power) classification miss rate across UK ESH laws. The workflow is: pick a sample of laws, analyse the misses, identify the highest-value pattern to add, add it with tests, re-enrich, measure.

## Architecture — Two Parallel Actor Systems

The taxa pipeline (`crates/fractalaw-core/src/taxa/`) has two actor-detection mechanisms that serve different purposes:

1. **`actors.rs`** — Comprehensive regex-based extraction with word-boundary matching. Returns structured labels like `Ind: Person`, `Org: Employer`, `SC: C: Contractor`. Used for enrichment metadata. Supports **family-gated specialist defs** (e.g. `OFFSHORE_GOVERNED_DEFS` only run for `OH&S: Offshore` laws).

2. **`duty_patterns.rs` `GOVERNED_ACTORS`** — Short flat substring list used by `has_governed_actor()` to **gate** DRRP pattern matching. If none of these substrings appear in the text, all governed duty patterns (`match_governed()`) return `None`.

**The gap**: `actors.rs` extracts an actor label, but if the corresponding keyword isn't in `GOVERNED_ACTORS`, the provision gets no DRRP classification. This is "Gap A".

## Gap Taxonomy

- **Gap A**: Actor extracted by `actors.rs` but not in `GOVERNED_ACTORS` — provision has a modal verb but no DRRP. Fix by adding keyword to `GOVERNED_ACTORS` (if appropriate).
- **Gap B**: Actor keyword exists in text but `actors.rs` boundary matching fails. Fix in `actors.rs` regex patterns.
- **Gap C**: Truly actor-less obligations — passive voice ("equipment must be maintained"), application/fitness provisions. Hardest gap, may be better left to AI polisher.

## Where the Data Lives

**CRITICAL**: Legislative text and taxa enrichment data are in **LanceDB**, NOT DuckDB/DataFusion.

- Table: `legislation_text` in LanceDB (`data/lancedb/`)
- The CLI `query` command hits DataFusion which does NOT have `legislation_text`
- Use `fractalaw taxa show <LAW_NAME>` to see live taxa classifications
- Use `fractalaw taxa enrich --laws <comma-separated>` to re-enrich after code changes

The `taxa show` command runs `taxa::parse()` **live** against LanceDB text — it always reflects the current code, not stale stored enrichment data.

## How to Survey Misses

### Step 1: Baseline from stored enrichment data

If the family has already been enriched (most families have been on first upload), query the **stored** LanceDB columns directly. This is much faster than re-running `taxa show` across every law.

```python
import lancedb, pyarrow.compute as pc, re
from collections import Counter

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")
MODAL_RE = re.compile(r'\b(shall|must|is required to|has a duty)\b', re.IGNORECASE)

# 1. Get the law list from DuckDB
#    cargo run -p fractalaw-cli -- query \
#      "SELECT name FROM legislation WHERE family = '<FAMILY>' ORDER BY name"

# 2. For each law, query stored enrichment columns
for law in laws:
    data = tbl.search().where(f"law_name = '{law}'").select(
        ["law_name", "section_id", "text", "drrp_types", "governed_actors", "purposes"]
    ).limit(5000).to_arrow()

    # Count: provisions, DRRP hits, modal verbs, misses (modal + no DRRP)
    # A "miss" is a provision with a modal verb but empty drrp_types

# 3. For misses, tabulate governed_actors labels — these are Gap A candidates
#    Misses with no governed_actors at all are Gap C (actor-less obligations)
```

Key columns in `legislation_text`:
- `drrp_types` (list<string>) — stored DRRP classification, empty = no classification
- `governed_actors` (list<string>) — actor labels extracted by `actors.rs`
- `government_actors` (list<string>) — government actor labels
- `purposes` (list<string>) — purpose classification (explains why some provisions are gated)

### When to use `taxa show` instead

Use `taxa show` **after making code changes** to verify the fix works live, or to inspect individual provisions in detail. It runs `taxa::parse()` live so always reflects the current code.

```bash
# Inspect a specific law after code changes
cargo run -p fractalaw-cli -- taxa show --limit 500 UK_uksi_1995_738 2>/dev/null
```

### Step 2: Classify the misses into Gap A / B / C

From the stored data:
- **Gap A**: `governed_actors` is non-empty but `drrp_types` is empty — actor extracted but not in `GOVERNED_ACTORS`
- **Gap B**: `governed_actors` is empty but the text contains an actor keyword that `actors.rs` should have matched — boundary matching failure
- **Gap C**: No actor in text at all — passive voice, actor-less obligations

Count actor labels in Gap A misses. Ignore actors already in GOVERNED_ACTORS that appear in interpretation/amendment provisions (purpose = `Interpretation+Definition`) — these are correctly gated.

### Step 3: Audit governed actor candidates

Before adding any keyword:
1. Count how many provisions mention the keyword across all sample laws
2. Count how many already have DRRP (true positives — working)
3. Count how many have modal + no DRRP (would be affected)
4. Count how many have no modal (unaffected)
5. **Read the affected provision texts** — is the keyword the duty-holder (subject of obligation) or just mentioned (object/beneficiary)?

### Step 4: Audit government actor coverage

New families may reference government bodies, agencies, or regulators not in `GOVERNMENT_DEFS`. Unlike governed actors, government actor gaps don't block DRRP classification (the gov_v1/gov_v2 patterns match independently), but missing specific patterns mean provisions get only generic labels (`Gvt: Agency` instead of `Gvt: Agency: Maritime and Coastguard Agency`), losing useful metadata for filtering.

1. **Survey extracted labels**: query `government_actors` across the family's provisions, tabulate by label. Look for high counts on generic labels (`Gvt: Authority`, `Gvt: Agency`, `Gvt: Ministry`) — these indicate specific bodies being captured generically.
2. **Scan for domain-specific bodies**: search provision text for known regulatory bodies in the domain (e.g. "Oil and Gas Authority", "Maritime and Coastguard Agency" for offshore; "Food Standards Agency" for food safety). Check whether they're getting specific or generic labels.
3. **Add specific patterns**: for any domain body appearing in 3+ provisions, add a specific pattern to `GOVERNMENT_DEFS` **before** the generic catch-all for its type. The list is ordered by specificity — specific patterns must come before their generic parent:
   - `Gvt: Agency: <Name>` before `Gvt: Agency`
   - `Gvt: Authority: <Name>` before `Gvt: Authority`
   - `Gvt: Ministry: <Name>` before `Gvt: Ministry`
4. **Include abbreviations**: many bodies have statutory abbreviations (MCA, OGA, NSTA, SEPA). Include these as regex alternations in the pattern.
5. **Add unit tests**: one test per new pattern, following the existing `extract_<body>()` convention.

**Why specific labels matter**: users filter provisions by government actor to find regulatory powers and responsibilities relevant to a particular body. A generic `Gvt: Agency` label forces them to read every provision mentioning any agency. A specific `Gvt: Agency: Maritime and Coastguard Agency` label lets them filter directly.

**Pattern ordering is critical**: `GOVERNMENT_DEFS` is processed top-to-bottom. The first matching pattern wins and its match is consumed from the text. If the generic `Gvt: Agency` pattern (which matches bare "agency") appears before a specific pattern like `Maritime and Coastguard Agency`, the generic pattern captures "Agency" first and the specific name is lost. Always place specific patterns before their generic parent.

**Note on Python validation**: the `actors.rs` regex patterns use POSIX character classes (`[:punct:]`) which are valid in Rust's `regex` crate but NOT in Python's `re`. When validating patterns in Python scripts, be aware that Python test results may show false negatives. Always verify with `taxa show` (live Rust code) for ground truth.

### Step 5: Check for false-positive risk

Not all actors attract duties. Critical distinctions:
- **"worker"** — almost always the object/beneficiary, NOT the duty-holder. UK law assigns duties to employers/hirers toward workers, not to workers themselves. SKIP.
- **"competent person"** — usually a role that someone appoints, not the duty-holder. Check each provision individually.
- **"contractor"**, **"client"** — DO attract duties in CDM law. Safe to add.
- **"person"** — too broad. The existing predicates "person who", "every person", "no person" are intentionally specific. Bare "person" matches ~45 provisions but ~30 are false positives (passive voice, application/fitness provisions).

### Step 6: Family-gated specialist actors

When adding actors that are domain-specific (e.g. "licensee" in offshore law), do NOT add them to the flat `GOVERNED_DEFS` list. Instead, add them as **family-gated specialist defs** in `actors.rs`, mirroring the `fitness.rs` pattern:

- Core `GOVERNED_DEFS` (employer, employee, contractor, etc.) — always run against every provision
- Specialist defs (e.g. `OFFSHORE_GOVERNED_DEFS`) — only run when `family.starts_with("OH&S: Offshore")`
- `extract_actors_for_family(text, family)` handles the routing
- This avoids wasteful regex against families where the term never appears

See GH #31 and `actors.rs::specialist_governed_for()` for the implementation pattern.

### Step 7: Re-enrich and measure with confusion matrix

After implementing changes, re-enrich the family and measure the result as a 2x2 confusion matrix. This gives a proper precision/recall picture rather than a raw "miss rate".

```bash
cargo run -p fractalaw-cli -- taxa enrich --family "<FAMILY>" --force
```

Then build the confusion matrix:

```python
import lancedb, re

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")

# Broad modal: obligation + enabling (covers governed duties AND government powers)
ANY_MODAL_RE = re.compile(
    r'\b(shall|must|is required to|has a duty|may|power to|entitled to)\b',
    re.IGNORECASE
)
# Structural purposes — provisions with these as primary purpose should NOT get DRRP
STRUCTURAL = {"Interpretation+Definition", "Amendment", "Repeal", "Enactment+Commencement"}

tp = fp = tn = fn_ = fn_gap_a = fn_gap_c = 0

for law in laws:
    data = tbl.search().where(f"law_name = '{law}'").select(
        ["text", "drrp_types", "governed_actors", "government_actors", "purposes"]
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

        # Expected positive = any modal + operative (non-structural) purpose
        expected = has_modal and not is_structural

        if has_drrp and expected:     tp += 1
        elif has_drrp and not expected: fp += 1
        elif not has_drrp and not expected: tn += 1
        else:
            fn_ += 1
            if gov or gvt: fn_gap_a += 1
            else:          fn_gap_c += 1

precision = tp / (tp + fp) if (tp + fp) else 0
recall = tp / (tp + fn_) if (tp + fn_) else 0
f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
```

**Ground truth heuristic**: "Expected positive" = any modal verb (shall/must/may/power to) + operative purpose (not interpretation/amendment/repeal/enactment). This captures both governed duties (shall/must) and government powers (may/power to).

**Reading the matrix**:

|  | Predicted: DRRP | Predicted: No DRRP |
|--|----------------:|-------------------:|
| **Expected: DRRP** | TP | FN |
| **Expected: No DRRP** | FP | TN |

- **Precision** = TP / (TP + FP) — when we classify DRRP, how often are we right? Target: >95%.
- **Recall** = TP / (TP + FN) — of provisions that should have DRRP, how many do we catch? This is the metric that improves as we add actors/patterns.
- **FN Gap A** (actor present, no DRRP) — addressable via regex. These are the candidates for the iteration cycle.
- **FN Gap C** (no actor, no DRRP) — passive voice, beyond regex. These are the AI polisher frontier.

When Gap A shrinks to near zero for a family, regex improvements have hit diminishing returns. The remaining recall gap is Gap C.

## Test-Driven Iteration Cycle

```
1.  Pick highest-frequency miss pattern from the data
1b. Add "true negative" regression tests BEFORE changing anything —
    provisions that correctly get NO DRRP today, so regressions are visible
2.  Write failing true-positive tests using real provision text
3.  Implement minimal change (usually one line in GOVERNED_ACTORS)
4.  Full suite: cargo test -p fractalaw-core --lib taxa
5.  Re-enrich: cargo run -p fractalaw-cli -- taxa enrich --laws <affected>
6.  Measure improvement
```

### Where tests go

- **`duty_patterns.rs` `mod tests`** — unit tests for `match_governed()`, `match_government_v1/v2()`. True-negative tests that specific text doesn't match the pattern matcher.
- **`mod.rs` `mod tests`** — full-pipeline tests using `taxa::parse()`. Both true-negative (text with keyword but no DRRP expected) and true-positive (text should produce DRRP).

### Naming convention for tests

```
// True-negative (duty_patterns.rs):
fn <keyword>_heading_no_match()
fn <keyword>_cross_reference_no_match()
fn <keyword>_interpretation_no_match()

// True-negative (mod.rs — full pipeline):
fn <keyword>_definition_no_drrp()

// True-positive (mod.rs — full pipeline):
fn <keyword>_duty_<description>()
fn <keyword>_prohibition()
```

## Current GOVERNED_ACTORS List

```rust
const GOVERNED_ACTORS: &[&str] = &[
    "employer", "self-employed", "employee", "occupier",
    "manufacturer", "supplier", "designer", "importer", "installer",
    "contractor", "client",
    "person who", "every person", "no person",
    "duty holder", "responsible person",
];
```

## ESH Law Sample (7 Laws)

| Short Name | Law ID | Domain |
|---|---|---|
| HSWA 1974 | UK_ukpga_1974_37 | Health & Safety at Work Act |
| Electricity at Work 1989 | UK_uksi_1989_635 | Electrical safety |
| MHSWR 1999 | UK_uksi_1999_3242 | Management of H&S |
| CDM 2015 | UK_uksi_2015_51 | Construction (Design & Management) |
| Fire Safety Order 2005 | UK_uksi_2005_1541 | Fire safety |
| PUWER 1998 | UK_uksi_1998_2306 | Work equipment |
| Manual Handling 1992 | UK_uksi_1992_2793 | Manual handling |

When working with a different sample, update the law list in analysis scripts.

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | `GOVERNED_ACTORS`, `has_governed_actor()`, pattern matchers |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction (30+ governed, 40+ government patterns), family-gated specialist defs |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | Orchestrates tiers: gov_v1 → gov_v2 → governed → empty |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse()` pipeline, full-pipeline tests |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classification (gates DRRP via `should_skip_drrp()`) |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_show`, `cmd_taxa_enrich` |

## Diminishing Returns

After the first few iterations (contractor, client), Gap A shrinks rapidly. When the remaining Gap A is dominated by "Ind: Person" (broad, high false-positive risk) and actor labels that are beneficiaries not duty-holders (worker, competent person), you've hit diminishing returns on GOVERNED_ACTORS expansion.

Next frontiers:
- **Specific "person" predicates**: "a person must", "a person shall" (5-9 provisions, moderate precision)
- **Gap B**: Fix `actors.rs` boundary matching (27 provisions at baseline)
- **Gap C**: Actor-less passive obligations — fundamentally different approach needed
- **AI polisher**: Let the ONNX/Claude model handle what regex can't
