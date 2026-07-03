---
session: Gap C — AI-assisted DRRP detection research
status: closed
opened: 2026-04-15
closed: 2026-04-15
outcome: success
summary: 'Research and decision record for solving Gap C (implicit-actor DRRP provisions, 78% of false negatives). Established
  quality-first lens separating the parsing service from local-first constraints, selected ModernBERT-large as primary backbone
  with remote serverless-GPU inference, designed holder_inferred_from provenance schema, two-vocabulary model for regex-to-AI
  transition, and a 9-stage phased plan across three sessions and two repos.

  '
decisions:
- what: Quality-first lens for parsing service
  why: Parsing service feeds sertantai on server hardware; local-first constraints do not apply
  result: Unlocked ModernBERT-large, full fine-tune fp16, remote GPU serving
- what: ModernBERT-large as primary backbone, DeBERTa-v3-large as comparison baseline
  why: 8192 native context eliminates truncation; modern pretraining gives better fine-tune results
  result: Train both, pick winner on held-out laws
- what: Remote serverless-GPU inference via RunPod/Modal/HF Endpoints
  why: Scale-to-zero economics (<$500/year), server-side model updates, no client redeployment
  result: fractalaw-ai extended with remote provider alongside existing local ONNX
- what: No meta-labels in holder vocabulary
  why: Users search by concrete role; Passive/Inherited are not user-facing concepts
  result: Below-threshold detections suppressed rather than escape-hatched
- what: Separate training repo for Python/GPU work
  why: Different hardware, stack, lifecycle, licence, and reproducibility requirements
  result: Only training pipeline forks out; fractalaw unchanged
- what: Deprecated existing DistilBERT artefact at models/deberta-v3-drrp/
  why: Misnamed directory, 200 training examples, best_clause_acc 0.0, not functional
  result: New model trained from scratch; no weights, tokenizer, or heads reused
- what: holder_inferred_from provenance field on DrrpExtraction
  why: Audit trail and user-facing explainability for implicit-actor inferences
  result: Nullable citation string; tracked in Rust integration layer, not model head
- what: Two-vocabulary model for regex actors vs model holders
  why: 'Regex path needs placeholders (e.g. ": He") during transition; model vocabulary is user-facing only'
  result: Regex placeholders route through AI for resolution; suppressed if unresolved
lessons:
- title: Gap C is not one problem
  detail: Six sub-types (C1-C6) with different tractability; must scope AI to specific sub-types not the monolith
  tag: architecture
- title: External AI advice was under-informed about the repo
  detail: ChatGPT and Gemini advice was broadly sound but conflated model identity, assumed runtime LLM, and missed the local-first
    vs quality-first distinction
  tag: process
- title: Detection not polishing is the real gap
  detail: Gap C provisions produce no DRRP entry at all from regex, so even a working polisher would never be invoked on them
  tag: architecture
metrics:
  gap_c_provisions: 3275
  gap_c_share_of_fn: 78%
  holder_labels_current: 27
  holder_labels_needed: ~60+
  annual_cost_estimate: <$500
artifacts:
- .claude/plans/gap-c-research.md
- crates/fractalaw-ai/src/extractor.rs
- crates/fractalaw-core/src/taxa/actors.rs
- models/deberta-v3-drrp/holder_labels.json
depends_on:
- 02-26-26-taxa-regex-patterns
- 02-26-26-taxa-refinement
- 02-26-26-v2-promotion-enrichment
- taxa-gap-analysis/04-14-26-ohs-occupational-safety
enables:
- 04-15-26-gap-c-session-1-main-prep
- 04-15-26-gap-c-session-2-training-repo-todo
- 04-15-26-gap-c-session-3-main-integration
---

# Gap C \u2014 AI-assisted DRRP detection research (CLOSED)

**Date**: 2026-04-15
**Inputs**: `.claude/plans/gap-c-research.md` (ChatGPT + Gemini advice)
**Prior sessions**:
- `02-26-26-taxa-regex-patterns.md` (Gap A/B/C taxonomy)
- `02-26-26-taxa-refinement.md`
- `02-26-26-v2-promotion-enrichment.md` (ONNX polisher promoted)
- `taxa-gap-analysis/04-14-26-ohs-occupational-safety.md` (Gap C = 78% of FN)
- `04-15-26-gap-c-critical-review.md` (critique that prompted §0 below)

## 0. Scope clarification — quality-first, not local-first

**Added 2026-04-15** after the critical review flagged that earlier sections
conflated two distinct ambitions of fractalaw and imported local-first
constraints into a subsystem that shouldn't carry them.

Fractalaw serves two distinct roles:

1. **Local-first app for end-users.** CLI, offline search, on-device
   queries against local DuckDB/LanceDB. Governed by CLAUDE.md.
   CPU-friendly, modest memory, no network dependency. Embeddings,
   similarity search, and user-facing query commands live here.

2. **Parsing service that feeds sertantai.** Batch enrichment of
   legislative text into structured DRRP + taxa. Quality-first.
   Server-class hardware is acceptable. Runs on the developer's machine
   or rented infra; its outputs are published to sertantai via Zenoh.
   **The DRRP detector, polisher, and any future AI parsing components
   belong here.**

Decisions in this document — model size, precision, token budget,
compute, artefact size — are made under the **quality-first lens**. The
parsing service is optimised for legal-text accuracy, not device
footprint. Local-first constraints from CLAUDE.md do not apply to this
subsystem.

Concretely, what this clarification unlocks:

