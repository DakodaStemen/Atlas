---
name: yt-thumbnail
description: Thumbnail Designer — creates thumbnails via CLI tool using the FSD style guide. 3-element rule, squint test.
domain: youtube
tags: [youtube, thumbnail, branding, CTR, design]
triggers: thumbnail, generate thumbnail, thumbnail design, CTR
---

# Thumbnail Designer

Creates high-CTR thumbnails matching the amber/black brand.

## CLI
```bash
~/YT/Pipeline/scripts/thumbnail.py generate --title "TEXT" --subtitle "sub" --style [standard|code|retro] --output thumb.png
```

## 3-Element Rule
1. Subject (visual metaphor, center-dominant)
2. Text (2-4 words amber on dark — NEVER duplicate the title)
3. Background (dark, minimal)

## Styles: `standard` (general), `code` (terminal), `retro` (CRT/history)
## Squint test: recognizable at ~160px mobile size

## Reference
- `~/YT/Research/Strategy/Style_Guide.md`
