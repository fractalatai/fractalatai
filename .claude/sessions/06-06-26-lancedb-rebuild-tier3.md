# Session: Next — LanceDB Table Rebuild + Tier 3 Integration

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4 + Appendix A
**Prior**: Phase 2A shipped actors JSON struct, but stored as Utf8 due to LanceDB `add_columns` limitation. Tier 3 POC validated (8/9 correct).

## Objective

Rebuild the LanceDB `legislation_text` table with a clean schema that includes the actors struct as native Arrow `List<Struct>`, then wire Tier 3 LLM (Gemini 2.5 Flash) into the enrichment pipeline.

## Part 1: LanceDB Table Rebuild

### Pre-flight

1. **Back up to NAS** — see `memory/reference_nas_backup.md`
   ```bash
   mkdir -p /mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)
   cp data/fractalaw.duckdb /mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)/
   cp -r data/lancedb/ /mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)/lancedb/
   ```
2. **Export to Parquet** — preserve all columns including embeddings
3. **Verify row counts** before and after

### Schema changes

Replace JSON Utf8 `actors` with native Arrow struct:
```
actors: List<Struct(label: Utf8, role: Utf8, recipient_type: Utf8)>
```

All Gap C columns become native (no `ensure_gap_c_columns` migration needed):
- `extraction_method`: Utf8
- `holder_inferred_from`: Utf8
- `ancestor_distance`: Int32
- `actors`: List<Struct>

### Rebuild approach

- Export existing table to Parquet (preserves embeddings)
- Create new table with updated schema from Parquet + schema overlay
- Verify embeddings survived
- Remove `ensure_gap_c_columns()` — columns are native

### Risk

Embeddings: 97K rows, ~9 hours CPU. Parquet export preserves them. If rebuild fails, restore from NAS backup.

## Part 2: Tier 3 LLM Integration

### When it fires

After Tier 1, for inherited provisions where `governed_actors.len() > 1`.

### Implementation

- Call Gemini 2.5 Flash REST API via reqwest (already in workspace)
- Same prompt validated in POC (`.claude/skills/tier1-qa/tier3_poc.py`)
- Write to native Arrow actors struct: holder + recipient + beneficiary roles
- Update flat columns with holder-only (backward compat)
- `extraction_method = "agentic"`
- GEMINI_API_KEY from environment

### Expected outcome

- Multi-actor inherited provisions get correct holder/recipient classification
- Tier 1 QA precision improves from 76% to >85%
- Recipient data available for downstream filtering ("show me protections for workers")

## Exit criteria

- LanceDB table rebuilt with native actors struct
- Embeddings verified intact
- Full corpus re-enriched with `--gap-c --force`
- Tier 3 firing on multi-actor provisions
- QA precision >85%
- Published to sertantai

## References

- NAS backup: `memory/reference_nas_backup.md`
- LanceDB backup strategy: `memory/feedback_lancedb_enrichment.md`
- Actors struct design: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` Appendix A
- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Phase 2A session: `.claude/sessions/06-06-26-gap-c-phase-2a-actors-struct.md`
