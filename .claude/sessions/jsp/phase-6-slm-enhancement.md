---
session: JSP Phase 6 — SLM Enhancement
status: suspended
opened: 2026-07-20
---

# Session: JSP Phase 6 — SLM Enhancement (SUSPENDED)

**Suspended:** Needs corpus-level data from Phase 7 before SLM work is viable. RACI fine-tuning needs hundreds of examples, not 32. Classifier evaluation needs cross-chapter coverage.

## Problem

Phases 1-5 built the JSP pipeline using regex extraction. The existing legislation pipeline uses a 4-tier classification stack (regex → logistic regression classifier → SLM/Ollama → Gemini LLM) with confidence-gated escalation. JSP enrichment currently uses only Tier 1 (regex). Applying the higher tiers would improve quality across all extraction phases — especially RACI disambiguation, artefact property extraction, and control title generation.

## Proposals

### 1. DRRP + Obligation Strength (Tier 2: Classifier)

**Current state:** JSP obligation strength is classified by modal verb keyword only ("must" → Mandatory, "should" → Recommended). This misses context — "must not" is Mandatory but prohibitive, "will ensure" in a preamble is descriptive not mandatory.

**Proposal:** Reuse the existing DRRP classifier (v8, logistic regression on 384-dim embedding + 13 modal indicators). JSP provisions get embedded via the same all-MiniLM-L6-v2 model. The classifier already handles the binary Obligation/Liberty/none — add a JSP-specific modal indicator for "will" and "is to" as supplementary features.

**Approach:** Embed JSP provisions → run DRRP classifier → compare with regex Tier 1 → reconcile. No new model needed — reuse existing v8 weights with JSP-specific feature augmentation.

**Expected gain:** Better precision on ambiguous provisions (preambles, descriptive "will", conditional "may need to").

### 2. RACI Position (Tier 3: SLM/Ollama)

**Current state:** RACI is inferred from narrative patterns (actor + modal → R; "accountable for" → A; "shall be informed" → I). Passive voice ("shall be conducted") produces no RACI — no actor identified.

**Proposal:** Fine-tune a Gemma-3-4B RACI classifier on JSP text. Input: obligation text. Output: JSON array of `{role, assignment_type}`. Training data: the 32 RACI assignments from Phase 3 regex extraction (validated manually) plus the 117 obligations as negative/ambiguous examples.

**Approach:** Same RunPod fine-tuning pipeline as gemma3-position. LoRA R=16. Export GGUF → Ollama. Batch inference on provisions where regex RACI is empty but obligation exists.

**Expected gain:** RACI coverage on passive-voice obligations. Currently ~27% of obligations have RACI (32/117). SLM could increase to 60-80%.

### 3. Artefact Property Extraction (Tier 6: Gemini Batch)

**Current state:** Artefacts are detected by type only (regex: "risk assessment", "safety case"). No structured properties — owner, approver, reviewer, frequency, required content, acceptance criterion are all NULL.

**Proposal:** Gemini 2.5 Flash batch extraction. For each artefact, send the parent obligation text + surrounding context. Prompt for structured JSON: `{owner_role, approver_role, reviewer_role, review_frequency, required_content, acceptance_criterion, scope}`. Same batch pattern as `gemini_llm_batch.py`.

**Approach:** Batch of 32 artefacts × 1 Gemini call each = 32 API calls. ~$0.02 total. Results write to `jsp_mandated_artefacts` DuckDB table (add columns for extracted properties).

**Expected gain:** The mandated artefact abstraction from the plan (JSP-SERVICE.md) becomes fully populated. Each artefact carries its operational properties → direct mapping to L3 Controls and L4 Evidence.

### 4. Control Title Generation (Tier 6: Gemini Batch)

**Current state:** Control titles are template-generated: "A {artefact_type} is maintained by the {role}". Grammar issues ("A Inspection Report"), no domain-specific language, no indicative-mood refinement.

