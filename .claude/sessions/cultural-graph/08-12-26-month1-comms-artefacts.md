---
session: Month 1 Comms Artefacts
status: closed
opened: 2026-08-12
closed: 2026-08-12
outcome: success

summary: >
  Created all six Month 1 ("More Than Just a Form") campaign artefacts for
  the Cultural Graph comms plan. Core message brief, five composites infographic
  (SVG with Gemini-generated icons), toolbox talk script, SHE Director email
  template, QQ banner brief, and sector framing document. Ready for Comms
  team to refine and design services to produce final assets.

decisions:
  - what: Used Gemini 3.1 Flash Lite Image for icon generation
    why: Cheapest model available, produced clean flat icons at ~1,500 tokens each
    result: 6 icons generated for ~9,000 total tokens (negligible cost)
  - what: SVG format for infographic
    why: Text-based (writable without design tools), embeds raster icons as base64, scalable, editable by designers in Illustrator/Figma
    result: 2MB SVG with all icons embedded, renders correctly in browser
  - what: Regenerated Voice icon for diversity and Leadership icon for servant leadership
    why: Original Voice showed two identical male silhouettes (inclusion concern). Original Leadership showed pointing/commanding figure (autocratic, not servant leadership).
    result: Voice now shows diverse pair in equal dialogue. Leadership shows figure walking alongside colleagues with supportive hand on shoulder.
  - what: Three one-line message options instead of one
    why: Original line ("we can now read it") implied reports weren't read before. Reframed as patterns-at-scale, behaviours-at-scale, and measure-what-was-always-there hooks for Comms team to choose.
    result: Three options with labelled reasoning in core message brief
  - what: M&L subsector label corrected to Maritime & Land
    why: User corrected from "Materials & Logistics" to actual QinetiQ subsector name
    result: sector-framing.md updated

metrics:
  artefacts_produced: { count: 6, format: "markdown + SVG + PNG icons" }
  gemini_image_calls: { count: 8, model: "gemini-3.1-flash-lite-image", total_tokens: ~12000 }
  icons_generated: { count: 6, regenerated: 2 }

lessons:
  - title: Gemini interactions endpoint works for cheap icon generation
    detail: >
      The /v1beta/interactions endpoint with gemini-3.1-flash-lite-image produces
      clean flat icons at ~1,500 tokens per image. Response structure is
      steps[1].content[0].data (base64 JPEG). Good enough for Aunt Sally drafts
      that design teams can refine. Much cheaper than flash or pro image models.
    tag: tooling
  - title: ImageMagick cannot render SVG with embedded base64 images
    detail: >
      `convert` (IMv7) renders SVG text and shapes but drops embedded base64
      image elements. The SVG renders correctly in browsers. For previewing
      SVGs with embedded images, serve via HTTP and open in browser rather
      than converting to PNG.
    tag: infrastructure
  - title: Logo download sites block direct curl requests
    detail: >
      Wikimedia, seeklogo, companieslogo all returned HTML instead of images
      when fetched via curl, even with User-Agent headers. Logo assets need
      to be downloaded manually via browser or sourced from internal brand
      assets.
    tag: infrastructure
  - title: Diversity and leadership framing matter for comms artefacts
    detail: >
      Icon imagery carries implicit messages. Two identical male silhouettes
      sends the wrong signal on inclusion. A pointing/commanding figure
      contradicts servant leadership. Always review generated imagery
      through the lens of the comms team's concerns before presenting.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month1-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month1-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month1-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month1-artefacts/icon-voice.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-leadership.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-drift.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-care.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-growth.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-survey-vs-report.png
  - data/qq/cultural-graph/docs/month1-artefacts/toolbox-talk-script.md
  - data/qq/cultural-graph/docs/month1-artefacts/she-director-email.md
  - data/qq/cultural-graph/docs/month1-artefacts/qq-banner-brief.md
  - data/qq/cultural-graph/docs/month1-artefacts/sector-framing.md

depends_on:
  - 08-11-26-org-level-analytics.md

enables:
  - Month 2 artefacts ("Tell us who, not just what")
  - Comms team integrated comms plan development
  - Design services brief for infographic and QQ banner production
---

# Session: Month 1 Comms Artefacts (CLOSED)

## Problem

The Cultural Graph comms plan (v0.3) defines Month 1 — "More Than Just a Form" (September 2026) as the campaign launch. Group must provide five core artefacts plus sector-specific framing before site delivery can begin. None of these exist yet.

## Todo

- ✅ Core message brief — `month1-artefacts/core-message-brief.md`
- ✅ Five composites infographic — wireframe (`month1-artefacts/infographic-wireframe.md`) + 6 icons generated via Gemini
- ✅ Toolbox talk script — `month1-artefacts/toolbox-talk-script.md`
- ✅ SHE Director email template — `month1-artefacts/she-director-email.md`
- ✅ QQ banner asset — `month1-artefacts/qq-banner-brief.md`
- ✅ Sector framing — `month1-artefacts/sector-framing.md`

## Dependencies

- ✅ Comms plan v0.3 endorsed — `data/qq/cultural-graph/docs/comms-plan-q3q4-2026.md`
- ✅ Cultural Graph data available — 11,170 narratives, 25,286 edges, 68 sites, 5 composites
- ✅ Executive summary — `data/qq/cultural-graph/outputs/briefs/cultural-graph-executive-summary.md`
- ⬜ Design budget confirmed for infographic and QQ banner (comms team flagged this as safety budget)

## Outputs

All artefacts written to `data/qq/cultural-graph/docs/month1-artefacts/`.
