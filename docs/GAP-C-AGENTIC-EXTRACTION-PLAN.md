# Gap C Agentic Extraction — Design Document

**Status**: Draft v0.4 — incorporates Gemini + ChatGPT reviews (two rounds each)
**Date**: 2026-06-05
**Supersedes**: The fine-tuned classifier approach in `.claude/sessions/taxa-drrp/04-15-26-gap-c-ai-research.md` §4 (ModernBERT-large backbone, training repo, serverless GPU deployment)
**Informed by**: PageIndex research (`docs/PAGEINDEX-RESEARCH.md`) — structured document navigation as a pattern for context-dependent reasoning

## Problem Statement

The DRRP extraction pipeline uses deterministic regex patterns. It works well for provisions with explicit actors and modal verbs (Gaps A/B, ~60-70% coverage on typical UK OH&S laws). It fails on **Gap C: provisions where the duty holder is implicit** — inherited from a parent clause, defined in a cross-referenced section, or absent entirely. Gap C accounts for **~3,275 provisions (78% of all false negatives)** in the OHS corpus.

The previous plan proposed fine-tuning a classifier (ModernBERT-large) to detect duties in Gap C provisions. This won't work because **the information needed to resolve the duty holder is not in the provision text** — it's in surrounding provisions. A classifier seeing "record the assessment" in isolation cannot know the duty holder is "employer" from the parent clause.

## Core Insight

Gap C is a **reasoning problem with document context**, not a classification problem. The provisions aren't ambiguous — a human reading them in context always knows who holds the duty. The information exists in LanceDB; it just needs to be assembled and presented to an LLM.

## Gap C Sub-types and Resolution Strategy

| Sub-type | Example | Count | Strategy |
|----------|---------|-------|----------|
| C1. Thing-subject passive | "Equipment must be maintained" | ~42 | **Already handled** — Rule type in taxonomy |
| C2. Parent-clause inheritance | s.2(2)(b): "arrangements for ensuring..." (employer from s.2(1)) | ~1,500 | **Context assembly** — feed parent chain |
| C3. Delayed/inverted subject | "It shall be the duty of every employer to..." | — | **Already handled** — actors.rs patterns |
| C4. Cross-section actor | s.5 refers to "the duty-holder" defined in s.2 | ~800 | **Context assembly** — fetch cited provision |
| C5. Generic pronominal | "A person who designs..." | Low | **Actor dictionary** — extend GOVERNED_DEFS (ongoing) |
| C6. Truly actor-less passive | "Steps must be taken" | ~900 | **Context assembly** — Act-level general duty inference |

**C2 + C4 + C6 are the targets.** Together they represent ~3,200 provisions. C1, C3, C5 are handled by existing regex improvements.

Critical insight from ChatGPT review: **these are not one problem**. C2 is mostly deterministic (parent has actor, child inherits it). C4 needs citation infrastructure. C6 genuinely needs LLM reasoning. Track and measure them separately from day one.

Expected resolution rates:
- **C2**: high success (~80-90%) — structural inheritance, largely deterministic
- **C4**: medium success (~60-70%) — depends on citation resolver quality
- **C6**: lower success (~40-50%) — genuinely ambiguous, may have multiple valid actors

## Architecture

Four-tier resolution pipeline. Each tier only fires if the previous one failed. Most provisions resolve early; the LLM is reserved for the genuinely ambiguous minority.

