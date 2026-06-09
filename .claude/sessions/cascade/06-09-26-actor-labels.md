# Session: Actor Labels — Let the LLM Name, Dictionary Match

## Context

**Prior session**: `.claude/sessions/cascade/06-09-26-production-v01.md`
**Trigger**: Sertantai feedback on MHR publish + analysis of 24 invented labels

## The Insight

The LLM "invented" labels that fall into two categories:
1. **Format mismatches** (`Org_Employer`) — our label format confused the LLM
2. **Genuine discoveries** (`water undertaker`, `liquidator`, `special negotiating body`) — real legal actors not in our dictionary

Current approach: force the LLM to use our exact dictionary labels → LLM struggles with format, misses actors not in the dictionary.

**Proposed approach**: Let the LLM name actors freely in its own words, then:
1. **Fuzzy match** against the dictionary — `Employer` → `Org: Employer`, `HSE` → `Gvt: Agency: Health and Safety Executive`
2. **Flag unmatched** as discoveries — new actors to review and potentially add to the dictionary
3. **No format constraints** in the prompt — the LLM uses natural language actor names

## Why This Matters

- The actor dictionary was hand-built from UK domestic law. EU retained law introduces actors we didn't anticipate (`Notified Body`, `downstream user`, `economic operator`)
- The dictionary will always lag behind the corpus. The LLM sees the text — it knows who the actors are
- Forcing exact label match costs quality (the LLM focuses on matching format instead of understanding the provision)
- Discoveries feed back into the dictionary — the corpus teaches us what actors exist

## Implementation Plan

### 1. Fuzzy matching engine
- Build a mapping from common LLM outputs to canonical dictionary labels
- Use embedding similarity as fallback (LLM label embedding vs dictionary label embeddings)
- Confidence threshold: high match → canonical, low match → flag as discovery

### 2. Update Tier 2/3 prompts
- Remove "use EXACT labels" constraint
- Ask: "name each actor as they appear in the text"
- Post-process: fuzzy match → canonical label or discovery

### 3. Discovery pipeline
- Accumulate unmatched labels across the corpus
- Group by similarity (e.g., "water undertaker" and "sewerage undertaker" are related)
- Periodic review → add to dictionary or map to existing

### 4. Dictionary evolution
- The dictionary becomes a living document, growing from corpus discoveries
- Version the dictionary — track when labels were added
- Re-classify provisions when new labels are added

## Existing Discoveries (from 24 invented labels)

| LLM label | Likely canonical | Action |
|---|---|---|
| Org_Employer | Org: Employer | Format bug — map |
| water undertaker | NEW | Add to dictionary |
| liquidator | NEW | Add to dictionary |
| special negotiating body | NEW | Add to dictionary |
| Manufacturers | SC: Manufacturer | Fuzzy match |
| Importers | SC: Importer | Fuzzy match |
| competent national authorities | Gvt: Authority | Fuzzy match |
| young people | Ind: Person (or NEW?) | Review |

## References

- Actor dictionary: `crates/fractalaw-core/src/taxa/actors.rs`
- Actor dictionary docs: `docs/ACTOR-DICTIONARY.md`
- Production session: `.claude/sessions/cascade/06-09-26-production-v01.md`
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
