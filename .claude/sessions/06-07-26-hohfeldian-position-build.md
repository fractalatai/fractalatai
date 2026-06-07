# Session: Hohfeldian Position Model Build

## Context

**Prior session**: `.claude/sessions/06-06-26-tier3-llm-integration.md`
**Gemini review**: `docs/reviews/gemini-actors-struct-review-20260607.md`
**Sertantai briefing**: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`

The actors struct schema has been agreed after Hohfeldian research and Gemini review. This session implements the schema change.

## Agreed schema

```
actors: List<Struct>
├── label: Utf8          -- "Org: Employer", "Gvt: Agency: HSE"
├── position: Utf8       -- "active" | "counterparty" | "beneficiary" | "mentioned"
├── relates_to: Utf8?    -- linked actor label for pairwise relations (null when provision-wide)
├── label_source: Utf8   -- "canonical" | "invented"
└── reason: Utf8?        -- LLM reasoning (null for regex/inherited)
```

Position derives meaning from the provision's `drrp_types`:

| DRRP type | `active` | `counterparty` |
|-----------|----------|----------------|
| Duty | duty-holder | claim-holder |
| Right | right-holder | no-right holder |
| Responsibility | responsibility-holder | claim-holder |
| Power | power-holder | liable party |

## Changes from current schema

| Field | Current | New |
|-------|---------|-----|
| `role` | `primary-holder \| holder \| recipient \| beneficiary \| mentioned` | **removed** — replaced by `position` |
| `position` | n/a | `active \| counterparty \| beneficiary \| mentioned` |
| `recipient_type` | `protected_person \| regulated_actor \| informed_party` | **removed** — redundant with position + DRRP |
| `relates_to` | n/a | **new** — optional linked actor label |

## Implementation checklist

### 1. Schema (`fractalaw-core/src/schema.rs`)
- [ ] Replace actors struct fields: remove `role`, `recipient_type`; add `position`, `relates_to`
- [ ] Field count test unchanged (still same number of top-level columns)

### 2. Actor dictionary (`fractalaw-core/src/taxa/actors.rs`)
- [ ] Move `HM Forces` from `GOVERNMENT_DEFS` to `GOVERNED_DEFS`
- [ ] Update `all_actor_labels()` test if needed

### 3. CLI enrichment (`fractalaw-cli/src/main.rs`)
- [ ] Rename `ActorEntry.role` → `position`, remove `recipient_type`, add `relates_to`
- [ ] Regex provisions: all actors get `position: active` (they were found in the text bearing the DRRP)
- [ ] Tier 1 inherited: same as regex — `position: active`
- [ ] Tier 3 prompt: update to ask for `position` classification (active/counterparty/beneficiary/mentioned)
- [ ] Tier 3 response parser (`parse_tier3_actors`): map to new position values
- [ ] Batch builder: update struct fields (5 fields: label, position, relates_to, label_source, reason)
- [ ] Taxa schema in batch builder: match new struct
- [ ] Flat column dual-write: `active` actors → `governed_actors`/`government_actors` (same logic, different field name)

### 4. Tests
- [ ] Update `ParsedTier3Actor` struct
- [ ] Update all canned response tests for new field names and values
- [ ] Add test for `relates_to` field
- [ ] Run `cargo test -p fractalaw-core` and `cargo test -p fractalaw-cli`

### 5. LanceDB migration
- [ ] Run migration script to rebuild table with new struct
- [ ] Verify row counts and embeddings

### 6. Sertantai briefing
- [ ] Update briefing doc with final schema (already done, verify `relates_to` is included)

### 7. Re-enrich and publish
- [ ] Re-enrich OH&S family with new schema
- [ ] Publish to sertantai via zenoh
- [ ] Human review in sertantai UI

## References

- Hohfeldian analysis: [legaldesire.com](https://legaldesire.com/legal-rights-and-duties-hohfeldian-analysis/)
- LKIF-Core norm.owl: [github.com/RinkeHoekstra/lkif-core](https://github.com/RinkeHoekstra/lkif-core)
- Gemini review: `docs/reviews/gemini-actors-struct-review-20260607.md`
- Prior session: `.claude/sessions/06-06-26-tier3-llm-integration.md`
