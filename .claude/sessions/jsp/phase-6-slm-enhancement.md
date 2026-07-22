---
session: JSP Phase 6 — SLM Enhancement
status: closed
opened: 2026-07-20
closed: 2026-07-21
outcome: success

summary: >
  Built obligation strength classifier (97.7%), fine-tuned 3 SLMs on RunPod
  (RACI, artefact properties, control titles), batch-classified 8,500+
  provisions, expanded actor dictionary from 25 to 60 roles, implemented
  role cleanup (blacklist + fuzzy map), consolidated artefacts from 922 to 406.

decisions:
  - what: Obligation strength classifier, not DRRP classifier, for JSPs
    why: DRRP is a legal taxonomy (Hohfeldian). JSPs assign organisational responsibilities. Different vocabulary avoids confusion with the legislation pipeline.
    result: strength_classifier_v1.json — 97.7% accuracy, 4 classes (Mandatory/Recommended/Permissive/None)
  - what: All inference signals in PG, only reconciled results in DuckDB
    why: PG is the inference store (embeddings, classifier, SLM). DuckDB is the publish store (reconciled final signal). Classifier results initially went to DuckDB — backed out.
    result: Clean separation. PG holds all tiers. DuckDB holds COALESCE(cls, regex). Reconcile script bridges.
  - what: SLM batch scripts run on RunPod pod, write to PG via reverse tunnel
    why: Hot path is Ollama inference (localhost on pod). DB writes are the cold path. Running scripts locally with Ollama tunnelled adds latency to every inference call.
    result: 12.7/s RACI throughput vs estimated 2-3/s if tunnelled
  - what: Expand actor dictionary with SLM-surfaced roles rather than filtering to original 25
    why: SLM found genuine MoD roles not in the hand-built dictionary (radiation, range, safety specialists). Filtering would lose real data.
    result: 25 → 60 canonical roles. 35 new roles across 6 categories (radiation, range, safety, technical, supply, operational)
  - what: Artefact consolidation key is (source_id, artefact_type, primary_raci_role)
    why: Chapter+type alone (239) merges genuinely distinct artefacts. Adding RACI role (487) distinguishes "CO's risk assessment" from "user's risk assessment". Merging unassigned into assigned siblings (406) removes noise.
    result: 922 raw → 406 consolidated. Honest number for exec reporting.
  - what: SLM preference over LLM — use LLM only for residual low-confidence
    why: SLM is cheaper (RunPod GPU rental vs Gemini API per-token), faster (12/s vs rate-limited), no data leaves the infrastructure. LLM for the cases SLM can't handle.
    result: Three SLMs trained in one pod session (~$3 total). Gemini not needed for bulk classification.

metrics:
  strength_classifier: { accuracy: 0.977, classes: 4, features: 399, training_examples: 4816, test: 1205 }
  slm_raci: { provisions: 6021, assigned: 3106, empty: 2915, errors: 0, speed: 12.7, time_min: 8 }
  slm_artefact_props: { provisions: 1065, extracted: 1059, errors: 6, speed: 5.9, time_min: 3 }
  slm_control_titles: { provisions: 1459, generated: 1447, errors: 12, speed: 3.1, time_min: 8 }
  actor_dictionary: { original: 25, expanded: 60, new_roles: 35 }
  role_cleanup: { blacklisted: 14, fuzzy_mapped: 81, slm_roles_before: 164, after: 56 }
  artefact_consolidation: { raw: 922, first_pass: 483, after_merge: 406 }
  runpod: { gpu: "RTX 5090 32GB", training_time_min: 30, inference_time_min: 19, total_cost_est: 3.00 }
  tests_passing: 83