**Proposal:** Gemini 2.5 Flash generation with the same design constraints as the legislation controls pipeline (COMPLIANCE-CONTROLS.md): indicative mood, referent not paperwork, discriminating test, honest limits.

**Approach:** For each of the 32 JSP controls, send: artefact type + obligation text + RACI assignments + competence requirements. Prompt for: title (indicative), description (what reality), what_it_checks (discriminating test). Same few-shot pattern as `generate_controls.py`.

**Expected gain:** JSP controls reach the same quality standard as Gemini-generated legislation controls. The template titles are replaced with domain-specific, checkable statements.

### 5. Source Traceability (Tier 6: Gemini Batch)

**Current state:** JSP provisions reference legislation at document level ("in accordance with the Electricity at Work Regulations 1989"). Phase 2 resolves these to `law_name`. But which *specific* legislative provision does each JSP obligation implement? This is unresolved.

**Proposal:** Gemini batch matching. For each JSP obligation, send: obligation text + candidate legislative provisions (from the referenced law, filtered to HIGH/MEDIUM significance obligations). Prompt: which provision(s) does this JSP obligation implement?

**Approach:** Per referenced law, pull obligation-type provisions from Postgres. Send JSP obligation + N candidates to Gemini. Returns: `[{section_id, confidence}]`. Populates `related_control_ids` on JSP controls and enriches `source_links`.

**Expected gain:** Provision-level traceability. "JSP 375 para 30 implements Electricity at Work Regulations reg.4(2)" — the gap analysis becomes specific.

### 6. Unresolved Reference Resolution (Tier 6: Gemini Batch)

**Current state:** 11/63 references (17%) unresolved in the pilot — mostly HSE guidance (HSG85, INDG139) and standards (BS 7671) which aren't in the fractalaw corpus.

**Proposal:** For genuinely unresolvable references (external standards/guidance), flag them as external and store the citation. For dense/implicit references ("in accordance with the relevant health and safety legislation"), use Gemini to identify which specific law(s) are meant.

**Expected gain:** Resolution rate from 82% to 90%+. External references flagged rather than silently dropped.

## Prioritisation

| Proposal | Effort | Value | Priority |
|----------|--------|-------|----------|
| 1. DRRP classifier | Low (reuse v8 weights) | Medium | Do first |
| 2. RACI SLM | Medium (fine-tune + batch) | High | Do second |
| 3. Artefact properties | Low (32 Gemini calls) | High | Do third |
| 4. Control titles | Low (32 Gemini calls) | Medium | Do fourth |
| 5. Source traceability | Medium (candidate set prep) | High | Do fifth |
| 6. Unresolved refs | Low (11 Gemini calls) | Low | Do last |

## Todo

- ⬜ Embed JSP provisions (all-MiniLM-L6-v2 via fractalaw-ai ONNX)
- ⬜ Run DRRP classifier (v8) on JSP provisions — compare with regex Tier 1
- ⬜ Reconcile classifier + regex DRRP — update jsp_enrichment
- ⬜ Fine-tune Gemma-3-4B RACI classifier on JSP training data
- ⬜ Batch RACI inference on passive-voice obligations (RunPod/Ollama)
- ⬜ Gemini batch: artefact property extraction (owner, approver, frequency, criterion)
- ⬜ Gemini batch: control title generation (indicative mood, domain-specific)
- ⬜ Gemini batch: source traceability (JSP obligation → legislative provision)
- ⬜ Gemini batch: unresolved reference resolution
- ⬜ Republish enriched data to sertantai

## Dependencies

- ✅ Phase 1-5 complete — regex extraction pipeline operational
- ✅ Embeddings infrastructure (all-MiniLM-L6-v2 ONNX in fractalaw-ai)
- ✅ DRRP classifier v8 + position classifier v3 (JSON weights in fractalaw-cli/config/)
- ✅ Gemini batch infrastructure (gemini_llm_batch.py pattern)
- ✅ RunPod fine-tuning infrastructure (finetune_runpod.py pattern)
- ✅ Ollama batch inference infrastructure (runpod_slm_batch.py pattern)