- **Model size**: ModernBERT-large (~395M params, 8192 native context) is
  the primary backbone; DeBERTa-v3-large (~435M params, 512 context) is
  kept as a comparison baseline. Size is a quality decision, not a device
  decision. The existing DistilBERT artefact at `models/deberta-v3-drrp/`
  (misnamed — it is DistilBERT, not DeBERTa-v3) is **deprecated** and
  contributes nothing to the new training; see §4.1 and §9.
- **Precision**: fp16 is the default; int8 quantisation is optional and
  only used if it preserves accuracy while helping throughput.
- **Token budget**: 512 or 1024 tokens, driven by the length of UK
  provisions plus retrieved context — not constrained to 128 for
  on-device memory reasons.
- **Runtime footprint**: ONNX via `ort` in-process is still preferred
  for determinism and audit, but running the parser as a separate
  service process on server hardware is acceptable if it simplifies
  deployment. The publish boundary to sertantai is unchanged.
- **Compute**: inference can use a GPU during enrichment runs. Enrichment
  is a batch job run on demand, not a per-user query path.

What this clarification does **not** change:

- **No runtime LLM for parsing.** Determinism, auditability, licence,
  and cost-at-scale still rule out prompting a frontier model per
  provision. A fine-tuned classifier remains the right tool; the
  quality-first lens just means we pick a bigger, better one.
- **No meta-labels in outputs.** User-facing taxonomy rationale
  (§4.1 Q1 resolution) is independent of deployment scope.
- **`holder_inferred_from` provenance.** Quality-first actually
  strengthens the case for audit provenance.
- **Separate training repo.** Still correct — see §7.
- **Local-first for everything else.** Embeddings, DuckDB queries,
  LanceDB search, the CLI — all stay local-first per CLAUDE.md. Only
  the AI parsing pipeline sits outside that constraint.

Every subsequent section is written under the quality-first lens.
Reviewers noticing a size/compute/footprint argument creeping back in
should challenge it against §0.

### 0.1 Deployment model — remote serverless-GPU for hub-side parsing

The parsing service runs as a **remote serverless-GPU endpoint** (RunPod
Serverless, Modal, or HF Inference Endpoints). The local `fractalaw` CLI
does regex parsing on CPU; when a provision falls into Gap C (post-gate,
no regex DRRP), the CLI calls the remote endpoint with text + context.
The endpoint returns structured DRRP + holder + `holder_inferred_from`,
or `unresolved`.

Why serverless rather than always-on:
- The call pattern is bursty. After initial catchup, new UK ESH laws
  arrive in the low tens per month, producing tens-to-low-hundreds of
  Gap C provisions.
- Scale-to-zero means near-zero idle cost. Pennies per month ongoing
  after the one-off catchup.
- Model updates happen server-side. The local CLI keeps calling the
  same endpoint; no client redeployment when the taxonomy evolves.

### 0.2 Relationship to the micro-apps architecture

The parsing service does **not** fork from fractalaw. The existing
micro-apps plan (`.claude/plans/micro-apps.md`) anticipates this exact
pattern and provides the abstraction boundary we need:

- Every micro-app calls AI through the WIT interface
  `fractal:ai/inference`. It does not know what runs behind the interface.
- Hub-side micro-apps are explicitly allowed to use capable generative
  models (8B+ named in the plan). Server-class was always in the design.
- The **DRRP Polisher is the plan's anchor example #1** for hub-side
  micro-apps. The Gap C detector is the same shape with a better model.

What changes is the **provider behind `fractal:ai/inference`**:

| Model | Provider | Justification |
|---|---|---|
| MiniLM embeddings (edge + hub) | Local ONNX via `ort` — unchanged | Edge offline requirement |
| Lightweight classifiers (edge) | Local ONNX — unchanged | Same |
| Gap C detector / DRRP polisher (hub) | **Remote serverless-GPU** | Quality-first, hub-only, network-acceptable |

`fractalaw-ai` is **not rewritten**. It is extended with a remote-
inference provider alongside the existing local-ONNX provider. Dispatch
is per-model. Existing edge micro-apps keep working unchanged. The new
Gap C hub micro-app uses the new remote provider. The WIT contract, the
WASM sandbox, audit logging, and event emission are unchanged.

An earlier draft of this document suggested rewriting `fractalaw-ai` as
a thin HTTP client — that was wrong and would have broken the micro-
apps intent. Superseded by this subsection.

### 0.3 Scope of the fork — training only, not fractalaw

Only one thing actually splits out of fractalaw:

- **Training repo** (Python, GPU, labelled datasets, experiment
  tracking, ONNX/artefact export). Separate repo per §7.
- **Inference deployment** (serverless wrapper around the artefact).
  Can live in the training repo or a small third repo. Not fractalaw.
- **Everything else** — the Gap C micro-app, context retrieval,
  `holder_inferred_from` plumbing, publishing to sertantai, QA
  workflows, taxonomy management — stays in fractalaw.

One fork (training), not two. No split of fractalaw itself.

## 1. What Gap C actually is

Gap C was coined in `02-26-26-taxa-regex-patterns.md` and refined in the OHS
analysis. It is **not one problem** — it is a family of provisions where the
duty-holder is not present as a recognisable token in the provision text:

| Sub-type | Example | Frequency | Regex tractable? |
|----------|---------|-----------|-------------------|
| C1. Thing-subject passive rule | "All systems shall be…", "Equipment must be…" | 42 in 7-law baseline, dominant in product-safety SIs | Partially — GH #16 "Rule" type captures most |
| C2. Sub-clause inherits parent actor | "(2) The employer shall… (a) ensure… (b) record…" where (b) has no explicit actor | Very common in long sections | No — needs structural context |
| C3. Delayed / inverted subject | "It shall be the duty of every employer to…" | Already handled by actors.rs `person_having_control` family | Mostly yes |
| C4. Implicit cross-section actor | Section 5 duty referring to "the duty-holder" defined in s.2 | Common in HSWA, CDM | No — needs whole-act context |
| C5. Generic pronominal subject | "A person with a duty… must cooperate", "Where a person designs…" | Low volume, actionable | Partially — can extend GOVERNED_ACTORS |
| C6. Truly actor-less passive | "Steps must be taken", "Arrangements must be in place" | Moderate | No — semantic role labelling required |

The OHS gap analysis reports **Gap C = 3,275 provisions (78% of false
negatives)**. That figure conflates C1–C6. Any AI approach has to be scoped to
specific sub-types, not "Gap C" as a monolith.

## 2. What the advice got right

Cross-checking both ChatGPT and Gemini against the repo reality:

### Correct and directly useful
- **Structured-output extraction (strict JSON schema)** is the right framing.
  Already in use — `DrrpExtraction { holder, ai_clause, qualifier, clause_ref,
  confidence }` in `crates/fractalaw-ai/src/extractor.rs`.
- **Retrieval for long-distance dependencies.** Gemini's point — fetch
  definitions / neighbouring sections when extracting a duty — maps directly
  to C2 and C4. LanceDB holds every provision with 384-dim MiniLM embeddings,
  so sibling/parent retrieval is cheap and already available.
- **Ontology / canonical holder labels.** ChatGPT's "normalise to an
  ontology" is partially implemented: 27 holder classes in
  `models/deberta-v3-drrp/holder_labels.json` (directory misnamed — the
  artefact inside is DistilBERT, not DeBERTa), partially aligned with
  `actors.rs`. Vocabulary reconciliation is a known gap and addressed
  by review item #3 (Session 1 scope). The ontology concept is right; the
  specific label set needs expansion and cleanup.
- **Don't over-invest in regex for the remainder.** Confirmed by the
  "actionable remainder ~10 provisions" finding at the end of
  `02-26-26-taxa-regex-patterns.md`.

### Correct and still ahead of us
- **Fine-tune a small-to-medium specialist model rather than call a
  general LLM at runtime.** The deterministic-classifier approach is
  right; the specific artefact in `models/deberta-v3-drrp/` attempted
  this but is a deprecated DistilBERT trained on 200 examples with
  `best_clause_acc: 0.0` — not functional. The advice to "pick
  Llama-3.1-8B + Ollama + JSON mode" would still be a regression:
  non-auditable, licence drag, and we already rejected the runtime-LLM
  pattern on determinism grounds. Correct target under §0's quality-first
  lens: **ModernBERT-large** (primary) with **DeBERTa-v3-large** as a
  comparison baseline, full from-scratch training (no heads reused from
  the deprecated artefact).
- **Synthetic labelling via a teacher model.** Correct pattern, not yet
  executed at sufficient scale. Session 2 Phase B.

### Misaligned with this project
- **Runtime Ollama / vLLM.** Still ruled out — determinism, audit,
  licence, cost-at-scale. §0.1 covers the correct deployment pattern
  (remote serverless-GPU serving a fine-tuned classifier).
- **"Use Python + Pydantic to validate."** Validation happens in Rust via
  `serde` + the taxa pipeline's downstream consumers. Not a gap.
- **Model size recommendations (Llama 4-70B, GPT-OSS-120B).** Over-specified
  for a classification + span-extraction task. ModernBERT-large is the
  right size class under the quality-first lens.
- **"Dependency parsing with spaCy".** Valid technique, but adds a Python
  runtime to an all-Rust inference path. If we want structural features,
  extend `clause_structure.rs` rather than bolt on spaCy.

## 3. The actual missing capability

The `DrrpExtractor` in `crates/fractalaw-ai/src/extractor.rs` was designed
to **polish** entries that the regex pipeline has already detected. The
shipped artefact (DistilBERT, 200 training examples, `best_clause_acc:
0.0`) is not functional and is deprecated under §0. Regardless of that
artefact's status, Gap C provisions produce **no DRRP entry at all** from
regex — so even a working polisher would never be invoked on them. The
real gap is therefore:

> **A detection/gating step that can (a) decide whether a provision is a
> genuine duty, and (b) attribute an implicit actor from surrounding context
> when no explicit actor is present in the clause.**

This is a different model shape from a polisher. A polisher takes
`(drrp_type, regex_holder, source_text, article)` — both `drrp_type` and
`regex_holder` are pre-supplied by the regex pipeline. Gap C needs a model
that takes raw provision text (plus optional parent/section context) and
outputs `(is_duty, drrp_type | None, holder | "inherit" | "passive", spans)`.

## 4. Recommended approach

### 4.1 Architecture — one backbone, detector + extraction heads

**Backbone**: **ModernBERT-large** (primary, ~395M params, 8192 native
context), with **DeBERTa-v3-large** (~435M params, 512 context) trained
in parallel as a comparison baseline during Session 2. Full fine-tune,
fp16. No weights are reused from the deprecated DistilBERT artefact at
`models/deberta-v3-drrp/` — it is ignored entirely.

ModernBERT-large is the default for two reasons consistent with the
quality-first lens in §0:
1. Native 8192-token context eliminates the truncation problem in
   §4.2's context retrieval. Parent clause + section definitions +
   act-general-duty can be concatenated without fighting a 128- or
   512-token ceiling.
