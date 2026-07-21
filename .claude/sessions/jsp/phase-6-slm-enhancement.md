---
session: JSP Phase 6 — SLM Enhancement
status: active
opened: 2026-07-20
---

# Session: JSP Phase 6 — SLM Enhancement (ACTIVE)

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
- ⬜ RunPod: fine-tune Gemma-3-4B on RACI training data
- ⬜ RunPod: batch RACI inference on 3,765 obligations without RACI
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
