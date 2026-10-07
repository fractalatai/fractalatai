---
session: Month 6 Comms Artefacts
status: closed
opened: 2026-08-13
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 6 ("What Happens Next Matters") campaign artefacts. Verified Care and
  Leadership data against DuckDB — corrected injury cares-for from comms plan 0.145 to 0.242.
  Central insight: the injury gap (cares-for 0.24 highest but Leadership only 0.14, less than
  half near-miss rate). Paired bar chart infographic makes the response gap visually immediate.
  Site data cards use Care/Leadership bars with org avg markers for three framings.

decisions:
  - what: Injury gap as central teaching point, not overall Care rate
    why: >
      Care 0.53 org average is stable and unremarkable. The injury gap — cares-for 0.24 highest
      but Leadership 0.14 lowest — tells a specific, actionable story about what gets captured.
    result: All artefacts frame around the response is the culture, not the caring rate
  - what: Paired bar chart (Care vs Leadership by report type) as main infographic visual
    why: >
      Four report types with two bars each makes the injury gap visually stark. Also reveals
      the positive obs reversal (Leadership 0.51 > Care 0.45), adding visual interest.
    result: Left panel paired bars, right panel zooms into injury gap with components
  - what: Three site data card framings based on Care and Leadership vs org average
    why: >
      Two dimensions (Care and Leadership) create natural categories — strong response (both
      above), response gap (Care above, Leadership below), building the response (Care below).
    result: Three framings with different framing boxes and actions
  - what: Org Leadership average is 0.34, not 0.33
    why: >
      Comms plan cited near-miss Leadership 0.33 as a headline. Actual org-wide weighted
      average is 0.34. Used 0.34 for site data card org avg markers.
    result: Near-miss 0.33 used in workforce materials, org 0.34 used in site cards

metrics:
  artefacts_produced: { count: 4, formats: markdown + SVG + PNG }
  svg_files: { count: 3 }
  png_files: { count: 3 }
  zip_size: 1.2MB
  org_care_rate: 0.53
  org_leadership_rate: 0.34
  injury_cares_for: 0.242
  injury_leadership: 0.14
  nearmiss_care: 0.63
  nearmiss_leadership: 0.33
  sites_strong_response: 12
  sites_response_gap: 4
  sites_building_response: 8

lessons:
  - title: Comms plan injury cares-for figure was wrong (0.145 vs actual 0.242)
    detail: >
      The comms plan cited injury cares-for as 0.145. DuckDB verification showed 0.242 —
      nearly double. Always verify before using comms plan figures in artefacts. The correction
      actually strengthened the story (caring even higher than claimed, making the Leadership
      gap more dramatic).
    tag: data
  - title: Positive obs Leadership > Care reversal adds visual interest to paired bars
    detail: >
      In the infographic, positive obs is the one report type where Leadership (0.51) exceeds
      Care (0.45). This breaks the visual pattern and draws attention, showing that the
      relationship between Care and Leadership isnt uniform. Worth annotating rather than hiding.
    tag: methodology
  - title: Org avg dashed lines on site data card bars make relative position instantly clear
    detail: >
      Adding a dashed org avg marker line on each bar (Care avg 0.53, Leadership avg 0.34) lets
      site safety leads see at a glance whether their site is above or below on each dimension.
      More effective than stating the comparison in text alone.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month6-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month6-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month6-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month6-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month6-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month6-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month6-artefacts/infographic-v0.1.png
  - data/qq/cultural-graph/docs/month6-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month6-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month6-artefacts/site-data-card-v0.1.png
  - data/qq/cultural-graph/docs/month6-artefacts.zip

depends_on:
  - 08-13-26-month5-comms-artefacts.md

enables:
  - Month 7 artefacts (Six months in — what changed)
  - Supervisor engagement on injury report response capture
---

# Session: Month 6 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 6 — "What happens next matters" (February 2027) — as the second Broaden month, focusing on Care and Leadership composites in near-miss and injury reports. The key insight: many reports describe the incident but not the response. "What happened next" is often missing. The comms plan cites Care 0.53 org average, near-miss Leadership 0.33, and injury cares-for 0.145 — all need verification against DuckDB. This is the penultimate month before the close-the-loop summary.

## Todo

- ✅ Create `month6-artefacts/` directory
- ✅ Core message brief — `month6-artefacts/core-message-brief.md`
- ✅ Conversation primer — `month6-artefacts/toolbox-talk-script.md` + branded SVG/PNG
- ✅ Care infographic — `month6-artefacts/infographic-wireframe.md` + `infographic-v0.1.svg/.png`
- ✅ Site data cards — `month6-artefacts/site-data-card-template.md` + `site-data-card-v0.1.svg/.png`

## Dependencies

- ✅ Comms plan v0.3 — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Month 5 artefacts complete — `data/qq/cultural-graph/docs/month5-artefacts/`
- ✅ Branding guidelines — `data/qq/cultural-graph/docs/branding/`
- ✅ Cultural Graph data — PowerBI CSV + DuckDB pipeline data
- ✅ Care/Leadership figures verified against DuckDB
