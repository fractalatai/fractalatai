---
session: Month 4 Comms Artefacts
status: closed
opened: 2026-08-13
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 4 ("Your Voice Matters — We Can Prove It") campaign artefacts.
  Verified Voice trend data against DuckDB, correcting comms plan figures (1.01→1.21 to
  real 1.06→1.24, 4,455 to 4,305). Major reframe: ASH reclassified from concern to
  standout improvement story (0.27→1.26), BRS "statistically declining" claim flagged
  as unsupported (n=17).

decisions:
  - what: Corrected Voice trend from 1.01→1.21 to 1.06→1.24
    why: DuckDB data shows FY2022 baseline was 1.06 not 1.01, FY2027 is 1.24 not 1.21
    result: Real trend is stronger than comms plan claimed — better story
  - what: Corrected speaks-up-to from 4,455 to 4,305
    why: Pipeline recount — 150 fewer than comms plan figure
    result: Minor correction, number still compelling
  - what: Reframed ASH from concern to improvement story
    why: >
      All-years average (0.61) masked dramatic improvement. FY-by-FY trend is
      0.27→0.29→0.44→0.62→1.24→1.26. ASH now above org average in FY2026-27.
    result: Recommended as anonymised campaign example — strongest improvement in dataset
  - what: Flagged BRS "statistically declining" as unsupported
    why: Only 17 narratives across 3 years (7+3+7). Variation 0.86→1.0→0.57 is noise with n this small.
    result: Recommended volume conversation with site management instead of Voice quality conversation
  - what: Added Voice component breakdown to all artefacts
    why: >
      Voice has three components (shares_info 44%, speaks_up 37%, cooperates 19%)
      and this is the first month with enough depth to show them. Adds granularity
      for site safety leads who want to understand which behaviour to coach.
    result: Component breakdown in brief, primer, infographic right panel, and site data card row
  - what: Bar chart infographic with dip-and-recovery narrative annotations
    why: >
      First month using longitudinal trend as the hook. The FY2025 dip and recovery
      is the strongest narrative arc in the campaign. Visual design needed to tell
      that story — lighter dip bar, purple recovery bars, curved arrow annotation.
    result: 6-bar chart with annotations + component breakdown panel

metrics:
  artefacts_produced: { count: 4, formats: "markdown + SVG + PNG" }
  svg_files: { count: 3 }
  png_files: { count: 3 }
  sites_in_data: { total: 39, below_avg: 18, above_avg: 21 }
  data_corrections: { voice_trend: "1.01-1.21 → 1.06-1.24", speaks_up: "4455 → 4305", ash_reframe: true, brs_flagged: true }
  priority_sites: { MHA: "0.33, improving slowly", ASH: "0.61 all-years but 1.24-1.26 recent", BRS: "0.76, n=17 only" }

lessons:
  - title: All-years averages can mask dramatic site-level improvement trajectories
    detail: >
      ASH's all-years Voice average (0.61) put it on the concern list. But FY-by-FY
      data shows 0.27→1.26 — the strongest improvement in the dataset, now above org
      average. Always check per-year trends before accepting an all-years figure as the
      story. The comms plan was written from aggregated data that hid the trajectory.
    tag: data
  - title: Small sample sizes invalidate statistical trend claims
    detail: >
      BRS was flagged for a "statistically significant" Voice decline (-0.167/year).
      But BRS has only 17 narratives total across 3 years. No statistical method can
      reliably detect a trend from 7+3+7 observations. Always check n before accepting
      a trend claim — the comms plan didn't.
    tag: methodology
  - title: Bar chart dip-and-recovery is a stronger narrative than a line chart
    detail: >
      The FY2025 dip bar rendered as lighter/dashed stands out visually in a way that
      a line dip doesn't. The curved arrow from dip to recovery, plus "dip" and
      "recovery" text annotations, tells the story without needing surrounding prose.
      For trend data with a narrative arc, bar charts with annotations beat line charts.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month4-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month4-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month4-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month4-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month4-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month4-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month4-artefacts/infographic-v0.1.png
  - data/qq/cultural-graph/docs/month4-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month4-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month4-artefacts/site-data-card-v0.1.png

depends_on:
  - 08-13-26-month3-comms-artefacts.md
  - 08-12-26-month1-comms-artefacts.md
  - 08-11-26-org-level-analytics.md

enables:
  - Month 5 artefacts ("Everyone's view counts")
  - Comms plan data correction (Voice trend, speaks-up count, ASH reframe, BRS caveat)
  - Auto-generation script for 39 site Voice data cards from pipeline
---

# Session: Month 4 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 4 — "Your voice matters — we can prove it" (December 2026) — as the month that shows Voice trending upward across the organisation (1.01→1.21 over six years), with a dip in FY2025 that recovered. This is the first month using longitudinal trend data as the hook. Three sites need targeted intervention: MHA (Voice 0.33), ASH (Voice 0.61 across 875 reports), and BRS (statistically declining Voice). Group must provide core artefacts before site delivery. None exist yet. Comms plan Voice figures need verification against DuckDB (as with Month 3).

## Todo

- ✅ Create `month4-artefacts/` directory
- ✅ Core message brief — `month4-artefacts/core-message-brief.md`
- ✅ Conversation primer — `month4-artefacts/toolbox-talk-script.md` + branded SVG/PNG
- ✅ Voice trend infographic — `month4-artefacts/infographic-wireframe.md` + `infographic-v0.1.svg/.png`
- ✅ Site data cards — `month4-artefacts/site-data-card-template.md` + `site-data-card-v0.1.svg/.png`

## Dependencies

- ✅ Comms plan v0.3 — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Month 3 artefacts complete — `data/qq/cultural-graph/docs/month3-artefacts/`
- ✅ Branding guidelines — `data/qq/cultural-graph/docs/branding/`
- ✅ Cultural Graph data — PowerBI CSV + DuckDB pipeline data
- ✅ Voice trend figures verified — 1.01→1.21 corrected to 1.06→1.24, 4,455 to 4,305
