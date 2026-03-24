---
name: prompt-engineering-comprehensive
description: Prompt engineering patterns for LLMs covering model catalogs (Claude, GPT, Gemini), image generation (DALL-E, Sora), structured outputs, OpenAI API configuration, and voice/narration defaults. Use when crafting prompts, configuring LLM APIs, or selecting models.
domain: ai-engineering
tags: [prompt-engineering, claude, openai, gpt, gemini, dall-e, sora, image-generation, voice, model-selection]
triggers: prompt engineering, model catalog, claude models, openai, GPT, image generation, sora, dall-e, voice prompt, narration, prompt template
---

# Prompt Engineering Comprehensive

## 1. Model Selection Guide

### Claude Models (Anthropic)

| Model | Best For | Context |
|-------|----------|---------|
| Claude Opus 4 | Complex reasoning, code generation, research | 200K |
| Claude Sonnet 4 | Balanced speed/quality, most tasks | 200K |
| Claude Haiku 3.5 | Fast classification, extraction, routing | 200K |

### OpenAI Models

| Model | Best For | Context |
|-------|----------|---------|
| GPT-4o | Multimodal, complex reasoning | 128K |
| GPT-4o-mini | Cost-effective general use | 128K |
| o1/o3 | Deep reasoning, math, code | 200K |

### Google Models

| Model | Best For | Context |
|-------|----------|---------|
| Gemini 2.5 Pro | Long context, multimodal | 1M |
| Gemini 2.5 Flash | Fast, cost-effective | 1M |

## 2. Prompt Patterns

### System Prompt Structure

```text
Role: [Who the model is]
Task: [What it should do]
Constraints: [What it should NOT do]
Format: [Expected output structure]
Examples: [1-3 few-shot examples]
```

### Few-Shot Prompting

- Provide 2-5 examples that cover the range of expected inputs. Include edge cases.
- Order examples from simple to complex. Use consistent formatting across examples.

### Chain-of-Thought

- Add "Let's think step by step" or structured reasoning instructions.
- For complex problems, break into explicit sub-questions.
- Use `<thinking>` tags to separate reasoning from final output.

### Structured Output

- Use JSON schema constraints (`response_format: { type: "json_schema", ... }`).
- Define exact field names, types, and descriptions. Include `required` fields.
- Validate output against schema programmatically.

## 3. Image Generation

### Sora (Video/Image) Best Practices

- Be specific about visual elements: subject, action, setting, lighting, camera angle.
- Describe the desired aesthetic: photorealistic, illustration, cinematic, vintage.
- Specify composition: rule of thirds, centered, wide shot, close-up.
- Include negative guidance: "no text overlays", "no watermarks".
- Iterate on results: refine prompts based on what the model produces.

### GPT Image (DALL-E) Best Practices

- Start with the main subject, then add details progressively.
- Specify style explicitly: "oil painting style", "3D render", "flat vector illustration".
- Use reference artists or styles for consistent aesthetic.
- Describe lighting: "golden hour lighting", "dramatic chiaroscuro", "soft diffused light".
- Include context and environment: background, setting, atmosphere.

## 4. Voice and Narration Defaults

### IVR/Phone Prompts

- Keep prompts concise (under 20 words per option). Lead with the action: "Press 1 to..." not "If you'd like to..., press 1."
- Use consistent pacing. Pause 0.5s between options. Repeat menu after 5s of silence.
- Professional tone: warm but efficient. Avoid jargon.

### Narration/Explainer

- Match tone to audience: casual for consumer, formal for enterprise.
- Structure: hook (2-3s) → context → key points → call to action.
- Use active voice. Short sentences (8-15 words). Vary sentence length for rhythm.

## 5. OpenAI API Configuration

### Key Fields (openai.yaml)

| Field | Description |
|-------|-------------|
| `model` | Model identifier (gpt-4o, gpt-4o-mini, o1) |
| `messages` | Array of role/content objects (system, user, assistant) |
| `temperature` | Randomness (0-2, default 1). Lower = deterministic. |
| `max_tokens` | Maximum response tokens |
| `response_format` | Output format constraint (json_schema, text) |
| `tools` | Function calling definitions |
| `tool_choice` | auto, none, required, or specific function |
| `stream` | Boolean for streaming responses |
| `top_p` | Nucleus sampling (alternative to temperature) |

### Best Practices

- Use `temperature: 0` for deterministic tasks (classification, extraction).
- Use `temperature: 0.7-1.0` for creative tasks (writing, brainstorming).
- Never use both `temperature` and `top_p` simultaneously.
- Set `max_tokens` slightly above expected output to avoid truncation.

## Checklist

- [ ] Model selected based on task requirements and cost
- [ ] System prompt includes role, task, constraints, format
- [ ] Few-shot examples cover expected input range including edge cases
- [ ] Temperature set appropriately for task type
- [ ] Structured output validated against schema
- [ ] Image prompts include subject, style, lighting, composition
- [ ] Voice prompts tested for timing and clarity
