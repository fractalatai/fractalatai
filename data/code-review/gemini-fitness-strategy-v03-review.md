# Gemini Review: Fitness Extraction Strategy v0.3

**Date**: 2026-07-10
**Model**: gemini-2.5-pro (thinkingBudget: 8192, maxOutputTokens: 8192)
**Document**: `.claude/plans/fitness/FITNESS-STRATEGY.md` (v0.3)

---

Alright, team. Let's cut to the chase. V0.3 is a significant conceptual improvement over the brittle P-dimension model. Separating extraction, linking, and classification is the correct architectural pattern. You've moved from a dead-end to a viable path.

However, this document mistakes a high-level diagram for an architecture. You've identified the right building blocks but have no idea how they connect, what the load-bearing components are, or how the entire structure will handle real-world stress. You've designed the "what," but completely ignored the "how."

Let's address your questions directly.

### 1. Is the Mention layer needed?

**Yes, it's non-negotiable. But your justification is weak and your implementation is likely wrong.**

The point of the Mention layer isn't just to store a "span." It's to create an **immutable, auditable artifact of the extraction process**. It is the ground truth, anchored directly to the source text. When your downstream entity linking or classification logic inevitably changes, the Mention remains the stable foundation you can replay that new logic against. Without it, you'd have to re-run expensive NER/SLM extraction every time you tweak a facet.

Your question, however, hints at a critical implementation flaw. "Mentions are stored on the same row as the provision in the LAT table." This is either a denormalization shortcut or a misunderstanding of the data model. A single provision can contain multiple applicability mentions. For example: "This regulation applies to employers, except for those in the maritime sector, but extends to self-employed persons on construction sites." That's three distinct mentions (AppliesTo, DisappliesTo, ExtendsTo) in one provision.

Storing this as a JSON blob on the `LAT` row is a ticking time bomb for query performance and data integrity. The correct relational model is a `mentions` table with a foreign key to `provisions`. If you're denormalizing, you must justify the trade-off. Right now, it just looks like you haven't thought through the one-to-many relationship.

**Verdict:** The concept is essential. Your proposed physical storage model is suspect and needs rigorous justification.

### 2. Is the Facets layer positioning correct?

**No. Stop hedging. "Exemplar" is weak and ambiguous.**

The Facet layer *is* your classification system. It's not an "exemplar"; it's the core component that makes your data useful. The four scope dimensions (personal, material, territorial, temporal) are not just one possible scheme; they are your **foundational, domain-agnostic ontology**. All other schemes (SIC codes, DEFRA designations) are domain-specific extensions that build upon this base.

Frame it this way:
*   **Layer 3: Classification.** The mechanism is a flexible key-value store (facets).
*   **Base Schema:** The four scope dimensions are the mandatory, top-level keys. They provide the universal structure for all applicability rules.
*   **Extended Schemas:** Domain-specific vocabularies (SIC, HSE, etc.) are added as needed.

This framing forces you to confront the real problem you've ignored: **governance**. Who owns these facet schemas? How are they versioned? What's the process for adding a new SIC code or a new `actor_type`? Without a clear governance model, your "flexible" facet system will devolve into an unmanageable tag soup within six months.

**Verdict:** The positioning is wrong. Call it what it is—your classification layer—and define the governance model for the schemas within it.

### 3. Does the three-layer model solve the problems, or is it over-engineering?

**It is not over-engineering. It is the minimum viable architecture for this problem.**

The previous models were *under-engineered*. They conflated concerns, leading to the exact brittleness you're trying to escape. This separation is the only way to build a system that can evolve. You can upgrade your NER model (Layer 1) without touching your entity definitions (Layer 2). You can add a new classification scheme (Layer 3) without re-extracting from source text (Layer 1).

The real risk isn't over-engineering the layers, but under-specifying the interfaces between them. You've defined the boxes but not the arrows. How is confidence propagated from a Mention to a Link to a Facet? What happens when a low-confidence NER extraction is linked to a high-confidence dictionary entity? Your "staircase" model is a nice idea, but you have no strategy for error handling or confidence scoring across these boundaries.

