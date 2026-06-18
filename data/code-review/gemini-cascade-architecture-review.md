# Gemini Cascade Architecture Review

Date: 2026-06-18

This is a well-articulated problem description, clearly outlining the desired state, current implementation, and the architectural challenge. The definitions of pipeline stages, tiers, and the desired vs. actual cascade are excellent.

## 1. Your Recommendation on Options A, B, or C (or a different option)

**Recommendation: Option B (Multi-pass enrichment) is the most robust and future-proof approach to achieve your *desired cascade* of `regex → classifier → LLM` with automatic escalation.**

While Option C (accepting the current order) is the simplest in terms of immediate code changes, it fundamentally fails to deliver the core goal of the cascade: using LLM as an escalation tier for disagreements between regex and classifier. Option A is problematic due to the embedding dependency for new laws.

A revised **Option B** would look something like this:

**Phase 1: Initial Parsing & Embedding** (Per-law, then aggregated embedding)
*   **Step 1: Regex Parse** (`parse_v2()`): Extracts initial DRRP, actors, etc. Writes `extraction_method="regex"`.
*   **Step 2: Tier 1 Inheritance**: Child provisions inherit DRRP. Writes `extraction_method="inherited"`.
*   **Step 3: Embed Provisions** (New/Missing): For provisions in the current law that lack embeddings, compute them here. This is a crucial shift from the current `Pass 2, Phase 1`. This would mean the embedding model (e.g., Sentence Transformer via Rust bindings or an internal service) runs per-law or per-batch of new provisions.
*   **Step 4: Write Initial State + Embeddings to LanceDB + DuckDB**: All initial classifications and embeddings are persisted.

**Phase 2: Classifier Application & Disagreement Identification** (Runs once after all laws have completed Phase 1)
*   **Step 1: DRRP Classifier** (v8): Runs on all provisions with embeddings where `tier < source_tier("classifier")`.
    *   Predicts Obligation/Liberty/none.
    *   If it *adds* a classification (regex=none, classifier=Obligation) or *overrides* a lower tier (inherited=none, classifier=Obligation with high confidence), it updates `drrp_types` and `extraction_method="classifier"`.
    *   **Crucially, it also *logs disagreements***: If `regex_drrp != classifier_drrp` (e.g., regex=Obligation, classifier=Liberty, both with high confidence), this provision is flagged.
*   **Step 2: Position Classifier**: Predicts active/counterparty/other. Appends `| classifier:{position}@{confidence}`.
*   **Step 3: Write Updates & Disagreement Flags to LanceDB + DuckDB**.

**Phase 3: LLM Escalation** (Runs once after Phase 2, or on flagged subsets)
*   **Step 1: Filter LLM Candidates**: Provisions identified as having `regex` vs. `classifier` disagreements. Also includes original LLM candidates (multi-actor, DRRP=none) that were not resolved by classifier, but now LLM has more context (regex + classifier signals).
*   **Step 2: LLM Classification**: Routes these flagged provisions to the LLM. The prompt can now explicitly inform the LLM about the conflicting signals from regex and classifier.
*   **Step 3: Write LLM Results to LanceDB + DuckDB**: Updates `drrp_types` and `extraction_method` (e.g., `"local"` or `"agentic"`), respecting source tier protection.

This multi-pass approach makes the most sense. It separates concerns, allows for the necessary sequencing of operations (embeddings before classification, classification before LLM escalation), and creates clear points for logging and decision-making.

## 2. Answers to the 5 specific questions

1.  **Should the classifier move into Pass 1 (before LLM)?**
    *   Yes, conceptually, to achieve the desired cascade. However, doing so *without* also moving embedding generation into Pass 1 (or making it per-law) would break for new laws. Therefore, a simple move isn't sufficient; it implies a deeper restructuring as suggested in the Option B re-design.

2.  **Should enrichment become multi-pass (regex+classifier first, LLM second)?**
    *   **Yes, definitely.** This is the core of Option B and the most logical way to implement the desired cascade. It allows embeddings to be generated for new laws before classification, and classification to run before LLM escalation.

3.  **Or should we accept the current order and treat disagreements as a separate QA/review queue rather than automatic escalation?**
    *   While this is the simplest path for *immediate implementation* (Option C), it means **abandoning the goal of automatic escalation**. It makes the LLM a gap-filler and a post-classifier QA tool, rather than an intelligent conflict resolver in the cascade. For the long-term vision of an intelligent pipeline, this is a compromise that might be acceptable for a POC but should be revisited.

