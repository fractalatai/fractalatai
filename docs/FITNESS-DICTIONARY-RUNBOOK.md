# Fitness Dictionary Expansion Runbook

Repeatable workflow for expanding p-dimension dictionaries when new law families arrive or existing coverage needs improvement.

## Prerequisites

- LAT (provision text) populated in LanceDB for the target family
- LRT (law metadata with `family` column) populated in DuckDB
- `fractalaw taxa audit-fitness` command available

## Workflow

### Step 1: Audit the family

```bash
fractalaw taxa audit-fitness --family "FAMILY_NAME"
```

This produces a 5-section report:

1. **Coverage by Family** — how many APPLICATION_SCOPE provisions exist, what % have polarity, what % have tags
2. **Gap Provisions** — provisions where polarity was detected but zero p-dimension tags extracted (the main diagnostic)
3. **Candidate Terms** — n-gram frequency analysis of gap provision text, filtered against known dictionary terms
4. **No-Polarity Provisions** — APPLICATION_SCOPE provisions where the polarity regex failed
5. **Dictionary Utilisation** — hit counts for every dictionary term (core + specialist if family matches)

Use `--limit N` to control how many gap provisions are shown per family (default 10, 0 = all).

If the family has zero APPLICATION_SCOPE provisions, the purpose classifier doesn't recognise scope provisions in that family yet — that's a separate issue from dictionary coverage.

### Step 2: Review candidates

From the Section 3 candidate terms, identify terms that should be tagged:

- **Which dimension?** Person (who), Process (what activity), Place (where), Plant (what equipment/substance), Property (qualifying condition), Sector (industry)
- **Core or specialist?** If the term appears across many families, add to core dict. If family-specific, add to specialist dict.
- **Regex pattern**: Use `\b` word boundaries, `\s+` for spaces, `(?:s)?` for plurals. Compound terms first (longer before shorter).

### Step 3: Add dictionary entries

**Core dictionaries** — edit the relevant `*_DICT` static in `crates/fractalaw-core/src/taxa/fitness.rs`:

```rust
static PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        // Compound terms first
        ("new\\s+term\\s+pattern", "canonical term"),
        // ...
    ])
});
```

**Specialist dictionaries** — add entries to the existing family dict (e.g., `OHS_PERSON_DICT`) or create new family dicts:

```rust
// New family specialist
static FOOD_PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("food\\s+business\\s+operator", "food business operator"),
    ])
});
```

Then register in `specialist_dicts_for()`:

```rust
fn specialist_dicts_for(family: &str) -> Vec<(PDimension, &'static [DictEntry])> {
    if family.starts_with("OH&S") {
        vec![/* ... */]
    } else if family.starts_with("FOOD") {
        vec![
            (PDimension::Person, &FOOD_PERSON_DICT),
        ]
    } else {
        vec![]
    }
}
```

### Step 4: Add tests

Add tests in `fitness.rs` that verify:

- The new term matches with the correct family (`extract(text, Some("FAMILY"))`)
- The new term does NOT match without the family (`extract(text, None)`) — for specialist terms only
- Core terms match regardless of family

### Step 5: Validate

```bash
# Run tests
cargo test -p fractalaw-core

# Re-run audit to measure improvement
fractalaw taxa audit-fitness --family "FAMILY_NAME"

# Compare gap count and tagged% against Step 1 baseline
```

### Step 6: Re-enrich

```bash
# Re-enrich the target family to persist new tags
fractalaw taxa enrich --family "FAMILY_NAME" --force

# Verify with QA
fractalaw taxa qa --family "FAMILY_NAME"
```

### Step 7: Publish

```bash
# Publish changed laws to sertantai
fractalaw sync publish --changed
```

## Architecture

```
                 ┌──────────────┐
                 │  Core Dicts  │  Always applied (94 patterns)
                 │  PERSON_DICT │  6 dimensions x ~15 entries each
                 │  PROCESS_DICT│
                 │  PLACE_DICT  │
                 │  PLANT_DICT  │
                 │  PROPERTY_.. │
                 │  SECTOR_DICT │
                 └──────┬───────┘
                        │
    extract(text, family: Option<&str>)
                        │
         ┌──────────────┼──────────────┐
         │              │              │
  ┌──────┴───────┐  ┌──┴───┐    ┌─────┴──────┐
  │ OHS_PERSON   │  │ ...  │    │ Future:    │
  │ OHS_PROCESS  │  │      │    │ FOOD_*     │
  │ OHS_PLANT    │  │      │    │ MARITIME_* │
  │              │  │      │    │ RADIO_*    │
  │ starts_with  │  │      │    │            │
  │ ("OH&S")     │  │      │    │            │
  └──────────────┘  └──────┘    └────────────┘
```

## Key files

- `crates/fractalaw-core/src/taxa/fitness.rs` — all dictionaries, `extract()`, `specialist_dicts_for()`
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2(raw_text, family)`
- `crates/fractalaw-cli/src/main.rs` — `cmd_taxa_audit_fitness()`, `enrich_single_law()`
