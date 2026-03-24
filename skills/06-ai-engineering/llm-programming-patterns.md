---
name: llm-programming-patterns
description: LLM programming patterns covering structured output (JSON schema, function calling, constrained generation, validation) and DSPy (declarative LLM programming, signatures, modules, optimizers, assertions). Use when building reliable LLM pipelines with structured outputs or programmatic optimization.
domain: ai-engineering
tags: [structured-output, json-schema, function-calling, dspy, llm-programming, constrained-generation, optimizers]
triggers: structured output, json schema, function calling, dspy, constrained generation, LLM programming, output validation
---


# Structured Output / Constrained Generation Patterns for LLMs

## 1. The Three Mechanisms

There are three distinct mechanisms for getting structured data from an LLM. They operate at different layers and have different reliability guarantees.

### 1.1 JSON Mode (prompt-guided)

The model is instructed to return valid JSON via the system prompt or a `response_format: {type: "json_object"}` flag. The API guarantees syntactically valid JSON — nothing more. Field names, types, and schema shape are not enforced. A model can return `{"result": null}` and still satisfy JSON mode.

**When to use:** Quick prototyping, models that don't support strict schemas, or when schema enforcement is handled downstream.

**Gotcha:** JSON mode without a schema produces valid JSON but not necessarily *your* JSON. Always include the target schema in the prompt even when using this mode.

### 1.2 Provider-native Structured Outputs (schema-enforced)

The API accepts a JSON Schema and the provider enforces it at generation time, typically via constrained decoding on the backend.

**OpenAI** (`response_format` with `type: "json_schema"`):

```python
response = client.beta.chat.completions.parse(
    model="gpt-4o-2024-08-06",
    messages=[...],
    response_format=MyPydanticModel,  # SDK converts to JSON Schema
)
result = response.choices[0].message.parsed  # typed instance
```

OpenAI's strict mode was introduced August 2024. It improves schema compliance from ~35% (prompt-only) to ~100% on their benchmark. Setting `strict: true` disables extra fields and requires all properties to be listed under `required`.

**Anthropic Claude** (`output_config.format`):

```python
response = client.messages.create(
    model="claude-opus-4-6",
    max_tokens=1024,
    messages=[...],
    output_config={
        "format": {
            "type": "json_schema",
            "schema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "confidence": {"type": "number"}
                },
                "required": ["name", "confidence"],
                "additionalProperties": False
            }
        }
    }
)
parsed = json.loads(response.content[0].text)
```

Python and TypeScript SDKs expose `client.messages.parse()` which accepts Pydantic models or Zod schemas, handles schema transformation, and returns a `parsed_output` attribute. Claude's SDK strips unsupported constraints from the sent schema (e.g., `minimum`, `pattern`) but moves their semantics into field descriptions and re-validates locally against the original schema.

#### Schema features supported by both OpenAI and Claude

- `string`, `number`, `integer`, `boolean`, `array`, `object`, `null`
- `enum`, `required`, `additionalProperties: false`
- Nested objects and arrays
- `anyOf` for unions / optional fields

#### Not supported (causes silent stripping or errors)

- `minimum`, `maximum`, `minLength`, `maxLength`, `pattern`
- `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`
- `dependencies`
- Most `format` values (Claude supports `date`, `email`, `uri`, `uuid`)

Always set `additionalProperties: false` on every object in the schema. Without it, models may emit extra fields that pass schema validation but break downstream deserialization.

### 1.3 Tool Use / Function Calling

The model returns a structured `tool_use` or `function_call` block rather than text content. This is the oldest structured output mechanism and has the most provider support.

#### Anthropic strict tool use

```python
tools = [{
    "name": "extract_invoice",
    "description": "Extract invoice fields from the provided text.",
    "strict": True,
    "input_schema": {
        "type": "object",
        "properties": {
            "vendor": {"type": "string"},
            "amount": {"type": "number"},
            "date": {"type": "string", "format": "date"},
            "line_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "description": {"type": "string"},
                        "unit_price": {"type": "number"},
                        "quantity": {"type": "integer"}
                    },
                    "required": ["description", "unit_price", "quantity"],
                    "additionalProperties": False
                }
            }
        },
        "required": ["vendor", "amount", "date", "line_items"],
        "additionalProperties": False
    }
}]
```

