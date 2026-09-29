---
session: Re-enrichment Backlog
status: pending
opened: 2026-09-29
---

# Session: Re-enrichment Backlog (PENDING)

## Problem

The first #62 hub sync (2026-09-27) flagged 422 laws `reparse_needed`: their text changed, and 15,296 changed-text snapshots plus 38,161 superseded rows were archived with their tier data. Only QQ Tier 0 (122) and the live-fix 8 have been re-enriched since. Further laws have joined from legal: its 101-law re-enrichment list after the LAT re-parse, and 4 from the 42-parent-Act extent re-parse. Until re-enriched, these laws' law-level results in DuckDB (and in legal) reflect old actors, and they carry no provenance.

## Todo

- ⬜ Build the backlog list: `lat_sync_state.reparse_needed` ∪ legal's `reenrichment-list.csv` (101) ∪ extent re-parse 4 (UK_ukpga_1990_16, UK_ukpga_1988_52, UK_ukpga_1993_11, UK_anaw_2017_2), minus laws legal discarded as not Making (lean LAT) and the benchmark laws (#65)
- ⬜ Prioritise with legal: Making laws in customer registers first; QQ tiers beyond Tier 0
- ⬜ Run the pipeline in batches (customer-batch-parse): parse → dep → embed → classify → infer → reconcile → pod SLM (position/significance/fitness) → LLM tier (Gemini, fixed prompt, test batch of 10 then 50s) → hierarchy → backfill → fitness → verdict diff → Jason review → publish with provenance
- ⬜ 6 Tier 0 actors still `pending_slm` (next pod)
- ⬜ Audit the earlier corpus LLM labels (~2,449 `llm` actors) for the penalty-clause error fixed in `e7bc2ab`: find Obligation labels on offence/penalty text, clear, re-run with the fixed prompt
- ⬜ After legal #166 scopes large Acts: re-enrich the 6 Tier 0 big Acts on scoped LAT (Companies Act 2006, PH(S)A 2008, IPA 2016, PCA 2017, CTBSA 2019, EU(W)A 2018)
- ⬜ Keep DuckDB snapshots before each batch's parse (parse rewrites law-level DRRP); restore held/zero-actor laws afterwards

## Dependencies

- ✅ #62 hub sync + diff-apply; #63 provenance; refined zero-actor guard (`e7bc2ab`)
- ⬜ RunPod per batch (Jason launches when the local steps are done)
- ⬜ Legal's snapshots per publish; sertantai-legal #166 for the big Acts
