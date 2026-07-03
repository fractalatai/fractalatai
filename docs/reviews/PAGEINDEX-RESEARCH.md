# PageIndex Research — Applicability to Fractalaw DRRP Extraction

**Date**: 2026-06-05
**Source**: https://pageindex.ai/ | https://github.com/VectifyAI/PageIndex
**Context**: Fractalaw's DRRP extraction pipeline uses deterministic regex patterns that achieve good coverage on explicit duty language but fails on edge cases (Gap C: ~3,275 provisions, 78% of false negatives) where the duty holder is implicit, inherited from a parent clause, or requires cross-reference resolution.

## What PageIndex Is

PageIndex is a **vectorless, reasoning-based RAG framework** by Vectify AI (UK). MIT licensed, 32K+ GitHub stars. It transforms documents into hierarchical tree structures (JSON) and uses LLM agent reasoning to navigate them — replacing vector similarity search with structure-aware retrieval.

**Core thesis**: similarity does not equal relevance. Vector RAG finds semantically similar chunks but misses contextually appropriate information. PageIndex mimics how a human navigates a document: examine the structure, reason about which sections matter, drill into those pages.

### How It Works

**Phase 1 — Index Generation** (once per document):
- Detects or generates a document's structural hierarchy (TOC, sections, subsections)
- Produces a JSON tree where each node has: title, page range, summary, child nodes
- Handles documents with or without explicit TOCs via LLM-assisted structure detection
- Large documents processed in overlapping segments (~20K tokens each)

**Phase 2 — Agentic Retrieval** (per query):
- LLM agent given three tools: `get_document()`, `get_document_structure()`, `get_page_content(pages)`
- Agent reasons over the tree to identify relevant sections, fetches targeted page ranges
- Can make multiple retrieval passes — read a section, realise it needs another, go fetch it
- Fundamentally different from top-K vector retrieval

### Technical Details

- Uses LiteLLM for model abstraction (OpenAI, Anthropic, Deepseek, local models)
- Default model: gpt-4o; works with any capable LLM
- Input: PDF, Markdown
- Output: JSON tree index + retrieved text passages
- Self-hosted (open source) or cloud API
- Python SDK: `pip install pageindex`
- MCP server available for Claude Desktop integration

### Benchmarks

FinanceBench (financial document QA): **98.7% accuracy at 100% coverage** (vs 94% for next-best). No legal/regulatory benchmarks published, but financial filings share structural characteristics with legislation (long, hierarchical, cross-referential).

## Assessment for Fractalaw

### PageIndex solves a different problem

PageIndex is a **retrieval** framework — it finds the right content within a long document. Fractalaw's challenge is **structured extraction** from already-isolated provision text. The provisions are already in LanceDB as individual rows; retrieval is not the bottleneck.

However, PageIndex's approach has two properties that are directly relevant to the Gap C edge cases:

### Where PageIndex's approach IS relevant

**1. Cross-reference resolution (Gap C4)**

The most valuable insight from PageIndex is its **tree-based multi-step reasoning**. When a provision says "The person referred to in section 2(1) shall ensure...", the current regex pipeline fails because it can't look up section 2(1) to find the actor. PageIndex's agent pattern — reason about structure, fetch related content, reason again — maps directly to this problem.

We don't need PageIndex itself for this. But the pattern of giving an LLM:
- The target provision text
- A structural index of the parent Act
- A tool to fetch other provisions by citation

...is exactly the right architecture for resolving implicit actors. This is an **agentic extraction** pattern, not a retrieval pattern.

**2. Parent-clause actor inheritance (Gap C2)**

When "(2)(b) record the assessment" has no explicit actor, a human reads "(2) The employer shall — (a) assess... (b) record..." to inherit "employer" from the parent. The tree structure that PageIndex builds maps to legislative document structure. An LLM with the provision hierarchy could resolve this trivially.

**3. Whole-Act context for truly passive provisions (Gap C6)**

"Steps must be taken to ensure..." — who must take the steps? The answer requires understanding the Act's general duty framework. An LLM agent with access to the Act structure could reason: "This is in Part II of HSWA → Part II imposes duties on employers → the duty holder is the employer."

### Where PageIndex is NOT relevant

