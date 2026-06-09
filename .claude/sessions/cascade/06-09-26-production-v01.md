# Session: Production v0.1 — Publish QQ Corpus to Sertantai

## Context

**Prior session**: `.claude/sessions/cascade/06-09-26-tier2-classifier-training.md`
**Classifier**: `data/drrp_classifier_v6.pkl` (86.4%, 3-class: Obligation/Liberty/none)
**Golden dataset**: 1,759 regulation-level examples across 99 laws

The development cycle is complete. The v6 classifier is trained and exceeds the 80% accuracy target. This session wires it into the enrichment pipeline and publishes the QQ corpus to sertantai.

## Goal

**Session is complete when the QQ corpus is published to sertantai with the new Obligation/Liberty/none hierarchy.**

## The production pipeline

```
Provision text
    ↓
Regex sieve (structural filter, actor detection)
    ↓
Classifier (v6: embedding + modal → Obligation/Liberty/none)
    ↓
DRRP decomposition (actor label prefix → Duty/Responsibility/Right/Power)
    ↓
LanceDB (write with confidence + extraction_method)
    ↓
Zenoh publish to sertantai
```

## Implementation steps

### 1. Wire v6 classifier as TIER2_PROVIDER=classifier
- Load pickle at enrichment start
- For each provision: compute embedding + modal features → predict class
- Map Obligation/Liberty/none to DRRP sub-types using actor label prefix
- Write to LanceDB with `extraction_method = "classifier"`, `taxa_confidence` from model probability

### 2. Re-enrich QQ corpus
- Run with `TIER2_PROVIDER=classifier` on all 274 QQ laws
- Confidence protection preserves existing agentic (0.90) corrections
- Classifier replaces old regex DRRP types on everything else
- Should be fast — classifier is microseconds per provision, no API calls

### 3. Verify data shape
- Check DRRP distribution (Obligation/Liberty/none → decomposed to DRRP)
- Check confidence distribution (should be mostly ≥0.80)
- Spot check with `--report` on a few laws

### 4. Publish to sertantai
- `sync publish --provisions` for QQ laws
- Verify sertantai receives and displays correctly

## DRRP decomposition mapping

The classifier outputs 3 classes. The consumer (sertantai or the publish step) decomposes using the actor label prefix:

| Classifier output | Actor prefix | DRRP type |
|---|---|---|
| Obligation | Org:/Ind:/SC:/Spc: | Duty |
| Obligation | Gvt:/EU: | Responsibility |
| Liberty | Org:/Ind:/SC:/Spc: | Right |
| Liberty | Gvt:/EU: | Power |
| none | any | none |

## Shipped

### DRRP taxonomy migration
- 1,881 agentic provisions migrated: Duty→Obligation, Responsibility→Obligation, Right→Liberty, Power→Liberty
- 330 skipped (already `none`)
- Actors, confidence, extraction_method untouched — only `drrp_types` changed
- Verified on test provision (s.43B: Power+Duty → Liberty+Obligation)
- Compacted + NAS backup

## Development / Production Interface Rules

Two workflows operate on the same LanceDB dataset. They must not destroy each other's work.

### Development workflow
- **Purpose**: Improve data quality through QA, corrections, model training
- **Writes at**: `extraction_method = "agentic"`, `taxa_confidence = 0.90`
- **Tools**: Gemini QA with write-back, targeted enrichment, embedding backfill
- **Output**: Gold-standard provisions — verified by Gemini, corrected where wrong
- **Frequency**: Ad-hoc, per-law, small batches

### Production workflow
- **Purpose**: Classify the full corpus using the trained model
- **Writes at**: `extraction_method = "classifier"`, `taxa_confidence = 0.85`
- **Tools**: v6 classifier (embedding + modal features), no API calls
- **Output**: Bulk-classified provisions — 86.4% accurate, consistent taxonomy
- **Frequency**: Corpus-wide runs, hundreds of laws at once

### Interface rules

**Rule 1: Development always wins over production.**
Agentic (0.90) > classifier (0.85) > regex (0.30-0.80). The confidence hierarchy enforces this. A production run cannot overwrite a development correction.

**Rule 2: Production always wins over regex.**
The classifier at 86.4% is strictly better than regex at 22% for DRRP type. Every regex-classified provision should be reclassified by the production classifier. The classifier writes at 0.85, which is above the regex multi-actor confidence of 0.30.

**Rule 3: Taxonomy must be uniform before publish.**
LanceDB currently contains a mix of old taxonomy (Duty/Responsibility/Power/Right/Rule from regex) and new taxonomy (Obligation/Liberty/none from classifier + migrated agentic). Before publishing, ALL provisions must speak the same taxonomy. Options:
- (a) Classifier overwrites all regex provisions → uniform new taxonomy
- (b) Map old→new at publish time → LanceDB stays mixed but zenoh payload is uniform
- (c) Migrate all DRRP labels in LanceDB to new taxonomy first, then classifier fills gaps

