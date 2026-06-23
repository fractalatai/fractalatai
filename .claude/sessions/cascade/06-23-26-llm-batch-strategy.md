# Session: LLM Batch Strategy (ACTIVE)

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

## Investigation results (2026-06-23)

### Section clustering is weak

Pending_llm provisions are mostly singletons within sections. HSWA: 34 pending in 26 sections (18 singletons, max cluster = 2). Section-level batching barely reduces call count — not worth the complexity.

### Corpus law sizes favour whole-law validation

| Size bucket | Laws | Provisions |
|---|---|---|
| ≤20 | 52 | 643 |
| 21-50 | 58 | 1,950 |
| 51-100 | 104 | 8,032 |
| 101-200 | 128 | 17,933 |
| 201-500 | 112 | 34,616 |
| 500+ | 78 | 98,714 |

**214 laws (≤100 provisions)** could get whole-law LLM validation. 10,625 provisions total.

### Token cost comparison

| Strategy | Laws ≤100 provs | Calls | Tokens | Cost (Flash) |
|---|---|---|---|---|
| Per-provision (pending only, ~4%) | 214 laws | ~425 | ~850K | $0.13 |
| **Whole-law validation** | 214 laws | **~214** | **~1.2M** | **$0.18** |

Whole-law costs **1.4x more tokens** but uses **half the API calls** and catches the 170 hard-floor errors that per-provision escalation can never reach. At $0.18 for 214 laws, this is negligible cost.

For 101-200 provision laws (128 laws): 1.3x token ratio, ~$0.27. Still negligible.

### Recommended strategy

**Tiered approach based on law size:**

1. **≤100 provisions** → whole-law validation (send all provisions + parse results, one call per law)
2. **101-500 provisions** → section-batch (group pending by parent section) OR quality-adaptive (whole-law if quality signal is low)
3. **500+ provisions** → per-provision only (too large for single call, even with 1M context)

The quality-adaptive threshold for medium laws: if >5% of provisions are pending_llm or low-confidence, escalate to whole-law.

## LLM Auditability Plan

### Current state: no audit trail

The existing LLM tier (Tier 2/3 in `enrich_single_law`) has **zero persistence** of the LLM interaction:

| What | Persisted? | Where |
|---|---|---|
| Prompt sent to LLM | No | Lost after call |
| Raw LLM response | No | Parsed then discarded |
| LLM's DRRP classification | Partial | Written to `drrp_types` (overwrites regex) |
| LLM's actor labels/positions | Yes | Written to `actors` column |
| LLM's reasoning per actor | Yes | Actor `reason` field |
| LLM's overall reasoning | No | Not captured |
| Which model/provider was used | Partial | `extraction_method` = "agentic"/"local" |
| Timestamp of LLM call | No | Only `taxa_classified_at` (set during regex, not updated) |
| Token usage / latency | No | Not captured |
| Whether LLM agreed or overrode regex | No | Previous regex answer overwritten |

For regulated customers, this is a gap. If an LLM reclassifies a provision from Obligation → Liberty, there's no record of what the regex/classifier said before, what prompt the LLM saw, or what reasoning the LLM gave for the change.

### What needs to be auditable

1. **Pre-LLM state**: What did regex + classifier produce? (Already in `drrp_history` — the new JSON format captures tier entries)
2. **LLM input**: What prompt was sent? What context was included?
3. **LLM output**: Raw response (before parsing), parsed classification, reasoning
4. **Decision**: Did the LLM override a previous classification? What was the delta?
5. **Metadata**: Model, provider, timestamp, token count, latency

### Proposed: `llm_audit_log` JSON file per law

During LLM processing (whole-law or per-provision), write one JSON file per law to a configurable directory:

```
data/llm-audit/UK_uksi_1999_3242.json
```

Each file contains an array of LLM interactions:

```json
{
  "law_name": "UK_uksi_1999_3242",
  "strategy": "whole_law",
  "model": "gemini-2.5-flash",
  "timestamp": "2026-06-23T14:30:00Z",
  "token_usage": { "input": 4200, "output": 1800 },
  "latency_ms": 2340,
  "provisions": [
    {
      "section_id": "UK_uksi_1999_3242:reg.4(1)",
      "pre_llm": {
        "drrp_types": ["Obligation"],
        "extraction_method": "regex",
        "confidence": 0.70,
        "actors": [{"label": "Org: Employer", "position": "active"}]
      },
      "llm_output": {
        "drrp_type": "Obligation",
        "actors": [
          {"label": "Org: Employer", "position": "ACTIVE", "reason": "employer bears the duty"}
        ]
      },
      "delta": "no_change",
      "prompt_excerpt": "Classify this provision..."
    }
  ],
  "prompt_template": "full prompt text here..."
}
```

### Integration with existing traceability

- `drrp_history` JSON already captures per-tier predictions — LLM adds an `"agentic"` entry
- `--trace` flag captures regex/classifier decision trail — LLM audit log is the complementary trace for Tier 3
- The `decision_trail` from `parse_v2_with_trail` shows the regex journey; the audit log shows the LLM journey
- Together they form a complete audit chain: regex → classifier → LLM, all traceable

### Implementation approach

1. Add `--audit-dir data/llm-audit` flag to the escalate command
2. Before each LLM call, snapshot the pre-LLM state from LanceDB
3. After each LLM call, write the audit entry (prompt, raw response, parsed result, delta)
4. For whole-law strategy: one audit file per law with all provisions
5. For per-provision strategy: same file structure, one entry per provision

This is lightweight — JSON files on disk, no schema changes, no LanceDB columns. Queryable with `jq` for compliance review.

## Prior sessions

- `06-22-26-llm-elevation-optimisation.md` (CLOSED) — 134 LLM calls, 37% FP rate, 170 hard-floor errors
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation, --trace flag
