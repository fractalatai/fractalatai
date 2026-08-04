---
session: Near-Miss Reports Emergence Pass
status: closed
opened: 2026-07-30
closed: 2026-07-30
outcome: success

summary: >
  Processed 300 near-miss reports through emergence and constrained passes. Produced 1,244 typed cultural
  edges. Schema validated — no new types needed. Identified investigation reports as the missing source for
  blames/silences (different author perspective). Combined corpus now 899 narratives, 4,321 cultural edges.

decisions:
  - what: Run emergence pass on near-misses despite schema being finalised
    why: Near-misses are the first "incident" source — something actually happened. Different narrative register could surface new edge types around blame, silence, and intervention dynamics.
    result: Schema confirmed stable. No new types needed. Dismissed/disregarded/ignored map to normalises. Intervention verbs map to speaks-up-to.
  - what: Identify investigation reports as a separate data source
    why: Blames and silences remain absent across all three reporter-authored sources. The reporter avoids blame by cultural convention. The investigator writes in third-person analytical language where attribution is expected. Different author perspective, not different report type.
    result: Pending session created for investigation reports

metrics:
  emergence: { valid: 300, invalid: 0, yield_pct: 100 }
  constrained: { valid: 300, invalid: 0, yield_pct: 100 }
  cultural_edges: { total: 1244, speaks_up_to: 210, directs: 152, cooperates_with: 138, responds_to_failure_of: 83 }
  combined_corpus: { narratives: 899, cultural_edges: 4321, source_types: 3 }

lessons:
  - title: Near-misses are a distinct register — between positive observations and hazard reports
    detail: Speaking-up at 67% (vs PO 39%, HZ 84%). Workaround at 32% (matching HZ 33%). The speaking-up is reactive ("this nearly happened") rather than anticipatory (HZ) or incidental (PO). Each source captures a different moment in the safety cycle.
    tag: data
  - title: Blame is culturally suppressed in reporter-authored UK safety narratives
    detail: Only 2 blame/attribution verbs across 300 near-miss narratives. Even when something nearly went wrong, reporters describe what happened without assigning fault. This is not a schema gap — it's an author perspective gap. Investigation reports (written by investigators, not reporters) are where blame language surfaces.
    tag: methodology
  - title: Skip emergence pass for new sources when schema is stable
    detail: The emergence pass was valuable for near-misses because it was the first incident source. For future sources with the same narrative register (e.g., another organisation's hazard reports), constrained pass alone is sufficient. Only run emergence when the source type is genuinely novel.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/outputs/training/near-miss-training-narratives.parquet
  - data/qq/cultural-graph/outputs/training/near-miss-training-entities.parquet
  - data/qq/cultural-graph/outputs/training/near-miss-training-relationships.parquet
  - data/qq/cultural-graph/outputs/training/near-miss-training-cultural-edges.parquet
  - data/qq/cultural-graph/outputs/briefs/near-miss-reports-phase1-brief.md
  - data/qq/cultural-graph/outputs/briefs/near-miss-reports-phase2-brief.md
  - .claude/sessions/cultural-graph/07-30-26-investigation-reports-emergence.md

depends_on:
  - 07-29-26-schema-constrained-extraction.md
  - 07-30-26-hazard-reports-emergence.md

enables:
  - Combined training set across all reporter-authored sources
  - SLM fine-tuning with 4,321 cultural edges (2.4x positive-observations-only)
  - Cross-source site-level cultural profiling (3 source types)
---

# Session: Near-Miss Reports Emergence Pass (CLOSED)

## Problem

Near-miss reports are the fourth source type for the cultural graph extraction pipeline. These narratives describe incidents where harm was narrowly avoided — expected to show the strongest speaks-up-to, works-around, and silences signals of any source type. The near-miss register captures what organisations don't normally notice, making it the most culturally valuable data source.

## Todo

- ✅ Receive near-miss reports dataset — `Near Misses_strict_redacted(Sheet1).csv`
- ✅ Profile dataset — 300 rows, 30 sites, median 176 words, 99% have RE_ActionImmediate
- ✅ Run emergence pass — 300/300 valid, 67% speaking-up, 32% workaround, 19% negative polarity
- ✅ Check emerged verbs against schema — no new types needed; dismissed/disregarded/ignored map to normalises; blames/silences remain absent from explicit reporting
- ✅ Run constrained pass — 300/300 valid, 1,244 cultural edges
- ✅ Export training data to Parquet — 300 narratives, 4,174 entities, 1,244 cultural edges
- ✅ Produce executive briefs — phase 1 + phase 2
- ✅ NAS backup

## Schema Gap: Investigation Reports

The absence of `blames` and `silences` from all three reporter-authored sources points to a missing data source, not a schema problem. The reporting chain has different authors:

- **Reporter** (positive obs, hazards, near-misses) — first-person, immediate, describes what they saw/did. Culturally avoids blame.
- **Investigator** (investigation reports) — third-person, analytical, traces root causes, assigns contributing factors. This is where blame/silence dynamics surface: "the supervisor failed to ensure", "the concern had been raised previously but not actioned."

Investigation reports are a distinct narrative source — different author perspective, different language register, different cultural signal. Same schema, same scripts, but the training data would teach the model to recognise blame and silence from investigator language. Pending session created.

## Dependencies

- ✅ Extraction pipeline validated on positive observations
- ✅ Scripts ready: `emergence_pass.py`, `constrained_pass.py`, `export_training_data.py`
- ✅ Edge type schema finalised (13 cultural types + operational)
- ✅ Near-miss reports CSV received