- **Explicit duty extraction** (Gap A/B) — regex already handles these well
- **Thing-subject passives** (Gap C1) — "Equipment must be maintained" — these are Rules, not Duties. Already addressed by the Rule type in the taxonomy
- **Generic pronominal subjects** (Gap C5) — "A person who..." — better addressed by extending the actor dictionary (which we've been doing)

### How this changes the Gap C architecture

The existing Gap C plan (from `04-15-26-gap-c-ai-research.md`) proposed a fine-tuned classifier (ModernBERT-large) deployed as a serverless GPU endpoint. That approach treats each provision as an independent classification task.

PageIndex's contribution is the insight that **Gap C is not a classification problem — it's a reasoning problem that requires document context**. A fine-tuned classifier cannot look up section 2(1) to find the actor. An agentic approach can.

**Proposed hybrid architecture:**

```
Provision text (from LanceDB)
  │
  ├─ Regex pipeline (existing) → explicit DRRP
  │
  └─ Gap C: no DRRP extracted
       │
       ├─ C1 (thing-subject): Rule type ← already handled
       ├─ C5 (pronominal): extend actor dictionary ← ongoing
       │
       └─ C2/C4/C6 (context-dependent): agentic extraction
            │
            ├─ Build structural context:
            │   - Parent provision chain (from hierarchy_path)
            │   - Cross-referenced provisions (from citation parsing)
            │   - Act-level general duty section
            │
            └─ LLM call with structured prompt:
                "Given this provision and its context,
                 who holds the duty/right/responsibility/power?
                 Cite the source of your inference."
                 → holder + holder_inferred_from
```

This is cheaper and more accurate than fine-tuning a classifier because:
1. It leverages existing structural data (hierarchy_path, section_id, sort_key already in LanceDB)
2. It only fires for Gap C provisions (~3K out of ~97K), not the full corpus
3. The LLM can explain its reasoning (audit trail via `holder_inferred_from`)
4. No training data collection, labelling, or model training needed
5. Works immediately with Claude or any capable LLM via API

### Cost estimate

Gap C = ~3,275 provisions. Each needs: target provision (~200 tokens) + parent chain (~500 tokens) + cross-refs (~500 tokens) + prompt (~200 tokens) = ~1,400 input tokens. Output ~100 tokens.

At Claude Sonnet rates ($3/M input, $15/M output):
- Input: 3,275 × 1,400 = 4.6M tokens → $13.80
- Output: 3,275 × 100 = 328K tokens → $4.92
- **Total: ~$19 for the entire corpus** (one-time catchup)
- Ongoing: tens of new provisions per month → pennies

### What we already have that enables this

1. **Provision text** — in LanceDB with section_id, hierarchy_path, sort_key
2. **Structural hierarchy** — hierarchy_path encodes the document tree
3. **Cross-reference detection** — CROSS_REF_RE in fitness.rs already parses "regulation N, section N, article N" citations
4. **Purpose classification** — we know which provisions are duties vs scope vs interpretation
5. **Actor dictionaries** — the LLM output can be validated against known actor labels

### What we'd need to build

1. **Context assembler** — given a Gap C provision, fetch its parent chain and cross-referenced provisions from LanceDB
2. **Structured prompt** — template that presents the provision + context and asks for holder identification with citation
3. **Response parser** — extract holder label + inferred_from citation from LLM response, validate against actor dictionary
4. **Integration point** — call this after regex pipeline returns empty DRRP, write results with `holder_inferred_from` provenance

## Recommendation

**Don't adopt PageIndex as a dependency.** Its retrieval framework solves a problem we don't have (our provisions are already isolated). But adopt its **architectural insight**: tree-structured agentic reasoning over documents, not similarity search or independent classification.

For Gap C, build a lightweight agentic extraction step that:
1. Assembles structural context from existing LanceDB data
2. Calls an LLM API (Claude Sonnet) with a structured prompt
3. Returns holder + provenance citation
4. Fires only for provisions where regex extraction returned empty

This is simpler, cheaper, and more auditable than the fine-tuned classifier approach previously planned. It also extends naturally to EU law (where cross-reference patterns are different) without retraining.

## References

- PageIndex: https://pageindex.ai/ | https://github.com/VectifyAI/PageIndex (MIT, 32K stars)
- Gap C analysis: `.claude/sessions/taxa-drrp/04-15-26-gap-c-ai-research.md`
- Gap C orchestration: `.claude/sessions/taxa-drrp/04-15-26-gap-c-orchestration.md`
- Gap C sub-types: `.claude/sessions/taxa-drrp/04-14-26-ohs-occupational-safety.md`
