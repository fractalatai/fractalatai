# Gap C — Session 1: main-repo prep

**Date**: 2026-04-15
**Status**: Not started
**Orchestration**: [04-15-26-gap-c-orchestration.md](04-15-26-gap-c-orchestration.md)
**Spec**: [04-15-26-gap-c-ai-research.md](04-15-26-gap-c-ai-research.md)

## Scope

Stages S1a, S1b, S3 from the research doc. All work in the `fractalaw`
repo. Everything needed on the main side *before* training can start.

**Scope change 2026-04-15**: the earlier S2 (`holder_inferred_from`
schema extension through `DrrpExtraction` / DuckDB LRT / Zenoh publish)
is **deferred to Session 3 S7**, not de-scoped. It ships alongside the
remote detector in S7, which is the producer that populates the field
— deferring avoids adding unused schema to main before there's anything
to fill it. Field design in research doc §4.1a is unchanged.

## Entry criteria

- Research doc read and understood.
- Latest `master`, clean working tree.
- Disk check: `df -h /var/home` ≥ 5 GB free (see MEMORY.md).
- OHS gap-analysis outputs available (they define the Gap C candidate set).

## Deliverables

### S1a — Gap C sub-type classifier

**Goal**: every Gap C provision classified into C1–C6 with reproducible
rules. Produces the parquet sidecar that Session 2 consumes.

Tasks:
- [ ] Freeze the C1–C6 taxonomy in the `taxa-gap-analysis` skill
  (`.claude/skills/taxa-gap-analysis/SKILL.md`). Include the definitions
  already in the research doc §1.
- [ ] New CLI subcommand or script (probably under `crates/fractalaw-cli`
  or a one-off script in `scripts/`) that:
  - Reads Gap C candidates from LanceDB (modal-verb-present + no DRRP +
    not skip-gated).
  - Classifies each into C1–C6 using regex + clause-structure heuristics.
    Expected to need `clause_structure.rs` for C2 detection (sub-clause
    depth), heading/ToC lookups for C4.
  - Writes a parquet file: `law_id, article, text, sub_type,
    parent_article (nullable), section_ref (nullable), notes`.
  - Reports counts per sub-type — must reconcile with the OHS numbers
    (3,275 total for that run; sum of C1..C6 ≈ that figure).
- [ ] Tests covering at least one real provision per sub-type.

**Output artefact**: `scripts/gap_c_dataset.parquet` (gitignored — regenerable).

### S1b — Model-vocabulary reconciliation

**Goal**: a cleaned, reconciled `holder_labels.json` pinned for Session 2
training. Resolves critical review items #2 and #3. See research doc
§4.1's two-vocabulary section for rationale.

Tasks:
- [ ] **Fix the `"Gvt: Ministry:"` trailing-colon bug.** Grep for the
  exact string (including trailing colon) across the repo and replace
  with `"Gvt: Ministry"`. Check: `holder_labels.json`, `actors.rs`,
  tests, any DuckDB/sertantai join tables. Note any sertantai-side
  occurrences for the Session 3 backfill step.
- [ ] **Exclude `": He"` from the model vocabulary.** Do **not** remove
  it from `actors.rs` yet — the regex path still uses it as a pronoun
  placeholder. Just omit it from the training-pinned `holder_labels.json`
  and document the exclusion in a comment/note next to the file.
- [ ] **Audit `actors.rs` vs. current `holder_labels.json`.** Produce a
  diff list. Expected missing labels (from critical review #3 and the
  actors.rs grep):
  - Supply-chain: `SC: C: Contractor`, `SC: C: Principal Contractor`,
    `SC: C: Designer`, `SC: C: Principal Designer`, `SC: Manufacturer`,
    `SC: Importer`, `SC: Client`, `SC: T&L: Carrier`, `Svc: Installer`
  - Individuals: `Ind: Worker`, `Ind: User`, `Ind: Competent Person`
  - Organisations: `Org: Company`, `Org: Landlord`
  - Offshore: `Offshore: Licensee`
  - Specialist: `Spc: Employees' Representative`, `Spc: Trade Union`,
    `Spc: Assessor`, `Spc: Engineer`
  - Government specifics: missing agency names (Environment Agency,
    SEPA, ONR, OEP, ORR, OFCOM, NRW, MCA, OGA, HSE NI), authority
    specifics (Local, Planning, Fire and Rescue, Licensing, Waste,
    Public, Traffic, Market), ministry specifics (Treasury, HMRC,
    MoD, DETI), devolved admins, Commissioners, Officer,
    Appropriate Person
  - Any FIRE specialist actors not yet in holder_labels (per recent
    commits adding fire and rescue authority)
- [ ] **Add the reconciled set** to a new or refreshed `holder_labels.json`.
  Order/format should match the existing file convention.
- [ ] **Verify against regex output.** For each added label, confirm at
  least one real provision in LanceDB where regex extracts that label.
  If a label appears in `actors.rs` but never actually matches real
  text, flag it — may be a dead pattern.
