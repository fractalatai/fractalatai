# Gap C Sessions — Critical Review

**Date**: 2026-04-15
**Reviewer**: Claude (continuation of OH&S gap-analysis session)
**Docs reviewed**:
1. `04-15-26-gap-c-ai-research.md` (research/decision record)
2. `04-15-26-gap-c-orchestration.md` (cross-session coordination)
3. `04-15-26-gap-c-session-1-main-prep.md` (S1/S2/S3)
4. `04-15-26-gap-c-session-2-training-repo-todo.md` (S4/S5/S6)
5. `04-15-26-gap-c-session-3-main-integration.md` (S7/S8)

## Overall assessment

The research doc is exceptionally well-reasoned. The critical analysis of external advice (ChatGPT/Gemini) is sharp — correctly rejecting runtime LLM, spaCy dep-parsing, and model-size inflation while extracting the genuinely useful ideas (structured extraction, context retrieval, teacher-model bootstrapping). The staged plan is logically sequenced with clear hand-off artefacts and go/no-go gates.

However, the plan is built on a **materially incorrect assumption** about the current model artefact, which propagates through all five documents and undermines the architecture.

## Critical issues

### 1. Model identity mismatch — DistilBERT, not DeBERTa-v3

The research doc states throughout that the existing model is "a fine-tuned DeBERTa-v3 exported to int8 ONNX" (§2, §4.1, §4.4, §5). This is wrong.

Evidence from `models/deberta-v3-drrp/`:
- `metadata.json`: `"base_model": "distilbert-base-uncased"`
- `tokenizer_config.json`: `"tokenizer_class": "BertTokenizer"`, `"do_lower_case": true`
- `model.onnx` = 265 MB (consistent with DistilBERT ~66M params, not DeBERTa-v3-base ~86M+)

The directory is *named* `deberta-v3-drrp` but the model inside is DistilBERT. Every reference to "DeBERTa-v3-base as the backbone" and "add a second head set alongside the existing polisher" is predicated on a model that doesn't exist.

**Impact**: The plan to add detector heads to "the existing DeBERTa-v3 backbone" needs to either (a) upgrade the backbone to DeBERTa-v3 first, which is a training-from-scratch exercise, or (b) work with DistilBERT, which has weaker contextual representation (no disentangled attention, no relative position encoding) — relevant for the long-range context retrieval proposed in §4.2.

### 2. Polisher appears non-functional

`metadata.json`:
```json
"training_examples": 200,
"best_clause_acc": 0.0
```

200 training examples is far below the ~1k minimum for a 3-head extraction model. `best_clause_acc: 0.0` means the clause-span head never learned to extract — the model literally cannot find the duty clause in the text.

The research doc says "the existing `DrrpExtractor` **polishes** entries that the regex pipeline has already detected" (§3) and frames the gap as "the polisher is never invoked for Gap C provisions." In reality, the polisher may not be usable for *any* provisions. The entire architecture of "existing polisher heads + new detector heads on a shared backbone" assumes working polisher heads that can be frozen or jointly fine-tuned.

**Impact**: Session 2's training plan (Phase D) assumes it can "reuse `clause_span` + `qualifier_span` heads from the polisher" — there may be nothing to reuse. The plan should treat this as training from scratch, not extending a working model.

### 3. Broken holder labels

Two labels in `holder_labels.json` are truncated/corrupt:
- `": He"` — likely truncated from a longer label
- `"Gvt: Ministry:"` — trailing colon, no value

These are in the 27-class vocabulary the research doc treats as the fixed ontology. Session 1 (S1) pins `holder_labels.json` at hand-off to training, and Session 2 validates against it. Corrupt labels will propagate through teacher labelling, training, and inference.

**Impact**: Fix before any other work. Small but foundational.

### 4. max_length: 128 tokens — too short for context retrieval

The existing model uses `"max_length": 128`. The research doc's §4.2 proposes feeding `[CLS] query [SEP] source [SEP] context [SEP]` where context includes parent clause, section definitions, and act-level general duty. A single UK legislative provision can exceed 128 tokens on its own; adding context requires 256–512.

The `gap-c-context-format.md` spec (Session 1 S3 deliverable) will define the tokenizer input shape. If the new model trains at 256+ tokens, it's incompatible with the existing 128-token infrastructure. `DrrpExtractor::load` would need to handle variable max_length, and the existing polisher (if functional) couldn't share a session with the detector at a different sequence length.

**Impact**: The context-retrieval design (§4.2) is architecturally sound but the token budget needs explicit planning. Not mentioned in any session doc.

### 5. actors.rs vocabulary drift from holder_labels.json

`actors.rs` defines 40+ governed/government actor labels. `holder_labels.json` has 27. The research doc says they're "aligned" (§2) but they're not:

Missing from holder_labels but in actors.rs:
- `SC: C: Contractor`, `SC: C: Principal Contractor`, `SC: C: Designer`, `SC: C: Principal Designer`
- `SC: Manufacturer`, `SC: Importer`, `Svc: Installer`
- `Ind: Worker`, `Ind: User`, `Ind: Competent Person`
- `Org: Company`, `Org: Landlord`, `Offshore: Licensee`
- All FIRE specialist actors

