---
name: cloud-ai-platforms
description: Cloud AI platform patterns covering AWS Bedrock (model access, knowledge bases, agents, guardrails, fine-tuning, batch inference) and Google Vertex AI/Gemini (model garden, endpoints, pipelines, grounding, multimodal). Use when building AI applications on AWS or GCP managed AI services.
domain: ai-engineering
tags: [aws-bedrock, vertex-ai, gemini, cloud-ai, managed-ai, knowledge-bases, model-garden]
triggers: aws bedrock, vertex ai, gemini, cloud ai, bedrock agent, bedrock knowledge base, vertex pipeline, model garden
---


# AWS Bedrock AI Platform

Amazon Bedrock is a fully managed service that provides access to foundation models (FMs) from Anthropic, Amazon, Meta, Mistral, Cohere, and others through a unified API — no infrastructure to manage, no model weights to host.


## Converse API

The Converse API is the recommended inference path. It presents a single, model-agnostic request/response schema — the same code works across Claude, Nova, Llama, and Mistral.

**Boto3 client**: `bedrock-runtime`
**Operations**: `converse` (sync) and `converse_stream` (streaming)
**IAM action**: `bedrock:InvokeModel` (yes, even for Converse)

### Request structure

```python
import boto3

client = boto3.client("bedrock-runtime", region_name="us-east-1")

response = client.converse(
    modelId="anthropic.claude-3-5-haiku-20241022-v1:0",
    system=[
        {"text": "You are a concise technical assistant. Reply in plain text only."}
    ],
    messages=[
        {"role": "user", "content": [{"text": "Summarize the CAP theorem in two sentences."}]}
    ],
    inferenceConfig={
        "maxTokens": 512,
        "temperature": 0.3,
        "topP": 0.9,
        "stopSequences": ["END"],
    },
    additionalModelRequestFields={"top_k": 50},  # model-specific extras
)

output_text = response["output"]["message"]["content"][0]["text"]
stop_reason = response["stopReason"]  # end_turn | max_tokens | stop_sequence | tool_use
usage = response["usage"]  # inputTokens, outputTokens, totalTokens
```

### Multi-turn conversation

```python
messages = []

def chat(user_input: str) -> str:
    messages.append({"role": "user", "content": [{"text": user_input}]})
    resp = client.converse(
        modelId="anthropic.claude-sonnet-4-20250514-v1:0",
        messages=messages,
        inferenceConfig={"maxTokens": 1024},
    )
    assistant_msg = resp["output"]["message"]
    messages.append(assistant_msg)
    return assistant_msg["content"][0]["text"]
```

### Streaming

```python
stream_resp = client.converse_stream(
    modelId="anthropic.claude-3-5-haiku-20241022-v1:0",
    messages=[{"role": "user", "content": [{"text": "Write a haiku about distributed systems."}]}],
    inferenceConfig={"maxTokens": 256},
)

for event in stream_resp["stream"]:
    if "contentBlockDelta" in event:
        delta = event["contentBlockDelta"]["delta"]
        if "text" in delta:
            print(delta["text"], end="", flush=True)
    elif "messageStop" in event:
        print()  # newline at end
        stop_reason = event["messageStop"]["stopReason"]
```

### Tool use (function calling)

```python
tools = [
    {
        "toolSpec": {
            "name": "get_weather",
            "description": "Returns current temperature for a city.",
            "inputSchema": {
                "json": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"],
                }
            },
        }
    }
]

resp = client.converse(
    modelId="anthropic.claude-3-5-haiku-20241022-v1:0",
    messages=[{"role": "user", "content": [{"text": "What's the weather in Berlin?"}]}],
    toolConfig={"tools": tools},
    inferenceConfig={"maxTokens": 512},
)

# If the model wants to call a tool:
if resp["stopReason"] == "tool_use":
    tool_use_block = next(
        b["toolUse"] for b in resp["output"]["message"]["content"] if "toolUse" in b
    )
    tool_name = tool_use_block["name"]       # "get_weather"
    tool_input = tool_use_block["input"]     # {"city": "Berlin"}
    tool_id = tool_use_block["toolUseId"]

    # Execute your function, then feed result back
    result = {"temperature": "18°C", "condition": "partly cloudy"}
    messages = [
        {"role": "user", "content": [{"text": "What's the weather in Berlin?"}]},
        resp["output"]["message"],  # assistant turn with toolUse block
        {
            "role": "user",
            "content": [
                {
                    "toolResult": {
                        "toolUseId": tool_id,
                        "content": [{"text": str(result)}],
                        "status": "success",
                    }
                }
            ],
        },
    ]
    final_resp = client.converse(
        modelId="anthropic.claude-3-5-haiku-20241022-v1:0",
        messages=messages,
        toolConfig={"tools": tools},
        inferenceConfig={"maxTokens": 512},
    )
```