lessons:
  - title: SLM hallucinated 164 role labels from 25-role dictionary — but 35 were genuine new roles
    detail: "The SLM generalised beyond training data and invented role labels from text context. 93% were canonical, 7% were novel. Of the novel ones, 35 were genuine MoD roles not in the dictionary (Radiation Protection Supervisor, Range Conducting Officer, Diving Supervisor). The lesson: SLM output is a discovery signal, not just a classification signal. Review hallucinations before filtering — some are gold."
    tag: models
  - title: Batch scripts must run on the pod (near Ollama), not locally
    detail: "Initially planned to run scripts locally with Ollama tunnelled from pod. Each inference call through SSH tunnel adds 50-100ms latency. With 6,000 provisions × 4 workers, that's hours of overhead. Scripts on pod + PG reverse tunnel = fast inference, slow writes (acceptable)."
    tag: architecture
  - title: Classifier results belong in PG not DuckDB
    detail: "Initially wrote cls_strength to DuckDB alongside regex. Wrong — DuckDB is the publish store (reconciled signal only). PG is the inference store (all tier signals). Had to migrate, remove DuckDB columns, add reconcile script. Get the store boundary right from the start."
    tag: architecture
  - title: Artefact count inflates without consolidation — 922 vs 406
    detail: "922 raw artefact mentions sounded impressive but overcounted. 69 provisions in CH08 all reference the same risk assessment. Consolidation key (chapter + type + RACI role) + merging unassigned into assigned siblings produced the honest number: 406. Always consolidate before reporting."
    tag: methodology
  - title: Artefact property fill rates reflect source data, not model failure
    detail: "SLM extracted owner for only 8% of artefacts. Investigation showed the text genuinely doesn't name the owner — most provisions say 'a risk assessment must be conducted' without naming who. The owner comes from RACI or organisational context, not the provision. Low fill rate = honest extraction, not bad model."
    tag: data
  - title: Term conflicts (1,670) are mostly noise — same acronym different expansion is not a conflict
    detail: "AP = Authorised Person vs Accountable Person isn't a conflict — they're homonyms. The real conflict is when 'Accountable Person' is DEFINED differently across JSPs. That requires glossary parsing (sertantai-legal#127) which is currently broken."
    tag: methodology
  - title: One RunPod session for three fine-tunes saves setup overhead
    detail: "Each pod start incurs Ollama install, model upload, SSH setup. Batching three fine-tunes + three batch inferences in one session (RACI → artefact-props → control-titles) used ~50 min GPU total. Disk quota hit on third GGUF export — cleaned merged_model dirs between exports."
    tag: infrastructure

artifacts:
  - crates/fractalaw-cli/config/strength_classifier_v1.json
  - crates/fractalaw-core/data/jsp-actor-dictionary.yaml (expanded with 35 new roles)
  - crates/fractalaw-core/data/jsp-role-blacklist.yaml
  - crates/fractalaw-core/data/jsp-role-fuzzy-map.yaml
  - crates/fractalaw-core/src/jsp/role_cleanup.rs
  - scripts/ml/train_strength_classifier.py
  - scripts/ml/classify_jsp_strength.py
  - scripts/ml/reconcile_jsp_strength.py
  - scripts/ml/prepare_raci_training.py
  - scripts/ml/prepare_artefact_training.py
  - scripts/ml/prepare_control_title_training.py
  - scripts/ml/finetune_raci.py
  - scripts/ml/finetune_artefact_props.py
  - scripts/ml/finetune_control_titles.py
  - scripts/ml/runpod_raci_batch.py
  - scripts/ml/runpod_artefact_props_batch.py
  - scripts/ml/runpod_control_titles_batch.py
  - scripts/ml/Modelfile.raci
  - scripts/ml/Modelfile.artefact-props
  - scripts/ml/Modelfile.control-titles
  - models/gemma3-raci-q4.gguf
  - models/gemma3-artefact-props-q4.gguf
  - models/gemma3-control-titles-q4.gguf
  - docs/manual/JSP-SUMMARY.md (updated with consolidated numbers + role annex)

depends_on:
  - phase-7-corpus-enrichment.md