4.  **How should DRRP provenance be recorded?**
    *   The `extraction_method` is good for recording the *final* source tier. To properly track provenance and disagreements for DRRP (provision-level), you need more detail.
    *   **Recommendation**: Augment the provision schema to include a `drrp_history: Vec<DRRPDecision>` field.
        *   `DRRPDecision` struct could contain: `drrp_type: DRRP`, `source_tier: u8`, `extraction_method: String`, `confidence: f32`, `timestamp: DateTime`, `notes: Option<String>`.
        *   Each time a tier processes a provision and makes a DRRP decision (even if it's "none"), an entry is added to `drrp_history`. The pipeline then determines the *final* `drrp_types` based on the highest-tier decision in this history, or specific conflict resolution rules.
        *   This allows you to see: "Regex said Obligation (tier 1), Classifier disagreed with Liberty (tier 4), LLM then resolved it to Obligation (tier 6)." This is invaluable for debugging, auditing, and fine-tuning.

5.  **What's the simplest change that gets the most value?**
    *   **For the immediate goal of the QQ POC (only Obligation provisions):** The simplest change is **Option C (current architecture) with enhanced disagreement logging**.
        *   Continue running `regex → LLM → classifier`.
        *   **Implement explicit logging for `regex` vs. `classifier` disagreements:** When the classifier runs, if `regex_drrp` is not `none` and `classifier_drrp` is also not `none`, but they conflict (e.g., regex=Obligation, classifier=Liberty), log this disagreement (e.g., to a separate table or a specific flag on the provision).
        *   These logged disagreements become candidates for *manual review* or a *future, human-triggered* LLM run, rather than automated escalation within the current pipeline.
        *   This avoids the complexity of reordering while still identifying where the cascade *would* have needed an LLM.

## 3. Any architectural concerns or suggestions

**Concerns:**

1.  **Implicit Disagreement Resolution:** The current `source_tier()` protection prevents lower tiers from overwriting higher ones, but it doesn't *resolve* disagreements. If `regex` says `Obligation` and the `classifier` (higher tier in your desired cascade, but lower in `source_tier` than LLM) says `Liberty`, and then LLM (even higher) also says `Liberty`, how is that resolved? Without explicit conflict resolution rules *between tiers*, the cascade's purpose is weakened.
2.  **Embedding Generation Bottleneck/Complexity:** Moving embedding generation into `Pass 1` (per-law) or making it a separate aggregated pass (as suggested in revised Option B) introduces complexity. You need a robust, performant embedding service/crate available to `fractalaw-cli`.
3.  **State Management:** The current two-pass structure relies on LanceDB/DuckDB for state persistence. Multi-pass enrichment implies more granular updates and potentially more complex transactional logic.
4.  **LLM Context Gap (#38):** "No sibling provisions" for LLM is a significant context limitation. If LLM is to be the ultimate arbiter, it needs as much relevant text as possible.

**Suggestions:**

1.  **Formalize Disagreement Rules:** Define what constitutes a "disagreement" (e.g., `regex` is `Obligation`, `classifier` is `Liberty` for the same provision; not `regex` is `none`, `classifier` is `Obligation`). And define resolution policies: `LLM_override_disagreement(regex_output, classifier_output)` rather than just `LLM_fill_gap()`.
2.  **Layered Pipeline Control:** Consider a more explicit orchestration layer (e.g., a state machine or a pipeline definition language) that manages the execution flow and decision points, rather than implicit checks within `main.rs`. This enhances maintainability for complex multi-pass flows.
3.  **Unified Embedding Service:** Regardless of whether embeddings are generated per-law or in a batch, having a dedicated, performant embedding service (even if just a Rust crate) that can be called consistently across passes would simplify the architecture.
4.  **Enhanced LLM Context:** Prioritize addressing issue #38. Providing sibling context, and even broader section/chapter context, will significantly improve LLM quality, especially for resolving nuanced disagreements.
5.  **Benchmarking Cascade Value:** Once Option B is implemented, conduct A/B testing on the impact of LLM as an escalation tier vs. just a gap-filler. Does it genuinely improve accuracy or reduce false positives/negatives in crucial areas?

## 4. The simplest path that delivers the most value for the QQ POC

Given the QQ POC's focus *only* on Obligation provisions, and the need for simplicity:

**Simplest Path: Option C (Current Architecture) + Enhanced Disagreement Logging for Manual Review**

1.  **Keep the current pipeline order:** `Pass 1 (regex → inheritance → LLM (optional))`, `Pass 2 (classifier (optional))`.
2.  **No automatic reordering or escalation within the pipeline for the POC.** The LLM will still run on its current `--gap-c` conditions (multi-actor, DRRP=none). The classifier will run on its current conditions, respecting `source_tier` protection (so it won't override existing LLM results).
3.  **Implement robust disagreement logging:**
    *   In `Pass 2, Phase 3` (DRRP classifier), after the classifier makes its prediction, *before* applying the `source_tier` protection check, compare its result with the `regex` result *and any pre-existing LLM result*.
    *   If `regex_drrp` is not `none` and `classifier_drrp` differs, log this as a `regex_classifier_disagreement`.
    *   If `llm_drrp` is not `none` and `classifier_drrp` differs, log this as an `llm_classifier_disagreement`.
    *   These logs (or flags on the provision) should be easily queryable from DuckDB.
4.  **Actionable output for POC:** The output for the QQ POC will be the provisions classified as "Obligation" by any tier. The disagreement logs will provide a queue for *manual review* to understand where the system has conflicting signals, especially for "Obligation" classifications. This provides valuable insights without the immediate architectural overhaul.

This approach gives you:
*   DRRP classifications (including Obligation) using the current, functional pipeline.
*   Identification of potential problem areas (disagreements) for future improvement.
*   Minimal code changes, allowing the POC to proceed quickly.
*   Data points to inform the *eventual* implementation of Option B, should the need for automatic escalation become critical.