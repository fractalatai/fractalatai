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
| Wire taxa parser into CLI | [x] | `48bbf4e` — `fractalaw taxa show <law>` queries LanceDB text sections and runs taxa pipeline |
| Wire enriched annotations into drrp-polisher | [x] | `f3d64a7` — `taxa enrich` pre-computes into DuckDB; guest queries taxa_classifications per annotation |

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

## Commits

- `70accbd` — Implement DRRP/Taxa regex classification pipeline in pure Rust (#16) — 15 files, 3,332 insertions, 116 tests
- `48bbf4e` — Wire taxa classifier into CLI as `fractalaw taxa show` command
- `f3d64a7` — Wire taxa enrichment into drrp-polisher pipeline (`taxa enrich` + guest prompt enrichment)

## Phase 4: DRRP Polisher Testing Against Real Laws

### Context

The taxa regex parser (Phase 1–3) is complete and populates the DRRP Taxa columns (s1.9 in `docs/SCHEMA.md`). The next step is testing the DRRP polisher — the AI component that refines the `duties`, `rights`, `responsibilities`, `powers` JSONB detail columns (each containing `List<DRRPEntry>` with holder, duty_type, clause, article).

### What the DRRP Polisher Does

The polisher reviews each DRRP detail entry and takes one of five actions:
1. **Accept** — the entry is correct as-is
2. **Delete** — the entry is incorrect and should be removed
3. **Expand front** — the clause is missing necessary context at the beginning; prepend it
4. **Trim front** — the clause has bloat at the beginning; remove it
5. **Expand/trim back** — same adjustments to the tail of the clause

### Schema Addition: AI Output Columns

To test polishing we need to preserve both the before (regex-extracted) and after (AI-polished) DRRP detail. New columns in the LRT schema alongside the existing `duties`, `rights`, `responsibilities`, `powers`:

| Column | Arrow Type | Nullable | Description |
|--------|-----------|----------|-------------|
| `duties_ai` | List\<DRRPEntry\> | yes | AI-polished duty entries (post-polisher) |
| `rights_ai` | List\<DRRPEntry\> | yes | AI-polished rights entries |
| `responsibilities_ai` | List\<DRRPEntry\> | yes | AI-polished responsibility entries |
| `powers_ai` | List\<DRRPEntry\> | yes | AI-polished power entries |

This enables side-by-side comparison: the original regex-extracted entries in `duties`/`rights`/`responsibilities`/`powers` vs the AI-refined entries in `*_ai` columns. Once the AI model is validated, the `*_ai` columns may replace the originals — but during testing both are retained.

### Architecture Decision: No Claude Code in Production

The current drrp-polisher guest component uses Claude Code as the LLM backend via `fractal:ai/inference`. This was useful for prototyping but is **not aligned with the project's objectives** (see `docs/fractal-plan.md`):

- **Local-first principle** — no mandatory cloud dependency; the system must be functional offline
- **Fractal self-similarity** — the same architectural unit runs at every scale (edge, hub, cluster); a cloud LLM API breaks this
- **Data sovereignty** — ESH regulatory data should not leave the organisation's infrastructure
- **Cost and latency** — per-token API costs scale poorly for batch processing thousands of law sections

Testing with Claude Code is not needed since this will not go into production using Claude Code.

### Target Architecture: ONNX Structured Extraction Model

The DRRP polishing task is a **structured extraction** problem, not a general-purpose reasoning task. It requires:
1. Reading a clause of legislative text
2. Deciding if the clause boundary is correct (accept/expand/trim/delete)
3. If adjusting, identifying the exact text boundary

This is well-suited to a focused, fine-tuned model running via ONNX Runtime (`fractalaw-ai` crate, `onnx` feature gate):

**Training pipeline:**
1. Use LanceDB LAT table as the source — it holds the full legislative text with embeddings
2. Run the taxa regex classifier to produce initial DRRP annotations
3. Generate a representative training sample by running the polisher on a diverse set of laws
4. Curate the before/after pairs (`duties` vs `duties_ai`) as training data
5. Fine-tune a small encoder model (e.g. DeBERTa-v3-base, ~86M params) for the structured extraction task
6. Quantise to INT8 via ONNX Runtime for edge deployment
7. Validate against held-out laws, measuring accept/delete/expand/trim accuracy

**Why this works:**
- The task is highly constrained — 5 possible actions on bounded text windows
- Legislative text follows predictable structural patterns (the regex classifier already exploits this)
- A fine-tuned small model will outperform a general LLM on this narrow task
- INT8 quantised DeBERTa runs in <10ms per clause on CPU — fast enough for batch processing entire Acts
- Fits the fractal architecture: same model binary runs on hub and edge nodes via ONNX Runtime

### Testing Plan

1. **Select representative laws** — pick a diverse sample from the LanceDB LAT table:
   - Primary Acts (HSWA 1974, Environment Act 1995, CDM 2015)
   - Statutory Instruments (MHSWR 1999, COSHH 2002, LOLER 1998)
   - Mix of heavily-amended and clean laws
   - Mix of government-duty and governed-entity provisions

2. **Run taxa classifier** — produce `duties`/`rights`/`responsibilities`/`powers` entries for each section

3. **Run polisher against sample** — store results in `*_ai` columns

4. **Manual review** — examine before/after pairs for accuracy:
   - Are deletions correct? (false positives removed)
   - Are expansions capturing necessary context?
   - Are trims removing genuine bloat?
   - Are accepts genuinely correct entries?

5. **Measure metrics:**
   - Precision: what fraction of AI-accepted/modified entries are actually correct?
   - Recall: what fraction of genuinely correct entries does the AI preserve?
   - Boundary accuracy: when expanding/trimming, is the new boundary at the right position?

6. **Iterate** — refine the model/prompts based on error analysis, re-run on the sample

### Architecture Decision: Polisher Writes to LanceDB Only (Revised 2026-02-25)

**Decision**: The drrp-polisher guest writes AI-refined results directly to `legislation_text` in LanceDB (per-provision `ai_*` columns). No `polished_drrp` intermediate table. No DuckDB writes.

**Previous approach (superseded)**: The polisher wrote to a `polished_drrp` DuckDB table and aggregated into LRT `*_ai` columns. This was wrong because `legislation_text` lives in LanceDB — querying it via DuckDB silently failed. The intermediate table and law-level aggregation added unnecessary complexity.

**Revised rationale:**

1. **Co-location** — taxa data (the DRRP "map") and AI refinement live alongside the source text they were derived from. One table, one store, one query to get everything.

2. **LanceDB is the AI working store** — it's designed for exactly this: storing embeddings, structured classifications, and AI outputs alongside source data. DuckDB is for analytical queries.

3. **Locality of Logic** — the polisher has all context in one LanceDB row: source text, taxa classification, and writes AI results back to the same row. No cross-store joins needed.

4. **DuckDB is a copy** — law-level aggregates in DuckDB are derived from LanceDB's per-provision data. Copying from LanceDB → DuckDB is a separate concern (future task), not the polisher's responsibility.

### Implementation Steps

**Phase A: Schema & Plumbing (complete, partially superseded by Phase C)**

| Task | Status | Notes |
|------|--------|-------|
| Add `duties_ai`, `rights_ai`, `responsibilities_ai`, `powers_ai` to LRT schema.rs | [x] | **Superseded by Phase C** — AI output now goes to per-provision `ai_*` columns in `legislation_text` (LanceDB), not law-level LRT columns (DuckDB). |
| Add `*_ai` columns to DuckDB LRT table creation | [x] | **Superseded** — polisher no longer writes to DuckDB. The `ensure_drrp_ai_columns()` function remains but is unused by the polisher. |
| Add `polished_drrp` to `docs/SCHEMA.md` | [x] | **Superseded** — `polished_drrp` table no longer used. Polisher writes directly to `legislation_text` in LanceDB. |
| Rework drrp-polisher to read from LRT DRRP columns | [x] | **Superseded by Phase C rewrite** — polisher now reads/writes `legislation_text` in LanceDB exclusively. |
| Create SCHEMA-DIAGRAM.md | [x] | Still valid for DuckDB analytical schema. LanceDB `legislation_text` additions documented in Phase C. |
| Fix `polished_drrp` DDL bug (`text` → `ai_clause`) | [x] | **Superseded** — `polished_drrp` table no longer used. |
| Select representative law sample from LAT | [x] | 12 laws selected — see sample table below. |

**Phase B: Build ONNX model (next)**

The polisher needs an inference backend. Claude Code is not appropriate for production (see `fractal-plan.md`). Build a focused ONNX model that runs locally via `fractalaw-ai`.

| Task | Status | Notes |
|------|--------|-------|
| Design ONNX training pipeline | [x] | DeBERTa-v3-base extractive QA with 3 span heads (clause, holder, qualifier). Silver labels from fuzzy matching + gold labels for 12-law sample. See detailed design below. |
| Curate training dataset from existing DRRP data | [x] | `fractalaw export-training-data` CLI command. LCS fuzzy matching for clause spans, regex qualifier detection (10 patterns), law-level train/val/test splits. See Phase B Task 2 notes below. |
| Fine-tune and quantise ONNX model | [x] | Pipeline validated end-to-end. DistilBERT on 200-example CPU subset: clause_acc=60.5%, holder_acc=41.5%. ONNX export + INT8 quantisation working: 63.7MB model, 27.5ms/inference on CPU. See Phase B Task 3 notes below. |
| Wire ONNX model into polisher guest | [x] | `DrrpExtractor` in fractalaw-ai, ONNX routing in host `generate_impl`, CLI auto-loads model. Guest unchanged. See Phase B Task 4 notes below. |

**Phase C: DRRP Map in LanceDB + LanceDB-Only Polisher (2026-02-25)**

The polisher was originally designed to query DuckDB for DRRP entries and LAT source text. This was architecturally wrong — `legislation_text` (LAT) lives in LanceDB, and the polisher is an AI refinement task that should work entirely within the AI working store.

**Key architectural decision:** LanceDB is the AI working store. DuckDB is for analytical queries. The polisher works with LanceDB only — no DuckDB queries, no DuckDB writes. Copying polished results from LanceDB to DuckDB is a separate concern.

| Task | Status | Notes |
|------|--------|-------|
| Add 17 DRRP taxa + AI columns to `legislation_text` schema | [x] | 10 taxa columns + 7 ai_* columns. Field count 30→47. See schema details below. |
| Add LanceStore taxa/polisher write methods | [x] | `update_taxa()`, `update_polished()`, `query_unpolished()` — all use `merge_insert` keyed on `section_id` |
| Add LanceStore to host runtime with query/mutation routing | [x] | Free functions to avoid Send/Sync issues with DuckDB Connection. Routes `legislation_text` SQL to LanceDB. |
| Update `taxa enrich` to write per-provision taxa to LanceDB | [x] | Per-provision RecordBatch with taxa columns written via `lance.update_taxa()`. DuckDB law-level aggregates kept. |
| Wire LanceStore in CLI `cmd_run` | [x] | `lancedb` feature on host crate. LanceStore opened and passed in RunOptions. |
| Rewrite polisher guest for LanceDB-only | [x] | Major rewrite — provision-level processing, no DuckDB queries/writes. See details below. |
| All tests pass | [x] | 15 host tests, 188 core tests, workspace check clean. |

#### Schema: New `legislation_text` Columns

**Section 3.9 — DRRP Taxa** (per-provision classification from regex pipeline):

| Column | Arrow Type | Description |
|--------|-----------|-------------|
| `drrp_types` | `List<Utf8>` | "Duty", "Right", "Responsibility", "Power" |
| `governed_actors` | `List<Utf8>` | "Org: Employer", etc. |
| `government_actors` | `List<Utf8>` | "Gov: Minister", etc. |
| `duty_family` | `Utf8` | "Government", "Governed", "Unknown" |
| `duty_sub_type` | `Utf8` | "RegulationMaking", "SfairpDuty", etc. |
| `popimar` | `List<Utf8>` | "Policy", "Organising", etc. |
| `purposes` | `List<Utf8>` | "Enforcement", "General", etc. |
| `clause_refined` | `Utf8` | Modal-window extracted clause |
| `taxa_confidence` | `Float32` | Regex clause confidence score |
| `taxa_classified_at` | `Timestamp(ns, UTC)` | When taxa was run |

**Section 3.10 — AI-Refined DRRP** (polisher output, stored back in LanceDB):

| Column | Arrow Type | Description |
|--------|-----------|-------------|
| `ai_holder` | `Utf8` | AI-validated/corrected holder category |
| `ai_clause` | `Utf8` | AI-refined clause text |
| `ai_qualifier` | `Utf8` | Extracted qualifier phrase |
| `ai_clause_ref` | `Utf8` | Normalised article reference |
| `ai_confidence` | `Float32` | AI model confidence |
| `ai_model` | `Utf8` | "onnx" or "claude" |
| `ai_polished_at` | `Timestamp(ns, UTC)` | When AI refinement was run |

#### Host: LanceDB Query/Mutation Routing

The host's `data-query` and `data-mutate` WIT implementations now route `legislation_text` SQL to LanceDB when a LanceStore is attached:

- **Query routing**: `sql.contains("legislation_text") && lance.is_some()` → `lance_query_impl()` free function. Parses WHERE/LIMIT, handles `TO_JSON()` (serialises row to JSON for guest IPC compat) and `COUNT()` patterns.
- **Mutation routing**: `sql.contains("legislation_text") && lance.is_some()` → `lance_execute_impl()` free function. Parses `UPDATE ... SET ... WHERE ...`, calls LanceDB `table.update()` API.
- **Free functions**: LanceDB query/mutation functions are free functions (not methods on HostState) to avoid borrowing `&self` across `.await` points — DuckDB's Connection is not Sync, which would prevent the futures from being Send.

#### Polisher Guest: LanceDB-Only Rewrite

The polisher was completely rewritten for provision-level LanceDB-only processing:

**Old flow** (DuckDB-centric):
1. Query `legislation` for laws with DRRP entries
2. Unnest `duties[]/rights[]/etc.` struct arrays per law
3. Fetch LAT source text from `legislation_text` (DuckDB) per provision
4. Call AI, write to `polished_drrp` table
5. Aggregate back to `*_ai` LRT columns

**New flow** (LanceDB-only):
1. Count unpolished provisions: `SELECT COUNT(*) FROM legislation_text WHERE drrp_types IS NOT NULL AND ai_clause IS NULL`
2. Fetch each provision as JSON: `SELECT to_json(...) FROM legislation_text WHERE ... LIMIT 1 OFFSET N`
3. Build prompt from co-located taxa data (drrp_types, governed_actors, government_actors, duty_family, clause_refined) + source text
4. Call AI inference (ONNX local-first, Claude fallback)
5. Write result back: `UPDATE legislation_text SET ai_holder=..., ai_clause=..., etc. WHERE section_id='...'`

**No `polished_drrp` table. No `*_ai` columns on `legislation`. No DuckDB writes.** The polisher reads from and writes to LanceDB exclusively. DuckDB aggregation is a separate concern.

#### Data Flow (Updated 2026-02-25)

```
sertantai scrapes + parses XML
    │
    │  (zenoh pub/sub — when built)
    ▼
LAT text in LanceDB (legislation_text)
    │
    ▼
fractalaw taxa enrich (Rust regex)
    │
    ├─► LanceDB: per-provision taxa columns (drrp_types, actors, duty_family, etc.)
    │   Co-located with the source text it was derived from.
    │
    └─► DuckDB: law-level aggregates (duty_holder[], duty_type[], role[], etc.)
        For analytical queries and faceted search.
    │
    ▼
fractalaw run drrp-polisher.wasm (ONNX local-first)
    │
    └─► LanceDB: per-provision AI columns (ai_holder, ai_clause, ai_qualifier, etc.)
        Written back alongside the taxa data and source text.
```

**Key principle:** LanceDB holds per-provision text + per-provision DRRP map + per-provision AI refinement, all co-located. AI reads from one place, writes back to the same place. DuckDB holds law-level aggregates derived from LanceDB — a copy for fast analytical queries, not the AI working store.

#### Files Modified in Phase C

| File | Change |
|------|--------|
| `crates/fractalaw-core/src/schema.rs` | +17 columns to `legislation_text_schema()` (30→47 fields) |
| `crates/fractalaw-store/src/lance.rs` | +`update_taxa()`, `update_polished()`, `query_unpolished()` |
| `crates/fractalaw-host/Cargo.toml` | +`lancedb` feature with `fractalaw-store/lancedb` + `dep:serde_json` |
| `crates/fractalaw-host/src/lib.rs` | LanceStore in HostState/RunOptions, query+mutation routing, 3 free functions, updated tests |
| `crates/fractalaw-cli/Cargo.toml` | +`lancedb` feature on host dependency |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_enrich` writes per-provision to LanceDB; `cmd_run` opens LanceStore |
| `guests/drrp-polisher/src/lib.rs` | Complete rewrite for LanceDB-only, provision-level processing |

### Phase B Task 1: ONNX Training Pipeline Design

#### The Problem

The regex taxa parser produces rough DRRPEntry records. Examining the data:

```
"shall be the duty of every employer,..."       (truncated at comma)
"shall be the duty of every employer..."         (truncated with ellipsis)
"safety representatives from amongst the         (mid-sentence fragment)
 employees, and those representatives shall..."
```

These have three classes of defect:
1. **Truncated clauses** — regex cut off at arbitrary boundaries (commas, ellipses, line breaks)
2. **Misclassified holders** — regex assigns the wrong taxonomy category (e.g. `Org: Employer` when the provision actually targets `Gov: Inspector`)
3. **Missing qualifiers** — phrases like "so far as is reasonably practicable" not captured separately

The polisher's job: given the regex-extracted DRRPEntry and the full section text from LAT, produce a corrected entry with precise clause boundaries and separated qualifiers. The holder taxonomy (`Org: Employer`, `Org: Client`, `Gov: HSE`, etc.) is a fixed classification scheme that users query against — the 87 holder categories are an intentional faceted search dimension, not raw text to be replaced. The verbatim entity text ("every employer", "the employer concerned") is already visible in the clause. The holder head should **validate** the assigned category (flagging misclassifications) rather than extracting verbatim spans.

#### Training Data Inventory

| Metric | Value |
|--------|-------|
| Total DRRPEntry records | 110,366 (41K duties + 14K rights + 29K resp + 26K powers) |
| Laws with DRRP data | ~2,600 |
| Distinct holder categories | 87 |
| Distinct article references | 1,922 |
| Median clause length | 62 chars |
| 95th percentile clause length | 270 chars |
| Max clause length | 1,089 chars |
| Article format | `section-N` (53%), `regulation/N` (27%), `rule/N` (12%), other (8%) |

The 110K existing DRRPEntry records are the **input side** of the training data. We need to generate the **output side** (corrected entries) to create labelled pairs.

#### Task Formulation

This is a **hybrid extraction + validation** task with three outputs per entry:

1. **clause** (span extraction) — extract the precise provision text with correct boundaries from the source text
2. **qualifier** (span extraction) — extract any qualifying phrase (e.g. "so far as is reasonably practicable"), or null
3. **holder** (classification) — validate the regex-assigned holder category against the source text; output the correct taxonomy category from the fixed 87-class set, or confirm the original

The clause and qualifier outputs use **extractive QA** (SQuAD-style span extraction). The holder output is a **multi-class classification** over the fixed taxonomy — it preserves the faceted search dimension while correcting misclassifications.

#### Model Architecture

**Base model**: `deberta-v3-base` (86M params, 768-dim, 12 layers)

**Why DeBERTa:**
- State-of-the-art on extractive QA benchmarks (SQuAD 2.0)
- Disentangled attention handles long legal text well
- 86M params quantises cleanly to INT8 (~22MB ONNX file)
- HuggingFace `tokenizers` crate already in `fractalaw-ai` deps

**Task heads** (two span extraction + one classification):

```
Input:  [CLS] {drrp_type} : {regex_holder} [SEP] {section_text} [SEP]
         ↓
DeBERTa encoder (12 layers, 768-dim)
         ↓
    ┌────┴────┬──────────┐
    ▼         ▼          ▼
 clause    qualifier   holder
 start/end start/end   87-class
 (spans)   (or null)   (taxonomy)
```

- **Clause head**: linear layer over token embeddings producing start/end logit pairs — extracts the precise provision span from section text
- **Qualifier head**: same architecture as clause head, plus a "no qualifier" class for entries without qualifying phrases
- **Holder head**: linear layer over the `[CLS]` embedding producing logits over the 87 holder categories — validates or corrects the regex-assigned category

**Token budget**: DeBERTa supports 512 tokens. Median clause is 62 chars (~20 tokens). The section text is the long part — 95% of LAT sections fit within 512 tokens. For the 5% that exceed, truncate from the end (the relevant provision is usually near the top of the section).

#### Training Data Generation

**Step 1: Match DRRPEntry to LAT sections**

For each DRRPEntry in `duties[]/rights[]/etc.`:
- Parse the `article` field to get the provision number (e.g. `section-2` → `2`)
- Join to `legislation_text` on `law_name` + `provision`
- This gives us `(regex_entry, source_text)` pairs

**Step 2: Generate silver labels**

For each pair, generate labels for the three heads:
- **Clause span**: fuzzy string matching (Levenshtein or longest common substring) to align the regex clause to a span in the source text. The matched span boundaries become the silver label for clause start/end
- **Holder category**: the regex-assigned taxonomy category (e.g. `Org: Employer`) is used as-is for the silver label. The 87 categories form the fixed label set. Misclassifications will be caught during gold curation
- **Qualifier span**: regex for known qualifying patterns (`so far as is reasonably practicable`, `unless`, `except where`, `subject to`, etc.) applied to the source text around the clause span

**Step 3: Gold label curation (subset)**

For the 12-law sample (~700 entries), manually review and correct the silver labels. This is the validation set.

**Expected dataset sizes:**
- Silver training set: ~80K pairs (entries with successful LAT match)
- Gold validation set: ~700 pairs (12 sample laws, manually reviewed)
- Held-out test set: ~300 pairs (from 4-5 laws not in sample)

#### Training Pipeline

```
1. Export training data
   fractalaw export-training-data --output data/drrp-training/
   → Parquet files: train.parquet, val.parquet, test.parquet

2. Fine-tune (Python, outside fractalaw)
   python scripts/train_drrp_model.py \
     --base-model microsoft/deberta-v3-base \
     --train data/drrp-training/train.parquet \
     --val data/drrp-training/val.parquet \
     --epochs 5 --batch-size 16 --lr 2e-5 \
     --output models/deberta-v3-drrp/

3. Export to ONNX
   python scripts/export_onnx.py \
     --model models/deberta-v3-drrp/ \
     --output models/deberta-v3-drrp/model.onnx \
     --quantize int8

4. Validate
   python scripts/validate_drrp_model.py \
     --model models/deberta-v3-drrp/model.onnx \
     --test data/drrp-training/test.parquet
```

**Training happens in Python** (PyTorch + HuggingFace Transformers). The resulting ONNX model is consumed by `fractalaw-ai` in Rust. This is the standard pattern — training in Python, inference in Rust via ONNX Runtime.

#### Integration into fractalaw-ai

Add a `DrrpExtractor` struct alongside the existing `Embedder`:

```rust
// crates/fractalaw-ai/src/extractor.rs
pub struct DrrpExtractor {
    session: Session,       // ONNX Runtime
    tokenizer: Tokenizer,   // DeBERTa tokenizer
}

impl DrrpExtractor {
    pub fn load(model_dir: &Path) -> Result<Self>;

    /// Extract refined DRRP entry from regex input + source text.
    pub fn extract(
        &self,
        drrp_type: &str,
        regex_holder: &str,
        regex_clause: &str,
        source_text: &str,
    ) -> Result<DrrpExtraction>;

    pub fn extract_batch(...) -> Result<Vec<DrrpExtraction>>;
}

pub struct DrrpExtraction {
    pub holder: String,      // taxonomy category (e.g. "Org: Employer") — validated/corrected
    pub holder_changed: bool,// true if model corrected the regex-assigned category
    pub clause: String,      // precise span from source text
    pub qualifier: Option<String>,
    pub clause_ref: String,  // normalised article reference
    pub confidence: f32,     // min of clause span + holder classification confidence
}
```

#### WIT Interface

The existing `ai-classify` WIT interface is not the right fit. This is extraction, not classification. Options:

1. **Use `ai-inference::generate`** — the current polisher approach. Works but means the WASM guest calls a generic "generate" function and the host routes to ONNX instead of Claude. Clean abstraction — guest doesn't care about the backend.

2. **Add `ai-extract` interface** — more precise but more WIT surface area.

**Recommendation**: Option 1. Keep `ai-inference::generate` as the interface. The host switches backend based on config: Claude API key present → Claude, ONNX model present → ONNX. The guest code doesn't change at all. The JSON request/response format stays the same.

#### Performance Targets

| Metric | Target |
|--------|--------|
| Inference latency | <10ms per entry (INT8, CPU) |
| Model size | <25MB (INT8 quantised ONNX) |
| Clause boundary accuracy | >90% exact match on gold set |
| Holder classification accuracy | >95% on gold set (most regex assignments are already correct) |
| Qualifier detection F1 | >80% |
| Throughput | >100 entries/sec (batch of 32) |

#### Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Silver labels too noisy | Fuzzy matching with confidence threshold; discard low-confidence pairs |
| 512 token limit insufficient for long sections | Sliding window with stride; extract from the window containing the regex clause |
| DeBERTa too large for edge | Try `deberta-v3-xsmall` (22M params) first; fall back to `base` if accuracy insufficient |
| Training data domain shift (legacy sertantai vs new taxa) | Include both sertantai and taxa-generated entries in training set |

### Phase B Task 2: Training Data Export Implementation (2026-02-25)

**New files:**
- `crates/fractalaw-core/src/training.rs` — Pure Rust module with all training data logic (29 unit tests)

**Modified files:**
- `crates/fractalaw-core/src/lib.rs` — Added `pub mod training;`
- `crates/fractalaw-store/src/duck.rs` — Added `extract_flat_drrp_entries()` using DuckDB `UNNEST`
- `crates/fractalaw-cli/src/main.rs` — Added `ExportTrainingData` command variant + handler
- `crates/fractalaw-cli/Cargo.toml` — Added `parquet` workspace dep

**Core logic in `training.rs`:**
- `parse_article_to_provision()` — Parses `section/2`, `regulation-4` etc. to bare provision numbers
- `find_clause_span()` — LCS (longest common substring) DP algorithm for fuzzy matching regex clauses against LAT source text. Case-insensitive. Returns char offsets + match ratio
- `find_qualifier()` — 10 UK ESH qualifier patterns via `LazyLock<Vec<Regex>>`: SFAIRP, "so far as is practicable", "where reasonably practicable", "as far as possible", "to the extent that", "subject to", "provided that", "unless", "except where", "except in so far as". Returns nearest qualifier to clause span
- `generate_silver_label()` — Orchestrates the above into a `TrainingExample` with identity, inputs, silver labels, quality metrics
- `training_example_schema()` — 16-column Arrow schema for Parquet output
- `examples_to_record_batch()` — Converts `Vec<TrainingExample>` to Arrow RecordBatch

**DuckDB extraction:**
- `extract_flat_drrp_entries()` — UNION ALL of four UNNEST queries over `duties/rights/responsibilities/powers` columns. Returns `(law_name, drrp_type, holder, clause, article)` flat rows

**CLI command: `fractalaw export-training-data`:**
- `--output` (default `./data/drrp-training`) — Parquet output directory
- `--val-laws` — Optional file of validation law names (one per line)
- `--test-laws` (default 5) — Number of laws for held-out test set
- `--min-match-ratio` (default 0.3) — Minimum LCS match quality to include
- Split strategy: by law (prevents information leakage). Val from file, test via deterministic hash selection, rest = train
- Handler: DuckDB extraction → group by law → LanceDB join (one query per law) → silver label generation → Parquet write → statistics report

**Parquet schema (16 columns):**
`law_name, drrp_type, article, provision, split, regex_holder, regex_clause, source_text, clause_start, clause_end, holder_label, qualifier_start, qualifier_end, qualifier_text, match_ratio, match_quality`

**First run results (2026-02-25):**

```
Total DRRP entries:       110,366
Laws with DRRP data:        3,285
Laws with LAT text:          ~400 (only scraped laws have text in LanceDB)

Matched to LAT:             7,019  (6.4%)
Unmatched (no LAT):         99,286  (90.0%)
Unparseable article:            22  (0.0%)

Match quality distribution:
  High   (>0.8):            2,263  (32.2%)
  Medium (0.5-0.8):           918  (13.1%)
  Low    (<0.5):            3,838  (54.7%)

With qualifier:               616  (8.8%)
Holder categories:            119
Distinct laws in output:      235

Split sizes:
  Train:  7,017 examples (no --val-laws file provided)
  Test:       2 examples (5 test laws, but most had no LAT)
```

**Spot-check on HSWA 1974 (UK_ukpga_1974_37):**
- s.2 "shall be the duty of every employer" — high (0.92), correct span
- s.7 "shall be the duty of every employee" — high (0.92), correct span
- s.8 "No person shall intentionally or recklessly interfere with..." — high (1.00), perfect match
- s.9 "No employer shall levy or permit..." — high (1.00), perfect match
- Low-quality matches are mostly too-short regex clauses ("an inspector shall ,...") that can't uniquely locate in source

**Key observations:**
1. Only 6.4% LAT coverage — most laws haven't been scraped yet. Training set will grow as more laws are scraped into LanceDB
2. The 2,263 high-quality matches are solid training signal for clause span extraction
3. 119 holder categories found (vs 87 predicted from DuckDB query) — some categories have prefix/suffix variants
4. Low-quality matches (55%) are dominated by very short regex clauses (<15 chars) — the LCS minimum threshold filters out the worst
5. Qualifier detection working: 616 entries with SFAIRP and similar patterns detected

### Phase B Task 3: Fine-tune and Quantise ONNX Model (2026-02-25)

**New files:**
- `scripts/train_drrp_model.py` — PyTorch training script with 3-head model (clause span, qualifier span, holder classification)
- `scripts/export_onnx.py` — ONNX export with INT8 quantisation and validation

**Model outputs:**
- `models/deberta-v3-drrp/final_model.pt` — Trained checkpoint (DistilBERT, 200 examples, 3 epochs)
- `models/deberta-v3-drrp/model.onnx` — Full ONNX model (253.4 MB)
- `models/deberta-v3-drrp/model.int8.onnx` — INT8 quantised (63.7 MB)
- `models/deberta-v3-drrp/metadata.json` — Training metadata
- `models/deberta-v3-drrp/holder_labels.json` — 27-class label mapping

**Training architecture (`DrrpExtractorModel`):**
- Encoder (DistilBERT for CPU validation; DeBERTa-v3-base for GPU production)
- Clause head: Linear(hidden, 2) → start/end logits over sequence positions
- Qualifier head: Linear(hidden, 2) + Linear(hidden, 2) → start/end logits + has_qualifier
- Holder head: Linear(hidden, num_classes) → 87-class taxonomy classification from [CLS] token
- Combined loss: clause (1.0) + qualifier (0.5) + holder (0.3) weighting
- Differential learning rates: encoder at lr, heads at lr×10

**Training data:**
- 200-example high-quality subset (match_ratio > 0.8) from the 7,019 matched entries
- max_length=128 tokens (CPU-feasible)
- Train/val split: 80/20

**CPU training results (DistilBERT, 200 examples, 3 epochs):**
```
Epoch 1: loss=4.91, clause_acc=0.500, holder_acc=0.268
Epoch 2: loss=3.87, clause_acc=0.512, holder_acc=0.268
Epoch 3: loss=3.41, clause_acc=0.605, holder_acc=0.415
```

Model is learning (loss decreasing, accuracy improving) but underfitting — needs more data and epochs. Full training on GPU with DeBERTa-v3-base on the complete 7K+ dataset will produce significantly better results.

**ONNX export results:**
- Full model: 253.4 MB (DistilBERT base)
- INT8 quantised: 63.7 MB (75% compression)
- Inference latency: 27.5 ms/inference on CPU (100 runs)
- Smoke test: Correctly predicted `Org: Employer` for HSWA s.2 test case
- Clause span head outputs `[13-8]` (start > end) — expected with minimal training

**Key implementation decisions:**
1. Used `dynamo=False` in `torch.onnx.export()` — the dynamo exporter had shape inference errors with multi-output model
2. Opset version 17 (instead of 14) for better operator coverage
3. ONNX Runtime dynamic quantisation (`QuantType.QInt8`) — no calibration dataset needed
4. Legacy TorchScript exporter works reliably for this architecture

**Production path:**
- Train on GPU machine with DeBERTa-v3-base, full 7K+ silver labels, 5+ epochs, max_length=512
- Expected INT8 model size: ~22MB (DeBERTa-v3-base quantised)
- Expected latency: <10ms/clause on CPU with DeBERTa
- Rerun `fractalaw export-training-data` as more laws are scraped into LanceDB for larger training set

### Phase B Task 4: Wire ONNX Model into Polisher Guest (2026-02-25)

**Design decision:** Keep `ai-inference::generate` as the single WIT interface. The host routes to ONNX when available, falls back to Claude. Guest code unchanged — it calls `generate()` and gets back JSON regardless of backend.

**New files:**
- `crates/fractalaw-ai/src/extractor.rs` — `DrrpExtractor` struct with ONNX inference pipeline (7 unit tests, all passing)

**Modified files:**
- `crates/fractalaw-ai/src/lib.rs` — Added `mod extractor; pub use DrrpExtractor, DrrpExtraction`
- `crates/fractalaw-ai/Cargo.toml` — Added `serde`, `serde_json` deps
- `crates/fractalaw-host/Cargo.toml` — Added `onnx = ["fractalaw-ai/onnx", "dep:serde_json"]` feature
- `crates/fractalaw-host/src/lib.rs` — Added `extractor` to HostState/RunOptions, ONNX routing in `generate_impl`, `parse_drrp_prompt()` helper
- `crates/fractalaw-cli/Cargo.toml` — Enabled `onnx` feature on host
- `crates/fractalaw-cli/src/main.rs` — Auto-loads `DrrpExtractor` from `models/deberta-v3-drrp/` or `DRRP_MODEL_DIR` env var
- `scripts/export_onnx.py` — Added `tokenizer.save_pretrained()` for Rust `tokenizers` crate compatibility
- `models/deberta-v3-drrp/tokenizer.json` — Saved from HuggingFace cache

**`DrrpExtractor` (`extractor.rs`):**
- Loads `model.int8.onnx` (preferred) or `model.onnx` + `tokenizer.json` + `metadata.json` + `holder_labels.json`
- `extract(drrp_type, regex_holder, source_text, article)` → `DrrpExtraction { holder, ai_clause, qualifier, clause_ref, confidence }`
- Tokenizes as text pair: `[CLS] {drrp_type} : {holder} [SEP] {source_text} [SEP]` with `OnlySecond` truncation + fixed padding
- 6 output tensors: clause start/end, qualifier start/end, has_qualifier, holder logits
- Span decoding: argmax → clamp to source text region → `tokenizer.decode()`
- Holder: softmax argmax → label lookup from `holder_labels.json`
- `DrrpExtraction::to_json()` serialises to guest's expected `PolishedOutput` format

**Host routing (`generate_impl`):**
1. If ONNX extractor attached → parse user_prompt → extract structured fields → call `extractor.extract()` → return JSON as `GenerateResponse`
2. If prompt doesn't match DRRP format → fall through to Claude (supports non-DRRP guests)
3. If no extractor → Claude API path (unchanged)
4. If neither → error

**`parse_drrp_prompt()`:** Line-by-line parser for the known guest prompt format. Extracts `drrp_type`, `holder`, `article`, `source_text`. Returns `None` on parse failure → graceful fallthrough.

**CLI model loading:** Checks `DRRP_MODEL_DIR` env var or default `models/deberta-v3-drrp`. Logs info on success, warns on failure (falls back to Claude), debug on not-found.

**Test results:**
- 7 extractor unit tests: all passing (load, extract HSWA s.2, JSON format, clause_ref variants, argmax, softmax, clamp_span)
- 3 AI host tests: all passing (embed stubs, generate without config)
- 9 data host tests: all passing (query, insert, execute, data-test guest, polisher no-data, polisher inference errors)
- Workspace check: clean

**Phase B complete.** All 4 tasks done. The ONNX pipeline is wired end-to-end:
1. `fractalaw export-training-data` → Parquet with silver labels
2. `scripts/train_drrp_model.py` → 3-head model training
3. `scripts/export_onnx.py` → ONNX export + INT8 quantisation + tokenizer save
4. `fractalaw-ai::DrrpExtractor` → Rust inference via ONNX Runtime
5. `fractalaw-host::generate_impl` → routes polisher guest to local ONNX
6. `fractalaw-cli::cmd_run` → auto-discovers and loads model

## Backlog

1. Explore `RegexSet` pre-filter optimisation for batch classification
2. Consider `rayon` parallelism for classifying all sections of large Acts
3. ~~LanceDB query host function for polisher~~ — **Done in Phase C**
4. ~~In-memory pipeline optimisation: taxa → polisher without intermediate DuckDB write~~ — **Addressed in Phase C** (polisher works with LanceDB only, no DuckDB intermediary)
5. Copy polished results from LanceDB → DuckDB for analytical queries (law-level aggregates)
6. Run taxa → polisher end-to-end on 12-law sample with ONNX model
7. Build comparison tooling (before/after diff for taxa vs AI-refined results in LanceDB)
8. Clean up superseded Phase A artifacts: `polished_drrp` DDL in `duck.rs`, `ensure_drrp_ai_columns()`, `*_ai` LRT columns in `schema.rs`
