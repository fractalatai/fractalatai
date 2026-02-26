# Skill: Taxa DRRP Gap Analysis

## When This Applies

When iteratively improving the taxa regex patterns to reduce DRRP (Duty, Right, Responsibility, Power) classification miss rate across UK ESH laws. The workflow is: pick a sample of laws, analyse the misses, identify the highest-value pattern to add, add it with tests, re-enrich, measure.

## Architecture — Two Parallel Actor Systems

The taxa pipeline (`crates/fractalaw-core/src/taxa/`) has two actor-detection mechanisms that serve different purposes:

1. **`actors.rs`** — Comprehensive regex-based extraction with word-boundary matching. Returns structured labels like `Ind: Person`, `Org: Employer`, `SC: C: Contractor`. Used for enrichment metadata.

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

### Step 1: Run `taxa show` for each law and pipe through analysis

```bash
# Dump all classifications for the sample laws
for law in UK_uksi_1989_635 UK_uksi_2015_51 UK_ukpga_1974_37 \
           UK_uksi_1999_3242 UK_uksi_2005_1541 UK_uksi_1998_2306 \
           UK_uksi_1992_2793; do
  cargo run -p fractalaw-cli -- taxa show --limit 500 "$law" 2>/dev/null
done
```

The output format per provision:
```
--- <provision_number> ---
  DRRP:    Duty
  Pattern: Governed / Prescriptive (55%)
  Governed:   Org: Employer, Ind: Employee
  Government: Gvt: Minister
  POPIMAR: Risk Control
  Purpose: Process+Rule+Constraint+Condition
  Text:    The employer shall ensure the safety...
```

Provisions with a `Governed:` line but NO `DRRP:` line are **Gap A** candidates.

### Step 2: Parse and count actor labels in Gap A

Pipe the output through Python to extract Gap A provisions (have governed actor + modal verb + no DRRP) and count actor labels:

```python
# Key logic: a provision is Gap A when:
# 1. No DRRP line in its taxa show output
# 2. Has Governed: actors listed
# 3. Text contains a modal verb (shall/must/is required to/has a duty)

# Filter out actors already in GOVERNED_ACTORS — those work fine.
# The interesting ones are actors extracted by actors.rs but NOT in the list.
```

See the full analysis script in the session doc `02-26-26-taxa-regex-patterns.md`.

### Step 3: For the top candidate, audit the affected provisions

Before adding any keyword:
1. Count how many provisions mention the keyword across all sample laws
2. Count how many already have DRRP (true positives — working)
3. Count how many have modal + no DRRP (would be affected)
4. Count how many have no modal (unaffected)
5. **Read the affected provision texts** — is the keyword the duty-holder (subject of obligation) or just mentioned (object/beneficiary)?

### Step 4: Check for false-positive risk

Not all actors attract duties. Critical distinctions:
- **"worker"** — almost always the object/beneficiary, NOT the duty-holder. UK law assigns duties to employers/hirers toward workers, not to workers themselves. SKIP.
- **"competent person"** — usually a role that someone appoints, not the duty-holder. Check each provision individually.
- **"contractor"**, **"client"** — DO attract duties in CDM law. Safe to add.
- **"person"** — too broad. The existing predicates "person who", "every person", "no person" are intentionally specific. Bare "person" matches ~45 provisions but ~30 are false positives (passive voice, application/fitness provisions).

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
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction (30+ governed, 40+ government patterns) |
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
