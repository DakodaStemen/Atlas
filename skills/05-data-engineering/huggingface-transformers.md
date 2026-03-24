---
name: huggingface-transformers
description: Comprehensive guide to the Hugging Face Transformers ecosystem covering the pipeline API, AutoModel/AutoTokenizer pattern, Trainer API, PEFT/LoRA fine-tuning, QLoRA with BitsAndBytes quantization, model hub push/pull, batched inference, tokenizer gotchas, and common errors.
domain: ai-ml
category: nlp
tags: [HuggingFace, transformers, PEFT, LoRA, QLoRA, fine-tuning, inference, quantization, BitsAndBytes, Trainer, SFTTrainer, trl]
triggers: [huggingface, transformers, fine-tune, LoRA, PEFT, QLoRA, pipeline inference, AutoModel, AutoTokenizer, Trainer, BitsAndBytes, model hub]
---

# Hugging Face Transformers — Best Practices

## 1. Core Design

Transformers is the model-definition framework for the HF ecosystem. Every model is built from three classes: **configuration**, **model**, and **preprocessor** (tokenizer/feature extractor/processor). This means a model that loads correctly in `transformers` is automatically compatible with training frameworks (Axolotl, DeepSpeed, FSDP, PyTorch-Lightning, Unsloth) and inference engines (vLLM, SGLang, TGI).

The default model format is **safetensors** — prefer it over `pytorch_model.bin` for both security and load-speed reasons.

---

## 2. AutoModel / AutoTokenizer Pattern

Always use the `Auto*` classes unless you have a specific reason to instantiate a named class directly. They resolve the correct architecture from the Hub config automatically.

```python
from transformers import AutoTokenizer, AutoModelForCausalLM
import torch

model_id = "meta-llama/Meta-Llama-3.1-8B-Instruct"

tokenizer = AutoTokenizer.from_pretrained(model_id)
model = AutoModelForCausalLM.from_pretrained(
    model_id,
    torch_dtype=torch.bfloat16,   # bfloat16 is preferred over float16 on Ampere+
    device_map="auto",            # auto-distribute across available GPUs/CPU
    attn_implementation="flash_attention_2",  # requires flash-attn installed
)
```

Common `AutoModel` variants:

- `AutoModelForCausalLM` — decoder-only text generation (GPT, LLaMA, Mistral, Gemma)
- `AutoModelForSeq2SeqLM` — encoder-decoder (T5, BART, mT5)
- `AutoModelForSequenceClassification` — text classification
- `AutoModelForTokenClassification` — NER, POS tagging
- `AutoModelForQuestionAnswering` — extractive QA

**Gotcha:** always load the tokenizer from the same checkpoint as the model. Using a mismatched tokenizer (e.g., a base model tokenizer with an instruction-tuned model) causes incorrect special token handling.

---

## 3. Pipeline API

`pipeline` is the fastest path to inference. It wraps tokenization, forward pass, and post-processing.

```python
from transformers import pipeline

# Minimal — uses a default model for the task
classifier = pipeline("text-classification")

# Always specify the model explicitly in production to avoid surprise changes
classifier = pipeline(
    task="text-classification",
    model="distilbert/distilbert-base-uncased-finetuned-sst-2-english",
    device=0,          # GPU 0; use device_map="auto" for multi-GPU
)
result = classifier("This film was absolutely brilliant.")
# [{'label': 'POSITIVE', 'score': 0.9998}]
```

### Device assignment

```python
# Single GPU
pipe = pipeline("text-generation", model="google/gemma-2-2b", device=0)

# Multi-GPU / CPU offload
pipe = pipeline("text-generation", model="google/gemma-2-2b", device_map="auto")

# Apple Silicon
pipe = pipeline("text-generation", model="google/gemma-2-2b", device="mps")
```

### Batch inference

Batch inference can improve throughput on GPU but is disabled by default. Apply it carefully:

```python
pipe = pipeline("text-classification", model="...", device=0, batch_size=16)
results = pipe(["text one", "text two", "text three"])
```

Rules of thumb (from official docs):

- Do not batch if you are latency-constrained (live API serving).
- Do not batch on CPU.
- Do not batch when sequence lengths vary widely without OOM guards.
- Benchmark before committing — improvement is not guaranteed.

### Streaming large datasets

Use `KeyDataset` to avoid loading an entire dataset into memory:

```python
from transformers.pipelines.pt_utils import KeyDataset
from datasets import load_dataset

dataset = load_dataset("imdb", split="test")
pipe = pipeline("text-classification", model="...", device=0)
for out in pipe(KeyDataset(dataset, "text"), batch_size=8, truncation="only_first"):
    print(out)
```

### Quantized models in pipeline

