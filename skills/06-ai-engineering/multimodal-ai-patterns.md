---
name: multimodal-ai-patterns
description: Multimodal AI — vision LLM patterns (Claude Vision, GPT-4V, Gemini), image+text inputs, document understanding, image generation APIs (DALL-E, Stable Diffusion), and video models.
domain: ai-ml
category: multimodal
tags: [multimodal, vision, Claude-Vision, GPT-4V, Gemini, image-understanding, document-AI, DALL-E, Stable-Diffusion, image-generation]
triggers: vision model, multimodal AI, Claude vision, GPT-4V, image understanding, document extraction, OCR LLM, image generation API, DALL-E, Stable Diffusion API, image input LLM
---

# Multimodal AI Patterns

Practical reference for image+text LLM APIs, document understanding, and image generation. Covers Claude, GPT-4o/4V, Gemini, DALL-E 3, Stable Diffusion, and Flux.

---

## When to Use

**Vision LLM (Claude / GPT-4o / Gemini)** when you need:

- Chart or graph reading with interpretation (not just data extraction)
- Screenshot analysis — UI bugs, error messages, dashboards
- Document understanding where layout matters: invoices, forms, contracts
- Natural-language Q&A over an image (explain, summarize, compare)
- OCR + reasoning in a single pass (extract table then answer a question about it)

**Traditional OCR (Tesseract, AWS Textract, Google Document AI)** when you need:

- High-volume, cheap, structured field extraction from known form layouts
- Sub-second latency with no LLM budget
- Guaranteed character-level accuracy for regulated documents

**Multimodal pipeline** when documents are complex: run Textract/Document AI for bounding boxes and raw text, then pass the structured output to an LLM for reasoning, summarization, or anomaly detection. Avoid double-charging for vision tokens on simple field extraction.

**Image generation** (DALL-E 3, Stable Diffusion, Flux) when you need to produce images from text. None of the vision-understanding models (Claude, GPT-4V, Gemini) generate images.

---

## Image Input Formats

### Supported formats by provider

| Provider | Formats |
| --- | --- |
| Claude | JPEG, PNG, GIF, WebP |
| GPT-4o / GPT-4V | JPEG, PNG, GIF, WebP |
| Gemini | PNG, JPEG, WebP, HEIC, HEIF |

### Base64 vs URL

**Base64** — encode the image bytes and embed them in the request JSON. Works everywhere, no URL expiry issues, but inflates request size. Use when:

- The image is local / not publicly accessible
- You need deterministic behavior (URL content could change)
- You are in a multi-turn conversation and want to avoid re-fetching

**URL** — pass a publicly accessible HTTPS URL. Smaller request payload, but the provider fetches the image at request time. URL expiry will break your call silently on retry; presigned S3 URLs typically expire in minutes to hours. Avoid for production pipelines with retries.

### Size limits

| Provider | Max per image | Max per request |
| --- | --- | --- |
| Claude API | 5 MB | 32 MB total (standard endpoint) |
| Claude (claude.ai) | 10 MB | 20 images/turn |
| Claude API (multi-image) | 2000×2000 px if >20 images | Up to 600 images |
| GPT-4o | 20 MB | no hard image count limit documented |
| Gemini inline | 20 MB total request | 3,600 images per request |
| Gemini Files API | 2 GB per file | 20 GB total project storage |

### Token cost calculation

Claude: `tokens ≈ (width_px × height_px) / 750`

Optimal size for Claude: resize long edge to ≤1568 px, target ≤1.15 megapixels. Above that, Claude resizes automatically but latency increases. Images under 200 px on any edge degrade quality.

Gemini: images ≤384 px on both dimensions = 258 tokens. Larger images are tiled at 768×768 px, each tile = 258 tokens.

GPT-4o detail parameter controls token spend: `detail: "low"` = 85 tokens flat; `detail: "high"` tiles the image and costs proportionally more.

---

## Claude Vision

### Image block structure

Images go in the `content` array of a user message as typed blocks. Text and image blocks can be interleaved in any order, but **put images before the question** for best results.

```python
import anthropic
import base64
import httpx

client = anthropic.Anthropic()  # reads ANTHROPIC_API_KEY from env

# --- base64 input ---
image_bytes = httpx.get("https://example.com/chart.png").content
image_b64 = base64.standard_b64encode(image_bytes).decode("utf-8")

response = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": image_b64,
                    },
                },
                {"type": "text", "text": "What trend does this chart show?"},
            ],
        }
    ],
)
print(response.content[0].text)
```

