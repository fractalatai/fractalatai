---
session: Month 5 Comms Artefacts
status: closed
opened: 2026-08-13
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 5 ("Everyone's View Counts") campaign artefacts. Used sector
  headcounts (UKD ~3K, AUS ~500, UKI ~1.65K, USA ~850, HO ~900) to calculate real
  per-capita rates. Confirmed comms plan 0.3 (real: 0.32). Descoped UKI/USA/Head Office
  from campaign — structural barriers not addressable by comms. Campaign targets UKD
  (0.68/person, nudge) and AUS (0.25/person, encouragement + value). New Year hook
  throughout. 10×10 dot grid infographic.

decisions:
  - what: Descoped UKI, USA, and Head Office from participation campaign
    why: >
      UKI is equivalent to a tech company (1,650 office workers, 24 reports/yr).
      USA is behind trade restrictions and not on QQ. Head Office has low-risk
      occupations and reports are hard to separate from UKD/UKI — data quality issue.
      A comms campaign cannot address structural barriers.
    result: Campaign targets UKD and AUS only — sectors with operational sites and comparable risk profiles
  - what: Per-capita rates calculated from sector headcounts
    why: No site-level headcount available, but sector totals provided by user
    result: >
      Org-wide 0.32/person (confirming comms plan 0.3). UKD 0.68 (68% of target),
      AUS 0.25 (25%), UKI 0.01, USA 0.03, HO 0.00
  - what: Conversation primer reframed for line managers, not reporters
    why: >
      Month 5 audience shifts to non-reporters. The people who need to act are line
      managers inviting non-reporters, not reporters improving quality. Previous months'
      primers were for anyone — this one names line managers in the meta bar.
    result: Who to Invite section with four groups and ready-made one-line openers
  - what: New Year hook as structural element across all artefacts
    why: Month 5 lands in January — natural fresh-start energy for "submit your first report"
    result: New year new voice tagline, this year framing, resolution-style ask
  - what: 10×10 dot grid for participation infographic
    why: >
      32 purple dots and 68 grey makes the gap visceral — more impactful than a bar
      chart or percentage. Each dot represents a person, making absence personal.
    result: Most distinctive visual in the campaign — immediately communicates who's missing

metrics:
  artefacts_produced: { count: 4, formats: "markdown + SVG + PNG" }
  svg_files: { count: 3 }
  png_files: { count: 3 }
  zip_size: "1.1MB"
  org_headcount: 6900
  org_rate_fy2026: 0.32
  ukd_rate: 0.68
  aus_rate: 0.25
  gap_to_target: "~4,700 org-wide, ~1,300 for UKD+AUS only"
  sites_below_10_per_year: 22

lessons:
  - title: Headline participation rates mask structural sector differences
    detail: >
      Org-wide 0.32/person looks uniform but UKD is at 0.68 while UKI is at 0.01.
      The gap isn't motivational — it's structural (QQ access, role expectations,
      trade restrictions). Always break down by sector before designing interventions.
    tag: data
  - title: Sector headcounts turn absolute volumes into actionable rates
    detail: >
      Without headcounts, 2,039 UKD reports looks impressive. With headcounts,
      0.68/person shows UKD is 68% to target — a nudge away. The rate reframes
      the conversation from "how much" to "how close." Always seek denominators.
    tag: methodology
  - title: Dot grids are more impactful than bar charts for participation gaps
    detail: >
      A bar at 32% is abstract. 32 coloured dots in a 10×10 grid is visceral —
      each grey dot is a person whose perspective is missing. For audience-facing
      infographics about people, represent people as discrete units not continuous bars.
    tag: methodology
  - title: Text in SVG callout boxes should fill the available width
    detail: >
      First render had text wrapping at ~65 chars leaving the right half of 738px
      boxes empty. Reflowed to ~100 chars per line to fill the width. Always check
      text utilisation in boxed SVG elements — short lines waste space and look
      unfinished.
    tag: tooling

artifacts:
  - data/qq/cultural-graph/docs/month5-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month5-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month5-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month5-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month5-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month5-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month5-artefacts/infographic-v0.1.png
  - data/qq/cultural-graph/docs/month5-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month5-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month5-artefacts/site-data-card-v0.1.png
  - data/qq/cultural-graph/docs/month5-artefacts.zip

depends_on:
  - 08-13-26-month4-comms-artefacts.md
  - 08-13-26-month3-comms-artefacts.md

enables:
  - Month 6 artefacts ("What happens next matters")
  - Month 7 artefacts ("Six months in — what's changed")
  - Line manager participation action in UKD and AUS sites
---

# Session: Month 5 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 5 — "Everyone's view counts" (January 2027) — as the first "Broaden" month, shifting focus from report quality to participation. The target audience changes: non-reporters, new starters, underrepresented roles (admin, logistics, stores). The key data point is 0.3 reports per person per year — most people never report. Sector headcounts were provided to calculate real per-capita rates.

## Todo

- ✅ Create `month5-artefacts/` directory
- ✅ Core message brief — `month5-artefacts/core-message-brief.md`
- ✅ Conversation primer — `month5-artefacts/toolbox-talk-script.md` + branded SVG/PNG
- ✅ Participation infographic — `month5-artefacts/infographic-wireframe.md` + `infographic-v0.1.svg/.png`
- ✅ Site data — `month5-artefacts/site-data-card-template.md` + `site-data-card-v0.1.svg/.png`
- ✅ Zip — `month5-artefacts.zip`

## Dependencies

- ✅ Comms plan v0.3
- ✅ Month 4 artefacts complete
- ✅ Branding guidelines
- ✅ Cultural Graph data — PowerBI CSV + DuckDB
- ✅ Sector headcounts — UKD ~3K, AUS ~500, UKI ~1.65K, USA ~850, HO ~900 (user-provided)