```python
from transformers import pipeline, BitsAndBytesConfig
import torch

bnb_config = BitsAndBytesConfig(load_in_8bit=True)
pipe = pipeline(
    "text-generation",
    model="google/gemma-7b",
    dtype=torch.bfloat16,
    device_map="auto",
    model_kwargs={"quantization_config": bnb_config},
)
```

---

## 4. Tokenizer: Padding, Truncation, and Alignment Gotchas

These are the most common sources of silent bugs.

```python
# Always set padding_side for decoder models before batched generation
tokenizer.padding_side = "left"   # required for causal LM batch generation
# For encoder models (BERT etc.), padding_side = "right" is correct

# Ensure a pad token exists — many causal LMs (LLaMA, Mistral) ship without one
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token
    # do NOT add a new token unless you also resize model embeddings

inputs = tokenizer(
    ["Short sentence.", "A much longer sentence that needs padding."],
    padding=True,           # pad to the longest sequence in the batch
    truncation=True,        # truncate to model's max_position_embeddings
    max_length=512,
    return_tensors="pt",
).to(model.device)
```

Key rules:

- `padding=True` pads to the longest sequence in the batch; `padding="max_length"` pads to `max_length` — the latter wastes compute.
- `truncation=True` alone truncates to the model's default max; always also pass `max_length` explicitly in production.
- For instruction/chat models, use `apply_chat_template` to format inputs correctly before tokenizing.

```python
messages = [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the capital of France?"},
]
formatted = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
inputs = tokenizer(formatted, return_tensors="pt").to(model.device)
```

---

## 5. Trainer API

`Trainer` handles the training loop, mixed precision, gradient accumulation, `torch.compile`, and multi-GPU distribution.

```python
from transformers import Trainer, TrainingArguments, DataCollatorWithPadding

training_args = TrainingArguments(
    output_dir="./my-model",
    num_train_epochs=3,
    per_device_train_batch_size=8,
    per_device_eval_batch_size=8,
    gradient_accumulation_steps=4,       # effective batch = 8 * 4 = 32
    gradient_checkpointing=True,          # trade compute for memory
    gradient_checkpointing_kwargs={"use_reentrant": False},  # preferred
    learning_rate=2e-5,
    lr_scheduler_type="cosine",
    warmup_ratio=0.05,
    bf16=True,                            # use bf16 on Ampere+; fp16 otherwise
    logging_steps=50,
    evaluation_strategy="epoch",
    save_strategy="epoch",
    load_best_model_at_end=True,
    push_to_hub=True,                     # push after training
    hub_strategy="every_save",
    report_to="wandb",                    # or "tensorboard", "none"
)

trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=tokenized_train,
    eval_dataset=tokenized_eval,
    tokenizer=tokenizer,
    data_collator=DataCollatorWithPadding(tokenizer),
)
trainer.train()
trainer.push_to_hub()
```

For causal LM fine-tuning on instruction data, prefer `SFTTrainer` from `trl` — it handles chat template formatting and sequence packing automatically.

```python
from trl import SFTTrainer, SFTConfig

trainer = SFTTrainer(
    model=model,
    args=SFTConfig(
        output_dir="./my-model",
        max_seq_length=2048,
        packing=True,     # pack short sequences together to fill context window
        ...               # all TrainingArguments kwargs are valid
    ),
    train_dataset=dataset,
    processing_class=tokenizer,
)
trainer.train()
```

---

## 6. PEFT / LoRA Fine-Tuning

PEFT freezes base model weights and trains small adapter matrices. This cuts memory by 60–90% compared to full fine-tuning at minimal accuracy cost.

### LoRA configuration

```python
from peft import LoraConfig, TaskType, get_peft_model

lora_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    r=16,                           # rank; start at 8–16, increase if underfitting
    lora_alpha=16,                  # scaling = alpha/r; common to set equal to r
    lora_dropout=0.05,
    target_modules="all-linear",    # string shorthand; or list ["q_proj","v_proj",...]
    modules_to_save=["lm_head", "embed_tokens"],  # needed when chat template adds new special tokens
    bias="none",
    use_rslora=True,                # alpha/sqrt(r) scaling — more stable at higher ranks
    inference_mode=False,
)

model = get_peft_model(base_model, lora_config)
model.print_trainable_parameters()
# trainable params: 41,943,040 || all params: 8,072,560,640 || trainable%: 0.52
```

`target_modules` guidance:

- `"all-linear"` is the safest default for LLMs and covers q/k/v/o projections and FFN layers.
- Targeting only `["q_proj", "v_proj"]` (the original LoRA paper approach) uses fewer parameters but may underfit complex tasks.
- Always include `lm_head` and `embed_tokens` in `modules_to_save` when fine-tuning a **base** model with a new chat template — otherwise the new special tokens receive no gradient.

### Merging adapters for deployment

Keeping adapter weights separate adds loading overhead and inference latency. Merge before shipping to production:

```python
from peft import PeftModel

base_model = AutoModelForCausalLM.from_pretrained(model_id, torch_dtype=torch.bfloat16)
model = PeftModel.from_pretrained(base_model, "path/to/adapter")
merged = model.merge_and_unload()   # returns a plain transformers model
merged.save_pretrained("my-merged-model")
tokenizer.save_pretrained("my-merged-model")
```

### Adapter hotswapping (multi-adapter serving)

When switching between many LoRA adapters at inference time, hotswapping avoids memory accumulation and skips `torch.compile` recompilation:

```python
model = AutoModelForCausalLM.from_pretrained(...)
model.enable_peft_hotswap(target_rank=32)   # set to max rank across all adapters
model.load_adapter("adapter_1", adapter_name="default")
model = torch.compile(model)

# later, swap without reallocating or recompiling
model.load_adapter("adapter_2", hotswap=True, adapter_name="default")
```

---

## 7. QLoRA — 4-bit Quantized Fine-Tuning

QLoRA combines NF4 4-bit quantization (BitsAndBytes) with LoRA adapters. The base model is quantized (frozen); only adapter matrices are trained in bfloat16.

```python
from transformers import AutoModelForCausalLM, BitsAndBytesConfig
import torch

bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_quant_type="nf4",          # NF4 is the default and preferred
    bnb_4bit_compute_dtype=torch.bfloat16,
    bnb_4bit_use_double_quant=True,     # quantize the quantization constants too
)

model = AutoModelForCausalLM.from_pretrained(
    "meta-llama/Meta-Llama-3.1-8B",
    quantization_config=bnb_config,
    device_map="auto",
    attn_implementation="flash_attention_2",
)
model.config.use_cache = False           # must disable for gradient checkpointing
```

Then apply LoRA on top as normal with `get_peft_model`. After training, the adapter is saved separately. To deploy as a standalone model, merge (which dequantizes the base and adds adapter deltas):

```python
# merge_and_unload() on a quantized base will dequantize automatically
merged = model.merge_and_unload()
```

**LoftQ initialization** improves QLoRA accuracy by minimizing quantization error at init time:

```python
from peft import LoftQConfig, LoraConfig

loftq_config = LoftQConfig(loftq_bits=4)
lora_config = LoraConfig(
    ...,
    init_lora_weights="loftq",
    loftq_config=loftq_config,
)
# Load base model WITHOUT quantization_config when using LoftQ
base_model = AutoModelForCausalLM.from_pretrained(model_id)
model = get_peft_model(base_model, lora_config)
```

---

## 8. Model Hub Push / Pull

### Authentication

```python
from huggingface_hub import login
login(token="hf_...")   # or set HF_TOKEN env var
```

### Pull (download)

```python
# from_pretrained downloads and caches automatically
model = AutoModelForCausalLM.from_pretrained("owner/repo-name")
```

Control cache location with `HF_HOME` or `TRANSFORMERS_CACHE` env vars. Use `local_files_only=True` in offline environments.

### Push

```python
# After training
trainer.push_to_hub()

# Or manually
model.push_to_hub("my-username/my-model-name", private=True)
tokenizer.push_to_hub("my-username/my-model-name")

# Hub strategy during training — saves after each checkpoint
training_args = TrainingArguments(..., push_to_hub=True, hub_strategy="every_save")
```

### Gated models

Some models (LLaMA, Gemma) require accepting a license on the Hub before downloading. `from_pretrained` will raise an error with a URL — accept the license there, then re-run.

---

## 9. Quantization Reference (BitsAndBytes)

| Config | Memory | Speed | Quality |
| --- | --- | --- | --- |
| `load_in_8bit=True` | ~50% of fp16 | Slightly slower | Near-lossless |
| `load_in_4bit=True` + `nf4` | ~25% of fp16 | Faster on BNB kernels | Good for fine-tuning |
| `load_in_4bit` + double quant | ~23% of fp16 | Minimal overhead | Best memory efficiency |

8-bit is better for pure inference where quality is critical. 4-bit (QLoRA) is the standard for fine-tuning on consumer hardware.

---

## 10. Distributed Training

### Multi-GPU with Accelerate/DeepSpeed

```bash
accelerate launch \
  --config_file configs/deepspeed_zero3.yaml \
  --num_processes 8 \
  train.py --config my_config.yaml
```

DeepSpeed ZeRO-3 shards optimizer states, gradients, and parameters across GPUs. It allows training 70B+ models on a multi-GPU node. Use `gradient_checkpointing=True` alongside it.

**Performance reference** (10k samples, Llama-3.1-8B, QLoRA):

- Single GPU, no optimizations: ~360 min
- - Flash Attention 2: ~290 min
- - Flash Attention 2 + Liger Kernels: ~220 min
- - increased batch + packing: ~135 min
- 8× L4 GPU + all opts: ~18 min