```python
# --- URL input ---
response = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "url",
                        "url": "https://example.com/invoice.jpg",
                    },
                },
                {"type": "text", "text": "Extract the total amount due."},
            ],
        }
    ],
)
```

### Files API (upload once, reuse)

For multi-turn conversations or batch processing, upload images once and reference by `file_id`. This keeps request payloads small across conversation history.

```python
import anthropic

client = anthropic.Anthropic()

with open("invoice.pdf", "rb") as f:
    uploaded = client.beta.files.upload(
        file=("invoice.pdf", f, "application/pdf")
    )

file_id = uploaded.id  # store this; files persist for the session

response = client.beta.messages.create(
    model="claude-opus-4-6",
    max_tokens=2048,
    betas=["files-api-2025-04-14"],
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {"type": "file", "file_id": file_id},
                },
                {"type": "text", "text": "Extract all line items as JSON."},
            ],
        }
    ],
)
```

### Multi-image

Label images explicitly when comparing. Claude processes all images in the content array.

```python
response = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[
        {
            "role": "user",
            "content": [
                {"type": "text", "text": "Image 1:"},
                {"type": "image", "source": {"type": "url", "url": url_before}},
                {"type": "text", "text": "Image 2:"},
                {"type": "image", "source": {"type": "url", "url": url_after}},
                {"type": "text", "text": "What changed between these two screenshots?"},
            ],
        }
    ],
)
```

### Claude vision limitations

- Cannot identify people by face
- Cannot generate or edit images
- Spatial reasoning is limited (precise pixel coordinates, analog clocks, chess positions)
- Does not read image EXIF/metadata
- Image uploads via API are ephemeral — not stored after the request

---

## GPT-4V / GPT-4o

GPT-4o is the current unified multimodal model. GPT-4V is the predecessor. Both use the same `image_url` content block pattern.

### image_url content block

```python
from openai import OpenAI
import base64

client = OpenAI()  # reads OPENAI_API_KEY from env

# --- URL input ---
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": "https://example.com/diagram.png",
                        "detail": "high",  # "low" | "high" | "auto"
                    },
                },
                {"type": "text", "text": "Describe the system architecture shown."},
            ],
        }
    ],
    max_tokens=1024,
)
print(response.choices[0].message.content)
```

```python
# --- base64 input ---
with open("receipt.jpg", "rb") as f:
    b64 = base64.b64encode(f.read()).decode("utf-8")

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {
            "role": "user",
            "content": [
                {
                    "type": "image_url",
                    "image_url": {
                        "url": f"data:image/jpeg;base64,{b64}",
                        "detail": "auto",
                    },
                },
                {"type": "text", "text": "What is the grand total?"},
            ],
        }
    ],
)
```

### detail parameter

- `"low"` — 85 tokens regardless of image size. Fast, cheap. Use for quick classification or yes/no tasks where fine detail is not needed.
- `"high"` — tiles the image at 512×512 px chunks plus one 512×512 overview. Token cost scales with image size. Use for document extraction, small text, charts.
- `"auto"` — OpenAI picks based on image size. Reasonable default.

### Multi-turn with images

Images can appear in any turn. The full conversation history (including image bytes or URLs) is resent each turn — watch request size growth for long conversations.

```python
messages = [
    {
        "role": "user",
        "content": [
            {"type": "image_url", "image_url": {"url": url1}},
            {"type": "text", "text": "What product is shown?"},
        ],
    },
    {
        "role": "assistant",
        "content": "The image shows a running shoe.",
    },
    {
        "role": "user",
        "content": [
            {"type": "text", "text": "Now compare it to this one:"},
            {"type": "image_url", "image_url": {"url": url2}},
        ],
    },
]
response = client.chat.completions.create(model="gpt-4o", messages=messages)
```

---

## Gemini Vision

### Inline data (small images)

```python
import google.generativeai as genai
import PIL.Image

genai.configure(api_key="GEMINI_API_KEY")
model = genai.GenerativeModel("gemini-1.5-pro")

image = PIL.Image.open("chart.png")
response = model.generate_content(["What does this chart show?", image])
print(response.text)
```

For base64/raw bytes inline:

```python
import google.generativeai as genai

model = genai.GenerativeModel("gemini-2.5-pro")

with open("diagram.png", "rb") as f:
    image_bytes = f.read()

response = model.generate_content([
    {
        "inline_data": {
            "mime_type": "image/png",
            "data": image_bytes,
        }
    },
    "Explain the architecture in this diagram.",
])
```

