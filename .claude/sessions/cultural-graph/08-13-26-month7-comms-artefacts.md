---
session: Month 7 Comms Artefacts
status: closed
opened: 2026-08-13
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 7 ("Six Months In — What's Changed") campaign artefacts — the final
  month of the 7-month comms campaign. Verified all data against DuckDB. Corrected comms plan
  cultural edges total from 25,286 to 24,149. Designed as a close-the-loop + FY-end plan review
  evidence package. FY2027 partial data used with [TBD] slots for March 2027 update.

decisions:
  - what: FY2026 as baseline, not September 2026
    why: >
      PowerBI CSV has yearly granularity only (no monthly). Campaign runs Sep 2026-Mar 2027
      (second half of FY2027). FY2026 is the natural full-year pre-campaign baseline.
    result: All before/after comparisons use FY2026 vs FY2027, with FY2027 noted as partial
  - what: Corrected total cultural edges from 25,286 to 24,149
    why: Comms plan figure was unverified. DuckDB shows 24,149 cultural edges across 10,625 narratives.
    result: All artefacts use verified 24,149 figure
  - what: Artefacts designed as templates with [TBD] slots for March 2027
    why: >
      FY2027 is partial (5 months, Apr-Aug 2026, pre-campaign). Full-year figures including
      campaign period wont be available until March 2027. Artefacts need updating then.
    result: Core message brief has [TBD] column in comparison table, site cards note partial year
  - what: Separated FY-end plan review framing from workforce messaging
    why: >
      Workforce hears thank you and keep going. Leadership cascade gets the plan review evidence.
      Mixing the two dilutes both messages.
    result: Core message brief has dedicated FY-end section marked for leadership use only
  - what: Density trajectory bars as infographic centrepiece
    why: >
      Cultural density (2.25 to 2.53) tells the dip-and-recovery story that runs through the
      whole campaign. FY2027 being the tallest bar delivers the payoff visually.
    result: Six vertical bars with FY2025 faded (dip) and FY2027 bold (highest ever)
  - what: Three site data card framings based on density change
    why: >
      Density change is the most meaningful single metric for before/after. Top improver
      (>=+0.5), steady contributor (-0.3 to +0.5), needs attention (<=-0.3).
    result: Three framings with different framing boxes, colours, and actions
  - what: Colour-coded direction indicators on site data cards
    why: >
      Four metrics (density, Voice, Care, Leadership) each need independent direction signals.
      Green/grey/amber arrows make the card instantly scannable.
    result: SVG arrows and value colours per metric per site

metrics:
  artefacts_produced: { count: 4, formats: markdown + SVG + PNG }
  svg_files: { count: 3 }
  png_files: { count: 3 }
  zip_size: 1.3MB
  total_cultural_edges: 24149
  total_narratives: 10625
  fy2027_density: 2.53
  fy2027_voice: 1.24
  fy2027_leadership: 0.37
  fy2027_care: 0.55
  composites_rising: 2
  composites_stable: 2
  composites_watch: 1
  sites_top_improver: 4
  sites_steady: 7
  sites_needs_attention: 6
  campaign_months_completed: 7

lessons:
  - title: Comms plan total cultural edges was wrong (25,286 vs actual 24,149)
    detail: >
      Fourth data correction across the campaign. Every comms plan figure needs DuckDB
      verification before use. The pattern is consistent — approximate figures in planning
      docs, precise figures needed for artefacts. Always verify.
    tag: data
  - title: FY-level data cannot support within-year before/after comparisons
    detail: >
      The comms plan asked for Sept 2026 vs Mar 2027 comparison, but the PowerBI CSV only
      has FY-level granularity. The workaround (FY2026 baseline vs FY2027 full year) works
      but loses the within-campaign signal. If monthly data were available, the comparison
      would be much sharper. Flag this for future data pipeline design.
    tag: data
  - title: Partial year data makes site-level changes unreliable below n=50
    detail: >
      FY2027 has only 5 months of data. Several sites have n<15 in FY2027. Density changes
      of +2.48 (WFH, n=11) look dramatic but are likely noise. Always flag sample size
      prominently on site data cards and use indicative only language for small n.
    tag: methodology
  - title: Campaign arc table is the most powerful framing device for close-out
    detail: >
      Showing all 7 months, 7 asks in a single table makes the campaign structure visible
      for the first time. Each ask was one sentence — that simplicity is the story. Used in
      both the primer (as a bordered box) and the core message brief (as a table). Most
      effective element in the close-out artefacts.
    tag: methodology
  - title: Drift rising needs careful framing — not a simple negative
    detail: >
      Drift went from 0.18 to 0.22. Rising Drift could mean worsening culture (more
      normalisation) OR better detection (people now describe adaptive behaviour they
      previously wouldnt mention). The brief explicitly frames this as watch and provides
      the alongside other composites interpretation guide.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month7-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month7-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month7-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month7-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month7-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month7-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month7-artefacts/infographic-v0.1.png
  - data/qq/cultural-graph/docs/month7-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month7-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month7-artefacts/site-data-card-v0.1.png
  - data/qq/cultural-graph/docs/month7-artefacts.zip

depends_on:
  - 08-13-26-month6-comms-artefacts.md

enables:
  - Safety improvement plan review evidence (March 2027)
  - FY2027 full-year data update (replace [TBD] slots)
  - Post-campaign Cultural Graph reporting baseline
---

# Session: Month 7 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 7 — "Six months in — what's changed" (March 2027) — as the final Sustain month, closing the loop on the 7-month campaign. The artefacts need a Sept 2026 vs Mar 2027 comparison: report volume, penetration, positive observation share, cultural density, and all five composite trends. This month also coincides with QQ's financial year end and the closure/review of the safety improvement plan that this comms campaign sits within — the artefacts may need to serve double duty as evidence for the plan review.

## Todo

- ✅ Create `month7-artefacts/` directory
- ✅ Core message brief — `month7-artefacts/core-message-brief.md`
- ✅ Conversation primer — `month7-artefacts/toolbox-talk-script.md` + branded SVG/PNG
- ✅ Campaign summary infographic — `month7-artefacts/infographic-wireframe.md` + `infographic-v0.1.svg/.png`
- ✅ Site data cards — `month7-artefacts/site-data-card-template.md` + `site-data-card-v0.1.svg/.png`

## Dependencies

- ✅ Comms plan v0.3 — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Month 6 artefacts complete — `data/qq/cultural-graph/docs/month6-artefacts/`
- ✅ Months 3-6 artefacts complete — established branding, format, data verification patterns
- ✅ Branding guidelines — `data/qq/cultural-graph/docs/branding/`
- ✅ Cultural Graph data — PowerBI CSV + DuckDB pipeline data
- ✅ FY2026 vs FY2027 comparison figures verified against DuckDB (FY2027 partial — [TBD] slots for March update)