- [ ] Document the pin: a short note alongside `holder_labels.json`
  (e.g. `holder_labels.README.md`) stating "this file is the
  training-pinned model vocabulary, distinct from `actors.rs`'s regex
  detection vocabulary. See research doc §4.1."

**Output artefact**: reconciled `holder_labels.json` committed to main,
plus the diff list as a record of what changed. This is a **hand-off
gate** — Session 2 labelling and training consume this file; it must
be stable before Phase B starts.

### S2 — deferred to Session 3

The earlier S2 (`holder_inferred_from` schema extension through
`DrrpExtraction` / DuckDB LRT / Zenoh publish) is **deferred to Session
3 S7**, not de-scoped. Rationale: the schema has no producer in main
without the remote detector, so landing the field in Session 1 adds
dead columns and an unused code path. Deferring lets the field ship
in one atomic change alongside the code that populates it.

Field design specification stays intact in research doc §4.1a — the
design doesn't change, only the timing.

### S3 — Context-retrieval helper

**Goal**: deterministic, tested helper that fetches the parent clause,
section heading/definitions, and act-level general duty for a given
provision from LanceDB. Its output format is the contract Session 2 must
match.

Tasks:
- [ ] New module in `fractalaw-store` (probably `context.rs`) with a
  function roughly like:
  ```
  pub fn fetch_context(
      law_id: &str,
      article: &str,
  ) -> Result<ProvisionContext>
  ```
  where `ProvisionContext` has:
  - `parent_clause: Option<String>` — for C2
  - `section_definitions: Option<String>` — for C4 (interpretation section)
  - `act_general_duty: Option<String>` — for C6 (act's headline duty)
  - `sources: Vec<CitationRef>` — the clause refs that contributed,
    used to populate `holder_inferred_from` downstream.
- [ ] Metadata-first retrieval: by `law_id` + `article` hierarchy. No
  vector search needed for same-act lookups.
- [ ] Cross-act references (rare): document as out-of-scope for S3; note
  where they'd slot in (probably a separate vector-search path).
- [ ] Tests covering HSWA s.2 as general duty, CDM 2015 reg-hierarchy
  parent lookup, and an interpretation-section lookup.
- [ ] **Write the format spec**: `docs/gap-c-context-format.md` in-tree.
  This is the contract training must conform to. Include a worked
  example: text fed to the tokenizer verbatim.
- [ ] **Pin the token budget in the format spec**. Per research doc
  §4.2, ModernBERT-large has an 8192-token native context. Target
  **2048 tokens** at training/inference time for the combined
  `[clause | parent | section definitions | act-general-duty]` window:
  - Generous headroom for long UK provisions plus full context
  - Avoids GPU waste from padding most examples wouldn't use
  - Well within ModernBERT-large's native ceiling; also works for
    DeBERTa-v3-large comparison baseline with truncation prioritised
    as below
  - Truncation priority (longest-first trim) if any example exceeds
    2048: (1) keep the target clause intact, (2) trim
    act-general-duty, (3) trim section definitions, (4) trim parent
    clause from the start, (5) as a last resort, trim target clause
    from the end.
  - Report the exceed-rate during S1b/S3 dev — if more than ~5% of
    Gap C provisions exceed 2048, raise the budget rather than
    aggressively truncating.

**Output artefact**: `fetch_context` + `docs/gap-c-context-format.md`
committed to main.

## Exit criteria

Per the orchestration doc:
- Gap C parquet exists; sub-type counts reconcile with OHS numbers
  (S1a). **C2 count is the critical number** — it's what Session 2
  Phase 1 trains on.
- Reconciled `holder_labels.json` committed; trailing-colon bug fixed;
  `": He"` excluded; ~30+ concrete roles added (S1b). This is a hard
  gate for Session 2.
- Context-retrieval helper has tests for C2 (parent-clause lookup).
  C4 and C6 tests are nice-to-have for Phase 1; required before Phase 2.
- Context format spec (`docs/gap-c-context-format.md`) committed with
  the 2048-token budget pinned.
- Zero regression in existing taxa tests; precision 96.4% intact.
- `holder_inferred_from` work is **not** an S1 exit criterion —
  deferred to Session 3 S7 (see scope change above).

## Hand-off package (to Session 2)

Produce a hand-off note at the end of this session listing:
1. Path to the Gap C parquet (or how to regenerate it).
2. The committed `docs/gap-c-context-format.md` path + commit SHA.
3. The `holder_labels.json` SHA at hand-off time.
4. Any taxonomy revisions made during the session (vs. research doc §1).

## Open items

- Whether the S1 classifier lives as a CLI subcommand (`fractalaw taxa
  gap-c classify`) or a one-off `scripts/` Python/Rust script. Lean CLI
  subcommand for reproducibility, but script is faster if this is
  exploratory.
- Whether context retrieval should be a LanceDB query or span multiple
  tables (DuckDB for act-general-duty lookups might be cleaner since
  heading/structure data is metadata). Decide during S3.