This means the detector can never predict these roles. The Gap A analysis found Manufacturer (34 misses), Installer (13), Contractor (12), User (32) — all absent from the model vocabulary. The research doc's §4.1 says "existing 27-class holder softmax" for the detector but these common roles are missing.

**Impact**: The 27-class vocabulary needs expansion before training, not "reactively" after (as §5 suggests). Training a model that can't predict Contractor or Manufacturer would be a known blind spot from day one.

## Positive findings

### Architecture decisions are correct
- DeBERTa-v3-base (or equivalent) as backbone: right size class
- Detector as fallback after regex: right pipeline position
- No runtime LLM: correct for local-first Rust architecture
- No meta-labels (Passive/Inherited): correct for user-facing taxonomy
- Separate training repo: correct separation of concerns
- `holder_inferred_from` provenance field: well-designed audit trail
- Feature flag + confidence threshold for precision protection: sound

### Orchestration is well-structured
- Clear hand-off artefacts between sessions
- Go/no-go gates with specific criteria
- Risk tracking across sessions (taxonomy drift, context-format drift, precision regression)

### Gap C sub-type taxonomy is useful
- C1–C6 classification gives structure to what was a monolithic "no actor" category
- Correctly notes C1 is mostly handled by existing Rule patterns
- C2 (sub-clause inheritance) and C4 (cross-section actor) are well-identified as the highest-value targets

### External advice triage is excellent
- Correctly identifies what's already done, what's useful, and what's misaligned
- The "no spaCy, no Ollama, no Python in production" constraints are consistent with CLAUDE.md

## Recommendations

### Must-fix before proceeding

1. **Audit the existing model artefact.** Determine: is it DistilBERT or DeBERTa-v3? If DistilBERT, rename the directory and update all references. Decide whether to proceed with DistilBERT or upgrade to DeBERTa-v3 as part of Session 2.

2. **Fix broken holder labels.** Clean up `": He"` and `"Gvt: Ministry:"` before pinning the vocabulary for training.

3. **Reconcile holder vocabulary.** Expand `holder_labels.json` to include Contractor, Manufacturer, Importer, Installer, Designer, Worker, User at minimum. Do this in Session 1, not reactively after training.

4. **Assess polisher functionality.** Run the existing `DrrpExtractor::extract()` on a sample of regex-detected provisions and report accuracy. If `best_clause_acc: 0.0` reflects reality, Session 2 should plan to train the polisher alongside the detector, not extend it.

### Should-fix

5. **Plan token budget explicitly.** Add a section to Session 1's S3 (context-retrieval helper) specifying the target max_length for the new model (256 or 512), how truncation priority works across the three segments, and backwards-compatibility with the 128-token polisher.

6. **Account for Fix 2's 50 provisions.** The OH&S gap analysis identified 50 genuine v2 matcher bugs (subordinate clause + pronoun reference, epistemic "may" rejection) that will be covered by the AI detector. Note these as expected pickups in Session 3's validation plan.

7. **Clarify the "two heads on one backbone" vs "two models" decision.** Session 2 defers this to "after first training run" but the Session 1 hand-off package and Session 3 integration plan both assume a single artefact. If two models are needed, the integration code doubles. Make a provisional decision now and flag the alternative.

### Consider

8. **De-scope `holder_inferred_from` to Session 3.** The provenance field is well-designed but threads through every storage and publish layer (S2 deliverable). It's not needed for the detector to work — the detector outputs a holder and confidence; provenance is a presentation concern. Moving it to Session 3 would simplify Session 1 and let the detector land faster.

9. **Start with C2 (sub-clause inheritance) as a focused pilot.** C2 is high-frequency, structurally detectable, and the context retrieval is simpler (just the parent paragraph). A pilot on C2 alone would validate the full pipeline (labelling → training → integration → eval) before investing in C4/C6 which need more complex context.

---

## Post-revision appraisal

**Date**: 2026-04-15
**Scope**: All five Gap C docs revised in response to the critical review above.

### Verdict: all nine review items addressed

Every critical, should-fix, and consider item from the original review has been
incorporated. Several have been addressed more thoroughly than requested, with
the author stepping back to resolve the underlying architectural tension rather
than patching the symptoms.

### Item-by-item status

