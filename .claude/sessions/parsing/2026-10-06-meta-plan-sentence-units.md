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
4. **Stop signals.** Per review batch, track the **approve-unchanged rate** of the justifier's proposals (the main convergence measure) and the number of new candidate patterns. If the approve rate doesn't rise and patterns don't fall, stop and question the scheme, not the labels.
5. **From the regex end.** Each tier decides what it can decide reliably and passes the residual up (carried from the closed plan).
6. **Paid services: walk, don't run.** Total cost approved before every run, pilot first, cost cap.
7. **Plan two phases ahead.** Session stubs only for the next one or two phases; later phases stay as lines here until their inputs exist.

## Todo

- ✅ Close the regex-end meta-plan, phase 0a and phase 0b; delete the stale phase 1–6 stubs (`3faf59e`)
- ✅ (raw: `data/code-review/drrp-meta-plan-sentence-units.md`; actions below and folded into the stubs) Gemini stress test of this plan and the two stubs
- ✅ (Jason, 2026-10-06: "happy with the meta-plan. I think the answers to the questions will emerge from the data through using the app") Jason's rulings on the Gemini actions: plan accepted. Open policy questions are settled from the data in the review app, not up front: phase A starts with provisional defaults and the sentence cards show where they break
- ⬜ **Phase A: Sentence units** (`parsing/2026-10-06-phaseA-sentence-units.md`). Define and build the unit: the assembler, edge cases (closing words, schedules, long lists, nested stems, whole-sentence detail of another sentence), counts on the 61 test laws and the gold selection
- ⬜ **Phase B: Principles and precedents** (`parsing/2026-10-06-phaseB-principles-precedents.md`). Prune the 123-rule catalogue to principles; the rest become precedents. Starts alongside phase A; finalised after A's unit definition
- ⬜ **Phase C: Gold set at the sentence level.** Re-key the 1,000 selection to its 883 units; carry the 240 reviewed (127 carry over, 27 need item actors only, 77 relabelled); review the rest in batches of about 100, tracking the principle rate; freeze. Reuses the phase 0a tooling (evidence pack, justifier brief, `drrp_gold`), with a review page reworked for sentence cards and prototyped before the first batch. The 77 relabelled units are labelled blind (old item decisions shown only after). A top-up for classes still scarce after re-keying (applying-provision holders, functions lists) from other held-out laws. An intra-rater check: about 50 units re-reviewed blind after a gap. **Domain scope:** until legal stores only the EHS&HR parts of the massive Acts, flag or drop units from out-of-domain Parts of the selection (Companies Act 1989 first). Don't top them up, and don't build rules or dictionary entries from them
- ⬜ **Phase D: Tier measurement** on the gold set: regex first, then the dependency-feature classifier vs the SLM (bake-off), with error analysis per class (Liberty, beneficiary, applying holders). **Baseline: an LLM alone**, using the existing Gemini v1.3 labels scored against gold (no new cost); it sets the ceiling the cheap tiers are judged against. The **row-to-sentence scoring spec** (how stem and item outputs combine into one sentence prediction) is written in phase C, before D runs
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
- dictionary queries from the first sentence review (2026-10-07):
  - add Gvt: Agency: Scottish Natural Heritage, a `Patient` label and Gvt: Judiciary: Employment Tribunal;
  - check whether `Ind: Licensee` is really an individual;
  - review `Authorised Person`;
  - split `SC: T&L:` into Supply Chain and Transport & Logistics;
  - **disambiguate `Operator`** into an individual who operates and an organisation that runs an installation, airport, well or activity (phase A doc);
- **Dictionary enrichment rule** (Jason, 2026-10-07, to stop dictionary drift):
  - A new named actor (party) label is added to fix a clash only when it is used **across 2 or more distinct Families**, or **repeatedly within one Family**. Families are DuckDB `legislation.family`. "Repeatedly" is to be set from the data; the default is about 3 laws.
  - Limited or one-off use never triggers enrichment. That clash stays on the generic label, and gold keeps both roles through the position key.
  - Candidates come from the law's own defined terms ("relevant person", "the operator") and are ranked by the clash counts: `Ind: Person` 54, `Gvt: Authority` 11, `Operator` 6, `Ind: Public` 5.
  - Each addition needs Jason's approval, mirroring phase B's promotion rule for principles.
- **#78 sized** (phase A, 2026-10-07): one entry per label loses a role in 198 of 1,413 multi-row sentences with actors (14%). Decide the actor key (label + role/referent, or `holds: both`) **before phase C labels at scale**;
- phase E: build the sentence assembler (`units.py`) in `fractalaw-core`, with its QA fixes.

