---
session: Month 2 Comms Artefacts
status: closed
opened: 2026-08-12
closed: 2026-08-13
outcome: success

summary: >
  Created all four Month 2 ("Tell Us Who, Not Just What") campaign artefacts for
  the Cultural Graph comms plan. Core message brief, thin/rich example pair card
  (wireframe + SVG), toolbox talk script (markdown + branded PNG), and site data
  card template with both below/above-average framings. Ready for Comms team to
  refine and design services to produce final assets.

decisions:
  - what: Two framings for site data cards (below/above average) with no traffic-light colours
    why: Brand cheatsheet prohibits excessive colours; red/amber/green implies judgement which contradicts the campaign's invitational tone
    result: Dark blue bar for below-average, purple for above — both brand colours, neither implies good/bad
  - what: Discussion-led toolbox talk format with 2-minute group discussion
    why: Month 2 is the first "Invite" phase — the group discovering the thin/rich difference themselves is more effective than being told
    result: Script budgets 2 min for discussion with prompts for quiet groups
  - what: Three one-line message options carried forward from Month 1 pattern
    why: Comms team prefers to choose the hook that fits their channel — gives them agency without requiring a separate brief
    result: Three options — "One sentence" (achievable ask), "the gap" (curiosity), "unique evidence" (reporter agency)
  - what: Site data card gauge uses 0–4.5 scale with org average marker at 2.3
    why: Observed range is 0.8–4.1; 4.5 provides headroom without compressing the visual
    result: Gauge renders clearly for both extremes (ASH 1.3 and PEN 4.1)

metrics:
  artefacts_produced: { count: 4, formats: "markdown + SVG + PNG" }
  svg_files: { count: 3, rendered_to_png: 2 }
  site_data_card_framings: { below_average: 1, above_average: 1, priority_sites: 4 }

lessons:
  - title: Text-only SVGs convert to PNG cleanly via ImageMagick magick command
    detail: >
      Unlike Month 1 where embedded base64 images broke ImageMagick rendering,
      text-only SVGs convert perfectly at 150 DPI. The magick command (IMv7)
      works; the deprecated convert command also works but warns. No need for
      browser-based screenshot workflows for text/shape SVGs.
    tag: tooling
  - title: Cultural density field is avg_cultural_per_narrative in PowerBI export, AVG(cultural_edge_count) in DuckDB
    detail: >
      Two different field names for the same concept depending on context.
      Pipeline scripts use AVG(cultural_edge_count) from the narratives table.
      PowerBI CSV export uses avg_cultural_per_narrative. Org average is 2.26
      (rounded to 2.3 for comms). Sites filtered to >=10 narratives.
    tag: data
  - title: Branding cheatsheet enables consistent visual output without design iteration
    detail: >
      Having the hex codes, font names, and do/don't rules in a single
      markdown file meant all SVGs were on-brand first time. The elevation
      line accent (10° angle) and the dark blue + purple palette carried
      through all three SVGs consistently. Worth creating cheatsheets for
      any recurring visual work.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month2-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month2-artefacts/example-pair-wireframe.md
  - data/qq/cultural-graph/docs/month2-artefacts/example-pair-v0.1.svg
  - data/qq/cultural-graph/docs/month2-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month2-artefacts/toolbox-talk-v0.1.svg
  - data/qq/cultural-graph/docs/month2-artefacts/toolbox-talk-v0.1.png
  - data/qq/cultural-graph/docs/month2-artefacts/site-data-card-template.md
  - data/qq/cultural-graph/docs/month2-artefacts/site-data-card-v0.1.svg
  - data/qq/cultural-graph/docs/month2-artefacts/site-data-card-v0.1.png
  - data/qq/cultural-graph/docs/month2-artefacts/preview.html

depends_on:
  - 08-12-26-month1-comms-artefacts.md
  - 08-11-26-org-level-analytics.md

enables:
  - Month 3 artefacts ("Positive reports, powerful signal")
  - Auto-generation script for 68 site data cards from DuckDB pipeline
  - Design services brief for example pair card and digital screen content
---

# Session: Month 2 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan defines Month 2 — "Tell us who, not just what" (October 2026) — as the month that introduces cultural density and teaches reporters how to write richer narratives. Group must provide four core artefacts before site delivery can begin. The emphasis is on showing, via concrete example pairs, the difference between a thin report and a rich one. Sites below org-average density receive targeted data cards. None of these artefacts exist yet.

## Todo

- ✅ Core message brief — `month2-artefacts/core-message-brief.md`
- ✅ Thin/rich example pair — wireframe (`month2-artefacts/example-pair-wireframe.md`) + SVG draft (`month2-artefacts/example-pair-v0.1.svg`)
- ✅ Toolbox talk script — `month2-artefacts/toolbox-talk-script.md` + branded PNG (`toolbox-talk-v0.1.png`)
- ✅ Site data cards template — spec (`month2-artefacts/site-data-card-template.md`) + SVG/PNG with both framings (`site-data-card-v0.1.svg/.png`)

## Dependencies

- ✅ Comms plan v0.3 — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Month 1 artefacts complete — `data/qq/cultural-graph/docs/month1-artefacts/`
- ✅ Cultural Graph data available — cultural density range 0.8–3.7 edges/narrative, org average 2.3
- ✅ Branding guidelines — `data/qq/cultural-graph/docs/branding-cheatsheet.md` provided and used throughout
