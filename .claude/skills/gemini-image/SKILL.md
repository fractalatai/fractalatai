---
description: Generate images via the Gemini API. Use when the user asks to create icons, infographic assets, diagrams, or any image via Gemini.
---

# Gemini Image Generation

## When This Applies

When the user wants to:
- Generate icons, logos, or infographic assets
- Create visual assets for comms materials, presentations, or documentation
- Edit or restyle an existing image
- Produce any image via the Gemini API

## Prerequisites

- `GEMINI_API_KEY` must be set. Source from `~/.bashrc` if not in the current environment:
  ```bash
  source ~/.bashrc
  ```

## Usage

### Generate a single image

```bash
/usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/gemini-image/scripts/generate.py \
  --prompt "A flat minimal icon of two people talking with a speech bubble" \
  --output /path/to/icon.png
```

### Options

| Flag | Values | Default | Purpose |
|------|--------|---------|---------|
| `--prompt` | text | (required) | Image generation prompt |
| `--output` | path | (required) | Output file path |
| `--model` | `lite`, `flash`, `pro` | `lite` | Model tier (see costs below) |
| `--input` | path | none | Input image for editing |
| `--size` | `1K`, `2K` | auto | Image resolution (2K only on flash/pro) |
| `--aspect` | e.g. `16:9`, `1:1` | auto | Aspect ratio |
| `--mime` | `image/png`, `image/jpeg` | `image/png` | Output format |

### Model tiers

| Shorthand | Model | Cost | Best for |
|-----------|-------|------|----------|
| `lite` | gemini-3.1-flash-lite-image | ~1,500 tokens/image | Icons, simple graphics, drafts |
| `flash` | gemini-3.1-flash-image | ~2,500 tokens/image | Versatile, higher quality |
| `pro` | gemini-3-pro-image | ~5,000 tokens/image | Premium quality, complex scenes |

**Default to `lite`** — it produces clean flat icons and simple graphics at negligible cost. Only upgrade to `flash` or `pro` if `lite` output isn't good enough.

### Examples

**Flat icon (cheapest):**
```bash
/usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/gemini-image/scripts/generate.py \
  --prompt "A flat minimal icon of a seedling with a lightbulb, teal colour (#0D9488), white background, no text, no gradients" \
  --output icon-growth.png
```

**Landscape banner:**
```bash
/usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/gemini-image/scripts/generate.py \
  --prompt "A wide banner showing a survey clipboard vs a report with checkmarks, split composition" \
  --output banner.png \
  --aspect 16:9 \
  --model flash
```

**Edit an existing image:**
```bash
/usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/gemini-image/scripts/generate.py \
  --prompt "Change the background colour to dark navy blue" \
  --input original.png \
  --output edited.png
```

## Prompting Tips for Good Icons

These patterns produced clean results in the Month 1 comms artefact session:

### What works
- **"Flat minimal icon"** — triggers the right visual style
- **"Single [colour] colour (#hex)"** — keeps it monochrome, specifiable
- **"White background"** — clean extraction
- **"No text, no shading, no gradients"** — prevents unwanted complexity
- **"Suitable as an infographic icon"** — anchors the style
- **"Clean simple silhouette style"** — avoids photorealistic output

### What to watch for
- **Diversity in people icons.** If generating icons with people, specify diverse silhouettes (e.g. "different hair styles"). Two identical figures reads as exclusionary.
- **Leadership imagery.** Avoid pointing/commanding figures — use "supportive", "walking alongside", "guiding" for servant leadership framing.
- **Consistency across a set.** When generating multiple icons for the same context, include "matching a corporate icon set" or reference the style of previous icons.
- **Borders/frames.** The model sometimes adds borders. If unwanted, add "no border, no frame" to the prompt.

## API Details

- **Endpoint:** `https://generativelanguage.googleapis.com/v1beta/interactions`
- **Auth:** `x-goog-api-key` header
- **Response structure:** Image data is at `steps[1].content[0].data` (base64-encoded)
- **Step types:** `steps[0]` is `thought`, `steps[1]` is `model_output` containing the image

## Troubleshooting

- **403 Permission denied:** API key not set or invalid. Check `echo $GEMINI_API_KEY`.
- **No image in response:** The model may return text-only if the prompt is ambiguous. Make the prompt explicitly about creating a visual.
- **Image quality too low:** Upgrade from `lite` to `flash`. Add more specific style direction.
- **Wrong colours:** Specify hex codes explicitly: `"blue colour (#0066CC)"`. Colour names are interpreted loosely.
- **Icons look too different in a set:** Generate them in sequence with consistent style prompts. Include "matching style to [previous icon description]" in later prompts.
