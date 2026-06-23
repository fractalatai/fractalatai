# Session: LLM Batch Strategy (PENDING)

## Problem

The current LLM escalation model sends one provision per call with full section context. When multiple provisions from the same section are flagged `pending_llm`, this means:

- The same section text is sent N times (once per provision)
- The LLM re-reads the same context repeatedly
- Token cost scales linearly with provision count, not section count

Across the 16 benchmark laws, 134 provisions are flagged for LLM. If these cluster within sections (likely — a section with ambiguous modal language will have multiple ambiguous provisions), the waste could be substantial.

## Investigation plan

1. **Profile clustering**: How many of the 134 pending_llm provisions share a parent section? What's the distribution — mostly singletons, or clusters of 3-5 from the same section?
2. **Section-level batching**: Could we send one LLM call per section with all its pending provisions, asking the LLM to classify them together? The LLM sees the full section once and returns DRRP for each provision.
3. **Context windowing**: What's the optimal context? Full section? Parent + siblings? The provision alone is sometimes insufficient (cross-references, defined terms from parent clauses).
4. **Prompt design**: A batch prompt ("classify these 4 provisions from section 29") is different from a single-provision prompt. Need structured JSON output per provision.
5. **Cost modelling**: Estimate token cost for per-provision vs per-section vs per-law batching strategies across the full corpus.

## Design considerations

- Provisions within a section share legal context (defined terms, scope, subject matter)
- The hierarchy_path in LanceDB groups provisions by section
- Some sections have 1 pending provision, others might have 10+ — need adaptive batching
- The LLM tier already handles multi-actor provisions — extending to multi-provision batches is a natural fit
- Gemini context window (1M tokens) can handle entire laws, but cost scales with input tokens

## Key files

- `fractalaw-cli/src/main.rs` — `enrich_single_law`, LLM escalation logic
- `fractalaw-store/src/lance.rs` — `query_legislation_text` (provisions with hierarchy_path)
- LanceDB schema: `hierarchy_path`, `section_id`, `law_name` for grouping

## Prior sessions

- `06-22-26-llm-elevation-optimisation.md` (CLOSED) — 134 LLM calls, 37% FP rate, 170 hard-floor errors
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation, --trace flag