```
Provision arrives from LanceDB (during taxa enrich)
  │
  ├─ Tier 0: Regex pipeline (parse_v2)
  │   → DRRP extracted? → YES → done
  │
  └─ NO DRRP (Gap C candidate)
       │
       ├─ Purpose gate: is this a duty-bearing provision?
       │   (skip Interpretation, Amendment, Enactment, etc.)
       │
       ├─ Context assembly (shared across Tiers 1-3)
       │   Fetch once per law, cache for all Gap C provisions:
       │   - Parent chain (hierarchy_path prefix query)
       │   - Interpretation section (always, for citation resolution)
       │   - Act general duty (first substantive section)
       │   Per-provision additions:
       │   - Cross-referenced provisions (citation-specific)
       │
       ├─ Tier 1: Deterministic parent inheritance (C2)
       │   Walk UP the hierarchy from target, DEEPEST-FIRST.
       │   Stop at the NEAREST ancestor that yields an actor
       │   via extract_actors(). Do NOT walk to root — an
       │   intermediate child may override the root actor.
       │   Example: s.12 (employer) → s.12(3) (tenant) → s.12(3)(a)
       │   Target s.12(3)(a) inherits "tenant" from s.12(3),
       │   NOT "employer" from s.12.
       │   → Resolved? → YES → done (extraction_method: "inherited")
       │
       ├─ Tier 2: Deterministic cross-reference resolution (C4)
       │   IF target text contains a citation (CROSS_REF_RE)
       │   AND cited provision contains an explicit actor
       │   THEN propagate cited actor.
       │   ONE-HOP RECURSION: if cited provision contains only
       │   another citation (no actor), follow exactly once.
       │   If still unresolved after two hops, pass the entire
       │   trace to Tier 3 as context (the LLM gets the chain).
       │   → Resolved? → YES → done (extraction_method: "cross_ref")
       │
       └─ Tier 3: LLM reasoning (C4-complex, C6)
            Only provisions that Tiers 1-2 could not resolve.
            Input: target text + assembled context (structural tags)
            + valid actor enum from actors.rs
            Output: JSON { holder, drrp_type, evidence_sections,
                           reasoning_type }
            Confidence derived externally (see below), not self-reported.
            → Resolved? → YES → done (extraction_method: "agentic")
            → NO → stays unenriched (same as today)
```

### Why this ordering matters

- **Tier 1 is cheap and reliable.** No API calls, deterministic, auditable. Run `extract_actors()` on the parent text — if it finds an actor, inherit it. This probably resolves 50-70% of Gap C on its own.
- **Tier 2 is cheap but needs infrastructure.** Citation resolver is engineering work, but the resolution itself is deterministic once citations are mapped.
- **Tier 3 is the expensive fallback.** Reserved for provisions where no deterministic path exists. Expected to handle ~20-30% of Gap C provisions.

### Conflict resolution (when tiers disagree)

Tiers 1 and 2 may identify different actors for the same provision. Example: parent chain says "employer" (Tier 1) but a cross-reference points to "designer" (Tier 2). This will happen in CDM, PUWER, LOLER, and REACH.

**Precedence policy** (highest to lowest):

| Priority | Source | Rationale |
|----------|--------|-----------|
| 1 | Explicit actor in target provision text | Direct statement overrides all inference |
| 2 | Cross-reference actor (Tier 2) | Specific citation is more targeted than structural inheritance |
| 3 | Nearest-parent inheritance (Tier 1) | Structural default |
| 4 | General-duty inference (Tier 3) | Weakest signal |

When conflicting candidates exist, record **all** candidates in a `conflicting_actors` field for QA. The highest-priority actor wins for the primary `holder` field, but the alternatives are preserved for audit and review. Silent actor drift from unrecorded conflicts is hard to diagnose.
```

## Context Assembly — The Hard Part

This is where the real work is. The LLM call itself is straightforward — the quality depends entirely on what context we assemble.

### 1. Parent Chain (for C2)

**Data available**: Every provision has `hierarchy_path` (e.g., `part.I/heading.2/provision.2/sub.2/para.a`) and `law_name`.

**Algorithm**:
```
target: UK_ukpga_1974_37:s.2(2)(a)
  hierarchy_path: part.I/heading.2/provision.2/sub.2/para.a

Walk up the hierarchy:
  1. part.I/heading.2/provision.2/sub.2  → s.2(2) text
  2. part.I/heading.2/provision.2        → s.2 text (heading only)
  3. part.I/heading.2                    → heading text
  4. part.I                              → part title