The response comes back as `content[x].input` (Anthropic) or `choices[0].message.tool_calls[0].function.arguments` (OpenAI, as a JSON string that still needs `json.loads()`).

**Forcing a tool call:** Set `tool_choice` to the specific tool name to guarantee the model invokes it rather than responding in prose. Without this, the model may choose to answer in text instead.

```python
# Anthropic
tool_choice={"type": "tool", "name": "extract_invoice"}

# OpenAI
tool_choice={"type": "function", "function": {"name": "extract_invoice"}}
```

**Tool use vs. response_format:** Tool use is better when the structured data is the *action* or *side effect* (calling an external API, triggering a workflow). `response_format`/`output_config` is better when the structured data is the *final answer* to the user. In agentic systems, you often use both: tools for intermediate steps, a final output schema for the terminal response.


## 3. Schema Design for Reliable Extraction

### 3.1 Keep schemas flat and specific

Deep nesting is the primary cause of extraction failures. Each nested level adds uncertainty. If you have a 4-level hierarchy, the model must correctly predict the structure at every level simultaneously.

#### Prefer

```python
class OrderSummary(BaseModel):
    order_id: str
    customer_name: str
    total_amount: float
    item_count: int
    shipped: bool
```

**Over deeply nested alternatives** when a flat representation is possible. If nesting is unavoidable, break the extraction into multiple passes: extract top-level first, then extract nested details in a second call with context.

### 3.2 Use enums for constrained vocabularies

```python
from enum import Enum

class Sentiment(str, Enum):
    POSITIVE = "positive"
    NEGATIVE = "negative"
    NEUTRAL = "neutral"

class Review(BaseModel):
    sentiment: Sentiment
    summary: str
    score: int  # 1-5
```

Enum fields give the model a closed set of valid tokens to choose from, dramatically reducing hallucinated values. Use `str` as the base type so JSON serialization works without extra configuration.

### 3.3 Optional fields

Mark fields optional when they might genuinely be absent in the source text. Never use `Optional` as a shortcut to avoid schema design — it creates ambiguity.

```python
from typing import Optional

class Contact(BaseModel):
    name: str                          # always required
    email: Optional[str] = None        # present only if mentioned
    phone: Optional[str] = None
    company: Optional[str] = None
```

In JSON Schema terms, `Optional[str]` renders as `{"anyOf": [{"type": "string"}, {"type": "null"}]}`. With `additionalProperties: false`, this is the only correct way to express nullable fields.

### 3.4 Use Field descriptions as prompts

The model reads field descriptions. Write them as instructions, not just labels.

```python
class Analysis(BaseModel):
    reasoning: str = Field(
        description="Step-by-step reasoning before reaching the conclusion. "
                    "Think through the evidence carefully."
    )
    conclusion: str = Field(
        description="Final answer in one sentence."
    )
    confidence: float = Field(
        description="Confidence from 0.0 (uncertain) to 1.0 (certain).",
        ge=0.0,
        le=1.0,
    )
```

The `ge`/`le` constraints are stripped from the schema sent to the API but appear in the description after SDK transformation and are validated locally on the returned value.

### 3.5 Two-pass extraction for complex documents

For tasks where reasoning quality matters, separate thinking from formatting:

```python
# Pass 1: free-form reasoning
reasoning_response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "system", "content": "Analyze the contract and explain your findings."},
        {"role": "user", "content": contract_text},
    ]
)

# Pass 2: structured extraction from the reasoning
structured_response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "user", "content": f"Given this analysis:\n{reasoning_response.choices[0].message.content}\n\nExtract the structured fields."}
    ],
    response_model=ContractSummary,
)
```

Research (Tam et al., 2024) found 10–15% accuracy degradation on complex reasoning tasks when the model is forced directly into JSON output, because it can't use chain-of-thought naturally. The two-pass approach recovers this. Alternatively, include a `reasoning` field in the schema itself so the model can think before committing to the structured fields.


## 5. Streaming Structured Output

Provider-native streaming with structured outputs works by buffering the token stream and parsing at completion. True incremental partial-object streaming requires a library like Instructor.

