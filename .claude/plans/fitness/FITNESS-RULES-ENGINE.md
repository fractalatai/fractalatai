# Fitness Rules Engine: Customer Matching

## Problem

Given a customer profile and a law's extracted fitness data, determine: **does this law apply to this customer?**

Simple facet intersection is insufficient. Real legislation has boolean logic, hierarchical matching, negation, conditional applicability, and temporal scope. The engine narrows 19K laws to ~400 for a typical customer at onboarding time (not real-time).

## Architecture: Two-Stage Filter + Expression Trees

### Stage 1: Coarse Filter (Hierarchical Index)

Pre-compute an inverted index: for each canonical entity, store which laws mention it.

```sql
-- entity_index: maps entities to laws
CREATE TABLE entity_index (
    law_name    TEXT,
    entity_uri  TEXT,
    scope_dimension TEXT,   -- personal / material / territorial / temporal
    polarity    TEXT        -- AppliesTo / DisappliesTo
);
```

At query time:
1. Expand the customer profile using hierarchies (SIC 08.11 → Section B → "mining and quarrying")
2. Union the index lookups to get candidate laws
3. This is the 19K → ~800 coarse filter

Hierarchy expansion uses lookup tables (SIC tree, HSE activity tree, jurisdiction tree). Pre-compute ancestor sets for each leaf code. This is static reference data maintained alongside entity dictionaries.

### Stage 2: Expression Tree Evaluation (Per-Law)

Each law's applicability is compiled into a boolean expression tree at enrichment time. The tree is built from extracted mentions and stored as JSON in DuckDB.

```
ApplicabilityNode:
  Match   { dimension, codes[], op: AnyOf | AllOf }   -- leaf: does customer match?
  And     [ children ]                                  -- all must match
  Or      [ children ]                                  -- any must match
  Not     ( child )                                     -- exclusion
  Conditional { condition, then }                       -- match IF condition holds
  TimeWindow  { from, to, inner }                       -- temporal applicability
```

Evaluation is a recursive tree walk against the customer profile. Each `Match` leaf checks whether the customer's expanded attribute set intersects the required codes.

### How the Five Cases Are Handled

**1. Boolean logic** — "operates a vehicle OR a vessel"
```json
{ "op": "Or", "children": [
    { "match": { "dimension": "material", "codes": ["vehicle_operation"], "op": "AnyOf" }},
    { "match": { "dimension": "material", "codes": ["vessel_operation"], "op": "AnyOf" }}
]}
```

**2. Hierarchical matching** — SIC 08.11 (quarrying) vs SIC section B (mining)
Hierarchy expansion happens before evaluation. The customer's profile `sic: 08.11` is expanded to `[08.11, 08.1, 08, B]`. A law targeting `mining_and_quarrying` (mapped to SIC B) matches because B is in the expanded set.

**3. Negation** — "applies to construction work, except domestic premises"
DisappliesTo mentions compile to `Not` nodes:
```json
{ "op": "And", "children": [
    { "match": { "dimension": "material", "codes": ["construction_work"], "op": "AnyOf" }},
    { "op": "Not", "child": {
        "match": { "dimension": "material", "codes": ["domestic_premises"], "op": "AnyOf" }
    }}
]}
```

**4. Conditional applicability** — "applies to employers IF they handle asbestos"
```json
{ "op": "Conditional",
  "condition": { "match": { "dimension": "material", "codes": ["asbestos_handling"], "op": "AnyOf" }},
  "then": { "match": { "dimension": "personal", "codes": ["employer"], "op": "AnyOf" }}
}
```

**5. Scope narrowing** — Part-level scope + section-level override
Part-level tree is the default; section-level trees are merged with `And`:
```json
{ "op": "And", "children": [
    /* Part-level scope */ { "match": { "dimension": "personal", "codes": ["employer"], "op": "AnyOf" }},
    /* Section override */ { "match": { "dimension": "material", "codes": ["confined_spaces"], "op": "AnyOf" }}
]}
```

**6. Temporal** — commencement and sunset
```json
{ "op": "TimeWindow", "from": "2025-10-01", "to": null,
  "inner": { /* ... rest of tree ... */ }
}
```

## Rule Compilation

The expression tree is **compiled from extracted mentions**, not hand-authored. The compilation step runs at enrichment time:

1. Group mentions by law and scope unit (Part/Chapter/law)
2. AppliesTo mentions become `Match` leaves
3. Multiple AppliesTo mentions in the same scope → `And` node (cumulative)
4. DisappliesTo mentions → `Not` wrapping the exclusion
5. Scope hierarchy → nested `And` (Part scope AND section scope)
6. Commencement/sunset → `TimeWindow` wrapper

The compiled tree is stored as JSON in DuckDB alongside the law's LRT record.

## Confidence and Human Review

Applicability is fundamentally binary — the law applies or it doesn't. But extraction confidence affects how sure we are of the compiled rule.

- **High confidence (>0.9)**: include in default results
- **Medium confidence (0.5-0.9)**: flag for human review
- **Low confidence (<0.5)**: exclude from default, available in "extended" view

Each `Match` leaf carries the extraction confidence from the mention → entity → classification pipeline. The tree's overall confidence is the minimum of its leaves. This gives a clean UX: "400 laws apply, 35 need review."

## What Compliance Platforms Do

Regology, Ascent, CUBE, and Corlytics all use broadly similar approaches: entity extraction → taxonomy matching → expert review. None fully automate the matching. They use ML/NLP to get to ~80% and have regulatory analysts validate.

The expression tree engine is the automated layer. The confidence scoring identifies where human review is needed. The hierarchy indexes (SIC, HSE, jurisdiction) are the taxonomy — their completeness is the key differentiator.

## Architecture Split: Fractalaw / Sertantai

The rules engine spans two systems. Fractalaw enriches; sertantai serves.

### Fractalaw (Rust, enrichment-time)

Compiles the expression tree from extracted mentions and publishes it as part of the law's LRT record — the same publish pathway as DRRP enrichment.

1. Extract mentions (Phase 2 pipeline)
2. Propagate scope (FITNESS-GRAPH.md)
3. Compile mentions into expression tree JSON per law
4. Publish compiled tree + entity index to sertantai via Zenoh

The compiled tree is a static artifact. It changes only when the law is re-enriched (new LAT, updated extraction model, scope change).

### Sertantai (Elixir, query-time)

Evaluates compiled trees against customer profiles when a customer asks "what applies to me?"

1. Store compiled expression trees (received via Zenoh publish)
2. Build/maintain the inverted entity index from published entity data
3. Stage 1: coarse filter via index lookup + hierarchy expansion
4. Stage 2: evaluate expression trees for candidate laws
5. Present results with confidence flags ("400 apply, 35 need review")

The tree evaluator is a recursive pattern match on node types — natural in Elixir, probably ~200 lines. No external rules engine dependency needed. The hierarchy expansion tables (SIC tree, jurisdiction tree) are reference data managed in sertantai.

### What Travels Over Zenoh

The compiled expression tree is JSON, published alongside the LRT record:

```
fractalaw/@dev/lrt/{law_name}
  → existing: DRRP holders, duty types, fitness P-dims, triage, significance
  → new: compiled_applicability (JSON expression tree)
  → new: entity_index (list of {entity_uri, scope_dimension, polarity})
```

Sertantai already receives and stores LRT updates. The compiled tree is just new fields on the same payload.

### Priority

1. Define `ApplicabilityNode` schema and JSON serialisation (shared contract)
2. Build the tree compiler in fractalaw (from extracted mentions)
3. Build the inverted index publisher in fractalaw
4. Build the evaluator in sertantai (recursive tree walk)
5. Build hierarchy expansion in sertantai (SIC tree, jurisdiction)
6. Add temporal filtering

## Dependencies

- FITNESS-STRATEGY.md Phase 2 (three-layer extraction) for structured mentions
- FITNESS-GRAPH.md for scope propagation (tells the compiler which mentions scope which provisions)
- Hierarchy reference data: SIC tree, jurisdiction tree, HSE activity codes (sertantai-managed)
- Customer profile schema definition (sertantai)
- Zenoh publish payload extension (ZENOH-SPEC.md)

## Open Questions

- How to handle "any person" (universal personal scope) — does it match all customers automatically, or is it a special case?
- How to represent graduated applicability (e.g. "5 or more employees" — the threshold is a condition, not a binary match)
- How to handle "Crown application" provisions — government-facing obligations that don't apply to private employers