```

**LanceDB query**: filter by `law_name` + `hierarchy_path` prefix match, select `section_id`, `text`, `hierarchy_path`, order by `depth` descending. This is a single query per provision.

**Example context for s.2(2)(b)**:
```
PARENT s.2(1): "It shall be the duty of every employer to ensure, so
far as is reasonably practicable, the health, safety and welfare at
work of all his employees."

PARENT s.2(2): "Without prejudice to the generality of an employer's
duty under the preceding subsection, the matters to which that duty
extends include in particular—"

TARGET s.2(2)(b): "arrangements for ensuring, so far as is reasonably
practicable, safety and absence of risks to health in connection with
the use, handling, storage and transport of articles and substances;"
```

Given this context, any LLM will correctly identify "employer" as the duty holder.

### 2. Cross-Referenced Provisions (for C4)

**Data available**: Citation patterns already parsed by `CROSS_REF_RE` in fitness.rs. Section_ids in LanceDB use a consistent citation format.

**Algorithm**:
```
target text: "The duty-holder shall comply with section 2(1)."
  1. Extract citations: ["section 2(1)"]
  2. Resolve to section_id: "UK_ukpga_1974_37:s.2(1)"
  3. Fetch text from LanceDB
```

**Challenge**: Citation resolution. "section 2(1)" needs to become `s.2(1)` in the section_id namespace. The mapping is:
- "section N" → `s.N`
- "regulation N" → `reg.N`
- "article N" / "Article N" → `art.Article N`
- "paragraph N" → `sub.N` or `para.N` (context-dependent)

This mapping already partially exists in the sort_key normalisation logic. Needs a dedicated resolver function.

### 3. Act General Duty (for C6)

**Data available**: First few provisions of each law are in LanceDB, sorted by `sort_key`.

**Algorithm**: For truly passive provisions ("Steps must be taken"), fetch:
1. The Act title (section_type = 'title')
2. The first substantive section (section_type = 'section', position ≤ 5)
3. Any provision already enriched with DRRP for this law (from the regex pass)

This gives the LLM enough context to infer: "This is Part I of HSWA 1974, which imposes duties on employers → the implicit duty holder is the employer."

## LLM Prompt Design

```
You are a legal analyst identifying duty holders in UK and EU legislation.

Given a provision that imposes a duty but does not explicitly name the
duty holder, identify WHO holds the duty based on the surrounding context.

## Context
{assembled_parent_chain}
{assembled_cross_refs}
{act_general_duty}

## Target Provision
{provision_text}
Section: {section_id}

## Task
1. Who holds the duty in the target provision?
2. What type of DRRP is it? (Duty / Right / Responsibility / Power)
3. Where did you infer the holder from? (cite the specific section)
4. How confident are you? (0.0-1.0)

Respond in JSON:
{
  "holder": "employer",
  "drrp_type": "Duty",
  "holder_inferred_from": "s.2(1)",
  "confidence": 0.95,
  "reasoning": "s.2(1) establishes the general duty of every employer;
                s.2(2) lists matters to which that duty extends;
                paragraph (b) is one such matter."
}