| # | Issue | Status | Where addressed |
|---|-------|--------|-----------------|
| 1 | Model identity (DistilBERT not DeBERTa-v3) | **Resolved** | Research §0, §2, §3, §4.1, §4.4, §9. Every reference corrected. Artefact explicitly deprecated throughout. S9 removes it. |
| 2 | Polisher non-functional (200 examples, 0.0 acc) | **Resolved** | Research §3, §4.1. Acknowledged as non-functional. No heads reused. S7 disables the call; S9 removes the artefact. |
| 3 | Broken holder labels | **Resolved** | Research §4.1 two-vocabulary section. Session 1 S1b is an entire new deliverable for vocabulary reconciliation. Hard exit gate. |
| 4 | max_length 128 too short | **Resolved** | Research §0, §4.1, §4.2. ModernBERT-large (8192 native context) as backbone. Token budget pinned at 2048 in Session 1 S3 with truncation priority. |
| 5 | actors.rs / holder_labels.json drift | **Resolved** | Subsumed by #3. S1b lists all ~30+ missing labels. Audit + verify step included. |
| 6 | Account for Fix 2's 50 provisions | **Resolved** | Research §5 S8, Session 3 S8 dedicated bullet, orchestration Phase 1 exit gate. |
| 7 | Clarify single-model vs multi-model | **Resolved** | Research §4.1: "server-side concern only" — the remote-service architecture makes this invisible to fractalaw. Clean resolution. |
| 8 | Defer holder_inferred_from to Session 3 | **Accepted** | Session 1 scope change note. S7 absorbs the work. Orchestration updated. |
| 9 | C2 pilot first | **Accepted** | Research §5, Session 2 phasing section, orchestration Phase 1/Phase 2 gates. |

### Significant new material beyond the review

The revisions go well beyond patching the flagged issues. Three additions
deserve comment:

**§0 — Quality-first reframing.** The review flagged model identity; the
author diagnosed the deeper problem: local-first constraints were being applied
to the parsing service where they don't belong. The quality-first / local-first
separation is the most important change in the revision. It unlocks
ModernBERT-large, full fine-tune, fp16 precision, and remote serving — all
decisions that follow cleanly once the constraint is corrected. Well reasoned.

**§0.1–0.3 — Remote serverless-GPU architecture.** This is a significant
departure from the original local-ONNX plan. The review didn't suggest it;
it emerged from §0's reframing. The economics are sound (scale-to-zero,
<$500/year steady-state), the micro-apps WIT boundary handles the abstraction
cleanly, and the offline fallback (S7: endpoint unreachable → regex-only, no
emission) is correct. The main risk — network dependency during enrichment — is
acceptable because enrichment is a batch job, not a user-facing query path.

**§8 — Iterative taxonomy and retrain cadence.** The review said "reconcile
the vocabulary"; the revision designs a long-lived feedback loop
(QA → taxonomy-change-log → head-swap retrain → new model version → endpoint
hot-swap). Phase F in Session 2 formalises the training repo as a persistent
asset. This is forward-looking and correct — the ontology *will* grow.

### Two-vocabulary model (§4.1)

The research doc's treatment of the `": He"` problem is more sophisticated
than the review's "fix broken labels." Rather than deleting the regex-era
placeholders, the revision:
- Preserves them in `actors.rs` for regex detection (they still serve a
  purpose until the detector is proven)
- Excludes them from the model vocabulary (training never sees them)
- Routes regex-emitted placeholders through the detector for resolution (S7)
- Suppresses unresolved placeholders from publish (never reaches sertantai)
- Retires them from `actors.rs` in S9 once the detector covers their cases

This is a better solution than wholesale deletion and handles the transition
cleanly.

### One stale reference to fix

**Orchestration hand-off artefacts, Session 1 → Session 2, item #4** still
reads:

> DuckDB LRT migration note — the new `holder_inferred_from` column already
> exists in main at hand-off

But Session 1 now defers the DuckDB column to Session 3 S7. The column does
**not** exist at the Session 1 → Session 2 boundary. The exit-gate section
is correct (it notes the deferral), but the hand-off list contradicts it.

Suggested fix: replace item #4 with a note that the field *design* spec
exists in research doc §4.1a and that training labels must carry an
`inferred_from` field, but the DuckDB column itself ships in Session 3.

### Session 3 S7 scope creep — acceptable but flag it

S7 now absorbs:
- `holder_inferred_from` schema (deferred from Session 1)
- Remote-inference provider (new)
- Deprecated-polisher disabling (new)
- Placeholder routing rule (new)
- The original detector integration + feature flag + confidence threshold

This is the largest single deliverable across all three sessions. The
consolidation is correct — shipping the schema alongside its producer avoids
dead columns — but Session 3 should budget for the expanded scope. If S7
needs splitting at execution time, the natural seam is: (a) schema +
remote-provider + polisher-disable as one PR, (b) parse_v2 integration +
placeholder routing + tests as a second.

### Minor observations

- Session 2 core-libraries table still lists `peft (LoRA/QLoRA)` without
  noting it's fallback-only. The prose below is clear; the table could match.
- Session 2 `src/train/model.py` description says "ModernBERT-large or
  DeBERTa-v3-large" but Phase D trains both. The description should say
  "both configs dispatched via CLI arg" (which it does in the task list —
  just not in the repo layout comment).
- The cost envelope in §8 is helpful. Worth cross-referencing from the
  orchestration doc so it's visible at the coordination level.

### Overall assessment

The revisions are thorough, internally consistent (one stale reference
excepted), and in several places improve on what the review asked for. The
quality-first reframing, remote-service architecture, and iterative-taxonomy
design are genuine improvements to the project plan, not just responses to
the review's complaints. Ready to proceed to Session 1 execution.
