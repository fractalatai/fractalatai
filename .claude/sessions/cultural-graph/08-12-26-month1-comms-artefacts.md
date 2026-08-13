---
session: Month 1 Comms Artefacts
status: closed
opened: 2026-08-12
closed: 2026-08-13
outcome: success

summary: >
  Created all six Month 1 ("More Than Just a Form") campaign artefacts, then
  reopened to produce v0.2 infographic with QinetiQ branding. Regenerated all 6
  icons in keyline/outline style (single purple, consistent stroke weight) via
  Gemini, rebuilt SVG with brand palette, Manrope font, and elevation line accent.
  v0.2 SVG is 84KB vs 2MB v0.1.

decisions:
  - what: Used Gemini 3.1 Flash Lite Image for icon generation
    why: Cheapest model available, produced clean flat icons at ~1,500 tokens each
    result: 6 icons generated for ~9,000 total tokens (negligible cost)
  - what: SVG format for infographic
    why: Text-based (writable without design tools), embeds raster icons as base64, scalable, editable by designers in Illustrator/Figma
    result: 84KB SVG with all icons embedded, renders correctly in browser
  - what: Regenerated Voice icon for diversity and Leadership icon for servant leadership
    why: Original Voice showed two identical male silhouettes (inclusion concern). Original Leadership showed pointing/commanding figure (autocratic, not servant leadership).
    result: Voice now shows diverse pair in equal dialogue. Leadership shows figure walking alongside colleagues with supportive hand on shoulder.
  - what: Three one-line message options instead of one
    why: Original line ("we can now read it") implied reports weren't read before. Reframed as patterns-at-scale, behaviours-at-scale, and measure-what-was-always-there hooks for Comms team to choose.
    result: Three options with labelled reasoning in core message brief
  - what: M&L subsector label corrected to Maritime & Land
    why: User corrected from "Materials & Logistics" to actual QinetiQ subsector name
    result: sector-framing.md updated
  - what: Switched from filled/silhouette multi-colour icons to keyline/outline single-colour icons
    why: QinetiQ brand guide specifies keyline icons in one colour, matching wayfinding/signage style. Multi-colour filled icons violated "colours should not be mixed within a particular graphic" rule.
    result: All 6 icons regenerated in purple #9A258F outline style, consistent with brand icon set
  - what: Resized icons to 200x200 before base64 embedding
    why: Gemini generates at 1024px, producing 180-340KB per icon. At 200x200, icons are 4-15KB each, reducing SVG from ~1.75MB embedded to 84KB total.
    result: 24x reduction in SVG file size with no visible quality loss at infographic scale

metrics:
  artefacts_produced: { count: 6, format: "markdown + SVG + PNG icons" }
  gemini_image_calls: { total: 14, v01: 8, v02: 6, model: "gemini-3.1-flash-lite-image" }
  icons_generated: { v01: 6, v02: 6, total: 12, regenerated_in_v01: 2 }
  svg_size: { v01: "2MB", v02: "84KB" }

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
  - title: Gemini lite produces on-brand keyline icons first time with explicit style prompts
    detail: >
      Including "OUTLINE ONLY, no fills, thin consistent stroke weight about 3px,
      single colour purple #9A258F" plus "wayfinding icon style like WC signs"
      reliably produced keyline icons matching the QinetiQ brand guide. All 6
      v0.2 icons were usable without regeneration. The wayfinding reference
      anchors the style more effectively than "minimalist" alone.
    tag: tooling
  - title: Resize Gemini icons before base64 embedding to control SVG size
    detail: >
      Gemini generates at 1024px (180-340KB per icon). Embedding 6 raw icons
      produces a ~1.75MB SVG. Resizing to 200x200 via magick first drops each
      icon to 4-15KB, producing an 84KB SVG. No visible quality loss at
      infographic scale. Always resize before embedding.
    tag: methodology

artifacts:
  - data/qq/cultural-graph/docs/month1-artefacts/core-message-brief.md
  - data/qq/cultural-graph/docs/month1-artefacts/infographic-wireframe.md
  - data/qq/cultural-graph/docs/month1-artefacts/infographic-v0.1.svg
  - data/qq/cultural-graph/docs/month1-artefacts/infographic-v0.2.svg
  - data/qq/cultural-graph/docs/month1-artefacts/icon-voice.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-voice-v2.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-leadership.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-leadership-v2.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-drift.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-drift-v2.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-care.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-care-v2.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-growth.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-growth-v2.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-survey-vs-report.png
  - data/qq/cultural-graph/docs/month1-artefacts/icon-survey-vs-report-v2.png
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

## v0.2 Infographic — QinetiQ Branding

- ✅ Regenerate 6 icons in QinetiQ keyline style — `icon-*-v2.png` (outline only, single purple #9A258F, consistent stroke)
- ✅ Rebuild SVG with brand palette — `infographic-v0.2.svg` (Dark Blue #002744, Purple #9A258F, Manrope font, elevation line, 10% tint card backgrounds)
- ✅ Embed new icons (resized to 200x200 for 84KB SVG vs 2MB v0.1)

## Outputs

All artefacts written to `data/qq/cultural-graph/docs/month1-artefacts/`.