### Content blocks supported

The `content` array in each message can hold: `text`, `image` (bytes or S3 URI), `document` (PDF, DOCX, CSV, etc.), `video`, `toolUse`, `toolResult`, `cachePoint` (prompt caching), `reasoningContent` (extended thinking).


## Model Selection

All models require explicit access enablement in the Bedrock console under **Model access** before they can be called.

### Claude (Anthropic)

| Model ID | Best For | Notes |
| --- | --- | --- |
| `anthropic.claude-3-5-haiku-20241022-v1:0` | High-volume, low-latency tasks | Cheapest Claude on Bedrock |
| `anthropic.claude-sonnet-4-20250514-v1:0` | Balanced quality/cost | Strong coding, analysis |
| `anthropic.claude-opus-4-1-20250805-v1:0` | Complex reasoning, long context | Highest capability, highest cost |

All Claude models support streaming, tool use, vision (except 3.5 Haiku text-only), and extended thinking where applicable.

### Amazon Nova

| Model ID | Modalities | Notes |
| --- | --- | --- |
| `amazon.nova-micro-v1:0` | Text only | Fastest, cheapest Nova |
| `amazon.nova-lite-v1:0` | Text, Image, Video | Good multimodal budget option |
| `amazon.nova-pro-v1:0` | Text, Image, Video | High-capability multimodal |
| `amazon.nova-premier-v1:0` | Text, Image, Video | Top-tier Nova |

### Meta Llama

| Model ID | Notes |
| --- | --- |
| `meta.llama3-1-8b-instruct-v1:0` | Fast, small footprint |
| `meta.llama3-1-70b-instruct-v1:0` | Strong open-weight option |
| `meta.llama3-1-405b-instruct-v1:0` | Largest Llama on Bedrock |
| `meta.llama4-maverick-17b-instruct-v1:0` | Multimodal (text+image) |

### Mistral

| Model ID | Notes |
| --- | --- |
| `mistral.mistral-7b-instruct-v0:2` | Smallest Mistral |
| `mistral.mistral-large-2407-v1:0` | High-quality text |
| `mistral.pixtral-large-2502-v1:0` | Vision-capable |

### Cohere

| Model ID | Use Case |
| --- | --- |
| `cohere.command-r-plus-v1:0` | RAG and chat |
| `cohere.embed-english-v3` | Text embeddings |
| `cohere.rerank-v3-5:0` | Reranking retrieved docs |


## Knowledge Bases

Bedrock Knowledge Bases implements fully managed RAG: ingest documents from S3, embed them, store vectors in a supported vector store, and query via the `Retrieve` or `RetrieveAndGenerate` APIs.

### Vector store backends

- **Amazon OpenSearch Serverless** (auto-provisioned from console, simplest)
- **Amazon Aurora PostgreSQL** (pgvector extension)
- **Amazon RDS PostgreSQL** (pgvector)
- **Pinecone** (bring your own)
- **Redis Enterprise Cloud** (bring your own)
- **MongoDB Atlas** (bring your own)

### Data sources and chunking

- S3 is the primary data source. Connect buckets or prefixes; Bedrock syncs on demand or on a schedule.
- Supported formats: PDF, DOCX, TXT, HTML, CSV, XLSX, MD, JSON.
- Chunking strategies: **Fixed size** (token count + overlap), **Semantic** (sentence-boundary aware), **Hierarchical** (parent-child for precision + recall).
- Embedding models: `amazon.titan-embed-text-v2:0` (default), `cohere.embed-english-v3`, `cohere.embed-multilingual-v3`.

### Syncing

```python
bedrock_agent = boto3.client("bedrock-agent", region_name="us-east-1")

sync_job = bedrock_agent.start_ingestion_job(
    knowledgeBaseId="KB12345678",
    dataSourceId="DS87654321",
)
job_id = sync_job["ingestionJob"]["ingestionJobId"]
```

### Retrieve API