If the provision does not create a duty/right/responsibility/power,
respond: { "drrp_type": null, "reasoning": "..." }
```

## Validation and Confidence

### Tier 1-2 (deterministic): confidence derived from signal strength

Confidence is computed by the pipeline, not self-reported:

| Signal | Confidence |
|--------|-----------|
| Parent contains explicit actor + actor dict match + no conflicting actors | 0.95 |
| Parent contains actor + actor dict match + conflicting actors in siblings | 0.75 |
| Cross-ref resolves to provision with explicit actor | 0.90 |
| Cross-ref resolves but actor is generic ("Ind: Person") | 0.70 |

### Tier 3 (LLM): confidence derived from evidence quality

Do **not** ask the LLM for a confidence score. Instead, require structured evidence:

```json
{
  "holder": "Org: Employer",
  "drrp_type": "Duty",
  "evidence_sections": ["s.2(1)", "s.2(2)"],
  "reasoning_type": "parent_inheritance"
}
```

Then derive confidence externally:

| Reasoning type | Evidence quality | Derived confidence |
|---------------|-----------------|-------------------|
| parent_inheritance | evidence_sections all exist + actor in dict | 0.90 |
| cross_reference | cited section exists + actor in dict | 0.85 |
| general_duty_inference | general duty section cited + actor in dict | 0.70 |
| delegated_eu_duty | both government + governed actors identified | 0.75 |
| novel reasoning | any evidence_section missing or actor not in dict | 0.50 (flag for review) |

### Actor label validation

All tiers: holder label must match an entry in `actors.rs`. For Tier 3, the valid actor enum is injected into the prompt (see R4 in review section). Novel actors return `"novel_actor"` with a `"suggested_label"` — flagged for dictionary review, not silently written.

### Failure mode

Failed validation → provision stays unenriched (same as today). Each tier is an enhancement, not a replacement. The regex pipeline remains the primary extraction path.

## Integration Point

In `enrich_single_law()` (main.rs ~line 2850), after the regex `parse_v2()` loop:

```rust
// After regex enrichment, collect Gap C provisions
let gap_c: Vec<_> = provisions.iter()
    .filter(|p| p.drrp_types.is_empty() && is_duty_bearing_purpose(&p.purposes))
    .collect();

if !gap_c.is_empty() {
    let resolved = agentic_extract(&lance, law_name, &gap_c).await?;
    // Merge resolved entries into the provision taxa batch
}
```

The agentic extraction is:
1. **Batched per law** — context assembly queries LanceDB once for the whole law's hierarchy, not per provision
2. **Concurrent LLM calls** — provisions within a law are independent once context is assembled
3. **Rate-limited** — respect API rate limits, retry with backoff
4. **Optional** — gated behind a feature flag or `--ai` CLI flag initially

## Cost Model

### Tiered cost structure

The key insight: most Gap C provisions don't need LLM calls at all.

| Tier | Cost per provision | Expected % of Gap C | Provisions | Cost |
|------|-------------------|---------------------|------------|------|
| Tier 1 (deterministic inheritance) | ~$0 (LanceDB query only) | 50-70% | ~1,600-2,300 | $0 |
| Tier 2 (deterministic cross-ref) | ~$0 (LanceDB query only) | 10-20% | ~300-650 | $0 |
| Tier 3 (LLM reasoning) | ~$0.006/provision | 20-30% | ~650-1,000 | $4-6 |

### Tier 3 token budget (tiered context expansion per ChatGPT review)

Start minimal, expand only if unresolved:

| Level | Context | Tokens | When to use |
|-------|---------|--------|-------------|
| Level 1 | Parent chain only | ~400 | First attempt |
| Level 2 | + cited provisions | ~800 | If Level 1 returns low-confidence |
| Level 3 | + Interpretation section | ~1,200 | If citations reference defined terms |
| Level 4 | + Act general duty | ~1,500 | Truly passive (C6) provisions |

### Corpus-level cost (Claude Sonnet: $3/M input, $15/M output)

| Scope | LLM provisions | Estimated cost |
|-------|---------------|---------------|
| UK corpus one-time (Tier 3 only) | ~650-1,000 | **$4-6** |
| EU corpus one-time (Tier 3 only) | ~400-600 | **$2-4** |
| Development iterations (5x) | ~5,000-8,000 | **$30-50** |
| Monthly ongoing | ~10-30 | **$0.06-0.18** |

### Comparison to previous approaches

| | Fine-tuned classifier | Full LLM (v0.1 plan) | Tiered (v0.3) |
|---|---|---|---|
| Upfront cost | GPU + labelling (weeks) | ~$29 | ~$6-10 + $30-50 dev |
| Ongoing cost | ~$0.01/provision | ~$0.006/provision | ~$0.001/provision (blended) |
| Can resolve cross-refs | No | Yes | Yes |
| Deterministic coverage | No | No | ~70-80% (Tiers 1-2) |
| Audit trail | Confidence score | Reasoning text | evidence_sections + reasoning_type |
| Time to first result | Weeks | Days | Days (Tier 1 is hours) |

## Data Flow

```
LanceDB (provisions with hierarchy_path, sort_key, text)
  │
  ├─ Regex enrichment (existing) → writes taxa columns
  │
  └─ Gap C provisions (no DRRP after regex)
       │
       ├─ Context assembly (reads from LanceDB)
       │   ├─ Parent chain query
       │   ├─ Cross-ref resolution + fetch
       │   └─ Act general duty fetch
       │
       ├─ LLM call (Claude Sonnet API)
       │   └─ Structured JSON response
       │
       ├─ Validation (holder against actor dict, section_id exists)
       │
       └─ Write to LanceDB
            ├─ drrp_types, governed_actors (same columns as regex)
            ├─ holder_inferred_from (new column — provenance)
            └─ taxa_confidence (from LLM)