### Gemini Files API (large files, video, repeated use)

Files persist for 48 hours. Max 2 GB per file, 20 GB total project storage. Free in supported regions.

```python
import google.generativeai as genai

genai.configure(api_key="GEMINI_API_KEY")

# Upload once
myfile = genai.upload_file("large_document.pdf")

# Reference in any subsequent call during the 48h window
model = genai.GenerativeModel("gemini-1.5-pro")
response = model.generate_content([myfile, "Summarize the key findings."])
print(response.text)
```

### Video frames

Gemini can process video files via the Files API and reason over temporal content.

```python
import google.generativeai as genai
import time

genai.configure(api_key="GEMINI_API_KEY")

video_file = genai.upload_file("demo.mp4")

# Wait for processing
while video_file.state.name == "PROCESSING":
    time.sleep(5)
    video_file = genai.get_file(video_file.name)

model = genai.GenerativeModel("gemini-1.5-pro")
response = model.generate_content([
    video_file,
    "Describe what happens in this video step by step.",
])
print(response.text)
```

### Token cost (Gemini)

- Images ≤384 px on both dimensions: 258 tokens flat
- Larger images: tiled at 768×768 px, 258 tokens per tile
- Control cost with `media_resolution` parameter: `"low"` | `"medium"` | `"high"`

### Gemini 2.5 capabilities

- Object detection: returns bounding boxes normalized to 0–1000 scale
- Image segmentation: returns base64-encoded PNG probability maps as contour masks
- For segmentation, set thinking budget to 0 for better results

---

## Document Understanding

Vision LLMs handle documents better than traditional OCR when:

- The document has complex layout (multi-column, tables, mixed fonts)
- You need to reason over the content, not just extract fields
- The form is non-standard or handwritten

### PDF extraction pattern

```python
# Claude — upload PDF via Files API, extract structured data
import anthropic, json

client = anthropic.Anthropic()

with open("invoice.pdf", "rb") as f:
    uploaded = client.beta.files.upload(file=("invoice.pdf", f, "application/pdf"))

response = client.beta.messages.create(
    model="claude-opus-4-6",
    max_tokens=2048,
    betas=["files-api-2025-04-14"],
    messages=[
        {
            "role": "user",
            "content": [
                {"type": "image", "source": {"type": "file", "file_id": uploaded.id}},
                {
                    "type": "text",
                    "text": (
                        "Extract the following fields from this invoice and return JSON only, "
                        "no prose:\n"
                        '{"vendor": "", "invoice_number": "", "date": "", '
                        '"line_items": [{"description": "", "qty": 0, "unit_price": 0}], '
                        '"subtotal": 0, "tax": 0, "total": 0}'
                    ),
                },
            ],
        }
    ],
)

data = json.loads(response.content[0].text)
```

### Table parsing prompt pattern

When the image contains a table, guide the model explicitly:

```text
Extract the table in this image as a JSON array of objects.
Each object should represent one row. Use the header row as keys.
Return only valid JSON with no markdown fences.
```

For multi-page PDFs, process page-by-page or use Gemini 1.5 Pro / Claude with large context — both handle multi-page documents natively when the PDF is uploaded as a file.

---

## Vision Prompting Patterns

### Describe → Extract → Reason

A reliable three-step structure for complex documents:

```text
Step 1 — Describe: Briefly describe what type of document this is and its overall layout.
Step 2 — Extract: List all data fields visible (labels and values).
Step 3 — Reason: Answer: [specific question about the data].
```

This forces the model to ground its answer in what it actually sees before reasoning.

### OCR from screenshots

```text
This is a screenshot of a terminal. Transcribe the exact text visible,
preserving line breaks and indentation. Do not interpret or summarize — copy exactly.
```

### Chart reading

```text
Describe the chart type, axes (labels and ranges), any legend entries,
and the key trend or insight shown. Be specific about data values where visible.
```

### Grounding — prevent hallucination

```text
Only describe elements you can directly see in the image.
If a value is partially obscured or illegible, say so rather than guessing.
```

### Structured output from visual input

Combine with a JSON schema to ensure parseable output:

```python
prompt = """
Look at this receipt image and return JSON matching this schema exactly:
{
  "store_name": string,
  "date": string (YYYY-MM-DD),
  "items": [{"name": string, "price": float}],
  "total": float,
  "payment_method": string
}
Return only the JSON object, no other text.
"""
```

---

## Image Generation: DALL-E 3

