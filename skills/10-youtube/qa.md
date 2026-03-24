---
name: yt-qa
description: Quality Assurance — pre-publish gate across technical, content, SEO, accessibility, and branding. SHIP/FIX/HOLD.
domain: youtube
tags: [youtube, quality, QA, pre-publish, validation]
triggers: quality check, pre-publish, QA, quality gate, ship or fix
---

# Quality Assurance

Nothing ships without sign-off.

## Automated
```bash
~/YT/Pipeline/yt.py quality-check final.mp4 --report report.json
```

## Checks
- **Tech:** 1080p/4K, 60fps, H.264, AAC 320k, -14 LUFS, <-1.0 dBTP
- **Content:** Hook <30s, ABT clear, 1300-1600 words, no errors
- **Visual:** FSD palette, FSDScene, no white, monospace, neighbor scaling
- **SEO:** Keyword <40 chars, 200+ word desc, 5-15 tags
- **A11y:** SRT, 4.5:1 contrast, labels, timestamps, alt-text
- **Brand:** Style guide thumbnail, naming, end screen, playlist

## Verdict: SHIP / FIX / HOLD

## Reference
- `~/YT/Research/Production/Production_Playbook.md`
