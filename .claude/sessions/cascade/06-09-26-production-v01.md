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

## Exit criteria

- [ ] v6 classifier wired as `TIER2_PROVIDER=classifier`
- [ ] QQ corpus re-enriched with classifier
- [ ] Data verified (confidence, DRRP distribution, spot checks)
- [ ] Published to sertantai via zenoh
- [ ] Sertantai confirms receipt

## References

- Classifier: `data/drrp_classifier_v6.pkl`
- Training session: `.claude/sessions/cascade/06-09-26-tier2-classifier-training.md`
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Enrich skill: `.claude/skills/enrich-and-publish/`
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