DALL-E 3 is accessed via the OpenAI images API. Note: separate from the chat/vision API.

### Core parameters

| Parameter | Values | Notes |
| --- | --- | --- |
| `model` | `"dall-e-3"` | Use `"dall-e-2"` for cheaper, lower quality |
| `size` | `"1024x1024"`, `"1024x1792"`, `"1792x1024"` | DALL-E 3 only. Square, portrait, landscape |
| `quality` | `"standard"`, `"hd"` | `hd` = finer detail, ~2× cost |
| `style` | `"vivid"`, `"natural"` | vivid = hyper-real, natural = more realistic |
| `n` | `1` | DALL-E 3 only supports n=1 per request |
| `response_format` | `"url"`, `"b64_json"` | URLs expire after ~1 hour |

```python
from openai import OpenAI

client = OpenAI()

response = client.images.generate(
    model="dall-e-3",
    prompt=(
        "A photorealistic image of a modern home office with a standing desk, "
        "dual monitors, warm lighting, plants, minimalist decor, golden hour"
    ),
    size="1792x1024",
    quality="hd",
    style="natural",
    n=1,
    response_format="b64_json",  # use b64_json to avoid URL expiry issues
)

import base64
image_bytes = base64.b64decode(response.data[0].b64_json)
with open("output.png", "wb") as f:
    f.write(image_bytes)
```

### Prompt engineering for DALL-E 3

DALL-E 3 interprets prompts literally and generates longer compositions well. Effective patterns:

- Lead with style/medium: "Oil painting of...", "Isometric illustration of...", "Technical diagram of..."
- Specify lighting explicitly: "soft diffused light", "neon backlight", "studio lighting"
- Specify aspect/composition: "wide shot", "close-up portrait", "bird's-eye view"
- Use negative framing sparingly — DALL-E 3 does not support explicit negative prompts; rephrase positively

### Content policy

DALL-E 3 refuses real person likenesses, explicit content, and copyrighted characters. Test your prompts; the API returns a content filter error on refusal (400 with `content_policy_violation`).

The revised prompt is returned in `response.data[0].revised_prompt` — DALL-E 3 may rewrite your prompt and this field shows what was actually used.

---

## Image Generation: Stable Diffusion API

Stable Diffusion is available via Stability AI's hosted API, Replicate, and Together AI. The APIs differ in parameter names but the core concepts are the same.

### Core SD concepts

- **CFG scale** (classifier-free guidance): 7–12 is standard. Higher = closer to prompt but less creative. Lower = more varied output.
- **Negative prompt**: explicitly describe what to exclude. More effective than in DALL-E. Example: `"blurry, low quality, watermark, text, deformed hands"`
- **Steps**: 20–50 is the typical range. More steps = more refined but slower and more expensive.
- **Seed**: fix for reproducibility.
- **Sampler**: DPM++ 2M Karras and Euler a are widely used defaults.

### Stability AI API (stable-diffusion-3)

```python
import requests, base64, os

response = requests.post(
    "https://api.stability.ai/v2beta/stable-image/generate/core",
    headers={
        "authorization": f"Bearer {os.environ['STABILITY_API_KEY']}",
        "accept": "image/*",
    },
    files={"none": ""},
    data={
        "prompt": "A futuristic cityscape at dusk, cyberpunk aesthetic, rain-slicked streets",
        "negative_prompt": "blurry, low quality, watermark, text",
        "aspect_ratio": "16:9",
        "output_format": "png",
        "seed": 42,
    },
)

if response.status_code == 200:
    with open("city.png", "wb") as f:
        f.write(response.content)
else:
    raise Exception(response.json())
```

### Replicate (Stable Diffusion / Flux)

```python
import replicate

# Stable Diffusion XL
output = replicate.run(
    "stability-ai/sdxl:39ed52f2a78e934b3ba6e2a89f5b1c712de7dfea535525255b1aa35c5565e08b",
    input={
        "prompt": "A serene Japanese garden in autumn, photorealistic",
        "negative_prompt": "cartoon, illustration, painting, low quality",
        "width": 1024,
        "height": 1024,
        "num_inference_steps": 30,
        "guidance_scale": 7.5,
        "scheduler": "DPMSolverMultistep",
        "seed": 0,
    },
)

with open("garden.png", "wb") as f:
    f.write(output[0].read())
```

---

## Image Generation: Flux and Others

### Flux.1 via Replicate

Flux.1 (Black Forest Labs) is the current quality leader for open-weight image generation as of early 2026. Available in three variants:

- `flux-schnell` — fastest, 4-step distilled, good for iteration
- `flux-dev` — higher quality, 50 steps
- `flux-pro` — hosted premium, closed weights

```python
import replicate

# Flux Schnell — fast iteration
output = replicate.run(
    "black-forest-labs/flux-schnell",
    input={
        "prompt": "A close-up photograph of a dewdrop on a spider web at sunrise",
        "num_outputs": 1,
        "aspect_ratio": "1:1",
        "output_format": "webp",
        "output_quality": 90,
    },
)

with open("dewdrop.webp", "wb") as f:
    f.write(output[0].read())
```

```python
# Flux Dev — better quality
output = replicate.run(
    "black-forest-labs/flux-dev",
    input={
        "prompt": "Product photography of a glass perfume bottle on marble, studio lighting",
        "num_inference_steps": 50,
        "guidance": 3.5,
        "seed": 12345,
    },
)
```

### ControlNet concepts

ControlNet adds structural conditioning to SD/Flux: you feed a control image (depth map, edge map, pose skeleton, or canny edges) alongside the prompt and the model generates an image that matches both the prompt and the structure of the control image.

Common control types:

- `canny` — edge detection, good for architecture and product shots
- `depth` — depth map from monocular depth estimation, good for scene layout
- `openpose` — human pose skeleton, good for character generation
- `scribble` — rough sketch to detailed render

Available on Replicate under `lllyasviel/controlnet` and various SDXL ControlNet models.

---

## Batch Document Processing

### Parallel image processing

```python
import anthropic
from concurrent.futures import ThreadPoolExecutor, as_completed
import base64
from pathlib import Path

client = anthropic.Anthropic()

def extract_invoice(image_path: str) -> dict:
    with open(image_path, "rb") as f:
        b64 = base64.standard_b64encode(f.read()).decode("utf-8")

    suffix = Path(image_path).suffix.lower()
    media_map = {".jpg": "image/jpeg", ".jpeg": "image/jpeg",
                 ".png": "image/png", ".webp": "image/webp"}
    media_type = media_map.get(suffix, "image/jpeg")

    response = client.messages.create(
        model="claude-opus-4-6",
        max_tokens=512,
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "image", "source": {"type": "base64",
                                                  "media_type": media_type,
                                                  "data": b64}},
                    {"type": "text", "text": 'Extract: {"vendor":"","total":0,"date":""}. JSON only.'},
                ],
            }
        ],
    )
    import json
    return {"path": image_path, "data": json.loads(response.content[0].text)}

invoice_paths = ["inv1.jpg", "inv2.jpg", "inv3.png"]  # your files

results = []
with ThreadPoolExecutor(max_workers=5) as executor:
    futures = {executor.submit(extract_invoice, p): p for p in invoice_paths}
    for future in as_completed(futures):
        results.append(future.result())
```

### Rate limits and cost

Claude rate limits are per-minute (requests and tokens). For batch jobs:

- Use `ThreadPoolExecutor(max_workers=5)` as a starting point; tune down if hitting 429s
- Each image at 1MP ≈ 1,334 tokens at ~$0.004/image on claude-sonnet-4-6
- 1,000 invoices at 1MP ≈ $4 in image tokens alone plus output tokens
- For very high volume, use the Anthropic Batch API (async, 50% discount, 24h turnaround)

```python
# Anthropic Batch API for bulk processing
import anthropic, base64, json

client = anthropic.Anthropic()

def make_batch_request(image_path: str, custom_id: str) -> dict:
    with open(image_path, "rb") as f:
        b64 = base64.standard_b64encode(f.read()).decode("utf-8")
    return {
        "custom_id": custom_id,
        "params": {
            "model": "claude-opus-4-6",
            "max_tokens": 512,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "image", "source": {"type": "base64",
                                                      "media_type": "image/jpeg",
                                                      "data": b64}},
                        {"type": "text", "text": 'Extract {"vendor":"","total":0}. JSON only.'},
                    ],
                }
            ],
        },
    }

batch = client.messages.batches.create(
    requests=[make_batch_request(p, f"inv-{i}") for i, p in enumerate(invoice_paths)]
)
print(batch.id)  # poll batch.id for results
```

---

## Structured Extraction from Images

### Receipt / form extraction