```python
# Instructor partial streaming — fields become available as tokens arrive
from instructor import from_openai
from pydantic import BaseModel
from openai import OpenAI

client = from_openai(OpenAI())

class Report(BaseModel):
    title: str
    executive_summary: str
    key_findings: list[str]
    recommendation: str

for partial in client.chat.completions.create_partial(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Write a report on..."}],
    response_model=Report,
):
    if partial.title:
        update_ui_title(partial.title)
    if partial.executive_summary:
        update_ui_summary(partial.executive_summary)
```

For streaming a list of homogeneous objects (e.g., rows of extracted data), `create_iterable` emits one complete validated object per yield, which is more useful than partial field access.


## 7. Common Failure Modes

### 7.1 Truncated output

**Symptom:** The JSON cuts off mid-object, causing a parse error.
**Cause:** `max_tokens` is too low for the schema. JSON overhead is substantial — an object with 10 fields and string values easily consumes 300+ tokens just in structure.
**Fix:** Estimate token budget: `max_tokens = len(schema_fields) * avg_value_tokens * 3`. When using strict structured outputs, token counting also includes JSON scaffolding.

### 7.2 Hallucinated field names

**Symptom:** The response includes fields not in the schema (e.g., `"summary_text"` instead of `"summary"`).
**Cause:** JSON mode without schema enforcement, or a model that hasn't been fine-tuned on your schema.
**Fix:** Use strict mode + `additionalProperties: false`. If that's unavailable, include the exact JSON structure as an example in the system prompt.

### 7.3 Type coercion errors

**Symptom:** `"34"` returned for an `integer` field, or `"true"` (string) for a `boolean`.
**Cause:** Model treats all values as strings in JSON mode; more common with smaller models.
**Fix:** Use `field_validator` to coerce, or rely on Pydantic's automatic coercion in `model_validate`. For critical fields, add explicit validator:

```python
@field_validator("count", mode="before")
@classmethod
def coerce_int(cls, v):
    return int(v) if isinstance(v, str) else v
```

### 7.4 Empty optional fields as empty string

**Symptom:** `"email": ""` returned instead of `"email": null` for an absent field.
**Cause:** Model confuses `null` with `""` for optional string fields.
**Fix:** Add a validator:

```python
@field_validator("email", mode="before")
@classmethod
def empty_str_to_none(cls, v):
    return None if v == "" else v
```

### 7.5 Reasoning contamination

**Symptom:** The model includes reasoning text inside a string field (e.g., `"name": "Let me analyze... the name is John"`).
**Cause:** The model is doing chain-of-thought inside structured fields.
**Fix:** Include a dedicated `reasoning` field before the answer fields so the model exhausts its thinking there, or use the two-pass approach. Explicitly instruct: "Do not include reasoning in any field except `reasoning`."

### 7.6 Schema too complex for the model

**Symptom:** High retry rate, fields consistently wrong, model ignores schema constraints.
**Cause:** Schema cognitive load exceeds model capacity. Schemas with 20+ fields, deep nesting, and complex business logic are hard for 7B–13B models.
**Fix:** Decompose into multiple simpler extractions. Use a capable model (GPT-4o, Claude Sonnet/Opus, Gemini 1.5 Pro) for complex schemas. Simplify field names to be unambiguous.


## 9. Quick Decision Tree

```text
Need structured output?
│
├─ Using commercial API (OpenAI/Anthropic/Gemini)?
│   ├─ Python? → Use Instructor with appropriate mode
│   │   ├─ OpenAI → instructor.from_openai(), TOOLS mode (default)
│   │   └─ Anthropic → instructor.from_anthropic(), ANTHROPIC_TOOLS mode
│   └─ Raw API?
│       ├─ OpenAI → response_format json_schema + strict:true
│       └─ Anthropic → output_config.format + tool strict:true
│
├─ Local model?
│   ├─ Need hard guarantees, fixed schema → Outlines or Guidance
│   ├─ vLLM → guided_json parameter (XGrammar backend)
│   ├─ llama.cpp → GBNF grammar or JSON schema mode
│   └─ Flexible → Instructor with JSON_SCHEMA mode
│
└─ Complex reasoning + extraction?
    └─ Two-pass: free reasoning first, structured extraction second
```


---


# DSPy — Programmatic LLM Pipeline Compilation