```

## Implementation Phases

Narrow phasing per ChatGPT review — prove each tier independently before combining.

### Phase 1A: Deterministic parent inheritance (Tier 1 only)
- Build parent-chain resolver (hierarchy_path prefix query on LanceDB)
- Run `extract_actors()` on parent text; propagate if child has no actor
- Structural tag wrapping for provenance
- Schema columns: `holder_inferred_from`, `extraction_method`, `reasoning_type`
- Test on HSWA s.2(2)(a)-(e) — should deterministically resolve "Org: Employer"
- **Measure**: precision, recall, cost on 100 provisions across 5 laws
- **Exit criterion**: >85% precision on C2 provisions. Quantify what % of Gap C this resolves.
- **Expected outcome**: 50-70% of Gap C resolved without any LLM calls

### Milestone 1: Deterministic baseline assessment
- **Gate**: Before starting Phase 1B, verify the C2 deterministic rate.
- If C2 deterministic resolution >55% of Gap C → freeze Tier 3 budget at Level 2 max context. Don't invest in deep Level 4 prompts until they're proven necessary.
- If C2 deterministic resolution <40% → revisit assumptions, may need broader Tier 3 scope.
- Publish metrics: `inherited_count` / total Gap C, precision, false positive examples.

### Phase 1B: Cross-reference resolver (Tier 2)
- Citation resolver: "section N" → section_id within same law (intra-document only)
- Always fetch Interpretation section as context for "the Act" definitions
- Run `extract_actors()` on cited provision; propagate if found
- Test on CDM, HSWA cross-section references
- **Exit criterion**: >80% precision on C4 provisions with intra-document citations
- **Deferred**: cross-document citations ("section 2 of the Act") to Phase 2B

### Phase 1C: LLM reasoning (Tier 3 — proof of concept)
- Anthropic API client (Claude Sonnet)
- Structured prompt with: structural tags, valid actor enum, evidence_sections requirement
- Externally-derived confidence (not self-reported)
- Test on 50 hand-picked C6 provisions that Tiers 1-2 could not resolve
- **Measure**: precision, cost per provision, reasoning_type distribution
- **Exit criterion**: >70% precision on C6, understood failure modes

### Phase 2A: Single-law integration
- Wire Tiers 1-3 into `enrich_single_law()` after regex pass
- `--gap-c` CLI flag (opt-in)
- Full enrichment of HSWA with all tiers — QA report showing improvement
- Tier-level metrics: `inherited_count`, `cross_ref_count`, `agentic_count`
- **Exit criterion**: HSWA DRRP rate improves by >15 percentage points

### Phase 2B: Cross-document citations + EU support
- Cross-document citation resolver (Interpretation section → external law mapping)
- EU Directive dual extraction policy (R5: Member State responsibility + delegated duty)
- Test on REACH cross-references and Framework Directive delegated duties
- **Exit criterion**: EU corpus Gap C provisions resolved at comparable rates to UK

### Phase 3: Corpus-wide rollout
- Process all remaining Gap C provisions (~3,275 UK + ~2,000 EU)
- Publish enriched provisions to sertantai
- Tune confidence thresholds based on validation results
- Promote `--gap-c` to default (with `--no-gap-c` opt-out)
- **Exit criterion**: corpus-wide QA report, false-positive rate < 5%

### Phase 4: Automation
- Wire into `sync watch` pipeline (after regex enrichment, before publish)
- Rate limiting and error handling for API calls
- Cost monitoring and alerting
- Tiered context expansion (minimal context first, expand if unresolved)
- **Exit criterion**: new laws automatically get Gap C resolution

## Schema Changes

### LanceDB `legislation_text` (new columns)

| Column | Type | Purpose |
|--------|------|---------|
| `holder_inferred_from` | List<Utf8> | Section_ids the holder was inferred from (may be multiple) |
| `extraction_method` | Utf8 | "regex", "inherited", "cross_ref", or "agentic" |
| `reasoning_type` | Utf8 | "parent_inheritance", "cross_reference", "interpretation_lookup", "general_duty_inference", "delegated_eu_duty", "unresolved_external_reference" |
| `obligation_layer` | Utf8 | "primary", "delegated", "transposed" — for EU dual extraction |
| `conflicting_actors` | List<Utf8> | Alternative actor candidates when tiers disagree (QA/audit) |
| `ancestor_distance` | Int32 | Hierarchy hops from target to resolved actor (0=self, 1=parent, 2=grandparent) — for validating inheritance assumptions |

### DuckDB `legislation` (new columns)

| Column | Type | Purpose |
|--------|------|---------|
| `inherited_count` | Int32 | Provisions resolved by Tier 1 (deterministic inheritance) |
| `cross_ref_count` | Int32 | Provisions resolved by Tier 2 (cross-reference) |
| `agentic_count` | Int32 | Provisions resolved by Tier 3 (LLM reasoning) |

## Review Feedback (Gemini, 2026-06-05)

External review identified five substantive risks. Incorporated below with resolutions.

### R1. Structural context wrapping — don't concatenate raw text

**Risk**: Flat text concatenation of the parent chain loses structural semantics. The LLM sees "General duties of employers..." followed by sub-clause text but can't distinguish heading from operative text.

**Resolution**: Wrap context fragments in structural tags in the prompt:

```
<parent type="section" id="s.2(1)">
It shall be the duty of every employer to ensure...
</parent>

