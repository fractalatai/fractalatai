# Session: 2026-02-24 — DRRP Parsing Migration

## Context

**Phase**: 3 (MicroApp Runtime)
**Parent session**: [02-21-26-drrp-polisher.md](02-21-26-drrp-polisher.md)
**Objective**: Migrate the sertantai DRRP/Taxa regex parsing pipeline from Elixir into fractalaw-core as pure Rust.

## Background

The sertantai Elixir app currently owns the entire DRRP classification pipeline — a regex-based system that parses UK ESH legislative text and classifies each clause by:

1. **Duty type** — who bears the obligation (government vs governed entity) and what kind
2. **Actors** — which specific actors (employer, inspector, Secretary of State, etc.)
3. **POPIMAR** — management-cycle category (Policy/Organising/Planning/Implementing/Measuring/Auditing/Reviewing)
4. **Purpose** — clause function (general, specific, procedural, definitional, enforcement, exemption)
5. **Making detection** — whether the clause creates instruments (regulations, orders) vs operational duties
6. **Clause refinement** — extracting a focused, readable clause from raw regex captures
7. **Confidence scoring** — signal-counting for regex-extracted clause quality

Currently this pipeline runs in sertantai, producing rough `drrp_annotations` that get synced to fractalaw for AI polishing (the drrp-polisher guest component from the parent session). Migrating the parser into fractalaw means fractalaw can do the regex classification locally, reducing the dependency on sertantai and enabling richer pre-filtering before the AI polisher runs.

## Scope

**In scope**: The 18 taxa classification modules — the pure regex pipeline that classifies legislative text into DRRP records. This is the core logic being migrated.

**Out of scope**: The scraper/LAT pipeline (LatParser, LatReparser, LatPersister, CommentaryParser, CommentaryPersister). Sertantai will continue to own scraping and XML parsing. Fully parsed legislative texts will arrive in fractalaw's LanceDB via pub/sub (zenoh, when built). The LAT modules were included in `taxa-migration.zip` for context but are not being ported.

## Elixir Source Inventory

### Taxa Pipeline (18 modules)

| Elixir Module | Lines | Purpose |
|---|---|---|
| `Scraper.TaxaParser` | ~800 | **Orchestrator** — cleans text, runs all classifiers, assembles output via TaxaFormatter |
| `Taxa.TextCleaner` | ~110 | Strip HTML, collapse whitespace, normalise punctuation, strip leading numbering |
| `Taxa.DutyType` | ~170 | Top-level classifier: tries Government v1 → Government v2 → Governed → unknown |
| `Taxa.DutyTypeLib` | ~650 | Core engine: `find_role_holders`, windowed modal search, blacklist, actor base patterns |
| `Taxa.DutyTypeDefnGovernment` | ~90 | Government duty patterns v1: regulation_making, code_approval, enforcement, prescriptive, enabling |
| `Taxa.DutyTypeDefnGovernmentV2` | ~130 | Government patterns v2: direction, guidance, consultation, appointment, delegation, fees, parliamentary_reporting |
| `Taxa.DutyTypeDefnGoverned` | ~210 | Governed entity patterns: general_duty, prohibitive, sfairp_duty, information, risk_assessment, training, prescriptive, enabling |
| `Taxa.ClauseRefiner` | ~420 | Extracts focused clause window around modal verb (subject + modal + action) |
| `Taxa.ActorLib` | ~140 | Actor extraction: builds custom libraries, runs regex matching, removes matched text progressively |
| `Taxa.ActorDefinitions` | ~490 | 16 government + 16 governed actor fragments with regex patterns and blacklist |
| `Taxa.DutyActor` | ~50 | Struct: `%{label, role, confidence}` with `bearer/2` and `beneficiary/2` constructors |
| `Taxa.Popimar` | ~215 | POPIMAR classifier: scores text against 7 category pattern sets, returns sorted `[(cat, conf)]` |
| `Taxa.PopimarLib` | ~370 | Pattern definitions for 7 POPIMAR categories + `score/2` (max-weight matching) |
| `Taxa.PurposeClassifier` | ~400 | Purpose/scope classifier: enforcement, exemption, definitional, procedural, general, specific |
| `Taxa.MakingDetector` | ~60 | Making/creating pre-filter: scans signals, returns `%{making?, signals, confidence}` |
| `Taxa.MakingDetectorSignals` | ~200 | 10 making signal definitions with regex + confidence mapping (1→0.6, 2→0.8, 3+→0.95) |
| `Taxa.RegexClauseConfidence` | ~52 | Signal-counting confidence scorer: government (5 signals) vs governed (5 signals) |
| `Taxa.TaxaFormatter` | ~370 | Assembles final TAXA record map from all intermediate results |

