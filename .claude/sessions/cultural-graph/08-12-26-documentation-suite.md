---
session: Documentation Suite
status: closed
opened: 2026-08-12
closed: 2026-08-12
outcome: success

summary: >
  Built a 3-tier documentation pack (Overview → Your Site → Methodology) from
  disjointed artefacts. Rewrote executive summary, site profiles, added "Why This
  Model?" and "How to Read the Reports" guides, roadmap, narrative report skill,
  suite index. Renamed SMR→SRR, standardised compositional profiles, added Spongl
  footers. Exported as zip for QQ laptop.

decisions:
  - what: Restructure from 4 tiers to 3 tiers plus campaign materials
    why: >
      Original 4-tier structure (Leadership/Operational/Strategic/Technical) was
      too internally focused. User wanted a pack anyone at QQ could work through.
      Tier 4 (technical) was internal development artefacts not suited for the pack.
      Campaign materials are operational, not a reading tier.
    result: >
      3 tiers: Overview (4 docs), Your Site (4 docs), Methodology (2 docs).
      Campaign materials separated. 10 documents numbered sequentially.
  - what: Rename SMR to SRR (Standardised Rate Ratio) across all user-facing text
    why: >
      SMR = Standardised Morbidity Ratio carries clinical/disease connotations
      inappropriate for safety culture. "Morbidity" means illness/death. Gemini
      confirmed no established sociology/culture term exists — SRR is the best
      generic option.
    result: 14 files + 2 skills + report generator updated. Zero SMR/Morbidity remaining.
  - what: Separate stable model docs from volatile results docs
    why: >
      User identified that the executive summary mixed stable content (what is the
      Cultural Graph, the 12 types, the 5 composites) with volatile content (specific
      numbers, flag counts, sector comparisons) that goes stale quickly. Tier 1 should
      be stable; Tier 2 carries the results.
    result: >
      Executive summary now contains zero specific numbers — all results moved to
      site profiles brief (Tier 2). Executive summary won't need updating unless
      the model itself changes.
  - what: Standardise compositional profiles (edge type proportions per site)
    why: >
      Raw edge type proportions were confounded by report type mix — same issue as
      the blended org rates. A site submitting mostly positive observations would
      overrepresent Voice edge types.
    result: >
      Proportions now computed per report type within each site, then equal-weighted.
      ASH normalises went from unlisted to 11%. MHA normalises confirmed at 9%.
  - what: Add "Why This Model?" document explaining the dialectic origins
    why: >
      User identified that the 12 relationship types appeared arbitrary without
      explaining their provenance. The structured dialectic (5 rounds, hostile audits)
      and 6-tradition literature convergence is the novel intellectual contribution.
      Readers need enough to trust the model without a lecture on safety theory.
    result: >
      New document covering: functionalist/interpretivist problem, dialectic insight
      (nodes vs edges), 5P node types with 7-tradition convergence, 12 edge types
      from 6 research traditions, "why graph means network not chart", segue to roadmap.

metrics:
  docs_written: 4
  docs_rewritten: 2
  docs_updated: 3
  total_files_in_pack: 24
  zip_size_mb: 3.7
  smr_to_srr_files_updated: 16
  tiers: 3

lessons:
  - title: Executive summaries must not contain specific numbers that change

    detail: >
      The original executive summary had "Voice 1.09", "18 sites flagged", "56%
      prevalence" — all of which went stale within weeks as methodology evolved.
      The user correctly identified that Tier 1 docs should explain the model,
      not report results. Results belong in Tier 2 where they're expected to update.
      A stable Tier 1 document avoids the "last updated 6 months ago" problem.
    tag: methodology
  - title: Domain terminology matters — SMR carries clinical baggage in safety
    detail: >
      "Standardised Morbidity Ratio" is standard epidemiology but "morbidity"
      means illness/death. In a safety context where people are trying to prevent
      harm, using clinical disease terminology is tone-deaf. SRR (Standardised
      Rate Ratio) is equally precise without the baggage. The rename touched 16
      files — better to catch this before external distribution.
    tag: methodology
  - title: Graph means chart to most readers — must explain the network concept
    detail: >
      The user pointed out that people will think "graph" means a bar chart or
      line plot. The document needed an explicit section explaining that a graph
      is a network (dots and lines), using a social network analogy, before the
      concept of a "cultural signature" makes sense. Without this, the name
      "Cultural Graph" is confusing rather than illuminating.
    tag: methodology
  - title: Compositional profiles have the same report-type confound as rates
    detail: >
      Edge type proportions per site (e.g., "22% speaks-up-to, 10% normalises")
      were presented as raw counts. The user correctly identified that a site
      submitting mostly positive observations would overrepresent Voice edge types
      in these proportions. Same fix applied: compute per RT, equal-weight. This
      confound is pervasive — anywhere raw rates appear, the mix matters.
    tag: methodology
  - title: The dialectic provenance is the key differentiator and must be in Tier 1
    detail: >
      The 12 relationship types look arbitrary without context. The user identified
      that explaining they emerged from 5 rounds of adversarial dialectic, tested
      by hostile audit, drawing on 6 research traditions, is what makes this novel
      and trustworthy. This isn't background material — it's the core intellectual
      contribution and belongs in the overview, not buried in methodology.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/README.md
  - data/qq/cultural-graph/docs/briefs/cultural-graph-executive-summary.md
  - data/qq/cultural-graph/docs/briefs/cultural-graph-why-this-model.md
  - data/qq/cultural-graph/docs/briefs/site-cultural-profiles-brief.md
  - data/qq/cultural-graph/docs/how-to-read-the-reports.md
  - data/qq/cultural-graph/docs/roadmap.md
  - data/qq/cultural-graph/reports/site-cards/esk.md
  - .claude/skills/cultural-graph-narrative/SKILL.md
  - .claude/skills/cultural-graph-report/SKILL.md
  - scripts/cultural-graph/generate_report.py

