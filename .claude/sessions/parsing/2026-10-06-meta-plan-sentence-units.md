---
session: "Meta-plan: DRRP from the unit up (sentences, a small rulebook, then tiers)"
status: active
opened: 2026-10-06
issue: 72
related: [60, 65, 67, 74, 75, 76, 77, 78, "parsing/2026-10-06-pipeline-from-the-regex-end.md", "parsing/2026-10-06-phase0a-gold-set-scaffolding.md", "parsing/2026-10-06-phase0b-purpose-scheme-review.md"]
---

# Session: Meta-plan: DRRP from the unit up (ACTIVE)

## Problem

The regex-end meta-plan (closed 2026-10-06) planned around the pipeline's tiers but never fixed **what a label attaches to**. Its phase 0a gold set labelled the source's rows, so:
- 39% of the selection were pieces of a sentence with a stem, and 36% of reviewed relations became `continues`;
- 23 of 123 catalogue rules existed only for stems, items and continuation;
- purpose was forced onto snippets (10 of 27 reviewed multi-provision sections had mixed purposes);
- every review decision became a rule: 44 rules touched in one day, about 6 new rulings per batch, no convergence.

**Jason (2026-10-06):** "'continues the stem' is also another way of saying 'inherit'. It's telling us we're into the weeds of snippets of laws." And: "are we creating some large, very particular rulebook?"

## Principles

1. **Unit first.** Relation, holder and actors attach to the **legal sentence**: a stem joined with its items and closing words, or a row with no stem. Items are never labelled; they inherit. Purpose attaches to the sentence, with `inherit` (to the section) as the escape for sentences with no purpose of their own.
2. **A small rulebook.** About 30 to 40 general **principles** a model can learn. Decisions on individual provisions are **precedents**, kept as examples in the gold table. A pattern becomes a principle only when it recurs in about 3 or more provisions.
3. **Yardstick before tiers.** A human-reviewed gold set, at the sentence level, before any tier is scored. LLM labels are silver (evaluation evidence), never the yardstick.
4. **Stop signals.** Track new principles per review batch. If the rate doesn't fall, stop and question the scheme, not the labels.
5. **From the regex end.** Each tier decides what it can decide reliably and passes the residual up (carried from the closed plan).
6. **Paid services: walk, don't run.** Total cost approved before every run, pilot first, cost cap.
7. **Plan two phases ahead.** Session stubs only for the next one or two phases; later phases stay as lines here until their inputs exist.

## Todo

- ✅ Close the regex-end meta-plan, phase 0a and phase 0b; delete the stale phase 1–6 stubs (`3faf59e`)
- ⬜ Gemini stress test of this plan and the two stubs; Jason's rulings on it
- ⬜ **Phase A: Sentence units** (`parsing/2026-10-06-phaseA-sentence-units.md`). Define and build the unit: the assembler, edge cases (closing words, schedules, long lists, nested stems, whole-sentence detail of another sentence), counts on the 61 test laws and the gold selection
- ⬜ **Phase B: Principles and precedents** (`parsing/2026-10-06-phaseB-principles-precedents.md`). Prune the 123-rule catalogue to principles; the rest become precedents. Can run alongside phase A
- ⬜ **Phase C: Gold set at the sentence level.** Re-key the 1,000 selection to its 883 units; carry the 240 reviewed (127 carry over, 27 need item actors only, 77 relabelled); review the rest in batches of about 100, tracking the principle rate; freeze. Reuses the phase 0a tooling (evidence pack, justifier brief, `drrp_gold`, review page)
- ⬜ **Phase D: Tier measurement** on the gold set: regex first, then the dependency-feature classifier vs the SLM (bake-off), with error analysis per class (Liberty, beneficiary, applying holders). Row outputs map to sentences until the tiers label sentences
- ⬜ **Phase E: Tier improvement** driven by D: sentence assembly in `fractalaw-core`; regex relation and act; coarse purpose (statutory classes, section titles when served); the residual tiers (classifier or SLM, then a short LLM prompt)
- ⬜ **Phase F: Acceptance and the single run.** Per-tier and end-to-end thresholds set before the run; then the single run (`parsing/09-29-26-reenrichment-backlog.md`)

## Carried forward (from the closed sessions)

**Decisions that still stand:**
- the spec, not the v1.3 prompt, is the standard; one entry per label (POS-15), losses tracked in #78;
- purpose: 11 coarse statutory classes (Requirements/Permissions, Application incl. transition), single-select, layered, fine level only where a consumer needs it;
- purpose detail (Substantive vs Procedure/Detail) is derived, not labelled;
- the review loop: an artifact page with one db doc per unit for proposals and one for decisions; Jason approves easy rows in bulk and works the hard core.

**Open questions inherited:**
- whole-sentence detail of a duty in another sentence (31 of the 86 reviewed `continues`): keep `continues` for it, or `no` with a pointer? (phase A);
- the 3 queries left from batch 2 (a modification clause's purpose; deemed compliance; a `Gvt: Authorised Person` label);
- dictionary gaps: well-operator, support panel (CTSA 2015 s.36), nominated recipient (FIT); `Spc: Verifier` holder class.

**Legal-side refactoring list** (from the closed meta-plan; for after this plan):
- provision titles (P1group/Title) in legal's LAT parser, folded into the single run's re-parse wave: the cheapest purpose signal;
- one purpose vocabulary: retire legal's law-level PurposeClassifier and derive law purpose from ours (#172);
- the published `purposes` field migration;
- tell legal that items carry no DRRP of their own (the sentence holds it) but still send their actors;
- actor label renames already queued (`mix actors.rename_labels`).

## Dependencies

- ✅ Phase 0a tooling: selection (`data/gold/v3/selection.csv`), evidence pack, `drrp_gold`, review page (https://claude.ai/artifact/PkoqADCn2TXvSAEkPaTE6w), justifier brief
- ✅ 240 reviewed provisions (1,106 rows) in `drrp_gold` (gold-v3-draft)
- ✅ Silver labels: Gemini v1.3 (6,959), refereed batches 1–2
- ⬜ Provision titles from legal (single-run wave)
- Enables: the single run, gold v2's role (#74)