2. Modern pretraining corpus + flash attention give better downstream
   fine-tune results at lower training cost than older encoders.

DeBERTa-v3-large is kept as a comparison baseline — one extra training
run is cheap, and its disentangled attention may edge ahead on
short-context examples. Session 2 Phase D trains both and picks the
winner on held-out laws.

**Heads** (all trained from scratch — nothing to reuse):

- **Detector** — runs when the regex pipeline produces no DRRP for a
  provision that passed the skip gate.
  - `is_drrp: binary` (duty present at all)
  - `drrp_type: {Duty, Right, Responsibility, Power, None}`
  - `holder_class: one of the concrete user-facing roles (count grows
    per §8 iterative taxonomy; reconciled against `actors.rs` during
    Session 1 per review item #3)`
- **Extraction heads** — `clause_span`, `qualifier_span` for precise
  span extraction. Used both for detector output and for an integrated
  "detection-then-extraction" pipeline. No polisher head exists —
  the deprecated artefact's polisher never worked and is not retained.

**Single-model vs. multi-model — server-side concern only.** Under the
remote-service architecture (§0.1, §0.2), whether the serverless
endpoint runs one multi-head model or dispatches internally to
specialist models is invisible to fractalaw. The HTTP contract is
fixed: `POST /detect { text, context, article } → { holder_class,
drrp_type, confidence, spans, holder_inferred_from_hint }`. Session 2
Phase D chooses the server-side architecture based on training
stability and latency; Session 3 integration is unaffected either way.
This resolves critical review item #7.

**No meta-labels.** The holder must always resolve to a concrete role that a
user could wear — an employer looking up obligations will never select
`Passive` or `Inherited` from a facet. Every DRRP in UK ESH law has a
duty-holder; the task is to *name* it from context, not to admit defeat. If
the model cannot pick a concrete class with sufficient confidence, the
provision is **suppressed** (no DRRP emitted) rather than labelled with an
escape hatch. This keeps precision intact at the cost of leaving some Gap C
provisions uncovered — strictly better than polluting the published taxonomy.

If systematic Gap C labelling surfaces real roles that are missing from the
model vocabulary (e.g. designer, importer, notifier), **extend the
vocabulary with the real role**. Ontology growth is expected; meta-labels
are not.

#### Two label vocabularies — regex detection vs. model output

The project actually maintains **two related but non-identical label
spaces**, and conflating them produced the review's items #2 and #3. They
serve different purposes and should be documented as distinct:

| Vocabulary | Lives in | Purpose | Contents |
|---|---|---|---|
| **Regex actors vocabulary** | `crates/fractalaw-core/src/taxa/actors.rs` | Internal detection mechanism. Includes placeholders for patterns regex can't deterministically resolve. | Full ~40+ concrete roles **plus regex-era placeholders** (e.g. `": He"` for pronoun references to actors defined elsewhere). |
| **Model holder vocabulary** | `holder_labels.json` (pinned per model version) | User-facing taxonomy. What the detector can predict. What sertantai consumes and exposes as facets. | Concrete user-facing roles only. No placeholders, no meta-labels, no trailing-colon artefacts. |

Rules of engagement between the two spaces:

1. **Model vocabulary is a cleaned subset-plus.** It excludes regex-era
   placeholders (`": He"`) and fixes artefacts (trailing colon on
   `"Gvt: Ministry:"`). It adds concrete roles that actors.rs recognises
   but that the current `holder_labels.json` omits — the ~30+ missing
   labels catalogued in the critical review item #3 (Contractor,
   Principal Contractor, Designer, Principal Designer, Manufacturer,
   Importer, Installer, Worker, User, Competent Person, Company,
   Landlord, Licensee, agency specifics, ministry specifics, etc.).
2. **Regex placeholders route through AI for resolution.** When regex
   emits `": He"` (or any other placeholder that indicates "actor
   defined elsewhere"), the pipeline routes the provision through the
   Gap C detector to resolve the pronoun to a concrete role from the
   model vocabulary. This is the mechanism by which the regex path
   phases out its own placeholders: AI resolution replaces them.
3. **Publish is gated on model vocabulary.** Only entries whose holder
   is in the model vocabulary get published to sertantai. Regex-emitted
   placeholders that fail AI resolution above the confidence threshold
   are suppressed from publish rather than pushed out as `": He"`.
4. **Sertantai backfill.** Any `": He"` values already in sertantai
   (carried over from the original Airtable → Postgres history) get
   replaced as laws are re-enriched with AI enabled and re-published.
   No separate migration script needed — re-publish is the fix.
5. **End-state (post-S9).** Once the AI detector is proven, regex-era
   placeholders can be retired from `actors.rs` entirely. The two
   vocabularies converge: regex detects only concrete roles, AI
   handles everything else.

This preserves the published ontology's user-facing meaning (sertantai
consumes it and end-users search by role), avoids a second model artifact,
and keeps inference cost flat.

### 4.1a Schema extension — `holder_inferred_from` (inference provenance)

