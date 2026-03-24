---
name: yt-overview
description: Full Stack Descent YouTube channel — agent roster, production pipeline, and project map. Start here.
domain: youtube
tags: [youtube, full-stack-descent, overview, agents, pipeline]
triggers: youtube, full stack descent, FSD, channel, video production
---

# Full Stack Descent — YouTube Production System

**Project:** `~/YT/`
**Pipeline CLI:** `~/YT/Pipeline/yt.py`
**Research:** `~/YT/Research/`
**Manim Library:** `~/YT/Pipeline/manim_lib/`
**NotebookLM Index:** `~/YT/Research/Reference/NotebookLM_Index.md`

## Agents

Each file in this folder is an agent — an expert entity with a specific role in the production pipeline. Spawn the right one for the job.

| Agent | File | Does What |
|-------|------|-----------|
| **Producer** | `producer.md` | Orchestrates episodes end-to-end, delegates to other agents |
| **Researcher** | `researcher.md` | Deep topic research via NotebookLM, fact-checking |
| **Writer** | `writer.md` | Scripts using ABT framework, hooks, pacing |
| **Animator** | `animator.md` | Manim/Motion Canvas scenes, FSD component library |
| **Audio** | `audio.md` | Processing chain, LUFS, mixing, quality gate |
| **Sound** | `sound.md` | Music, SFX, sonic identity, FL Studio |
| **Thumbnail** | `thumbnail.md` | Thumbnail generation via CLI tool |
| **Accessibility** | `accessibility.md` | Captions, SRT, WCAG, color-blind safety |
| **QA** | `qa.md` | Pre-publish checks, final SHIP/FIX/HOLD |
| **SEO** | `seo.md` | Keywords, titles, descriptions, tags |
| **Distribution** | `distribution.md` | Reddit, X, HN, Shorts, 48-hour launch |
| **Community** | `community.md` | Discord, comments, polls, engagement |

## Production Flow

```
Researcher → Writer → [Record] → Audio → Animator → Sound → [Composite] → QA → SEO + Thumbnail + Accessibility → Distribution → Community
```

Parallel-safe: SEO + Thumbnail + Accessibility can run together. Sound can run alongside Animation.