```python
runtime = boto3.client("bedrock-agent-runtime", region_name="us-east-1")

results = runtime.retrieve(
    knowledgeBaseId="KB12345678",
    retrievalQuery={"text": "What is the refund policy for digital products?"},
    retrievalConfiguration={
        "vectorSearchConfiguration": {
            "numberOfResults": 5,
            "searchType": "HYBRID",          # SEMANTIC | HYBRID
            "overrideSearchType": "HYBRID",
        }
    },
)

for r in results["retrievalResults"]:
    print(r["content"]["text"])
    print(r["score"])
    print(r["location"]["s3Location"]["uri"])
```

### RetrieveAndGenerate API

Combines retrieval and generation in one call. The FM cites sources in the response.

```python
resp = runtime.retrieve_and_generate(
    input={"text": "Summarize the cancellation policy."},
    retrieveAndGenerateConfiguration={
        "type": "KNOWLEDGE_BASE",
        "knowledgeBaseConfiguration": {
            "knowledgeBaseId": "KB12345678",
            "modelArn": "arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-3-5-haiku-20241022-v1:0",
            "retrievalConfiguration": {
                "vectorSearchConfiguration": {
                    "numberOfResults": 5,
                    "searchType": "HYBRID",
                }
            },
        },
    },
)

answer = resp["output"]["text"]
citations = resp["citations"]  # list of source passage + S3 URI
```


## IAM and Permissions

Bedrock uses identity-based policies only — it does not support resource-based policies.

### Core IAM actions

| Action | Description | Resource |
| --- | --- | --- |
| `bedrock:InvokeModel` | Invoke any model (Converse, InvokeModel) | foundation-model, provisioned-model, inference-profile |
| `bedrock:InvokeModelWithResponseStream` | Streaming inference | Same as above |
| `bedrock:Retrieve` | Query a knowledge base | `knowledge-base/*` |
| `bedrock:RetrieveAndGenerate` | RAG in one call | `knowledge-base/*` |
| `bedrock:InvokeAgent` | Invoke an agent | `agent-alias/*/*` |
| `bedrock:ApplyGuardrail` | Apply guardrail standalone | `guardrail/*` |
| `bedrock:ListFoundationModels` | List available models | `*` |
| `bedrock:GetFoundationModel` | Get model metadata | `foundation-model/*` |
| `bedrock:CountTokens` | Count tokens before inference | `foundation-model/*` |

### Resource ARN formats

```text
arn:aws:bedrock:{region}::foundation-model/{modelId}
arn:aws:bedrock:{region}:{account}:knowledge-base/{kbId}
arn:aws:bedrock:{region}:{account}:agent/{agentId}
arn:aws:bedrock:{region}:{account}:agent-alias/{agentId}/{aliasId}
arn:aws:bedrock:{region}:{account}:guardrail/{guardrailId}
arn:aws:bedrock:{region}:{account}:provisioned-model/{resourceId}
arn:aws:bedrock:{region}:{account}:inference-profile/{resourceId}
```

### Example least-privilege policy (inference only)

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "bedrock:InvokeModel",
        "bedrock:InvokeModelWithResponseStream"
      ],
      "Resource": [
        "arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-3-5-haiku-20241022-v1:0",
        "arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-sonnet-4-20250514-v1:0"
      ]
    }
  ]
}
```

### Agent service role

The agent needs a service role that Bedrock assumes. Trust policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"Service": "bedrock.amazonaws.com"},
    "Action": "sts:AssumeRole",
    "Condition": {
      "StringEquals": {"aws:SourceAccount": "123456789012"},
      "ArnLike": {"aws:SourceArn": "arn:aws:bedrock:us-east-1:123456789012:agent/*"}
    }
  }]
}
```

The role needs `bedrock:InvokeModel` on the chosen FM, `bedrock:Retrieve` on associated knowledge bases, and `lambda:InvokeFunction` on action group Lambdas.

### ABAC with tags

```json
{
  "Effect": "Allow",
  "Action": "bedrock:InvokeModel",
  "Resource": "*",
  "Condition": {
    "StringEquals": {"aws:ResourceTag/Environment": "production"}
  }
}
```


## Prompt Management

Bedrock Prompt Management lets you version, A/B test, and deploy prompts independently of application code.

### Key concepts (Prompt Management)

- **Prompt**: A named, versioned object containing the system prompt, message template, model ID, and inference config.
- **Prompt version**: Immutable snapshot (numeric version string).
- **Prompt alias**: A mutable pointer to a version. Use aliases in application code so you can reroute traffic by updating the alias, not the code.

