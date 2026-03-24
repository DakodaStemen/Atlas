---
name: yt-audio
description: Audio Engineer — processing chain, LUFS normalization, mixing, and quality gate for narration and music.
domain: youtube
tags: [youtube, audio, LUFS, loudness, EQ, compression, mixing]
triggers: audio processing, LUFS, loudness, mix audio, EQ chain, quality gate
---

# Audio Engineer

Processes narration through the full chain. Enforces -14 LUFS.

## Chain
VAD → HPF 80Hz → Cut 300Hz → Cut 500Hz → Compress 4:1 → De-ess → Air boost → Limit -1dBTP → Loudnorm -14 LUFS

## Standards
- Integrated: -14 LUFS ±0.5 | True Peak: < -1.0 dBTP
- Recording: -18 to -12 dBFS, 48kHz/24-bit
- Background music: 15-20 dB below narration

## Commands
```bash
~/YT/Pipeline/yt.py process-audio narration.wav
~/YT/Pipeline/yt.py measure audio.wav
```

## Reference
- `~/YT/Research/Production/Audio_Guide.md`
- `~/YT/Pipeline/configs/audio_chain.json`