### Test Coverage (13 test files)

| Test File | Tests | Covers |
|---|---|---|
| `duty_type_test.exs` | ~12 | DutyType.classify — general/absolute/qualified/prohibition/risk_assessment/info/consultation/notification/record |
| `duty_type_lib_test.exs` | ~10 | DutyTypeLib helpers — actor detection, blacklist, role holders |
| `duty_actor_test.exs` | ~14 | DutyActor.classify — employer/employee/self-employed/designer/SoS/HSE/inspector/responsible_person/client/principal_contractor |
| `clause_refiner_test.exs` | ~12 | ClauseRefiner.refine — duty clause, qualifier extraction, multi-paragraph, batch |
| `popimar_test.exs` | ~10 | POPIMAR classify — policy/organising/planning/measuring/audit/review, multi-element, confidence |
| `purpose_classifier_test.exs` | ~10 | PurposeClassifier — all 6 categories |
| `making_detector_test.exs` | ~9 | MakingDetector.detect — regulation/code/order/exemption powers, non-making operational duties |
| `making_detector_signals_test.exs` | ~9 | Signal-level tests |
| `regex_clause_confidence_test.exs` | ~9 | Confidence scoring — high/low/medium quality clauses, signals |
| `taxa_formatter_test.exs` | ~8 | Output format structure |
| `taxa_integration_test.exs` | ~12 | End-to-end: HSWA s.2, HSWA s.15, MHSWR Reg 3, CDM 2015, pipeline consistency |
| `responsibility_pattern_comparison_test.exs` | ~7 | Pattern similarity, grouping, diffing |
| `clause_quality_integration_test.exs` | ~6 | Confidence→classification quality pipeline |

## Architecture Decisions

### 1. Where Does the Rust Code Live?

**Decision**: New module `crates/fractalaw-core/src/taxa/` (pure Rust, no external deps beyond `regex`).

**Rationale**:
- The taxa pipeline is regex-only — no DB, no network, no AI — so it belongs in the pure-Rust `fractalaw-core` crate
- Zero additional dependencies: the `regex` crate is already a transitive dep via Arrow
- All types and functions are usable by both the CLI (host-side) and guest components (via WIT/WASM)
- Keeps `fractalaw-core` as the single source of truth for DRRP types and classification

### 2. Module Structure

**Decision**: Mirror the Elixir module hierarchy but flatten for Rust idioms.

```
crates/fractalaw-core/src/
├── taxa/                         # NEW — regex DRRP classification pipeline
│   ├── mod.rs                    # pub mod declarations + TaxaRecord struct + parse() orchestrator
│   ├── text_cleaner.rs           # HTML stripping, whitespace normalisation, punctuation, numbering
│   ├── duty_type.rs              # Top-level classify() → DutyClassification{family, sub_type, confidence}
│   ├── duty_patterns.rs          # Government v1/v2 + Governed pattern matchers (merged from 3 Elixir modules)
│   ├── clause_refiner.rs         # Modal-window extraction: subject + modal + action
│   ├── actors.rs                 # Actor definitions + extraction (merged ActorLib + ActorDefinitions + DutyActor)
│   ├── popimar.rs                # POPIMAR classifier + pattern definitions (merged Popimar + PopimarLib)
│   ├── purpose.rs                # Purpose classifier (6 categories)
│   ├── making.rs                 # Making detector + signals (merged MakingDetector + MakingDetectorSignals)
│   └── confidence.rs             # Regex clause confidence scorer
├── drrp.rs                       # Existing — Annotation + PolishedEntry sync types (unchanged)
├── schema.rs                     # Existing — Arrow schemas (unchanged)
└── lib.rs                        # Add `pub mod taxa;`
```

### 3. Key Type Mappings: Elixir → Rust

| Elixir | Rust |
|---|---|
| `{:government, :regulation_making, 0.90}` | `DutyClassification { family: DutyFamily::Government, sub_type: DutySubType::RegulationMaking, confidence: 0.90 }` |
| `%DutyActor{label, role, confidence}` | `Actor { label: String, role: ActorRole, confidence: f32 }` |
| `[{:policy, 0.8}, {:implementing, 0.5}]` | `Vec<(PopimarCategory, f32)>` |
| `{:enforcement, 0.85}` | `(Purpose, f32)` |
| `%{making?: true, signals: [...], confidence: 0.95}` | `MakingResult { is_making: bool, signals: Vec<String>, confidence: f32 }` |
| `TaxaFormatter.format(...)` | `TaxaRecord` struct with all fields |