---

## 11. Common Errors and Fixes

### `RuntimeError: CUDA out of memory`

- Reduce `per_device_train_batch_size`, increase `gradient_accumulation_steps` proportionally.
- Enable `gradient_checkpointing=True`.
- Switch to 4-bit QLoRA.
- Use `torch.cuda.empty_cache()` between runs; restart kernel before running inference after training.

#### `ValueError: Tokenizer class ... does not exist`

- The tokenizer's `tokenizer_class` field in `tokenizer_config.json` references a class not in your installed version. Update `transformers`.

#### Infinite loss or NaN during training

- Check for empty labels. `DataCollatorForSeq2Seq` / `DataCollatorForLanguageModeling` sets padding tokens to `-100` automatically — do not include padding in the loss manually.
- Use `bf16=True` instead of `fp16=True` — bfloat16 has wider range and is less prone to overflow.

#### `AttributeError: 'NoneType' object has no attribute 'pad_token_id'`

- The model config has no `pad_token_id`. Set it: `model.config.pad_token_id = tokenizer.eos_token_id`.

#### Slow generation

- Set `model.config.use_cache = True` for inference (disable it only during training with gradient checkpointing).
- Use `torch.compile(model, mode="reduce-overhead")` for repeated inference on fixed-length inputs.

#### Wrong output format from chat models

- Always use `apply_chat_template` — do not manually concatenate `[INST]` / `<|user|>` strings. Templates differ across model families.

#### Adapter produces garbage output after merge

- Verify `modules_to_save` included `lm_head`/`embed_tokens` if new tokens were added. If not, the output projection was never fine-tuned for those tokens.

---

## 12. Full QLoRA Fine-Tuning Example (end-to-end)

```python
import torch
from datasets import load_dataset
from transformers import AutoTokenizer, AutoModelForCausalLM, BitsAndBytesConfig
from peft import LoraConfig, get_peft_model, TaskType
from trl import SFTTrainer, SFTConfig

MODEL_ID = "meta-llama/Meta-Llama-3.1-8B"

# 1. Quantization config
bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_quant_type="nf4",
    bnb_4bit_compute_dtype=torch.bfloat16,
    bnb_4bit_use_double_quant=True,
)

# 2. Load model and tokenizer
tokenizer = AutoTokenizer.from_pretrained(MODEL_ID)
tokenizer.padding_side = "right"  # right padding for training
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token

model = AutoModelForCausalLM.from_pretrained(
    MODEL_ID,
    quantization_config=bnb_config,
    device_map="auto",
    attn_implementation="flash_attention_2",
)
model.config.use_cache = False

# 3. LoRA config
lora_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    r=16,
    lora_alpha=16,
    lora_dropout=0.05,
    target_modules="all-linear",
    modules_to_save=["lm_head", "embed_tokens"],
    use_rslora=True,
    inference_mode=False,
)
model = get_peft_model(model, lora_config)
model.print_trainable_parameters()

# 4. Dataset — messages format
dataset = load_dataset("json", data_files="train.jsonl", split="train")
# Expected format: {"messages": [{"role": "user", "content": "..."}, {"role": "assistant", "content": "..."}]}

# 5. Train
trainer = SFTTrainer(
    model=model,
    args=SFTConfig(
        output_dir="./qlora-output",
        num_train_epochs=1,
        per_device_train_batch_size=4,
        gradient_accumulation_steps=4,
        gradient_checkpointing=True,
        gradient_checkpointing_kwargs={"use_reentrant": False},
        learning_rate=2e-4,
        lr_scheduler_type="constant",
        warmup_ratio=0.05,
        bf16=True,
        max_seq_length=2048,
        packing=True,
        push_to_hub=True,
        hub_strategy="every_save",
        logging_steps=25,
    ),
    train_dataset=dataset,
    processing_class=tokenizer,
)
trainer.train()
trainer.push_to_hub()

# 6. Merge and push standalone model
from peft import PeftModel
base = AutoModelForCausalLM.from_pretrained(MODEL_ID, torch_dtype=torch.bfloat16)
peft_model = PeftModel.from_pretrained(base, "./qlora-output")
merged = peft_model.merge_and_unload()
merged.push_to_hub("my-username/my-model-merged")
tokenizer.push_to_hub("my-username/my-model-merged")
```

---

## 13. Key Library Versions (2025 baseline)

```text
torch>=2.4
transformers>=4.46
peft>=0.13
trl>=0.12
accelerate>=1.1
bitsandbytes>=0.44
datasets>=3.1
flash-attn>=2.6      # optional, requires CUDA + Ampere+
liger-kernel>=0.4    # optional, Triton kernels for faster training
```

Always pin versions in production. The HF ecosystem moves fast and breaking changes between minor versions are common.
