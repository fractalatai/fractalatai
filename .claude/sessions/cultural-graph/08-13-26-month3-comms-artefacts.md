---
session: Month 3 Comms Artefacts
status: closed
opened: 2026-08-13
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 3 ("Positive Reports, Powerful Signal") campaign artefacts.
  Core message brief with verified DuckDB data (correcting comms plan "42x" to real
  8x/36x ratios), conversation primer reframed from structured TBT to flexible
  conversation primer, report type cycle infographic (landscape 1600x900 with QinetiQ
  logo), and site data cards with three framings (zero/below/above). All numbers
  verified against pipeline data. Zipped for distribution.

decisions:
  - what: Corrected comms plan "42x" and "3.5x" claims with real DuckDB data
    why: >
      "42x" was a rounding artefact (0.42/0.01 = 42, but precise is 0.427/0.012 = 36x).
      "3.5x recognition" was wrong — real is 25x. Growth vs hazard is 8.5x, not 42x.
    result: Brief uses 8x (vs hazard) and 36x (vs injury) — more honest, still striking
  - what: Reframed toolbox talk as "conversation primer"
    why: >
      TBT implies structured delivery with attendance sign-off. The real need is arming
      people with facts and confidence to raise the topic in whatever setting arises —
      1-to-1, safety moment, customer visit, team brief, or traditional TBT.
    result: No prescribed delivery model, no time budgets, just facts + question + pushback responses
  - what: Used "our" not "your" in infographic punchline
    why: >
      "They're our strongest signal" is more inclusive — the signal belongs to the
      organisation, not just to reporters. Consistent with the complementary framing.
    result: Punchline reads "They're our strongest signal for what's working"
  - what: Three site data card framings (zero, below, above) instead of Month 2's two
    why: >
      12 sites have zero positive observations — fundamentally different from low share.
      Zero sites need QQ access investigation before messaging, not just coaching.
    result: Template includes zero framing with investigation prompt and site list
  - what: AUS quality gap flagged in site data template
    why: >
      MEL_CTN (51% share, 0.98 density) and MSA (57.3%, 0.76 density) file plenty of
      positive obs but they're thin. High share + low density needs quality framing, not
      quantity framing. Different message from the universal "submit more."
    result: AUS-specific note in template with dedicated framing for high-share/low-density sites
  - what: Embedded QinetiQ white-out SVG logo directly in infographic
    why: Brand logos available as SVG in branding directory — no need for placeholder
    result: Logo renders cleanly in dark blue header band

metrics:
  artefacts_produced: { count: 4, formats: "markdown + SVG + PNG" }
  svg_files: { count: 3, total_size: "~30KB" }
  png_files: { count: 3 }
  zip_size: "1.3MB"
  sites_in_data: { total: 39, zero_pos_obs: 12, below_average: 14, above_average: 13 }
  data_corrections: { "42x_to_36x": true, "3.5x_to_25x": true, "voice_074_to_077": true }

lessons:
  - title: Comms plan headline numbers need pipeline verification before artefact production
    detail: >
      The comms plan's "42x" was a rounding artefact and "3.5x recognition" was simply
      wrong (real is 25x). Always pull real numbers from DuckDB before building artefacts.
      The corrected numbers are actually more nuanced and more useful (8x vs hazard is
      a fairer comparison than 42x vs injury).
    tag: methodology
  - title: Conversation primer framing unlocks wider delivery than structured TBT
    detail: >
      Removing prescribed delivery (timing, structure, attendance) and framing as
      "read once, use however" makes the artefact useful in more contexts — 1-to-1,
      safety moments, customer visits, not just formal toolbox talks.
    tag: methodology
  - title: Zero-positive-obs sites are a distinct category from low-share sites
    detail: >
      12 of 39 sites have zero positive observations. These need QQ access
      investigation, not reporting encouragement. Treating them as "below average"
      misses the root cause. Always check for zeros before applying a continuous framing.
    tag: data
  - title: AUS high-share low-density pattern needs quality framing
    detail: >
      Some AUS sites file many positive obs but they're very thin (0.76-0.98
      density vs 2.87 org average for positive obs). The message for these sites
      is quality not quantity — the opposite of the default campaign message.
    tag: data

artifacts:
  - data/qq/cultural-graph/docs/month3-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month3-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month3-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month3-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month3-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month3-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month3-artefacts/infographic-v0.1.png
  - data/qq/cultural-graph/docs/month3-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month3-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month3-artefacts/site-data-card-v0.1.png
  - data/qq/cultural-graph/docs/month3-artefacts.zip

depends_on:
  - 08-12-26-month1-comms-artefacts.md
  - 08-12-26-month2-comms-artefacts.md
  - 08-11-26-org-level-analytics.md

enables:
  - Month 4 artefacts ("Your voice matters — we can prove it")
  - Comms plan data correction (42x → 8x/36x, 3.5x → 25x)
  - Auto-generation script for 39 site data cards from pipeline
---

# Session: Month 3 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 3 — "Positive reports, powerful signal" (November 2026) — as the month that reframes positive observations from "soft reporting" to the most powerful source of cultural evidence. Group must provide core artefacts before site delivery can begin. None existed.

## Todo

- ✅ Create `month3-artefacts/` directory
- ✅ Core message brief — `month3-artefacts/core-message-brief.md`
- ✅ Conversation primer (was "toolbox talk") — `month3-artefacts/toolbox-talk-script.md` + branded SVG/PNG
- ✅ Report type cycle infographic — `month3-artefacts/infographic-wireframe.md` + `infographic-v0.1.svg/.png`
- ✅ Site data — `month3-artefacts/site-data-card-template.md` + `site-data-card-v0.1.svg/.png`
- ✅ Zip for distribution — `month3-artefacts.zip` (1.3MB)

## Dependencies

- ✅ Comms plan v0.3 — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Month 1 artefacts — `data/qq/cultural-graph/docs/month1-artefacts/`
- ✅ Month 2 artefacts — `data/qq/cultural-graph/docs/month2-artefacts/`
- ✅ Branding — `data/qq/cultural-graph/docs/branding/branding-cheatsheet.md` + QinetiQ logos
- ✅ Pipeline data — PowerBI CSV verified against DuckDB

## Data corrections from comms plan

| Comms plan claim | Real data | Correction |
|---|---|---|
| "42x more learning signal" | 36x vs injury, 8.5x vs hazard | Rounding artefact — 0.42/0.01 rounds to 42x but precise is 0.427/0.012 = 36x |
| "3.5x more recognition" | 25x vs hazard (0.25 vs 0.01) | Simply wrong — real number much stronger |
| Voice 0.74→1.64 | 0.77→1.63 | Minor — earlier data pull |
| Growth per positive obs 0.42 | 0.43 | Minor rounding |
| ~20% positive obs share | 20.0% exactly | Confirmed |
