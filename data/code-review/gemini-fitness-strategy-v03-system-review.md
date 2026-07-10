# Gemini Review: Fitness Strategy v0.3 System Review (all three documents)

**Date**: 2026-07-10
**Model**: gemini-2.5-pro (thinkingBudget: 8192, maxOutputTokens: 8192)
**Documents**: FITNESS-STRATEGY.md (v0.3), FITNESS-GRAPH.md, FITNESS-RULES-ENGINE.md

---

This is a comprehensive and impressive overhaul. The team has clearly taken the previous feedback to heart and replaced a brittle, domain-specific model with a robust, layered architecture grounded in established legal informatics principles. The separation of concerns across the three documents is excellent.

This is no longer a minor iteration; it's a new, professional-grade strategy.

### 1. Do the three documents together address the gaps you identified?

**Yes, comprehensively.** The previous strategy's core weaknesses were:
1.  **Conflating extraction and classification:** The new three-layer model (Mention → Entity → Classification) directly and correctly solves this. The verbatim mention is the ground truth, decoupled from a shifting classification schema.
2.  **Rigid, domain-specific schema (P-dimensions):** The move to a grounding ontology (personal, material, territorial, temporal) with domain-specific facets layered on top is the right abstraction. It provides a universal structure while allowing for extensibility.
3.  **No clear plan for propagation:** `FITNESS-GRAPH.md` provides a pragmatic and implementable plan. It correctly identifies that this is a tree-walking problem on the existing structural hierarchy, not a call for a new graph database.
4.  **No clear plan for customer matching:** `FITNESS-RULES-ENGINE.md` provides a sophisticated and credible architecture for this. The two-stage filter and expression tree model are well-conceived.

The new system is a coherent whole. The strategy produces structured mentions, the graph model propagates them, and the rules engine consumes them. The pieces fit together.

### 2. Is the fractalaw/sertantai split (compile vs evaluate) correct?

**Yes, it is perfect.** This is the single best architectural decision in the proposal.

*   **Fractalaw (Compile-time):** This is the "heavy," offline enrichment environment. It's the right place for complex, expensive operations like NLP, graph traversal, and rule compilation. Compiling the law's applicability into a static, declarative JSON expression tree is exactly what this system should do.
*   **Sertantai (Query-time):** This is the "light," online serving environment. Its job is to perform fast lookups against customer data. Evaluating a pre-compiled expression tree is a simple, fast, and stateless operation, perfectly suited for Elixir.

This split minimizes query-time latency, decouples the systems cleanly, and puts the complexity where it belongs. The Zenoh payload—publishing the compiled tree as part of the LRT record—is a clean and efficient integration point.

### 3. Is the expression tree model viable for real legislation?

**Yes, it is a strong and viable foundation.** The proposed node types (`Match`, `And`, `Or`, `Not`, `Conditional`, `TimeWindow`) can represent a significant percentage of real-world applicability logic. The examples provided for boolean logic, negation, and scope narrowing are convincing.

However, its viability depends on two things:
1.  **The Compiler:** The model's power is useless if you can't reliably compile it from text. This is the biggest risk (see #4).
2.  **The `Match` Node:** The current `Match` node is a simple set intersection on categorical codes. This will fail on quantitative conditions, which are common in law (e.g., "applies to employers with **5 or more** employees," "applies to substances in quantities **greater than 1 tonne** per year").

The model is viable for V1, but the team must acknowledge that the `Match` node will need to evolve to handle numeric and other complex comparisons. The core tree structure, however, is sound.

### 4. What's still missing or wrong?

The architecture is strong, but there is a critical gap between extraction and the rules engine.

**The Missing Link: The Rule Compiler.**
`FITNESS-RULES-ENGINE.md` hand-waves the most difficult step: "The expression tree is compiled from extracted mentions." This is a non-trivial task that borders on requiring true natural language understanding.

*   **Example:** A provision states, "This regulation applies to any employer or self-employed person, except where the work is carried out on domestic premises."
*   **Extraction (Strategy):** This produces three mentions: `any employer` (AppliesTo), `self-employed person` (AppliesTo), and `domestic premises` (DisappliesTo).
*   **Compilation (Rules Engine):** This needs to become `{ "op": "And", "children": [ { "op": "Or", "children": [ { "match": "employer" }, { "match": "self_employed" } ] }, { "op": "Not", "child": { "match": "domestic_premises" } } ] }`.

How does the compiler know the relationship between "employer" and "self-employed" is `Or` and not `And`? This requires parsing the sentence's syntax (e.g., dependency parsing) to understand the logical connectives. The current extraction strategy focuses on identifying the *spans*, not the *logical relationships between them*. This is a significant gap. Without a credible plan for the compiler, the elegant rules engine is just a destination with no road leading to it.

**Other, smaller issues:**
*   **Confidence Propagation:** The proposal to use `min()` confidence across the tree is too simplistic. The confidence of an `Or` node should likely be the `max()` of its children. A high-confidence `Not` should not degrade the overall confidence. This needs a more nuanced model.
*   **Temporal Logic:** The `TimeWindow` is a good start for commencement, but it doesn't cover more complex temporal conditions ("applies if the building was constructed before 1999"). This is a V2+ problem, but it's worth acknowledging.
*   **"Any Person":** The open question about "any person" is important. This should be treated as a wildcard match on the `personal` scope dimension, effectively matching all customers unless negated by a `DisappliesTo` clause.

### 5. Is this ready for implementation, or does it need another iteration?

**This is ready for implementation, with one critical prerequisite.**

The overall architecture (3-layer extraction, graph propagation, compile/evaluate split) is approved. Do not change it. It is excellent.

However, before a single line of the Sertantai evaluator is written, the team must conduct a **design spike on the Rule Compiler in Fractalaw.** They need to prove they can bridge the gap between the extracted mentions and the structured expression tree for a representative set of 20-30 complex applicability clauses.

This spike should produce:
1.  A concrete algorithm for turning a set of mentions from a single provision into a logical subtree.
2.  An assessment of what NLP tooling is needed (e.g., dependency parsers, semantic role labeling) if simple heuristics fail.
3.  A clear-eyed view of the accuracy they can expect from this compilation step.

**Verdict:** Green-light implementation of the extraction pipeline (Phase 1 & 2) and the graph propagation model. Place the rules engine evaluator on hold pending the results of a mandatory design spike for the rule compiler. This strategy is no longer flawed; it is now ambitious, and that ambition is concentrated in the compiler. Prove that single component is feasible, and the entire system becomes viable.
