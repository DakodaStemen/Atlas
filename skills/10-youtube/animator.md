---
name: yt-animator
description: Animation Director — designs and codes Manim/Motion Canvas scenes using the FSD component library and amber/black aesthetic.
domain: youtube
tags: [youtube, manim, animation, motion-canvas, rendering, visual]
triggers: manim scene, animation, render, motion canvas, FSDScene, component library
---

# Animation Director

Translates script visual cues into Manim CE scenes. Enforces the amber/black aesthetic.

## Component Library (`~/YT/Pipeline/manim_lib/`)
`FSDScene`, `StackDiagram`, `MemoryBlock`, `RegisterView`, `BinaryDisplay`, `CodeBlock`, `PacketDiagram`, `TimelineBar`, `SignalTrace`, `LayerHighlight`

## Colors (use constants, never raw hex)
`AMBER`, `BG_BLACK`, `PANEL_GRAY`, `WARM_ORANGE`, `DIM_AMBER`, `TERM_GREEN`, `SIGNAL_RED`, `STEEL_BLUE`

## Rules
- ALL scenes inherit `FSDScene`
- Relative positioning: `next_to()`, `align_to()`
- Dev: `-ql`, Final: `-qk`
- Pixel art: `flags=neighbor` only
- One scene class per script section
- Use `manim-voiceover` for audio sync

## Reference
- `~/YT/Research/Production/Animation_Guide.md`
- `~/YT/Pipeline/manim_lib/`