### 4. Regex Compilation Strategy

**Decision**: Use `std::sync::LazyLock<Regex>` for all patterns.

**Rationale**:
- The Elixir code uses module attributes (`@signals`) and `:persistent_term` caching — same intent
- `LazyLock` is stable in std since Rust 1.80 (we're on 1.93) — no `once_cell` dep needed
- Patterns compile once at first use, zero overhead on subsequent calls
- No `regex` crate feature flags needed — default is sufficient

### 5. Migration Priority

**Decision**: Migrate the core classification pipeline first. LAT/Commentary XML parsing is lower priority.

**Phase 1 (this session)**: Document the architecture, map types, plan modules.

**Phase 2**: Implement in this order (each module is independently testable):
1. `text_cleaner.rs` — simplest, no deps on other taxa modules
2. `duty_patterns.rs` — government v1/v2 + governed pattern matchers
3. `duty_type.rs` — orchestrates the pattern matchers
4. `actors.rs` — actor definitions + extraction
5. `clause_refiner.rs` — modal-window extraction
6. `popimar.rs` — POPIMAR classification
7. `purpose.rs` — purpose classification
8. `making.rs` — making detection
9. `confidence.rs` — clause confidence scorer
10. `mod.rs` — TaxaRecord + parse() orchestrator + TaxaFormatter equivalent

**Phase 3**: Wire into the drrp-polisher pipeline — taxa classification runs before AI polishing, providing richer pre-filtering and structured input.

### 6. What Changes About the Polisher Pipeline

Currently:
```
sertantai regex → drrp_annotations (rough) → sync pull → AI polisher → polished_drrp
```

After migration:
```
sertantai scrapes + parses XML
    │
    │  (zenoh pub/sub — when built)
    ▼
LAT text in LanceDB
    │
    ▼
fractalaw taxa parser (Rust regex) → drrp_annotations (enriched) → AI polisher → polished_drrp
```

The key shift: fractalaw runs the regex classification **locally** on legislation text already in LanceDB. Sertantai still owns scraping and XML parsing, delivering fully parsed texts via pub/sub (zenoh). Fractalaw no longer depends on sertantai for the initial regex pass — it can classify as soon as text lands. The AI polisher still refines, but it gets richer input (duty_family, sub_type, actors, POPIMAR, purpose, making signals) to work with.

### 7. Test Strategy

**Decision**: Port the Elixir test cases directly as Rust `#[test]` functions.

The Elixir tests use real UK legislative text (HSWA 1974, MHSWR 1999, CDM 2015, COSHH) which serves as excellent golden data. Each test file maps to a `#[cfg(test)] mod tests` in its corresponding Rust module.

Some test expectations in the Elixir code are loose (e.g. `assert result.type in [:risk_assessment, :assessment]`) because the Elixir API evolved over time. The Rust implementation will have more precise types, so tests will be tightened.

### 8. Rust-Specific Regex Optimisations

The Elixir implementation already went through significant optimisation (windowed modal search, `:persistent_term` caching of compiled regexes, progressive text removal). Rust's regex ecosystem offers additional opportunities that aren't available on the BEAM:

**Available at implementation time (build these in from the start):**

- **`RegexSet` for bulk pattern screening**: Rust's `regex::RegexSet` compiles multiple patterns into a single DFA automaton and matches all of them in a single pass over the text. This directly benefits the actor scanning (32 fragment checks), making signal scanning (10 patterns), POPIMAR scoring (7 categories × ~6 patterns each), and confidence signal counting. Instead of N sequential `Regex.match?` calls like Elixir, one `RegexSet::matches()` call returns a bitset of which patterns hit. The underlying engine is O(m×n) where m is pattern count and n is text length, but the constant factor is much lower than N separate passes.

- **`aho-corasick` for literal fragment scanning**: `ActorDefinitions.scan_government_actors` and `scan_governed_actors` are pure substring checks (`String.contains?`) across 16 fragments each. The `aho-corasick` crate (already a transitive dep of `regex`) builds a finite automaton that finds all 32 fragments in a single linear scan with SIMD-accelerated prefilters. This replaces 32 sequential `String.contains?` calls with one pass.

- **Byte-offset operations instead of string slicing**: Elixir's `String.slice` operates on grapheme clusters (O(n) to seek). Rust's regex crate returns byte offsets directly, and `&str[start..end]` is O(1). The windowed modal search — which does repeated slicing around modal positions — benefits significantly.

- **Zero-copy text processing**: The `TextCleaner` pipeline in Elixir creates a new binary at each step (5 `String.replace` calls = 5 allocations). In Rust, `regex::Regex::replace_all` can chain via `Cow<str>`, only allocating when a replacement actually happens. For text that's already clean (common in the polisher pipeline where text was pre-cleaned), this means zero allocations.

- **Compile-time regex validation**: `once_cell::sync::Lazy<Regex>` patterns are validated at first use and panic on invalid patterns (caught in tests). Elixir's `Regex.compile/2` returns `{:error, ...}` at runtime, requiring defensive error handling that clutters the Elixir code (visible in `DutyTypeLib.run_role_regex`). The Rust version eliminates this branch.

**Worth exploring after initial port (tuning phase):**

- **Window size tuning**: The Elixir implementation uses 400 chars before / 200 chars after the modal verb. These were tuned empirically. Once the Rust port has the same test suite passing, we can benchmark different window sizes to find the quality/speed sweet spot. Smaller windows = faster but risk missing context; larger windows = more context but diminishing returns.

- **`RegexSet` as pre-filter for heavy patterns**: Use a `RegexSet` containing simplified versions of the expensive patterns (e.g. just the literal anchors) to quickly discard text that can't possibly match, before running the full capture-group patterns on the survivors. This is the same principle as the windowed modal search but applied at the pattern level.

- **Parallelism via `rayon`**: When classifying a batch of clauses (e.g. all sections of a large Act), `rayon::par_iter` gives thread-level parallelism for free. The BEAM gives process-level concurrency via `Task.async_stream` but with higher per-task overhead. For CPU-bound regex work, Rayon's work-stealing scheduler on compiled patterns will be faster.

- **`memchr` for literal-heavy patterns**: Patterns like `\bshall\b` or `\bmust\b` have fixed literal prefixes. The `regex` crate already uses `memchr` internally for literal optimisation, but for the hot-path modal verb scan, a direct `memchr::memmem::find_iter` for "shall"/"must"/"may" could be faster than the regex engine's general-purpose literal extractor.

## Complexity Notes

### DutyTypeLib — The Most Complex Module

`DutyTypeLib` (~650 lines) is the engine of the pipeline. Key complexity:
- **Windowed modal search**: For texts >50K chars, it finds modal verb positions first, creates windows (400 chars before + 200 after), and only runs patterns within those windows
- **Actor-specific pattern building**: Dynamically constructs regex patterns per actor per role
- **Progressive text removal**: Matched text is removed to prevent duplicate matches
- **False positive filtering**: `holder_matches_clause_subject?` checks that the matched actor actually governs the modal verb
- **Deduplication**: Groups matches by normalised clause core and keeps the most specific holder

This module will need the most careful porting. The windowed search optimisation should port cleanly — Rust's regex crate supports `find_iter` with byte offsets which maps well.

### ClauseRefiner — Second Most Complex

`ClauseRefiner` (~420 lines) does sophisticated text extraction:
- Finds the LAST modal verb in a raw capture (because patterns typically end at the modal)
- Extracts subject by scanning backwards to sentence start
- Extracts action from section context text (not just the raw match)
- Smart truncation at sentence boundaries with ellipsis

### ActorDefinitions — Data-Heavy

`ActorDefinitions` contains 32 actor patterns (16 government + 16 governed) as keyword lists of regex strings. In Rust these become `&[(&str, &str)]` static arrays or `Lazy<Vec<(String, Regex)>>` compiled patterns.

## Files to Create

| File | Description |
|---|---|
| `crates/fractalaw-core/src/taxa/mod.rs` | Module declarations, TaxaRecord struct, parse() orchestrator |
| `crates/fractalaw-core/src/taxa/text_cleaner.rs` | Text normalisation (HTML strip, whitespace, punctuation, numbering) |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | Top-level duty classifier |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | Government v1/v2 + Governed regex pattern matchers |
| `crates/fractalaw-core/src/taxa/clause_refiner.rs` | Modal-window clause extraction |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor definitions, extraction, DutyActor struct |
| `crates/fractalaw-core/src/taxa/popimar.rs` | POPIMAR classifier + pattern definitions |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classifier |
| `crates/fractalaw-core/src/taxa/making.rs` | Making detector + signals |
| `crates/fractalaw-core/src/taxa/confidence.rs` | Regex clause confidence scorer |

## Files to Modify

| File | Change |
|---|---|
| `crates/fractalaw-core/src/lib.rs` | Add `pub mod taxa;` |
| `crates/fractalaw-core/Cargo.toml` | Add `regex` dep (uses `std::sync::LazyLock`, no `once_cell` needed) |

## Progress

| Task | Status | Notes |
|------|--------|-------|
| Read and catalogue Elixir source (18 taxa modules) | [x] | All source files analysed |
| Read and catalogue Elixir tests (13 test files) | [x] | ~130 test cases total |
| Review existing fractalaw DRRP code | [x] | drrp.rs, schema.rs, drrp-polisher guest |
| Design Rust module structure | [x] | 10 files under `taxa/` |
| Map Elixir types → Rust types | [x] | Enums + structs documented above |
| Document architecture decisions | [x] | 7 decisions |
| Implement text_cleaner.rs | [x] | 17 tests passing, uses `LazyLock` (std, no `once_cell`), `Cow<str>` zero-copy |
| Implement duty_patterns.rs | [x] | 29 tests. Merged 3 Elixir modules. Fixed pattern ordering (domain-specific before generic fallbacks), `\brisks?\b` for plurals, broadened general duty regex |
| Implement duty_type.rs | [x] | 9 tests. Top-level DRRP classifier orchestrating v1→v2→governed→unknown |
| Implement actors.rs | [x] | 8 tests. ~73 actor patterns (40 government + 33 governed), progressive text removal, blacklist |
| Implement clause_refiner.rs | [x] | 9 tests. Modal-window extraction. Replaced Elixir lookahead `(?=[A-Z])` with capture group approach |
| Implement popimar.rs | [x] | 11 tests. 16 POPIMAR categories. `const RAW_PATTERNS` + `LazyLock<Vec>` pattern (fixed E0492). Removed lookaheads |
| Implement purpose.rs | [x] | 12 tests. 15 purpose categories with `+` separator. Same `LazyLock<Vec>` pattern |
| Implement making.rs | [x] | 9 tests. Bayesian composite score, 4 tiers. Fixed "to make further provision" variant matching |
| Implement confidence.rs | [x] | 6 tests. Scores: V2 capture, clean ending, adequate length, strong modal |
| Implement mod.rs (orchestrator + TaxaRecord) | [x] | 5 tests. `TaxaRecord` struct + `parse()` pipeline |
| Port Elixir test cases to Rust | [x] | 116 tests total, all passing |
| Wire taxa parser into CLI | [ ] | |
| Wire enriched annotations into drrp-polisher | [ ] | |

## Key Porting Issues

### Rust `regex` crate: no lookahead/lookbehind
The Rust `regex` crate doesn't support `(?=...)` or `(?!...)`. Several Elixir patterns had to be rewritten:
- `clause_refiner.rs`: `[.;]\s*(?=[A-Z])` → `[.;]\s+([A-Z])` with capture group iteration
- `popimar.rs`: `(?!authority)` in competence → simplified to `[Cc]ompeten(?:t|ce|cy)\s`
- `popimar.rs`: `(?!to)` in Records → `[^t]`; `(?!representative)` in Permit → `[^r]`

### `LazyLock` interior mutability (E0492)
`LazyLock<Regex>` inside a struct inside `&[struct]` static borrows fails because `LazyLock` has interior mutability. Fixed by using `const RAW_PATTERNS: &[(&str, &str)]` + `static COMPILED: LazyLock<Vec<(&str, Regex)>>`.

### Pattern ordering
Elixir's `match_governed` order didn't work in Rust because domain-specific patterns (SFAIRP, risk, info, training) need to be checked before generic prescriptive/enabling fallbacks to avoid false matches.

### Description pattern flexibility
`"to make provision for securing"` didn't match `"to make further provision for securing"` (real UK legislation text). Added variant patterns with "further" for both `provision_securing` and `provision_for`.

## Commit

`70accbd` — Implement DRRP/Taxa regex classification pipeline in pure Rust (#16) — 15 files, 3,332 insertions, 116 tests

## Next Steps

1. Wire taxa parser into CLI (e.g. `fractalaw taxa <law_id>` command)
2. Wire enriched annotations into drrp-polisher guest (taxa classifications as additional context for AI polishing)
3. Explore `RegexSet` pre-filter optimisation for batch classification
4. Consider `rayon` parallelism for classifying all sections of large Acts