depends_on:
  - 08-11-26-org-level-analytics.md
  - 08-08-26-log-scale-glm.md
  - 08-08-26-empirical-bayes-shrinkage.md
  - 08-08-26-analytics-and-visualisation.md

enables:
  - Monthly site data card generation via /cultural-graph-narrative skill
  - QQ laptop distribution of documentation pack
  - September 2026 communications rollout with consistent, current documentation
---

# Session: Documentation Suite (CLOSED)

## Problem

The cultural graph has 14 static docs, 6 generated reports, and a comms plan — but no coherent reader journey. A site manager or leadership team member can't navigate from "what is this?" through to reading a caterpillar plot. Key docs (executive summary, site profiles brief) use stale methodology (blended rates, 30% threshold flags) instead of current prevalence + intensity + SMR/FDR/EB pipeline. There's no PPTX-friendly output, no AI-curated narrative report skill, and no reader-facing "next steps" document despite the project being mid-journey. Getting this right is essential for landing the work with the organisation and leadership team.

## Todo

- ✅ Audit current docs — identify stale numbers, gaps in journey, audience mismatches
- ✅ Design the document suite — reading order, audience levels, static vs generated, what updates when
- ✅ Update executive summary — prevalence + intensity, SMR/EB/FDR methodology, sector comparison, updated numbers
- ✅ Update site cultural profiles brief — SMR-based flagging, prevalence + intensity trajectory, standardised sector comparison
- ✅ Write "How to Read the Dashboard" guide — funnel plots (with phi explanation), caterpillar plots, trend caterpillars, flags, FAQ
- ✅ Write reader-facing "Roadmap" — delivered/now/next/planned/vision, draws from founding plan + comms plan
- ✅ Rewrite docs/README.md as 4-tier suite index with reading order, audience tags, update triggers
- ✅ PPTX-friendly conventions — sections sized to one slide, bold key numbers, max 6 columns, documented in suite design
- ✅ Build `/cultural-graph-narrative` skill — SKILL.md with data extraction pattern, output format, guidelines. Proof-of-concept site card for ESK generated
- ✅ Validate suite end-to-end — iterative review with user: stale numbers fixed, missing nodes added, graph concept explained, SMR→SRR renamed, compositional profiles standardised, Spongl footers added, zip exported for QQ laptop

## Dependencies

- ✅ Prevalence + intensity decomposition (session: Org-Level Analytics)
- ✅ Log-scale GLM + EB shrinkage + FDR pipeline (sessions: Log-Scale GLM, Empirical Bayes Shrinkage)
- ✅ Extended analytics — caterpillar plots, funnel plots, power analysis, sensitivity (session: Extended Analytics and Visualisation)
- ✅ Directory reorganisation — clean structure with READMEs (session: Org-Level Analytics)
- ✅ Comms plan v0.3 — communications strategy with Group feedback incorporated
- ✅ Month 1 artefacts drafted — `data/qq/cultural-graph/docs/month1-artefacts/`
- Founding plan: `.claude/plans/cultural-graph/initial-review.md`
- Comms plan: `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`

## Audit Results

### Stale (needs rewrite)
- **Executive summary** — leads with old blended rates (Voice 1.09) and 30% threshold, contradicts itself with SMR/FDR note at bottom
- **Site cultural profiles brief** — dashboard uses blended rates, flagging legend uses old format, no prevalence/intensity distinction

### Stale (minor fix)
- **Methodology doc** — monthly workflow section still says "flag sites where Voice/Drift/Care deviate" without SMR/FDR/EB
- **Comms plan** — uses "cultural density" not "prevalence + intensity" terminology (acceptable — different audience)

### Current (no changes needed)
- 8 phase briefs (training data docs, not production metrics)
- Model deployment brief (technical)
- Reports README (comprehensive, current methodology)
- Generated reports (auto-regenerated)