**Legal-side refactoring list** (from the closed meta-plan; for after this plan):
- provision titles (P1group/Title) in legal's LAT parser, folded into the single run's re-parse wave: the cheapest purpose signal;
- one purpose vocabulary: retire legal's law-level PurposeClassifier and derive law purpose from ours (#172);
- the published `purposes` field migration;
- tell legal that items carry no DRRP of their own (the sentence holds it) but still send their actors;
- actor label renames already queued (`mix actors.rename_labels`).
- **trailing text on section rows** (legal's BlockText after a subsection, e.g. CAA 1982 s.44's "shall pay such compensation …" belongs to s.44(6)): 17 rows in the 60 test laws. Attach it to the subsection at source.
- **LAT parser list-text corruption** (legal, 2026-10-07, `lat_parser.ex extract_element_text`):
  - What goes wrong: a row's text is all the list items followed by all the Text, de-duplicated. So the chapeau lands at the end, nested items appear twice, and items are joined without a space.
  - Reach: about 1,672 rows in 543 laws have a moved chapeau; about 403 rows in 224 laws have a missing space.
  - This explains our "source text faults" (the Biological Agents Art.8(1)(d) lead-in at the end of the item). It also bears on the assembler and on gold text.
  - Plan: legal will send the affected section_ids. Gold units touched by them get re-assembled and their text re-checked. If the meaning changed, they are re-reviewed.
  - The fix, the P1group/Title titles, the free `@ConfersPower` flag (a power signal for relation and Liberty) and the #166 scoping go into **one re-parse wave before the single run**.
- The **SLM is not retrained before phase D** of this plan. Gold text is checked against the corrected parse first.
- **store only the EHS&HR-relevant parts of massive Acts** (legal has an open issue for this). Today we hold the whole of the Companies Act 1989 (1,062 rows, 23 of the gold selection, 4 sentences already in the gold set); the Civil Aviation Act 1982 (1,385 rows) and Water Act 2003 (847) are similar. Drawing from the whole Act takes us into domains outside EHS&HR. That drifts the dictionary (company-law actors) and brings in patterns of legal drafting our target domains don't use (Jason, 2026-10-07).

## Gemini stress test (2026-10-06)

Raw review: `data/code-review/drrp-meta-plan-sentence-units.md` (Gemini 2.5 Pro). Claude's reading:

**Valid, folded into the plan and stubs:**
- **Units with several holders or types.** "The operator must submit…, and the regulator must publish…" is still one sentence. Our model is per actor, so two holders are two active actors; that's fine. The real risk is the **same label twice** (#78), which merging items makes more likely. → Phase A measures it: units with more than one active actor, mixed Obligation/Liberty, and same-label collisions.
- **Assembler QA before labelling.** A wrong join poisons the gold set. → Phase A: a QA sample of assembled units, checked before phase C.
- **Provisos and full-sentence items need a policy, not just a count.** → Phase A decisions.
- **Convergence measure.** Principles per batch is weak; the approve-unchanged rate is better. → Principle 4.
- **Promotion threshold.** "About 3" is a heuristic; require recurrence in about 3 provisions **from at least 2 laws**, with Jason approving each promotion. The justifier only reports candidate patterns. → Phase B.
- **Precedent retrieval.** Precedents must be found, not remembered: by pattern tag, same law and section, then text similarity. → Phase B.
- **A and B aren't fully parallel.** B can classify early but finalises after A. → Todo.
- **Review UI for long units.** Prototype sentence cards before phase C (p90 is 718 characters; 3 units exceed 4,000).
- **Scoring spec before phase D.** → Written in phase C.
- **An LLM-only baseline.** Gemini v1.3 labels already exist for 6,959 provisions, so scoring them against gold costs nothing and sets the ceiling. → Phase D.

**Partly valid:**
- **Carry-over bias.** The 127 single-row units are the same units, so no new judgment. The 77 relabelled units: label blind, show the old item decisions after. → Phase C.
- **Rare classes.** The selection already over-samples (Liberty 164, beneficiary cue 182 of 1,000); applying holders (4) and functions lists (2) are still scarce. → Top-up in phase C.
- **Single reviewer.** It's Jason's call. A cheap mitigation is an intra-rater check (about 50 units re-reviewed blind after a gap). → Phase C.

**Not adopted:**
- Replacing the tiers with one LLM: the regex-end plan exists because of LLM cost across 234K provisions. The baseline above answers the question with existing labels.
- Two-tier purpose (section label plus sentence override): it's what the `inherit` escape already does.

## Dependencies

- ✅ Phase 0a tooling: selection (`data/gold/v3/selection.csv`), evidence pack, `drrp_gold`, review page (https://claude.ai/artifact/PkoqADCn2TXvSAEkPaTE6w), justifier brief
- ✅ 240 reviewed provisions (1,106 rows) in `drrp_gold` (gold-v3-draft)
- ✅ Silver labels: Gemini v1.3 (6,959), refereed batches 1–2
- ⬜ Provision titles from legal (single-run wave)
- Enables: the single run, gold v2's role (#74)