enables:
  - Competence SLM extraction (fractalatai#50)
  - Remaining graph dimensions (fractalatai#51)
  - Glossary-based term conflict detection (sertantai-legal#127)
  - Sertantai artefact consolidation (sertantai-legal#128)
---

# Session: JSP Phase 6 — SLM Enhancement (CLOSED)

## Problem

Phases 1-5 built the JSP pipeline using regex extraction. The existing legislation pipeline uses a 4-tier classification stack (regex → logistic regression classifier → SLM/Ollama → Gemini LLM) with confidence-gated escalation. JSP enrichment currently uses only Tier 1 (regex). Applying the higher tiers would improve quality across all extraction phases — especially RACI disambiguation, artefact property extraction, and control title generation.

## Proposals

### 1. Obligation Strength Classifier (Tier 2: Classifier)

**Current state:** JSP obligation strength is classified by modal verb keyword only ("must" → Mandatory, "should" → Recommended). This misses context — "must not" is Mandatory but prohibitive, "will ensure" in a preamble is descriptive not mandatory.

**Proposal:** Train a strength classifier (logistic regression on 384-dim embedding + modal indicators) for JSP text. The existing DRRP classifier (v8) is trained on legislative text and its classes (Obligation/Liberty/none) don't map cleanly to JSP strength (Mandatory/Recommended/Permissive/None). A JSP-specific classifier trained on the 6,021 regex-classified provisions would be more accurate.

**Approach:** Embed JSP provisions → train strength classifier on regex labels → evaluate against held-out set → deploy as `strength_classifier_v1.json`. Add JSP-specific modal indicators ("will", "is to") as features.

**Expected gain:** Better precision on ambiguous provisions (preambles, descriptive "will", conditional "may need to").

### 2. RACI Position (Tier 3: SLM/Ollama)

**Current state:** RACI is inferred from narrative patterns (actor + modal → R; "accountable for" → A; "shall be informed" → I). Passive voice ("shall be conducted") produces no RACI — no actor identified.

**Proposal:** Fine-tune a Gemma-3-4B RACI classifier on JSP text. Input: obligation text. Output: JSON array of `{role, assignment_type}`. Training data: 1,719 RACI assignments from corpus-level regex extraction plus 5,028 obligations (3,309 without RACI as negative/ambiguous examples).

**Approach:** Same RunPod fine-tuning pipeline as gemma3-position. LoRA R=16. Export GGUF → Ollama. Batch inference on provisions where regex RACI is empty but obligation exists.

**Expected gain:** RACI coverage on passive-voice obligations. Currently 34% of obligations have RACI (1,719/5,028). SLM could increase to 60-80%.

### 3. Artefact Property Extraction (Tier 6: Gemini Batch)

**Current state:** Artefacts are detected by type only (regex: "risk assessment", "safety case"). No structured properties — owner, approver, reviewer, frequency, required content, acceptance criterion are all NULL.

**Proposal:** Gemini 2.5 Flash batch extraction. For each artefact, send the parent obligation text + surrounding context. Prompt for structured JSON: `{owner_role, approver_role, reviewer_role, review_frequency, required_content, acceptance_criterion, scope}`. Same batch pattern as `gemini_llm_batch.py`.

**Approach:** Batch of 922 artefacts. Can chunk by source (157 Gemini calls, one per chapter). ~$0.50 total. Results write to `jsp_mandated_artefacts` DuckDB table (add columns for extracted properties).

**Expected gain:** The mandated artefact abstraction from the plan (JSP-SERVICE.md) becomes fully populated. Each artefact carries its operational properties → direct mapping to L3 Controls and L4 Evidence.

### 4. Control Title Generation (Tier 6: Gemini Batch)

**Current state:** Control titles are template-generated: "A {artefact_type} is maintained by the {role}". Grammar issues ("A Inspection Report"), no domain-specific language, no indicative-mood refinement.

**Proposal:** Gemini 2.5 Flash generation with the same design constraints as the legislation controls pipeline (COMPLIANCE-CONTROLS.md): indicative mood, referent not paperwork, discriminating test, honest limits.

**Approach:** For each of the 922 JSP controls, send: artefact type + obligation text + RACI assignments + competence requirements. Prompt for: title (indicative), description (what reality), what_it_checks (discriminating test). Same few-shot pattern as `generate_controls.py`. Chunk by source.

**Expected gain:** JSP controls reach the same quality standard as Gemini-generated legislation controls. The template titles are replaced with domain-specific, checkable statements.

### 5. Source Traceability (Tier 6: Gemini Batch)

**Current state:** JSP provisions reference legislation at document level ("in accordance with the Electricity at Work Regulations 1989"). Phase 2 resolves these to `law_name`. But which *specific* legislative provision does each JSP obligation implement? This is unresolved.

**Proposal:** Gemini batch matching. For each JSP obligation, send: obligation text + candidate legislative provisions (from the referenced law, filtered to HIGH/MEDIUM significance obligations). Prompt: which provision(s) does this JSP obligation implement?

**Approach:** Per referenced law, pull obligation-type provisions from Postgres. Send JSP obligation + N candidates to Gemini. Returns: `[{section_id, confidence}]`. Populates `related_control_ids` on JSP controls and enriches `source_links`.

**Expected gain:** Provision-level traceability. "JSP 375 para 30 implements Electricity at Work Regulations reg.4(2)" — the gap analysis becomes specific.

### 6. Unresolved Reference Resolution (Tier 6: Gemini Batch)

**Current state:** 227/1,969 references (12%) unresolved across the corpus — mostly HSE guidance and standards which aren't in the fractalaw corpus.

**Proposal:** For genuinely unresolvable references (external standards/guidance), flag them as external and store the citation. For dense/implicit references ("in accordance with the relevant health and safety legislation"), use Gemini to identify which specific law(s) are meant.

**Expected gain:** Resolution rate from 82% to 90%+. External references flagged rather than silently dropped.

## Prioritisation

| Proposal | Effort | Value | Priority |
|----------|--------|-------|----------|
| 1. Strength classifier | Medium (train on JSP labels) | Medium | Do first |
| 2. RACI SLM | Medium (fine-tune + batch) | High | Do second |
| 3. Artefact properties | Low (32 Gemini calls) | High | Do third |
| 4. Control titles | Low (32 Gemini calls) | Medium | Do fourth |
| 5. Source traceability | Medium (candidate set prep) | High | Do fifth |
| 6. Unresolved refs | Low (11 Gemini calls) | Low | Do last |

## Todo

- ✅ Create `jsp_provisions` table in fractalaw PG (port 5433) — separate from legislation
- ✅ Populate from DuckDB — 6,021 provisions with text
- ✅ Embed all 6,021 provisions (all-MiniLM-L6-v2, 384-dim, stored in PG pgvector)
- ✅ Train strength classifier — 97.7% accuracy (Mandatory 98 F1, None 98 F1, Recommended 97 F1, Permissive 95 F1)
- ✅ Deploy `strength_classifier_v1.json` (399-dim: 384 embedding + 15 JSP modal indicators)
- ✅ Add `cls_strength` + `cls_confidence` columns to `jsp_enrichment` DuckDB table
- ✅ Batch classify 6,021 provisions — 97.1% agreement with regex (128 disagreements)
- ✅ Reconciled publish: `COALESCE(cls_strength, obligation_strength)` — classifier wins, regex fallback
- ✅ Prepare RACI training data — 1,763 examples (1,263 positive, 500 negative)
- ✅ Create fine-tuning script (scripts/ml/finetune_raci.py — namespaced /workspace/raci/)
- ✅ Create batch inference script (scripts/ml/runpod_raci_batch.py)
- ✅ Create Modelfile (scripts/ml/Modelfile.raci)
- ✅ RunPod: fine-tune gemma3-raci (RTX 5090, 9.9 min, 75.7% exact match)
- ✅ RunPod: fine-tune gemma3-artefact-props (RTX 5090, ~10 min)
- ✅ RunPod: fine-tune gemma3-control-titles (RTX 5090, 10.3 min)
- ✅ Download all three GGUFs locally (2.4GB each, GGUF headers verified)
- ✅ Back up GGUFs to NAS
- ✅ Pod stopped
- ✅ Rewrite batch scripts to write to PG — scripts run on pod, PG via reverse tunnel
- ✅ Add SLM columns to `jsp_provisions` PG table (slm_raci, slm_artefact_props, slm_control_title)
- ✅ Batch inference setup: pod started, 3 models loaded, reverse SSH tunnel for PG
- ✅ Batch RACI: 6,021 classified, 0 errors, 8 min (12.7/s)
- ✅ Batch artefact-props: 1,059 extracted, 6 errors, 3 min (5.9/s)
- ✅ Batch control-titles: 1,447 generated, 12 errors, 8 min (3.1/s)
- ✅ Pod stopped
- ✅ Backfill DuckDB from PG (3,106 RACI, 1,069 artefact props, 1,457 control titles)
- ✅ Republish full corpus (157/157 sources) with all SLM signals
- ✅ RACI role cleanup: expanded actor dictionary with 35 genuine new roles (radiation, range, safety, supply)
- ✅ RACI role cleanup: blacklist YAML (14 non-roles) + fuzzy map YAML (81 mappings) + Rust loader
- ✅ RACI role cleanup: applied to PG data — 164 roles → 56 (16 dropped, 133 remapped), backfilled DuckDB
- ✅ Artefact consolidation: 922 raw → 483 (chapter + type + role) → 406 (merge unassigned into siblings)

## Artefact Consolidation

Raw artefact mentions (922) overcount — multiple provisions in a chapter reference
the same risk assessment. Consolidation:

1. Group by `(source_id, artefact_type, primary_raci_role)` → 483
2. Merge "unassigned" into assigned sibling where same chapter+type exists → 406
3. 36 standalone unassigned remain (no assigned sibling for that chapter+type)

Result stored in `jsp_consolidated_artefacts` DuckDB table.

**Publish strategy:** Per-provision raw mentions still travel in the payload
(`mandated_artefacts_json`). The consolidation is a derived view — sertantai
can consolidate from the raw data it already receives, or fractalaw publishes
the consolidated table as a separate source-level payload when sertantai has
a resource for it. The per-provision data is the source of truth.
- ✅ Prepare artefact property training data — 922 examples (829 train, 93 val)
- ✅ Create fine-tuning script (scripts/ml/finetune_artefact_props.py — namespaced /workspace/artefact-props/)
- ✅ Prepare control title training data — 2,064 examples (1,000 legislation + 1,064 JSP)
- ✅ Create fine-tuning script (scripts/ml/finetune_control_titles.py — namespaced /workspace/control-titles/)
- ✅ Create Modelfiles for all three SLMs
- ⬜ RunPod: fine-tune 3 models in one session (RACI → artefact props → control titles)
- ⬜ Batch inference: RACI on 3,765 obligations without RACI
- ⬜ Batch inference: artefact properties on 922 artefacts
- ⬜ Batch inference: control titles on 922 JSP controls
- ⬜ LLM (Gemini) for residual low-confidence cases
- ⬜ Republish enriched data to sertantai

## Dependencies

- ✅ Phase 1-5 complete — regex extraction pipeline operational
- ✅ Phase 7 complete — full corpus enriched (11,351 provisions, 157 sources)
- ✅ Embeddings infrastructure (all-MiniLM-L6-v2 ONNX in fractalaw-ai)
- ✅ DRRP classifier v8 + position classifier v3 (JSON weights in fractalaw-cli/config/)
- ✅ Gemini batch infrastructure (gemini_llm_batch.py pattern)
- ✅ RunPod fine-tuning infrastructure (finetune_runpod.py pattern)
- ✅ Ollama batch inference infrastructure (runpod_slm_batch.py pattern)
- ✅ Corpus-level training data: 1,719 RACI, 5,028 obligations, 922 artefacts