### Gaps
- No "how to read the dashboard" guide for non-technical readers
- No reader-facing roadmap (founding plan is internal)
- No PPTX-friendly output format
- No AI narrative report skill
- No document suite index with reading order

### Existing month 1 artefacts (data/qq/cultural-graph/docs/month1-artefacts/)
- `core-message-brief.md` — campaign messaging for Month 1 ("More Than Just a Form")
- `infographic-wireframe.md` + `infographic-v0.1.svg/png` — five composites infographic (design brief + v0.1 draft)
- `qq-banner-brief.md` — QQ platform banner design brief
- `sector-framing.md` — sector-specific lead messages (UKD, AUS, UKI, USA)
- `she-director-email.md` — SHE Director launch email template
- `toolbox-talk-script.md` — 5-minute shopfloor script
- 5 composite icons (`icon-voice.png`, etc.) + survey-vs-report icon
- `preview.html` — browser preview of infographic

## Document Suite Design

### Tier 1: Leadership (5-min reads, PPTX-friendly)

Target: C-suite, SHE Director, board sponsors. Answer: "what is this and why should I care?"

| Doc | File | Status | Updates |
|-----|------|--------|---------|
| L1. Executive Summary | `docs/briefs/cultural-graph-executive-summary.md` | **REWRITE** — stale methodology | Annually or on methodology change |
| L2. Five Composites Infographic | `docs/month1-artefacts/infographic-v0.1.svg` | EXISTS (v0.1) | When composites change |
| L3. Site Data Card | *new — AI-generated per site* | **NEW** — `/cultural-graph-narrative` skill | Monthly after each data load |

### Tier 2: Operational (15-min reads, for site managers)

Target: Site Safety Leads, operational managers. Answer: "what do my site's numbers mean and what should I do?"

| Doc | File | Status | Updates |
|-----|------|--------|---------|
| O1. How to Read the Dashboard | *new* | **NEW** — funnel plots, caterpillar plots, flags, actions | When new chart types added |
| O2. Site Cultural Profiles | `docs/briefs/site-cultural-profiles-brief.md` | **UPDATE** — stale flagging, no prevalence/intensity | Quarterly |
| O3. Monthly Tracker | `reports/monthly-tracker.md` | CURRENT (generated) | After each data load |
| O4. Toolbox Talk Script | `docs/month1-artefacts/toolbox-talk-script.md` | EXISTS (Month 1) | Monthly (7 months) |

### Tier 3: Strategic (for SHE Director / programme sponsors)

Target: Programme sponsors, Group Communications, sector leads. Answer: "where are we going and how do we get there?"

| Doc | File | Status | Updates |
|-----|------|--------|---------|
| S1. Communications Plan | `docs/comms-plan-q3q4-2026.md` | CURRENT (v0.3) | Per release cycle |
| S2. Roadmap | *new* | **NEW** — draws from founding plan + comms plan | Quarterly |
| S3. Methodology | `docs/methodology.md` | **MINOR UPDATE** — workflow section stale | On model/pipeline changes |
| S4. Sector Framing | `docs/month1-artefacts/sector-framing.md` | EXISTS (Month 1) | Monthly |

### Tier 4: Technical (for data team / external review)

Target: Data team, Gemini reviewers, auditors. Answer: "how does this work and is it statistically sound?"

| Doc | File | Status | Updates |
|-----|------|--------|---------|
| T1. Reports README | `reports/README.md` | CURRENT | When methodology changes |
| T2. Phase Briefs (8 files) | `docs/briefs/*-phase{1,2}-brief.md` | CURRENT | Never (historical) |
| T3. Model Deployment Brief | `docs/briefs/cultural-graph-model-deployment-brief.md` | CURRENT | On model changes |
| T4. Gemini Reviews (4 files) | `data/code-review/cultural-graph-*.md` | CURRENT | After each review |

### What updates when

| Trigger | Documents affected |
|---------|-------------------|
| Monthly data load | L3 (AI-generated site cards), O3 (tracker), O4 (toolbox talk) |
| Methodology change | L1 (exec summary), O1 (dashboard guide), T1 (reports README) |
| Quarterly review | O2 (site profiles), S2 (roadmap) |
| New report type added | L2 (infographic), O1 (dashboard guide) |
| Comms cycle (monthly) | S4 (sector framing), O4 (toolbox talk) |

### PPTX conventions

For Tier 1 docs (PPTX-friendly):
- Sections sized to one slide (≤6 bullet points, ≤1 table per section)
- Tables max 6 columns — wider tables split or transposed
- Image references as `![alt](path)` — copy/paste from rendered markdown or PNG
- No nested bullets deeper than 2 levels
- Bold key numbers: `**56% prevalence**`, `**Voice 1.81**`