### Using a managed prompt in Converse

```python
# modelId is the prompt's ARN with version
resp = client.converse(
    modelId="arn:aws:bedrock:us-east-1:123456789012:prompt/PROMPT12345:1",
    promptVariables={
        "genre": {"text": "jazz"},
        "count": {"text": "5"},
    },
    messages=[
        {"role": "user", "content": [{"text": "Generate my playlist."}]}
    ],
)
```

When using a managed prompt, you cannot override `system`, `inferenceConfig`, `toolConfig`, or `additionalModelRequestFields` — those are locked in the prompt version.


## Python SDK

The boto3 Bedrock surface splits across three clients:

| Client | Used for |
| --- | --- |
| `bedrock` | Control plane: create/manage agents, KBs, guardrails, models |
| `bedrock-runtime` | Inference: `converse`, `converse_stream`, `invoke_model`, `apply_guardrail` |
| `bedrock-agent` | Agents build-time: create/update agents, action groups, KBs |
| `bedrock-agent-runtime` | Agents runtime: `invoke_agent`, `retrieve`, `retrieve_and_generate` |

### Full streaming example with error handling

```python
import boto3
from botocore.exceptions import ClientError

def stream_completion(prompt: str, model_id: str = "anthropic.claude-3-5-haiku-20241022-v1:0") -> str:
    client = boto3.client("bedrock-runtime", region_name="us-east-1")
    try:
        resp = client.converse_stream(
            modelId=model_id,
            messages=[{"role": "user", "content": [{"text": prompt}]}],
            inferenceConfig={"maxTokens": 2048, "temperature": 0.7},
        )
    except ClientError as e:
        code = e.response["Error"]["Code"]
        if code == "ThrottlingException":
            raise RuntimeError("Rate limit hit — back off and retry") from e
        elif code == "ValidationException":
            raise ValueError(f"Bad request: {e.response['Error']['Message']}") from e
        raise

    parts = []
    for event in resp["stream"]:
        if "contentBlockDelta" in event:
            delta = event["contentBlockDelta"]["delta"]
            if "text" in delta:
                parts.append(delta["text"])
    return "".join(parts)
```

### Counting tokens before inference

```python
token_count = client.count_tokens(
    modelId="anthropic.claude-3-5-haiku-20241022-v1:0",
    messages=[{"role": "user", "content": [{"text": long_document}]}],
)
print(token_count["inputTokenCount"])
```


## References