```python
import anthropic, json

client = anthropic.Anthropic()

EXTRACTION_PROMPT = """
Extract data from this receipt image. Return only a JSON object matching this schema:
{
  "store_name": "string",
  "date": "YYYY-MM-DD",
  "items": [
    {"name": "string", "quantity": number, "unit_price": number, "total": number}
  ],
  "subtotal": number,
  "tax": number,
  "tip": number,
  "grand_total": number,
  "payment_method": "string",
  "currency": "string (ISO 4217)"
}
If a field is not visible, use null. Numbers should be floats. Return only JSON, no prose.
"""

def extract_receipt(image_url: str) -> dict:
    response = client.messages.create(
        model="claude-opus-4-6",
        max_tokens=1024,
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "image", "source": {"type": "url", "url": image_url}},
                    {"type": "text", "text": EXTRACTION_PROMPT},
                ],
            }
        ],
    )
    return json.loads(response.content[0].text)
```

### Chart data extraction

```python
CHART_PROMPT = """
Analyze this chart image and return JSON:
{
  "chart_type": "bar|line|pie|scatter|other",
  "title": "string or null",
  "x_axis": {"label": "string", "unit": "string or null"},
  "y_axis": {"label": "string", "unit": "string or null"},
  "series": [
    {
      "name": "string",
      "data_points": [{"x": "value", "y": number}]
    }
  ],
  "key_insight": "one sentence summary of the main trend"
}
Only include data points you can read directly from the chart. Return JSON only.
"""
```

### Multi-page table extraction

For multi-page documents, process page-by-page then merge:

```python
def extract_table_from_page(client, file_id: str, page_instruction: str) -> list[dict]:
    response = client.beta.messages.create(
        model="claude-opus-4-6",
        max_tokens=4096,
        betas=["files-api-2025-04-14"],
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "image", "source": {"type": "file", "file_id": file_id}},
                    {
                        "type": "text",
                        "text": (
                            f"{page_instruction}\n"
                            "Return a JSON array of row objects using header names as keys. "
                            "If this page has no table rows, return []. JSON only."
                        ),
                    },
                ],
            }
        ],
    )
    import json
    return json.loads(response.content[0].text)
```

---

## Critical Rules / Gotchas

**Image token costs are real.** A 1024×1024 PNG on Claude costs ~1,400 tokens — at claude-sonnet-4-6 input pricing ($3/M tokens), that is $0.004 per image. At scale this dominates cost. Resize before sending.

**Resolution affects token count linearly.** Halving image dimensions quarters the tokens. For document extraction you usually don't need full resolution — 800 px wide is enough for text.

**URL expiry will break production pipelines.** Presigned S3 URLs expire. If you store a URL and retry hours later, you get a broken image. Use base64 or Files API for anything that needs to be retried.

**Claude cannot edit or generate images.** It analyzes only. DALL-E, Stable Diffusion, and Flux are entirely separate products.

**GPT-4V detail="low" is deceptively cheap.** 85 tokens regardless of image dimensions — useful for routing/classification tasks. For text/table extraction always use `"high"`.

**Gemini Files API files expire after 48 hours.** Do not store `file_id` references in a database expecting them to work long-term. Re-upload before use if older than 48h.

**DALL-E 3 rewrites your prompt.** Check `response.data[0].revised_prompt` to see what was actually used. Build your eval loop around the revised prompt, not your input.

**Multi-turn conversations resend full history.** In a 10-turn conversation with 5 images, you pay for those 5 images 10 times (once per turn, multiplied by conversation length). Use Files API to avoid re-encoding bytes; you still pay image tokens per turn but avoid base64 bloat.

**GIF support means animated GIF.** Claude processes only the first frame of animated GIFs. If you need motion analysis, use Gemini with video input.

**Gemini bounding box coordinates are 0–1000 normalized, not pixel coordinates.** Scale by image dimensions after receiving.

---

## References

- Anthropic Vision docs: <https://platform.claude.com/docs/en/docs/build-with-claude/vision>
- Anthropic Files API: <https://platform.claude.com/docs/en/docs/build-with-claude/files>
- OpenAI Vision guide: <https://platform.openai.com/docs/guides/vision>
- OpenAI Images API (DALL-E): <https://platform.openai.com/docs/api-reference/images>
- Gemini Vision: <https://ai.google.dev/gemini-api/docs/vision>
- Gemini Files API: <https://ai.google.dev/gemini-api/docs/files>
- Replicate Python client: <https://replicate.com/docs/get-started/python>
- Flux.1 on Replicate: <https://replicate.com/black-forest-labs/flux-schnell>
- Stability AI API: <https://platform.stability.ai/docs/api-reference>