**Verdict:** The architecture is correct, but the implementation details that make it work are missing. It solves the old problems by introducing new, more complex ones you haven't addressed.

### 4. Is the customer matching model (facet intersection) viable?

**No. It's naive and will fail immediately on real-world cases.**

"Facet intersection" is a toy model that works for your cherry-picked examples. It is not a compliance platform.

1.  **Boolean Logic:** Your model assumes every condition is an `AND`. What about a law that applies to "any person who operates a vehicle OR a vessel"? Your intersection model fails. Real applicability is a complex boolean expression, not a simple set overlap.
2.  **Negation:** How does `DisappliesTo` work? Is it a simple set subtraction? What if the exclusion is broader or narrower than the inclusion? "Applies to all construction work, except work on domestic premises with fewer than 5 employees." This requires a rules engine, not `set.intersection()`.
3.  **Hierarchy:** A customer is a quarry (`sic_code: 08.11`). A law applies to "Mining and Quarrying" (`sic_section: B`). Does it match? Not in your model, because the strings don't match. Your matching logic requires an external, hierarchical ontology for every facet scheme, and you haven't even mentioned it.
4.  **Conditionality:** Your model has no concept of conditional applicability. "Applies to employers *if* they handle asbestos." The match depends on the combination of facets, not their mere presence.

**Verdict:** The matching model is fundamentally unviable. You need to be thinking in terms of a **rules engine** that can evaluate boolean expressions and hierarchical relationships between customer and legal facets.

### 5. What's missing or wrong?

This is the main event.

1.  **Phase 3 Propagation is a Fantasy.** You've hand-waved the hardest part of the entire system. "Graph traversal for cross-references is a later enhancement" is an architecturally disqualifying statement. The law *is* a graph of cross-references. An SI that amends a primary Act inherits and modifies its scope. Ignoring this isn't an 80/20 simplification; it's a guarantee that your scope analysis will be wrong for the majority of the corpus. This isn't an "enhancement"; it's a **core, v1 requirement**.
2.  **Entity Resolution is a Black Box.** You say you'll "resolve each mention to one or more canonical entities." This is a multi-million dollar problem space known as Entity Resolution and Disambiguation. You have no strategy. What is your URI scheme for entities? Who curates the canonical list? How do you handle entity drift over time (e.g., the definition of "public authority" changing)? You've replaced a simple dictionary with a massive, unsolved ontology management problem.
3.  **The Temporal Dimension is Ignored.** You list it as a scope dimension and then never mention it again. Commencement dates, sunset clauses, and transitional provisions are not edge cases; they are fundamental to compliance. Without them, you'll tell a customer a law applies when it hasn't come into force yet. This is a critical failure.
4.  **No Feedback Loop.** Your pipeline is a one-way firehose. What happens when an analyst corrects a bad extraction? There is no described mechanism to feed corrections back to retrain the NER/SLM models. Without a human-in-the-loop feedback mechanism, your models will stagnate and your data quality will degrade over time.
5.  **LRT vs. LAT Aggregation is Underspecified.** You correctly state that fitness is a law-level signal (LRT) aggregated from provision-level data (LAT). But how? Is the law-level fitness just a union of all provision facets? What about conflicting signals (a `DisappliesTo` in one provision and an `AppliesTo` in another)? You need a clear aggregation and conflict resolution strategy.

### Conclusion

You've graduated from a simplistic, brittle model to a conceptually sound but dangerously underspecified architecture. You've identified the right layers, but you're glossing over the hardest problems: graph-based scope propagation, entity governance, a real matching engine, and a data quality feedback loop.

This v0.3 document is a starting point for a conversation, not a blueprint for implementation. Flesh out the missing pieces—especially the propagation model and the matching engine—before writing a single line of new code. The current plan will lead to a system that is complex, expensive, and incorrect.