<parent type="subsection" id="s.2(2)">
Without prejudice to the generality of an employer's duty...
</parent>

<target type="paragraph" id="s.2(2)(b)">
arrangements for ensuring, so far as is reasonably practicable...
</target>
```

This is cheap (a few extra tokens) and gives the LLM unambiguous structural hierarchy. Added to Phase 1 as a requirement.

### R2. Cross-document citation resolution

**Risk**: "The employer must maintain records in accordance with section 2 of the Act" — "the Act" refers to a different law. The resolver will wrongly map to section 2 of the current Regulations.

**Resolution**: The citation resolver needs two modes:
1. **Intra-document** (default): bare "section N" → current law
2. **Cross-document**: "section N of the Act" / "section N of [Law Title]" → resolve via Interpretation section definitions

The Interpretation sections (purpose = `Interpretation+Definition`) often define "the Act" → specific law name. The context assembler should **always fetch the Interpretation section** for the current law as part of C4 resolution, even though it's gated from DRRP extraction. This directly addresses the "Purpose Gate Leak" point — Interpretation sections are excluded from DRRP extraction but are **critical context for C4 resolution**.

This is the hardest engineering problem in the whole plan. Phase 1 should handle intra-document only; cross-document resolution is Phase 2 scope.

### R3. Context fan-out and cost multiplication

**Risk**: A provision citing "sections 2 to 4" or "Part I of this Act" could pull in thousands of tokens. Validation iterations multiply the one-time cost.

**Resolution**:
- **Token budget cap**: Hard limit of 3,000 input tokens per provision. If context assembly exceeds this, truncate cross-references (keep parent chain, which is highest-value).
- **Caching**: Cache context assembly results per law (the parent chain and general duty are shared across all Gap C provisions in the same law). Only cross-references vary per provision.
- **Iteration cost**: Budget 5x the one-time cost for validation iterations during Phase 2/3 development. That's $90 for UK + EU, not $18. Still cheap.

Updated cost model:

| Scenario | Provisions | Estimated cost |
|----------|-----------|---------------|
| Development (5 iterations) | ~5,275 × 5 | ~$90 |
| Production one-time | ~5,275 | ~$29 |
| Monthly ongoing | ~50-100 | ~$0.56 |

### R4. Taxonomy injection — constrain LLM output to valid actors

**Risk**: LLM outputs "employer" but actors.rs uses "Org: Employer". String mismatch wastes the extraction.

**Resolution**: Inject the valid actor enum into the prompt. Build the list dynamically from `actors.rs` labels for the current law's family:

```
## Valid duty holders (you MUST pick from this list or return "novel_actor"):
Governed: Org: Employer, Ind: Employee, Ind: Worker, SC: Manufacturer, ...
Government: Gvt: Minister, Gvt: Agency: Health and Safety Executive, ...
```

The actor list is already available via `extract_actors_for_family()` — the canonical labels are the output. This eliminates the mapping problem entirely. The LLM returns exact label strings; validation is a set membership check.

For novel actors: return `"novel_actor"` with a `"suggested_label"` field. These get flagged for review and potential dictionary expansion — a feedback loop that improves the regex pipeline too.

### R5. EU Directive sovereignty — dual duty holders

**Risk**: "Member States shall ensure that employers provide..." — the text creates a government responsibility (Member States) AND a delegated duty (employers). Treating EU Directives like UK Acts produces useless results.

**Resolution**: This is a **policy decision**, not an engineering problem. Two options:

**Option A — Dual extraction**: Extract both layers as **two separate records** (not one row with two actors — per Gemini round 2 review). Each row has exactly one actor paired with one DRRP type:

Record 1:
- `drrp_type`: Responsibility
- `holder`: EU: Member State
- `extraction_method`: "agentic"
- `reasoning_type`: "delegated_eu_duty"
- `obligation_layer`: "primary"

Record 2:
- `drrp_type`: Duty
- `holder`: Org: Employer
- `extraction_method`: "agentic"
- `reasoning_type`: "delegated_eu_duty"
- `obligation_layer`: "delegated"
- `holder_inferred_from`: ["art.Article 5(1)"]

The `obligation_layer` field ("primary" / "delegated" / "transposed") lets downstream consumers filter: compliance professionals want the delegated duty on the employer, legal analysts want the full chain. Without this field, dual extraction creates confusion rather than clarity — particularly for Framework Directive 89/391, REACH, and the Construction Directives where delegation chains are common.

**Option B — Skip government layer**: For retained EU Directives in UK law, Member State obligations are moot (already transposed). Only extract the governed-actor duty.

**Recommendation**: Option A for data completeness, but flag the government layer as `drrp_type: Responsibility` (not Duty). This matches the existing taxonomy: Member States don't "hold duties" in the DRRP sense — they have responsibilities to ensure others comply. The compliance-relevant extraction is the delegated duty on the employer/manufacturer/operator.

Add to Phase 1 prompt: "For EU Directives where 'Member States shall ensure that [actor]...', extract BOTH the Member State responsibility AND the delegated duty on the named actor."

## Open Questions

### Resolved by reviews

- ~~LLM self-reported confidence~~ → Externally derived from evidence quality (ChatGPT R6)
- ~~Batch vs streaming~~ → Per-provision with shared cached context per law (ChatGPT)
- ~~Taxonomy string matching~~ → Inject valid actor enum into prompt (Gemini R4)

### Resolved by round 2 reviews

- ~~Which LLM for Tier 3~~ → Tie to context level, not dynamic routing. Haiku for Level 1-2 (<800 tokens), Sonnet for Level 3-4 (≥800 tokens). Simple, no orchestration complexity. (Gemini round 2)
- ~~Range citations~~ → Expand ranges into individual targets before querying LanceDB ("sections 2 to 4" → `["s.2", "s.3", "s.4"]`). Sort by proximity to target provision. Token budget cap bounds the total. (Gemini round 2)
- ~~Tier 1 root-walking risk~~ → Deepest-first, stop at nearest ancestor with actor. (Gemini round 2)
- ~~Tier 2 circular references~~ → One-hop recursive resolution, then bail to Tier 3. (Gemini round 2)
- ~~EU dual extraction storage~~ → Two separate records per provision, not one row with two actors. (Gemini round 2)

### Resolved by ChatGPT round 2

- ~~EU Directive extraction policy~~ → **Model A (dual extraction) approved** with `obligation_layer` field ("primary"/"delegated"/"transposed") to distinguish the layers. Two separate records per provision. Sign-off before Phase 2B still needed on the specific layer taxonomy.
- ~~Cross-document citation fallback~~ → Don't fall back to C6. Instead use `reasoning_type: "unresolved_external_reference"` and send to Tier 3 with the unresolved citation visible. The LLM can often infer the actor from surrounding context even when the resolver fails. Preserves information rather than forcing a potentially wrong general-duty inference.
- ~~Provenance structure~~ → Keep `holder_inferred_from: List<Utf8>` for Phase 1. Design knowing it will probably become `List<Struct>` later. Added `ancestor_distance` and `conflicting_actors` columns to capture the metrics ChatGPT identified as critical.

### Still open

1. **Cross-document citation resolution**: Split into three complexity classes (ChatGPT):
   - Intra-document ("section 2") → Phase 1B
   - Explicit external ("section 2 of the Health and Safety at Work etc. Act 1974") → Phase 2B
   - Implicit external ("section 2 of the Act") → Phase 3+ (rabbit hole: amendment chains, retained EU legislation, definitions that change by Part)

   Measure `external citation detected / resolved / unresolved` before investing further. May discover only 2-3% of Gap C depends on external citations.

2. **What % of C2 is actually deterministic?** Phase 1A will answer this empirically. ChatGPT predicts 70-85% (higher than our 50-70% estimate). The key metric is **precision, not recall** — 55% recall at 95% precision is better than 80%/80%. Track `ancestor_distance` to validate that most resolutions come from distance 1-2.

3. **Precision vs recall threshold for Tier 1**: What false-positive rate is acceptable for deterministic inheritance? A parent actor propagated to the wrong child pollutes the taxonomy permanently. Need to define the QA sampling methodology in Phase 1A exit criteria.

## Dependencies

- Anthropic API key (or other LLM provider)
- `holder_inferred_from` column added to LanceDB schema (additive, safe)
- `extraction_method` column added to LanceDB schema (additive, safe)
- Citation resolver function (new code in fractalaw-core or fractalaw-cli)
- Structural tag wrapping in prompt template
- Dynamic actor enum injection from actors.rs

## References

- Gap C taxonomy: `.claude/sessions/taxa-drrp/04-15-26-gap-c-ai-research.md` §1
- Previous classifier plan: same doc §4 (superseded by this document)
- Gap C orchestration: `.claude/sessions/taxa-drrp/04-15-26-gap-c-orchestration.md`
- PageIndex research: `docs/PAGEINDEX-RESEARCH.md`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Gemini review round 1: 2026-06-05 (R1-R5 — structural wrapping, cross-doc citations, cost fan-out, taxonomy injection, EU sovereignty)
- ChatGPT review round 1: 2026-06-05 (deterministic inheritance tier, externally-derived confidence, narrower phasing, reasoning_type schema, C6 scepticism)
- Gemini review round 2: 2026-06-05 (deepest-first walk, one-hop recursion, dual records, model routing by context level, range expansion, milestone gate)
- ChatGPT review round 2: 2026-06-05 (obligation_layer field, conflict resolution policy, citation complexity classes, precision>recall for Tier 1, ancestor_distance metric, unresolved_external_reference reasoning_type)
- HSWA hierarchy example: s.2(1) → s.2(2) → s.2(2)(a)-(e) — parent chain resolves "employer"
