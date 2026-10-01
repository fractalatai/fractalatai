---
session: Benchmark Refresh
status: closed
opened: 2026-09-28
closed: 2026-10-01
outcome: abandoned
issue: 65
summary: >
  Superseded by gold v2 (#74, benchmarks/2026-09-30-gold-v2.md). The benchmark LAT sync, the gold
  snapshot and the new benchmark design were done there; the SLM-retraining question moved there too.
---

# Session: Benchmark Refresh (CLOSED — superseded by #74)

## Problem

The 20 benchmark laws are Gemini-labelled silver (~150 sampled provisions each, 3,014 labels) that were used to train the SLM, and they were labelled on older-generation LAT that no longer matches production. 5 are flagged but never labelled (e.g. the Water Industry Act 1991, whose hub copy holds 24% of the Act). Some benchmarks are dominated by big Acts. fractalatai #65.

## Todo

- ⬜ Drop `is_benchmark` from the 5 unlabelled laws (UK_ukpga_1991_56, UK_ukpga_1981_69, UK_ukpga_1997_8, UK_uksi_1999_1148, UK_uksi_2009_890); sync; add them to the re-enrichment backlog
- ⬜ Export the 15 labelled benchmarks' gold + text (snapshot), then sync them to current LAT (`--allow-benchmark`)
- ⬜ Re-label the 15 with Gemini (fixed prompt) on current-format text
- ⬜ Compare: old silver vs new LLM (drift); SLM vs new LLM (accuracy); Jason's manual spot-check
- ⬜ Decide on SLM retraining (incl. false positives: cross-references, commencement lists, conditions → Obligation)
- ⬜ New benchmark design: more, smaller laws across families, labelled in full; big Acts only via scoped parts (legal #166); human review of a stratified sample

## Dependencies

- ✅ #62 hub sync (`pull-lat --allow-benchmark`, archive/restore)
- ✅ Gemini prompt fix (offences/penalties, cross-references) `e7bc2ab`
- ⬜ sertantai-legal #166 scoped LAT for large Acts (for big-Act benchmarks)
- ⬜ RunPod for SLM re-scoring / retraining