DSPy ("Declarative Self-improving Python") is a Stanford NLP framework for *programming* language models rather than hand-crafting prompts. You write modular Python code; DSPy's optimizers automatically find the best instructions and demonstrations through evaluation-driven search. Published as an ICLR 2024 paper.

```bash
pip install dspy
```


## Core Abstraction: Signatures

A Signature declares *what* the LM should do — input fields, output fields, and optionally a task description (docstring). It separates interface from implementation; the optimizer can then rewrite the instructions without changing your code.

### Inline signatures

```python
import dspy

# Minimal
predict = dspy.Predict("question -> answer")

# With types
predict = dspy.Predict("context: list[str], question: str -> answer: str")

# Typed output
predict = dspy.Predict("sentence -> sentiment: bool")

# Multiple outputs
predict = dspy.Predict("question, choices: list[str] -> reasoning: str, selection: int")
```

### Class-based signatures

```python
from typing import Literal

class BasicQA(dspy.Signature):
    """Answer questions with short factoid answers."""
    question: str = dspy.InputField()
    answer: str = dspy.OutputField(desc="often between 1 and 5 words")

class Emotion(dspy.Signature):
    """Classify emotion."""
    sentence: str = dspy.InputField()
    sentiment: Literal['sadness', 'joy', 'love', 'anger', 'fear', 'surprise'] = dspy.OutputField()

class CheckCitationFaithfulness(dspy.Signature):
    """Verify text accuracy against provided context."""
    context: str = dspy.InputField(desc="facts assumed true")
    text: str = dspy.InputField()
    faithfulness: bool = dspy.OutputField()
    evidence: dict[str, list[str]] = dspy.OutputField(desc="Supporting evidence per claim")
```

**Field types supported:** `str`, `int`, `float`, `bool`, `list[str]`, `dict[str, int]`, `Optional[T]`, `Literal[...]`, Pydantic models, `dspy.Image`, `dspy.History`.

The docstring becomes the task instruction the optimizer can rewrite. Field `desc` provides extra context without being rewritten.


## Building Pipelines

Compose modules by subclassing `dspy.Module`. Define sub-modules in `__init__`, wire them in `forward()`.

```python
class MultiHopQA(dspy.Module):
    def __init__(self, num_docs=10, num_hops=4):
        self.num_docs = num_docs
        self.num_hops = num_hops
        self.generate_query = dspy.ChainOfThought("claim, notes -> query")
        self.append_notes = dspy.ChainOfThought("claim, notes, context -> new_notes: list[str]")

    def forward(self, claim: str) -> dspy.Prediction:
        notes = []
        for _ in range(self.num_hops):
            query = self.generate_query(claim=claim, notes=notes).query
            context = search(query, k=self.num_docs)  # your retrieval function
            prediction = self.append_notes(claim=claim, notes=notes, context=context)
            notes.extend(prediction.new_notes)
        return dspy.Prediction(notes=notes)

program = MultiHopQA()
result = program(claim="The Roman Empire fell in 476 AD.")
```

The `forward()` method is called when you invoke the module. It returns a `dspy.Prediction` object with named fields. DSPy tracks all LM calls made inside `forward()` for optimization.


## Evaluation

A metric is a Python function `(example, prediction, trace=None) -> float | bool`. The `trace` argument is non-None during optimization; you can use it to apply stricter constraints during compilation.

### Simple metric

```python
def parse_integer_answer(answer):
    try:
        answer = [tok for tok in answer.strip().split('\n')[0].split()
                  if any(c.isdigit() for c in tok)][-1]
        return int(''.join(c for c in answer.split('.')[0] if c.isdigit()))
    except (ValueError, IndexError):
        return 0

def gsm8k_metric(gold, pred, trace=None) -> bool:
    return parse_integer_answer(str(gold.answer)) == parse_integer_answer(str(pred.answer))
```

### LLM-as-judge metric

```python
class FactJudge(dspy.Signature):
    """Judge if the answer is factually correct based on the context."""
    context = dspy.InputField(desc="Context for the prediction")
    question = dspy.InputField(desc="Question to be answered")
    answer = dspy.InputField(desc="Answer for the question")
    factually_correct: bool = dspy.OutputField(desc="Is the answer factually correct?")

judge = dspy.ChainOfThought(FactJudge)

def factuality_metric(example, pred, trace=None):
    result = judge(context=example.context, question=example.question, answer=pred.answer)
    return result.factually_correct
```

