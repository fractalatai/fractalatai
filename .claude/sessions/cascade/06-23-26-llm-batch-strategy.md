# Session: LLM Batch Strategy (PENDING)

## Problem

The current LLM escalation model sends one provision per call with full section context. When multiple provisions from the same section are flagged `pending_llm`, this means:

- The same section text is sent N times (once per provision)
- The LLM re-reads the same context repeatedly
- Token cost scales linearly with provision count, not section count

Across the 16 benchmark laws, 134 provisions are flagged for LLM. If these cluster within sections (likely — a section with ambiguous modal language will have multiple ambiguous provisions), the waste could be substantial.

## Investigation plan

1. **Profile clustering**: How many of the 134 pending_llm provisions share a parent section? What's the distribution — mostly singletons, or clusters of 3-5 from the same section?
2. **Profile law sizes**: Distribution of provision counts per law across the corpus. How many laws are "small" (< 50, < 100 provisions)?
3. **Section-level batching**: Could we send one LLM call per section with all its pending provisions? The LLM sees the full section once and returns DRRP for each provision.
4. **Whole-law validation**: For small laws, send all provisions + parse results. Cost vs accuracy trade-off.
5. **Quality-adaptive threshold**: Define a per-law quality score (% low-confidence, % disagreements, % pending_llm). Laws below quality threshold get whole-law LLM validation regardless of size.
6. **Prompt design**: Batch prompts need structured JSON output per provision. The LLM receives context (provision text + regex DRRP + classifier DRRP + confidence) and returns corrections.
7. **Cost modelling**: Estimate token cost for per-provision vs per-section vs per-law strategies across the full corpus.

## Strategy 2: Whole-law LLM validation for small regulations

Many regulations are small enough to send entirely to the LLM — not just the pending_llm provisions, but ALL provisions with their regex/classifier output as context. The LLM validates and corrects the full parse in one call.

**Why this helps with the 170 hard-floor errors**: These are provisions where regex and classifier confidently agree on the wrong answer. No disagreement signal exists to flag them. But an LLM seeing the full law — all provisions, their DRRP types, actors, and the overall legislative structure — can spot inconsistencies that per-provision analysis misses.

**Example**: Manual Handling Operations Regulations 1992 — ~30 provisions. One LLM call with all 30 provisions and their parse results. The LLM corrects 2-3 misclassifications that both tiers missed because they require understanding the regulation's overall structure.

**Adaptive size threshold**: The decision to send the whole law vs just pending provisions could be based on:
- **Law size**: provisions < N (e.g. 50-100) → whole-law validation
- **Tier 1/2 quality signal**: high proportion of low-confidence or disagreement provisions → whole-law validation even for larger laws
- **Cost ceiling**: total tokens for the law vs budget per law

**Trade-off**: Sending all provisions (including correct ones) costs more tokens but catches the 170 "both tiers wrong" errors that per-provision escalation can never reach.

## Design considerations

- Provisions within a section share legal context (defined terms, scope, subject matter)
- The hierarchy_path in LanceDB groups provisions by section
- Some sections have 1 pending provision, others might have 10+ — need adaptive batching
- The LLM tier already handles multi-actor provisions — extending to multi-provision batches is a natural fit
- Gemini context window (1M tokens) can handle entire laws, but cost scales with input tokens
- Small regulations (< ~100 provisions) fit easily in a single Gemini call with full parse context
- The `taxa_confidence` and `extraction_method` fields provide quality signals for the adaptive threshold

## Key files

- `fractalaw-cli/src/main.rs` — `enrich_single_law`, LLM escalation logic
- `fractalaw-store/src/lance.rs` — `query_legislation_text` (provisions with hierarchy_path)
- LanceDB schema: `hierarchy_path`, `section_id`, `law_name` for grouping

## Prior sessions

- `06-22-26-llm-elevation-optimisation.md` (CLOSED) — 134 LLM calls, 37% FP rate, 170 hard-floor errors
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation, --trace flag