When the actor isn't in direct line of sight of the duty, the pipeline must
record **where** the inference came from. This is both an audit trail and a
user-facing explainability feature ("this duty applies to you as a
Principal Contractor because reg.4(1) defines the duty-holder").

**Field**: `holder_inferred_from: Option<String>` added to `DrrpExtraction`
in `crates/fractalaw-ai/src/extractor.rs`.

- Null / absent when the holder is explicit in the clause text (common
  case — regex detection or AI detection with in-clause evidence).
- Citation string when the holder was inferred from elsewhere in the law.

**Format**: same citation style as existing `clause_ref` (`s.2`, `reg.4`,
`r.3`), extended for sub-paragraphs and cross-act references:

| Case | Example | Meaning |
|------|---------|---------|
| Explicit holder in clause text | `null` | Regex or AI found actor inline |
| Parent clause (C2) | `"reg.4(1)"` | Same law, parent paragraph |
| Section definition (C4) | `"s.2(1)"` | Same law, interpretation/definitions |
| Act-level general duty (C6) | `"s.2"` | Act's headline duty establishes who holds passive-construction duties in its scope |
| Cross-act reference (rare) | `"UK_ukpga_1974_37:s.2"` | `{law_id}:{clause_ref}` when holder is defined in another Act (e.g. an SI leaning on HSWA s.2) |

**Schema change impact**:

1. **Rust** — additive field on `DrrpExtraction` with
   `#[serde(skip_serializing_if = "Option::is_none")]`. Backwards-compatible
   JSON — existing consumers ignoring unknown fields see no change, and the
   null case omits the key entirely.
2. **ONNX model** — no new head needed. Provenance is determined by the
   Rust integration layer based on which context segment was fed to the
   model (clause-only vs. clause+parent vs. clause+section vs.
   clause+act-general-duty). Keeping this out of the model keeps the ONNX
   artifact stable across retrains.
3. **DuckDB LRT** — new nullable column parallel to holder in the
   per-provision DRRP storage. Additive migration; existing rows null.
4. **Sertantai publish** — additive, nullable field in the Zenoh payload.
   Requires a contract note; shouldn't break consumers that tolerate
   unknown/null fields.

**Downstream value**:
- **Explainability**: user sees not just the duty but the basis for it.
- **Verification**: reviewers can jump to the referenced clause to confirm
  a Gap C inference. Makes spot-checks tractable.
- **Regression detection**: if a rebuild changes `holder_inferred_from`
  for a stable provision, flag it — the inference basis moved.
- **Diagnostics**: frequency-distribution of inference sources across a
  law tells us which sections are doing the heavy lifting for Gap C
  coverage, and whether the model is over-relying on e.g. HSWA s.2.

### 4.2 Context retrieval — the actual win from LanceDB

For sub-types C2 and C4, the provision text alone is insufficient. Before
invoking the detector, build a small context window from LanceDB:

1. Parent provision (same law, parent `article` in the clause hierarchy).
2. Section heading / preceding definition block.
3. Act-level "interpretation" section if the provision contains
   referential terms ("the duty-holder", "the person", "he").

Retrieval is metadata-first (by `law_id` + `article` prefix), not vector
search. Embedding retrieval is only needed for cross-act references which
are rare in UK ESH law. This is a LanceDB scan + filter, not a RAG call.

Pass the context as a second text segment to the detector, using a
`[CLS] query [SEP] source [SEP] context [SEP]` tokenization pattern.
With ModernBERT-large's 8192 context window the three context segments
can be concatenated without aggressive truncation. Session 1 S3's
`docs/gap-c-context-format.md` pins the exact format as the contract
between the main repo and the training repo.

### 4.3 Training data strategy

- **Labelled positives** are easy: every regex-detected DRRP is a training
  example. Already ~486 across 7 laws; more once enrichment re-runs.
- **Labelled negatives** need care: use the skip-gate population
  (headings, amendments, interpretation) — already deterministic.
- **Gap C golds** are the expensive part. Two routes:
  1. Bootstrap with a teacher model (Claude / GPT-4o via API) over the
     ~3,275 Gap C candidates from the OHS run. One-off cost, cached to
     parquet next to the LanceDB backup (see MEMORY.md backup strategy).
  2. Human spot-check a stratified sample per sub-type — especially C2/C4
     where teacher models still hallucinate actors. This is where the
     user's ESH domain expertise is the differentiator, as ChatGPT noted.
- Store labels in a new parquet sidecar (not in LanceDB directly) so the
  `legislation_text` table remains publish-safe and re-buildable.

### 4.4 What not to build

- **No runtime LLM (Ollama/vLLM/llama.cpp).** Determinism, audit,
  licence, cost-at-scale. Fine-tuned classifier served via remote
  serverless-GPU endpoint (§0.1) is the pattern. `fractal:ai/inference`
  WIT boundary abstracts the specific provider.
- **No new Python runtime in production.** Training scripts in Python
  live in the separate training repo (§7). The fractalaw client calls
  the remote endpoint via HTTPS.
- **No "polisher for polished output" chains.** Keep the pipeline
  one-pass: regex detect → (on miss) detector → extraction heads in
  the same forward pass.
- **No carry-over from the deprecated DistilBERT artefact.** No shared
  weights, no shared tokenizer, no shared head dimensions. New model
  from scratch.

## 5. Staged plan

The plan has been revised after the Q1–Q3 resolutions and the conclusion of
the OHS gap analysis (`taxa-gap-analysis/04-14-26-ohs-occupational-safety.md`),
which is the report that produced Gap C as the named remainder after regex
iteration. Earlier drafts of this plan included a regex-cleanup stage — now
dropped because that work *is* the OHS gap-analysis session, and its output
is the Gap C definition we're now solving.

Stages split across the two repos (main `fractalaw` vs. separate training
repo per §7):

| Stage | Repo | Deliverable | Risk |
|-------|------|-------------|------|
| S1a | main | Freeze Gap C sub-type taxonomy (C1–C6) in the `taxa-gap-analysis` skill. Add a script that classifies each Gap C provision into a sub-type (reads LanceDB, writes parquet sidecar). | Low — pure regex/structure |
| S1b | main | **Model-vocabulary reconciliation**: audit `actors.rs` vs. current `holder_labels.json`; produce cleaned, reconciled `holder_labels.json` pinned for Session 2 training. Fix `"Gvt: Ministry:"` trailing-colon bug. Exclude `": He"` (regex-era pronoun placeholder) from the model vocabulary. Add ~30+ concrete roles currently in `actors.rs` but missing from `holder_labels.json`. See §4.1 two-vocabulary section. | Low — mechanical but must be thorough; this is an exit-gate item for Session 2 to start |
| S3 | main | Build context-retrieval helper in `fractalaw-store` (parent/section/act-general-duty lookup in LanceDB by `article` hierarchy). Drives the detector input and downstream `holder_inferred_from` value. Token budget pinned at 2048 in the format spec. | Low |
| S4 | training | Teacher-model labelling pass over the ~3,275 Gap C candidates. **Phase 1 pilot: C2 sub-type only** (sub-clause inheritance — highest frequency, simplest context: just the parent paragraph). Phase 2 expands to C4 and C6 after the pilot integration proves the pipeline. Teacher emits one concrete role or `unresolved`. No meta-labels. Cache to parquet. | Medium — label quality |
| S5 | training | Stratified human review. C2 in Phase 1; C4 added in Phase 2. Domain-expert pass. | Medium — reviewer time |
| S6 | training | Fine-tune **ModernBERT-large** (primary) and **DeBERTa-v3-large** (comparison baseline) from scratch with detector + extraction heads. **Phase 1 model is C2-only.** fp16 full fine-tune. Pick the winner on held-out-law eval. Publish to HF Hub private repo with `model_version` tagged. Deploy behind a RunPod/Modal serverless-GPU endpoint per §0.1. Phase 2 adds C4/C6 via head-swap retrain per §8. | Medium — GPU infra on rented hardware; dual run is cheap; phasing de-risks training |
| S7 | main | Extend `fractalaw-ai` with a **remote-inference provider** alongside the existing local-ONNX provider (per §0.2, not a rewrite). Integrate detector into `taxa::parse_v2` as a fallback after the skip-gate pass. **S7 also lands the `holder_inferred_from` schema work** (`DrrpExtraction` field, DuckDB LRT column, Zenoh publish payload) — deferred from Session 1 so it ships alongside the provenance producer that populates it. Feature flag + confidence threshold; below-threshold = suppress (no emission). `": He"` routing rule (§8) ships here too. | Medium — must not regress precision (96.4%); larger-than-original S7 scope |
| S8 | main | Re-run OHS gap analysis and the cross-family gap analyses against the Phase 1 (C2-only) model. Confirm recall improvement from 48.6%, verify precision ≥96%. **Account for the 50 Fix-2 provisions** identified in the OHS analysis (subordinate clause + pronoun reference, epistemic "may" rejection) as expected pickups — they are regex matcher bugs the detector is designed to cover. Log inference-source distribution. | — |
| S8b | training + main | **Phase 2 expansion**. After Phase 1 ships and stabilises, head-swap retrain to add C4 and C6 coverage. Re-run gap analyses with the expanded model. Sub-type coverage report. | Medium — incremental, low risk |
| S9 | main | **Final cleanup** (deferred until S8 passes). Rename or remove `models/deberta-v3-drrp/` (misnamed DistilBERT artefact; `best_clause_acc: 0.0`; replaced by the remote endpoint). Remove or deprecate `DrrpExtractor::extract` (polisher path) in `crates/fractalaw-ai` if no edge micro-app depends on it. Update any stale references in code, docs, and CLAUDE.md. Do **not** do this before S8 — the old artefact serves as ignored-but-present reference while the new pipeline is being validated. | Low — mechanical cleanup |

Precision is the constraint. Gap C recall will never hit 100% from regex
alone, but the AI layer must not drag precision below the current 96.4%.
Suppression-on-low-confidence (per Q1 resolution) is the primary mechanism
for protecting precision.

**What's deliberately *not* a stage**: extending `holder_labels.json` with
meta-labels (Q1 forbids it) or with speculative new roles. Real new roles
get added *reactively*, only when labelling surfaces a Gap C pattern that
cannot resolve to any existing class — and each addition is its own small
change with a sertantai contract note, not batched into this plan.

## 6. Open questions

1. ~~Does the sertantai contract allow new holder labels (`Inherited`,
   `Passive`)?~~ **Resolved 2026-04-15**: no meta-labels. Holder must be a
   concrete user-facing role. Below-threshold detections are suppressed, not
   escape-hatched. Ontology may grow with real new roles (designer, importer,
   etc.) as Gap C labelling surfaces them.
2. ~~Should the detector emit a `holder_hint_article`?~~ **Resolved
   2026-04-15**: yes, as `holder_inferred_from: Option<String>` on
   `DrrpExtraction`. Null when the holder is explicit in the clause text;
   otherwise a citation of the clause the inference came from (parent
   paragraph, interpretation section, act-level general duty, or
   `{law_id}:{clause_ref}` for rare cross-act references). Rationale:
   auditability, user-facing explainability ("this applies to you because
   reg.4(1) defines the duty-holder"), and regression detection across
   rebuilds. Additive schema change — backwards-compatible JSON via
   `skip_serializing_if = Option::is_none`. Requires parallel column in
   DuckDB LRT and a contract note to sertantai.

   Provenance is tracked in the Rust integration layer (which context
   segment was fed to the model), not in a new model head — keeps the ONNX
   artifact stable.
3. ~~Training infra — Python subfolder in the workspace or ad-hoc
   external?~~ **Resolved 2026-04-15**: fully external, separate repo.
   See §7.

## 7. Training infrastructure — separation of concerns

Training is a separate project from `fractalaw`. The main repo is a
consumer of a pre-built ONNX artefact; training pipeline, datasets, and
fine-tuning scripts live elsewhere.

### Why separate

- **Hardware**: ModernBERT-large / DeBERTa-v3-large fine-tuning needs a
  GPU. This machine is CPU-only and Bluefin's atomic base makes CUDA
  tooling painful. Training runs on rented GPU (RunPod / Lambda / HF
  Spaces). Under the quality-first lens (§0) full fine-tune is the
  default; LoRA/QLoRA is a fallback only if full fine-tune's cost grows
  unexpectedly.
- **Stack**: training is Python (`transformers`, `datasets`, `accelerate`,
  `unsloth`/`axolotl`). `fractalaw` is Rust-first per CLAUDE.md. Adding
  Python to the workspace would violate that and inflate repo size with
  checkpoints and dataset snapshots.
- **Lifecycle**: models retrain on their own cadence (new laws, relabelling
  passes, teacher-model refreshes); the project ships on its own cadence.
- **Licence and secrets**: teacher-model outputs, raw law texts used for
  labelling, and API keys don't belong in an AGPL-3.0 repo.
- **Reproducibility**: training history (which data → which model) deserves
  its own commit history, not muddied into `fractalaw` commits.

### Ownership split

**Training repo (separate, private):**
- C1–C6 Gap C sub-type categorisation scripts
- Teacher-model labelling pipeline
- Gold-label parquet datasets (derived from LanceDB exports + human review)
- Fine-tuning scripts (**ModernBERT-large** primary backbone,
  **DeBERTa-v3-large** comparison baseline; detector + extraction heads
  trained from scratch — no weights reused from deprecated artefact)
- Evaluation harness: confusion matrix, precision/recall on held-out laws,
  regression against previous model versions
- ONNX export + int8 quantisation scripts
- Model card and training provenance notes

**`fractalaw` repo (this one):**
- Calls the remote serverless-GPU endpoint (per §0.1 / §0.2) — no local
  model loading for the Gap C detector.
- `holder_labels.json` is pinned per model version and delivered with
  the endpoint response or fetched alongside it. Main repo applies
  label-vocab deltas during Session 3 integration.
- `fractalaw-ai` is extended with a remote-inference provider (per
  §0.2). The existing local-ONNX path is retained for edge micro-apps
  (MiniLM embeddings, lightweight classifiers).
- No datasets, no Python, no training scripts.

### Artefact distribution

Two artefacts to distribute per model version:

1. **Inference endpoint** — the fine-tuned model served via RunPod
   Serverless / Modal / HF Inference Endpoints. This is what
   `fractalaw` calls. Updates are server-side and transparent to the
   client.
2. **Artefact archive** (for reproducibility and audit) — **HF Hub
   private repo**. Stores the trained weights, tokenizer, metadata,
   and `holder_labels.json` per model version. The serverless endpoint
   pulls from here on deploy; the fractalaw repo references the
   revision pin in its version-compat check.

HF Hub advantages:
- Built-in versioning (model revisions, tags)
- CDN-backed downloads, free at this size
- `huggingface_hub` CLI is standard in the Python training world
- Model card UX for per-version change notes

Alternatives considered:
- **Git-LFS in the training repo**: workable but less ergonomic for
  downstream fetch and costs LFS bandwidth.
- **S3/R2 bucket + pinned URL + SHA256**: simplest, but loses the
  model-card UX HF Hub gives.

### Model version contract

Bake a `model_version` string into `metadata.json` (already has
`base_model`, `num_holder_classes`, `max_length` — add `model_version` and
`min_fractalaw_ai_version`).

Extend `DrrpExtractor::load` to refuse artefacts that don't match a
compatible range declared in the `fractalaw-ai` crate. Prevents a
newly-published model from silently producing incompatible outputs against
an older `fractalaw` binary — and vice versa.

### Labelling-data provenance

Training repo snapshots (or pins an LRT-hash of) the law set each model
version was trained on. Continuous eval uses the existing OHS-style gap
analysis: re-run the report against each new model version, compare
precision/recall and Gap C sub-type coverage. The current gap-analysis
workflow effectively *is* the eval harness — it doesn't need rebuilding,
just formalising.

### Teacher model for bootstrap labelling

For the S5 Gap C labelling pass (~2–3k provisions, one-off cost):
- Claude via this CLI, or Claude/GPT-4o via API are both suitable.
- Prompt constraint is non-negotiable: the teacher must resolve to one of
  the concrete user-facing roles or return `unresolved` (dropped from
  training set). No meta-labels in the training data — same rule as for
  model output.
- Human review is mandatory for C2 (sub-clause inheritance) and C4
  (cross-section inference) — teacher models hallucinate actors most often
  in those sub-types. Stratified sample, domain-expert review.

## 8. Iterative taxonomy & retrain cadence

The holder ontology is not frozen. New actor roles surface during QA —
that is an explicit purpose of the QA step. Every new role is an event
that can affect the Gap C detector's training. Training must therefore
be planned as an **ongoing, versioned process**, not a one-off bootstrap.

### Feedback loop — QA produces a taxonomy-change log

QA sessions should produce a structured output, not just session notes:

- Parquet/CSV with rows: `new_role, example_law_id, example_article,
  example_text, suggested_definition, reviewer, status`.
- Committed to the training repo (or a shared artefact location).
- Each new role must be accompanied by ≥N labelled examples from the
  evidence that surfaced it, so retraining has supervised data for
  the new class from day one.

This converts an informal "I noticed X" into a machine-consumable
signal.

### Retrain pattern — head-swap, not full retrain

When `holder_labels.json` gains a new class:

1. Keep the trained backbone weights from the previous model.
2. Swap the classifier head for a wider one (N → N+1 classes).
3. Initialise the new class's weights as random + optional transfer
   from the closest existing class (useful if the new role is a
   specialisation, e.g. adding `SC: C: Principal Contractor` near
   existing `SC: C: Contractor`).
4. Fine-tune on the existing training set plus the new-role examples.
5. Evaluate: precision/recall on all classes, especially the new one
   and its neighbours (check for cannibalisation).
6. Publish as a new `model_version`. The RunPod/Modal endpoint hot-
   swaps to the new artefact.

Head-swap fine-tuning is roughly an order of magnitude cheaper than
full retraining. It is the standard pattern for vocabulary evolution
in classification models.

### Full retrain triggers

Do a full retrain (not head-swap) only when:

- The training data has substantially grown (e.g. a new family of
  laws with different drafting style lands).
- Evaluation shows cannibalisation or drift on existing classes after
  several head-swap cycles.
- The base model (ModernBERT-large or whatever we land on) is
  replaced with a newer backbone.

Otherwise, head-swap is the default.

### Regex-to-AI transition — routing regex-era placeholders

`": He"` (and any similar pronoun/placeholder labels in `actors.rs`)
existed in the pre-AI Regex pipeline as a record of "actor is defined
elsewhere; pronoun is the surface form." These predate the Gap C
solution and are not user-facing.

Transition rule, active during Session 3 onwards:

1. When the regex path emits a DRRP with holder in the placeholder set
   (currently: `": He"`, extensible), `taxa::parse_v2` routes the
   provision through the Gap C detector with context retrieved per §4.2,
   as if it were a Gap C candidate.
2. If the detector returns a concrete role above threshold, the
   placeholder is replaced with the concrete role and `holder_inferred_from`
   is populated with the source clause citation.
3. If the detector is below threshold, the entry is **suppressed from
   publish** — `": He"` no longer reaches sertantai.
4. Sertantai backfill is automatic: as laws are re-enriched with AI
   enabled and re-published, any legacy `": He"` values in the
   sertantai data (carried over from the original Airtable → Postgres
   import) are replaced by the new published values. No separate
   migration script is required.
5. Once the detector is proven (post-S8), the placeholders can be
   removed from `actors.rs` entirely — S9 cleanup. The regex path
   then detects only concrete roles and the two vocabularies (§4.1)
   converge.

### Cost envelope (2026 RunPod/Lambda rates, order of magnitude)

| Activity | Frequency | Cost per occurrence | Annual |
|---|---|---|---|
| Initial catchup inference over ~3k Gap C provisions | Once | ~$1–10 | — |
| Ongoing inference (20–100 Gap C provisions/month) | Monthly | <$1 | ~$10 |
| Head-swap retrain (new role added to taxonomy) | 4–12×/year | ~$5–20 | ~$50–250 |
| Full retrain (new family, new backbone, etc.) | ~1×/year | ~$50–200 | ~$100 |

Total annual steady-state: well under $500 for the AI side. Cheap
relative to the quality impact.

### Client-side implication

Because inference is remote and models are versioned server-side, the
local `fractalaw` CLI does **not** need to know which model version is
current. It calls the endpoint; the endpoint serves `latest`. The only
client concern is whether `holder_labels.json` in main is up to date so
that returned holder strings are recognised — that's Session 3's
"apply label-vocabulary delta" step, now a recurring task rather than
a one-off.

### Implication for Session 2 / training repo scope

Session 2 is not a "train once and hand off" project. The training repo
is a long-lived asset with:

- A retrain recipe (scripted head-swap).
- A model-version series, with published change notes per version.
- A QA → taxonomy-change-log → retrain workflow documented.
- An endpoint-update script that pushes a new artefact to the serverless
  provider.

Update Session 2's scope to reflect this. One-off bootstrap is Phase A–D;
ongoing retraining is a Phase F that persists.

## 9. Summary

The external advice is broadly sound but under-informed about this repo.
The real Gap C problem is **detection of implicit-actor duties**, not
polishing of regex-detected ones. The right move is:

1. A **new fine-tuned classifier** — ModernBERT-large primary,
   DeBERTa-v3-large baseline, trained from scratch (the existing
   DistilBERT artefact at `models/deberta-v3-drrp/` is deprecated and
   not reused).
2. **Context retrieval** from LanceDB for C2/C4 sub-types, feeding the
   model parent clause + section definitions + act-general-duty in its
   8192-token window.
3. **Remote serverless-GPU inference** (RunPod / Modal / HF Endpoints)
   called from fractalaw via the existing `fractal:ai/inference` WIT
   boundary. No local-first constraints on the hub-side parsing
   service (per §0) and no fork of fractalaw (per §0.2) — only the
   training pipeline forks out to a separate repo.
4. **Iterative taxonomy management**: QA surfaces new roles → head-swap
   fine-tune → new model version → endpoint hot-swap. Steady-state cost
   well under $500/year.

The expensive part remains labelled Gap C training data, not the model.