### Running evaluation

```python
from dspy.evaluate import Evaluate

# Dataset: list of dspy.Example with .with_inputs() specifying which fields are inputs
devset = [
    dspy.Example(question="What year did WWII end?", answer="1945").with_inputs("question"),
    # ...
]

evaluator = Evaluate(
    devset=devset,
    metric=gsm8k_metric,
    num_threads=16,
    display_progress=True,
    display_table=5,       # show first 5 rows
)

score = evaluator(program=my_program)
# Returns EvaluationResult with .score (float, e.g. 67.3) and .results list
```

**Built-in metrics:** `dspy.evaluate.answer_exact_match`, `dspy.evaluate.answer_passage_match`, `dspy.evaluate.SemanticF1`, `dspy.evaluate.CompleteAndGrounded`.


## RAG with DSPy

### Using dspy.Retrieve with ColBERT

```python
colbertv2 = dspy.ColBERTv2(url='http://20.102.90.50:2017/wiki17_abstracts')
dspy.configure(rm=colbertv2)

retriever = dspy.Retrieve(k=3)
results = retriever(query="When was the first FIFA World Cup held?")
for passage in results.passages:
    print(passage)
```

### Building a RAG module

```python
class RAG(dspy.Module):
    def __init__(self, num_passages=3):
        self.retrieve = dspy.Retrieve(k=num_passages)
        self.generate_answer = dspy.ChainOfThought("context, question -> answer")

    def forward(self, question: str) -> dspy.Prediction:
        context = self.retrieve(question).passages
        prediction = self.generate_answer(context=context, question=question)
        return dspy.Prediction(context=context, answer=prediction.answer)

rag = RAG()
result = rag(question="Who wrote the Harry Potter series?")
print(result.answer)
```

### Custom retriever

Any callable that returns a list of strings works. Wrap it so DSPy modules can call it naturally:

```python
import dspy

def my_retriever(query: str, k: int = 3) -> list[str]:
    # call your vector DB, BM25, or API
    return ["passage 1 ...", "passage 2 ...", "passage 3 ..."]

class RAGWithCustomRetriever(dspy.Module):
    def __init__(self, k=3):
        self.k = k
        self.generate_answer = dspy.ChainOfThought("context: list[str], question -> answer")

    def forward(self, question: str) -> dspy.Prediction:
        context = my_retriever(question, k=self.k)
        return self.generate_answer(context=context, question=question)
```

### Optimizing a RAG pipeline

RAG modules optimize identically to any other module — pass the program to an optimizer:

```python
from dspy.teleprompt import BootstrapFewShot

def rag_metric(example, pred, trace=None):
    return example.answer.lower() in pred.answer.lower()

optimizer = BootstrapFewShot(metric=rag_metric, max_bootstrapped_demos=3)
compiled_rag = optimizer.compile(RAG(), trainset=trainset)
```


## Saving and Loading

DSPy saves *program state*: the compiled instructions, few-shot demos, and per-module LM assignments. It does not save the Python class definition.

```python
# Save compiled state to JSON (recommended — human-readable, safe)
compiled.save("my_program.json")

# Save as pickle (needed only if state contains non-serializable objects)
compiled.save("my_program.pkl")

# Load: recreate the same program class first, then load state into it
loaded = MyProgramClass()
loaded.load(path="my_program.json")

# Save entire program (architecture + state), requires dspy >= 2.6.0
compiled.save("my_program_full.json", save_program=True)
```

What gets saved: signature field definitions, optimized instruction text per predictor, few-shot demo examples, per-predictor LM overrides.

**Security:** Never load `.pkl` files from untrusted sources — pickle can execute arbitrary code.


## References

- DSPy documentation: <https://dspy.ai>
- Optimizer reference: <https://dspy.ai/learn/optimization/optimizers/>
- Cheatsheet: <https://dspy.ai/cheatsheet/>
- GitHub: <https://github.com/stanfordnlp/dspy>
- ICLR 2024 paper: "DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines"
- DSPy Assertions paper: <https://arxiv.org/abs/2312.13382>
- Stanford HAI writeup: <https://hai.stanford.edu/research/dspy-compiling-declarative-language-model-calls-into-state-of-the-art-pipelines>