- [Amazon Bedrock User Guide](https://docs.aws.amazon.com/bedrock/latest/userguide/)
- [Converse API reference](https://docs.aws.amazon.com/bedrock/latest/userguide/conversation-inference-call.html)
- [Model IDs](https://docs.aws.amazon.com/bedrock/latest/userguide/model-ids.html)
- [Bedrock Agents overview](https://docs.aws.amazon.com/bedrock/latest/userguide/agents.html)
- [Knowledge Bases overview](https://docs.aws.amazon.com/bedrock/latest/userguide/knowledge-base.html)
- [Guardrails overview](https://docs.aws.amazon.com/bedrock/latest/userguide/guardrails.html)
- [IAM actions reference](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazonbedrock.html)
- [VPC endpoints](https://docs.aws.amazon.com/bedrock/latest/userguide/vpc-interface-endpoints.html)
- [Bedrock pricing](https://aws.amazon.com/bedrock/pricing/)
- [boto3 bedrock-runtime docs](https://boto3.amazonaws.com/v1/documentation/api/latest/reference/services/bedrock-runtime.html)

---


# Google Vertex AI and Gemini API

## When to Use (Google AI Studio vs Vertex AI)

**Google AI Studio** (`ai.google.dev`) is the developer-facing entry point:

- Free tier with API key auth — no GCP project needed to start
- Lower quota ceilings; not suitable for production workloads
- Use for prototyping, exploration, personal projects
- SDK: `google-genai` with `client = genai.Client(api_key=API_KEY)`

**Vertex AI** (`cloud.google.com/vertex-ai`) is the enterprise path:

- Auth via Application Default Credentials (ADC) or service account — no per-user API keys in prod
- Higher and configurable quotas; SLAs available
- Enterprise security: VPC-SC, CMEK, audit logging, data residency controls
- Access to model tuning, Vertex AI RAG Engine, Vertex AI Search grounding, batch prediction, and endpoint deployment
- SDK: `google-genai` with `GOOGLE_GENAI_USE_VERTEXAI=True` env var, or the legacy `vertexai` SDK

**Decision rule:** If you need IAM, VPC, compliance, higher quota, tuning, or Vertex-specific features (RAG Engine, Vertex AI Search grounding), use Vertex AI. Otherwise Google AI Studio is faster to start.


## Text Generation

### New SDK

```python
from google import genai
from google.genai import types

client = genai.Client()

response = client.models.generate_content(
    model="gemini-2.5-flash",
    contents="Explain transformer architecture in two paragraphs.",
    config=types.GenerateContentConfig(
        temperature=0.7,
        top_p=0.95,
        top_k=40,
        max_output_tokens=1024,
        stop_sequences=["END"],
        system_instruction="You are a senior ML engineer. Be precise and concise.",
    ),
)
print(response.text)
```

### Legacy vertexai SDK (Text Generation)

```python
import vertexai
from vertexai.generative_models import GenerativeModel, GenerationConfig, SafetySetting, HarmCategory, HarmBlockThreshold

vertexai.init(project="my-project", location="us-central1")

model = GenerativeModel(
    "gemini-2.5-flash",
    system_instruction="You are a senior ML engineer. Be precise and concise.",
)

generation_config = GenerationConfig(
    temperature=0.7,
    top_p=0.95,
    top_k=40,
    max_output_tokens=1024,
    stop_sequences=["END"],
    candidate_count=1,
    presence_penalty=0.0,
    frequency_penalty=0.0,
    seed=42,  # for deterministic outputs
    response_mime_type="text/plain",  # or "application/json"
)

safety_settings = [
    SafetySetting(
        category=HarmCategory.HARM_CATEGORY_HATE_SPEECH,
        threshold=HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    ),
    SafetySetting(
        category=HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT,
        threshold=HarmBlockThreshold.BLOCK_ONLY_HIGH,
    ),
    SafetySetting(
        category=HarmCategory.HARM_CATEGORY_HARASSMENT,
        threshold=HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    ),
    SafetySetting(
        category=HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT,
        threshold=HarmBlockThreshold.BLOCK_MEDIUM_AND_ABOVE,
    ),
]

response = model.generate_content(
    "Explain transformer architecture.",
    generation_config=generation_config,
    safety_settings=safety_settings,
)
print(response.text)
```

#### generation_config fields

| Field | Range / Default | Notes |
| --- | --- | --- |
| `temperature` | 0.0–2.0, default 1.0 | Lower = more deterministic |
| `top_p` | 0.0–1.0, default 0.95 | Nucleus sampling cutoff |
| `top_k` | int | Consider top-k tokens |
| `max_output_tokens` | int | ~4 chars per token |
| `candidate_count` | 1–8 (Preview) | Multiple response candidates |
| `stop_sequences` | up to 5 strings | Generation halt triggers |
| `presence_penalty` | -2.0–2.0 | Penalizes repeated tokens |
| `frequency_penalty` | -2.0–2.0 | Reduces repetition probability |
| `seed` | int | Deterministic output across calls |
| `response_mime_type` | `text/plain`, `application/json`, `text/x.enum` | Force JSON output |

**HarmBlockThreshold values:** `BLOCK_LOW_AND_ABOVE` (strictest), `BLOCK_MEDIUM_AND_ABOVE`, `BLOCK_ONLY_HIGH`, `BLOCK_NONE` / `OFF`.


## Chat Sessions

Chat sessions maintain conversation history automatically.

### New SDK (Chat Sessions)

```python
from google import genai
from google.genai import types

client = genai.Client()

chat = client.chats.create(
    model="gemini-2.5-flash",
    config=types.GenerateContentConfig(
        system_instruction="You are a helpful coding assistant.",
        temperature=0.5,
    ),
)

response = chat.send_message("What's the difference between a list and a tuple in Python?")
print(response.text)

response = chat.send_message("Which one should I use for coordinates?")
print(response.text)

# Inspect history
for turn in chat.get_history():
    print(f"{turn.role}: {turn.parts[0].text}")
```

### Legacy vertexai SDK (Chat Sessions)

```python
import vertexai
from vertexai.generative_models import GenerativeModel, Content, Part

vertexai.init(project="my-project", location="us-central1")

model = GenerativeModel("gemini-2.5-flash")
chat = model.start_chat(history=[])

response = chat.send_message("Explain Python decorators.")
print(response.text)

response = chat.send_message("Show me a real-world example.")
print(response.text)

# Seed history manually
seeded_history = [
    Content(role="user", parts=[Part.from_text("Hi, I'm debugging a memory leak.")]),
    Content(role="model", parts=[Part.from_text("I can help. What language and framework?")]),
]
chat = model.start_chat(history=seeded_history)
```


## Streaming

Use streaming to start displaying output before the full response is ready — critical for latency-sensitive UIs.

### New SDK (Streaming)

```python
from google import genai

client = genai.Client()

response_stream = client.models.generate_content_stream(
    model="gemini-2.5-flash",
    contents="Write a detailed explanation of how TCP/IP works.",
)

for chunk in response_stream:
    print(chunk.text, end="", flush=True)
```

### Streaming chat

```python
chat = client.chats.create(model="gemini-2.5-flash")

for chunk in chat.send_message_stream("Explain async/await in Python step by step."):
    print(chunk.text, end="", flush=True)
```

### Legacy vertexai SDK (Streaming)

```python
import vertexai
from vertexai.generative_models import GenerativeModel

vertexai.init(project="my-project", location="us-central1")
model = GenerativeModel("gemini-2.5-flash")

for chunk in model.generate_content("Describe quantum entanglement.", stream=True):
    print(chunk.text, end="", flush=True)
```

**Note:** `chunk.text` raises if the chunk has no text part (e.g., safety block or function call). Use `chunk.candidates[0].content.parts` to safely iterate all parts.


## Vertex AI RAG Engine

The RAG Engine manages corpus creation, document ingestion, chunking, and retrieval. Available only on Vertex AI.

### Create a corpus and ingest documents

```python
from vertexai.preview import rag
import vertexai

vertexai.init(project="my-project", location="us-central1")

# Create a corpus
corpus = rag.create_corpus(display_name="product-docs-corpus")
print(f"Corpus name: {corpus.name}")

# Import files from GCS
rag.import_files(
    corpus_name=corpus.name,
    paths=["gs://my-bucket/docs/"],  # GCS prefix or individual files
    chunk_size=512,       # tokens per chunk
    chunk_overlap=100,    # overlap between chunks
)

# Import from Google Drive
rag.import_files(
    corpus_name=corpus.name,
    paths=["https://drive.google.com/drive/folders/FOLDER_ID"],
)
```

### Query the RAG corpus directly

```python
response = rag.retrieval_query(
    rag_resources=[
        rag.RagResource(
            rag_corpus=corpus.name,
        )
    ],
    text="How do I reset my password?",
    similarity_top_k=5,
    vector_distance_threshold=0.5,
)

for context in response.contexts.contexts:
    print(context.text)
    print(f"Score: {context.distance}")
```

### Generate with RAG grounding

```python
from vertexai.generative_models import GenerativeModel, Tool
from vertexai.preview.generative_models import grounding

rag_retrieval_tool = Tool.from_retrieval(
    retrieval=grounding.Retrieval(
        source=grounding.VertexRagStore(
            rag_resources=[
                grounding.RagResource(rag_corpus=corpus.name)
            ],
            similarity_top_k=5,
            vector_distance_threshold=0.5,
        )
    )
)

model = GenerativeModel("gemini-2.5-flash", tools=[rag_retrieval_tool])
response = model.generate_content("How do I configure SSO?")
print(response.text)
```


## Model Tuning

Supervised fine-tuning (SFT) adapts a base Gemini model to domain-specific tasks using labeled examples.

### Dataset format

Training data is a JSONL file where each line is one example:

```jsonl
{"contents": [{"role": "user", "parts": [{"text": "Classify sentiment: 'The product broke on day 1.'"}]}, {"role": "model", "parts": [{"text": "negative"}]}]}
{"contents": [{"role": "user", "parts": [{"text": "Classify sentiment: 'Best purchase I've made this year!'"}]}, {"role": "model", "parts": [{"text": "positive"}]}]}
```

For multi-turn chat tuning:

```jsonl
{"contents": [{"role": "user", "parts": [{"text": "What is your return policy?"}]}, {"role": "model", "parts": [{"text": "You can return items within 30 days with receipt."}]}, {"role": "user", "parts": [{"text": "What if I lost the receipt?"}]}, {"role": "model", "parts": [{"text": "We can look up your purchase with your email address."}]}]}
```

### Start a tuning job

```python
import vertexai
from vertexai.preview.tuning import sft

vertexai.init(project="my-project", location="us-central1")

tuning_job = sft.train(
    source_model="gemini-2.5-flash-002",
    train_dataset="gs://my-bucket/tuning/train.jsonl",
    validation_dataset="gs://my-bucket/tuning/val.jsonl",
    epochs=3,
    learning_rate_multiplier=1.0,
    tuned_model_display_name="sentiment-classifier-v1",
)

# Poll for completion
tuning_job.refresh()
print(f"State: {tuning_job.state}")
print(f"Tuned model: {tuning_job.tuned_model_name}")
```

### Use the tuned model

```python
tuned_model = GenerativeModel(tuning_job.tuned_model_endpoint_name)
response = tuned_model.generate_content("Classify sentiment: 'Works exactly as advertised.'")
print(response.text)
```

#### Tuning best practices

- Minimum ~100 examples; 500–1000 typical for reliable improvement
- High-quality, diverse data beats raw quantity
- Always keep a held-out validation set
- Start with prompt engineering; tune only when you've hit a prompt ceiling
- Compare checkpoint performance — don't just take the final checkpoint


## Vertex AI Endpoints

For deploying custom or tuned models at scale.

### Online prediction endpoint

```python
import vertexai
from google.cloud import aiplatform

vertexai.init(project="my-project", location="us-central1")

# After tuning, the tuned model is automatically deployed to an endpoint
# Endpoint resource name format:
# projects/{project}/locations/{location}/endpoints/{endpoint_id}

endpoint = aiplatform.Endpoint(endpoint_name="projects/my-project/locations/us-central1/endpoints/ENDPOINT_ID")

# Predict directly via the endpoint
response = endpoint.predict(instances=[{"content": "What is machine learning?"}])
print(response.predictions)
```

### Autoscaling

Configure min/max replicas and traffic splits when deploying a model to an endpoint:

```python
model_resource = aiplatform.Model(model_name="projects/my-project/locations/us-central1/models/MODEL_ID")

endpoint = model_resource.deploy(
    machine_type="n1-standard-4",
    accelerator_type="NVIDIA_TESLA_T4",
    accelerator_count=1,
    min_replica_count=1,
    max_replica_count=5,
    traffic_percentage=100,
)
```

### Batch prediction

For high-throughput offline inference (e.g., processing millions of records overnight):

```python
batch_prediction_job = model_resource.batch_predict(
    job_display_name="batch-inference-run-1",
    instances_format="jsonl",
    predictions_format="jsonl",
    gcs_source=["gs://my-bucket/input/batch_input.jsonl"],
    gcs_destination_prefix="gs://my-bucket/output/",
    machine_type="n1-standard-4",
    starting_replica_count=1,
    max_replica_count=5,
)
batch_prediction_job.wait()
```


## References

- Vertex AI Generative AI overview: <https://cloud.google.com/vertex-ai/generative-ai/docs/learn/overview>
- Google Gen AI SDK for Python: <https://docs.cloud.google.com/vertex-ai/generative-ai/docs/sdks/overview>
- Vertex AI Python SDK reference: <https://cloud.google.com/python/docs/reference/vertexai/latest>
- Gemini API inference reference: <https://cloud.google.com/vertex-ai/generative-ai/docs/model-reference/inference>
- Gemini API quickstart (multimodal): <https://cloud.google.com/vertex-ai/generative-ai/docs/start/quickstarts/quickstart-multimodal>
- Function calling guide: <https://ai.google.dev/gemini-api/docs/function-calling>
- Grounding with Google Search: <https://ai.google.dev/gemini-api/docs/grounding>
- Vertex AI RAG Engine: <https://cloud.google.com/vertex-ai/generative-ai/docs/rag-overview>
- Context caching: <https://ai.google.dev/gemini-api/docs/caching>
- Embeddings: <https://ai.google.dev/gemini-api/docs/embeddings>
- Model tuning: <https://cloud.google.com/vertex-ai/generative-ai/docs/models/tune-models>
- System instructions: <https://cloud.google.com/vertex-ai/generative-ai/docs/learn/prompts/system-instructions>
- Vertex AI locations: <https://cloud.google.com/vertex-ai/generative-ai/docs/learn/locations>
- Google AI Studio: <https://aistudio.google.com>
