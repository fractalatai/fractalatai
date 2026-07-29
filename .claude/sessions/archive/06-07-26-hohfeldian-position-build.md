---
session: "Hohfeldian Position Model Build"
status: closed
opened: 2026-06-07
closed: 2026-06-07
outcome: success

summary: >
  Implemented the Hohfeldian position model replacing the role field with position
  (active/counterparty/beneficiary/mentioned). Position derives meaning from the provision's
  DRRP type -- active on DUTY = duty-holder, active on POWER = power-holder, counterparty on
  DUTY = claim-holder. Removed recipient_type field (redundant with position + DRRP). Added
  relates_to field for pairwise actor-to-actor linkage. Moved HM Forces from government to
  governed definitions. Implemented in commit 4a1e544.

decisions:
  - what: "Replace role field with position (active/counterparty/beneficiary/mentioned)"
    why: "Previous role=holder collapsed all DRRP types -- duty-bearing employer and power-wielding inspector both got holder"
    result: "Position + DRRP type gives full Hohfeldian relation without explicit role proliferation"
  - what: "Remove recipient_type field"
    why: "Redundant when position + DRRP tells the full story (counterparty on DUTY = claim-holder)"
    result: "Simpler schema with no information loss"
  - what: "Add relates_to field for pairwise actor linkage"
    why: "Flat actor lists cannot express which active actor's duty maps to which counterparty's claim (e.g., CDM multi-contractor sites)"
    result: "Optional field, null when relation is provision-wide (most cases)"
  - what: "Move HM Forces from government to governed definitions"
    why: "HM Forces act as duty holders in many provisions (HSWA s.48 binds the Crown including HM Forces)"
    result: "Agreed in Gemini review, Crown stays government but HM Forces moved to governed"

lessons:
  - title: "Position is a projection, not an enumeration"
    detail: "Four position values times four DRRP types gives 16 legal relations from a simple flat schema. Richer than enumerating all roles explicitly."
    tag: architecture
  - title: "LKIF-Core validates the Hohfeldian approach"
    detail: "The LKIF-Core norm.owl models the same Hohfeldian concepts with more granularity. Our position field is a pragmatic simplification suitable for Arrow storage."
    tag: research
---

# Session: 2026-06-07 — Hohfeldian Position Model Build (CLOSED)

## Context

**Prior session**: `.claude/sessions/cascade/06-06-26-tier3-llm-integration.md`
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
- Prior session: `.claude/sessions/cascade/06-06-26-tier3-llm-integration.md`