**Rule 4: Re-running development after production is safe.**
If we QA a provision that the classifier already classified at 0.85, and Gemini corrects it, the correction writes at 0.90 → overwrites the classifier result. Next production run skips it (0.90 > 0.85). The ratchet works in both directions.

**Rule 5: Re-running production after development is safe.**
The classifier skips anything at ≥0.85. Development corrections at 0.90 are protected. New regex provisions (from re-enrichment) get classified by production.

### The taxonomy question

The classifier outputs Obligation/Liberty/none. The DRRP decomposition (Obligation→Duty or Responsibility based on actor prefix) should happen:
- **At classification time** (write Duty/Responsibility/Power/Right to LanceDB) — sertantai gets familiar DRRP types
- **At publish time** (write Obligation/Liberty to LanceDB, decompose in zenoh payload) — LanceDB stays pure hierarchy

**Recommendation**: Decompose at classification time. Write Duty/Responsibility/Power/Right to LanceDB. Sertantai already expects these types. The Obligation/Liberty hierarchy is an internal model concept, not a data schema.

This means the classifier writes the SAME taxonomy as the old regex — but with much better accuracy. No taxonomy migration needed. The agentic provisions we already migrated to Obligation/Liberty need to be migrated back.

### What this changes

If we decompose at classification time:
1. No taxonomy migration needed for regex provisions
2. No taxonomy migration needed at publish time
3. The migrated agentic provisions (1,881 that we changed to Obligation/Liberty) need reverting back to Duty/Responsibility/Power/Right
4. The classifier internally predicts Obligation/Liberty but WRITES Duty/Responsibility/Power/Right
5. LanceDB always speaks DRRP, never Obligation/Liberty

### Classifier gate simplified (commit `7928e29`)
- Gate by extraction_method, not confidence threshold
- Skip `agentic` / `agentic_unvalidated` — everything else gets classified
- No confidence arithmetic needed
- Eliminates the regex 0.85/0.90 overlap problem Gemini flagged

### Agentic taxonomy reverted
- 1,882 provisions reverted from Obligation/Liberty back to DRRP
- Verified: zero Obligation/Liberty remaining in LanceDB
- Decomposition happens at classification time, not in schema

### Mixed-actor decomposition rule
- Uses ACTIVE actor prefix, not all actors
- If any active actor is Gvt:/EU: → Responsibility/Power
- Falls back to all actors if none marked active (regex provisions)

### QQ corpus classified (production run)
- **40,272 provisions classified in 400 seconds (101/s)**
- 1,611 agentic protected, 38,919 structural skipped
- Zero API calls — pure classifier inference
- Auto-compacted after run

### Corpus shape after classification

| DRRP type | Count |
|---|---|
| Duty | 13,243 |
| Responsibility | 8,127 |
| Power | 6,455 |
| Right | 4,976 |
| Rule | 454 |
| (none) | 47,546 |

| Method | Count |
|---|---|
| classifier | 40,272 |
| regex | 28,532 |
| inherited | 5,471 |
| none | 4,487 |
| agentic | 2,014 |

Agentic breakdown: Responsibility (812), Duty (536), Power (378), none (324), Right (160) — all DRRP types represented in gold data.

## Exit criteria

- [x] v6 classifier wired as `TIER2_PROVIDER=classifier`
- [x] Development workflow tested — agentic 0.90 data survives production run
- [x] QQ corpus classified (40,272 provisions in 400s)
- [x] Data verified (DRRP distribution, taxonomy uniform, agentic protected)
- [x] Published MHR to sertantai — 38 of 53 provisions accepted, 15 schedule orphans skipped
- [x] Sertantai confirmed: struct richer than flat, position classification correct

## Deferred: Full QQ publish

Held back from full corpus publish. Sertantai feedback surfaced actor label issues:
- 24 `agentic_unvalidated` provisions with invented labels (`Org_Employer`, novel actors)
- Novel labels are signal — LLM discovering actors not in our dictionary
- Need to explore: let LLM name actors freely, match to dictionary, flag new discoveries
- New session: actor dictionary and label strategy

## Key learnings

1. **Classifier in production works** — 40,272 provisions in 400s, no API calls
2. **Development/production coexistence proven** — agentic gold survives production runs
3. **DRRP decomposition at classification time** — LanceDB speaks DRRP, not Obligation/Liberty
4. **Schedule provisions stay but don't publish** — training value, sertantai skips automatically
5. **Invented labels are discoveries** — `water undertaker`, `liquidator`, `special negotiating body` are real actors the LLM found that our dictionary doesn't have

## References

- Classifier: `data/drrp_classifier_v6.pkl`
- Classifier skill: `.claude/skills/classifier-enrich/`
- Training session: `.claude/sessions/cascade/06-09-26-tier2-classifier-training.md`
- Gemini interface review: `docs/reviews/gemini-production-interface-review-20260609.md`
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Enrich skill: `.claude/skills/enrich-and-publish/`
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
