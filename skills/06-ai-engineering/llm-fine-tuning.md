---
name: llm-fine-tuning
description: LLM fine-tuning — LoRA, QLoRA, PEFT, SFT with TRL, DPO, dataset preparation, evaluation, and deployment of fine-tuned models.
domain: ai-ml
category: fine-tuning
tags: [fine-tuning, LoRA, QLoRA, PEFT, SFT, DPO, TRL, HuggingFace, dataset, RLHF, LLM]
triggers: fine-tuning LLM, LoRA fine-tune, QLoRA, PEFT adapter, SFT trainer, DPO training, instruction tuning, dataset preparation fine-tune
---

# LLM Fine-Tuning: LoRA, QLoRA, PEFT, SFT, DPO

## When to Fine-Tune

Fine-tuning is not the first tool to reach for. Work through this decision matrix:

| Situation | Recommendation |
| ----------- | --------------- |
| Task solvable with a good system prompt | Prompting — free, no infra |
| Need to inject private/changing knowledge | RAG — more maintainable |
| Need consistent output format/style across thousands of calls | Fine-tuning |
| Domain adaptation (medical, legal, code dialect) with >1k examples | Fine-tuning |
| Latency/cost: distill a larger model into a smaller one | Fine-tuning |
| Reduce hallucinations in a narrow vertical | Fine-tuning + RAG |
| General capability improvement | Don't fine-tune — get a better base |

**Data volume heuristic:** SFT benefits plateau below ~500 examples; 1k–10k high-quality examples is the practical sweet spot. More data rarely hurts if quality is controlled.

### Prompting vs RAG vs fine-tuning decision tree

1. Can you solve it with a 500-token system prompt? → Prompt.
2. Does correctness depend on facts that change or exceed context? → RAG.
3. Is the task narrow, repeatable, and latency-sensitive? → Fine-tune.
4. Do you want the model to *behave* differently (tone, refusals, persona)? → Fine-tune with preference data (DPO).

---

## LoRA — Low-Rank Adaptation

### Theory

LoRA freezes the pretrained weight matrix `W₀ ∈ ℝ^(m×n)` and instead learns a low-rank update:

```text
W_new = W₀ + ΔW,  where ΔW = A × B
A ∈ ℝ^(m×r),  B ∈ ℝ^(r×n),  r ≪ min(m, n)
```

The forward pass becomes `output = x @ W₀ᵀ + x @ Aᵀ @ Bᵀ`. During inference, you can fold `ΔW` back into `W₀` so there is zero added latency.

#### Parameter count example (7B model, attention layer 4096×4096)

- Full weight matrix: 16.7M params
- LoRA r=16: 4096×16 + 16×4096 = 131k params — 99.2% reduction

### Rank / Alpha Tradeoff

- `r` (rank): controls expressiveness. Higher rank = more capacity but more params and risk of overfitting. Common values: 8, 16, 32, 64.
- `lora_alpha`: scaling factor applied as `(alpha / r)` to the LoRA output. Setting `alpha = r` gives scale 1. Setting `alpha = 2×r` doubles the effective learning rate for the adapter.
- Start with `r=16, alpha=16` or `r=8, alpha=16`. Only go higher if validation loss plateaus and you have enough data.

### Target Modules

Apply LoRA to the projection matrices in attention and, optionally, the feed-forward layers. For Llama-family models:

```python
# Attention only (cheapest, usually sufficient):
target_modules = ["q_proj", "v_proj"]

# All attention projections (better for instruction tuning):
target_modules = ["q_proj", "k_proj", "v_proj", "o_proj"]

# All linear layers (best coverage, use "all-linear" shorthand in newer PEFT):
target_modules = "all-linear"
# then also save lm_head and embed_tokens when adding special tokens:
modules_to_save = ["lm_head", "embed_tokens"]
```

### LoRA in Python

```python
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import LoraConfig, get_peft_model, TaskType
import torch

model_id = "meta-llama/Meta-Llama-3.1-8B-Instruct"

tokenizer = AutoTokenizer.from_pretrained(model_id)
model = AutoModelForCausalLM.from_pretrained(
    model_id,
    torch_dtype=torch.bfloat16,
    device_map="auto",
)

lora_config = LoraConfig(
    r=16,
    lora_alpha=16,
    lora_dropout=0.05,
    bias="none",
    task_type=TaskType.CAUSAL_LM,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                    "gate_proj", "up_proj", "down_proj"],
)

model = get_peft_model(model, lora_config)
model.print_trainable_parameters()
# trainable params: 83,886,080 || all params: 8,114,671,616 || trainable%: 1.03
```

### Merging Adapter Weights After Training

```python
from peft import PeftModel

base_model = AutoModelForCausalLM.from_pretrained(
    model_id, torch_dtype=torch.bfloat16, device_map="cpu"
)
model = PeftModel.from_pretrained(base_model, "./lora-adapter")
merged = model.merge_and_unload()          # folds ΔW into W₀; no PEFT dependency
merged.save_pretrained("./merged-model")
tokenizer.save_pretrained("./merged-model")
```

---

## QLoRA — Quantized LoRA

QLoRA stacks 4-bit NF4 quantization of the base model on top of LoRA. The base weights are stored at 4-bit precision but computations are done in bfloat16. LoRA adapters remain in full bfloat16.

### Memory Footprint Math

| Precision | Bytes/param | 7B model | 13B model | 70B model |
| ----------- | ------------- | ---------- | ----------- | ----------- |
| FP32 | 4 | 28 GB | 52 GB | 280 GB |
| BF16/FP16 | 2 | 14 GB | 26 GB | 140 GB |
| 8-bit | 1 | 7 GB | 13 GB | 70 GB |
| 4-bit NF4 | 0.5 | 3.5 GB | 6.5 GB | 35 GB |
| 4-bit NF4 + activations + adapter | ~0.5–0.55 | ~6–8 GB | ~10–12 GB | ~40–45 GB |

A 7B model with QLoRA comfortably trains on a 24GB consumer GPU (RTX 3090/4090). A 70B model needs 2×A100 80GB.

### NF4 vs Other Formats

NF4 (4-bit Normal Float) is the best 4-bit format for normally-distributed weight data. With double quantization (quantizing the quantization constants themselves), it recovers perplexity to within ~0.01 nats of BF16, while Int4 has a 6-point PPL gap.

### QLoRA Config

```python
from transformers import AutoModelForCausalLM, BitsAndBytesConfig
from peft import prepare_model_for_kbit_training, LoraConfig, get_peft_model
import torch

bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_use_double_quant=True,       # saves ~0.5 bits/param extra
    bnb_4bit_quant_type="nf4",            # NF4 > FP4 for LLM weights
    bnb_4bit_compute_dtype=torch.bfloat16 # MUST be bfloat16, not float32
)

model = AutoModelForCausalLM.from_pretrained(
    model_id,
    quantization_config=bnb_config,
    device_map="auto",
)

# Required step before applying LoRA on quantized model
model = prepare_model_for_kbit_training(model)
model.config.use_cache = False  # incompatible with gradient checkpointing

lora_config = LoraConfig(
    r=16,
    lora_alpha=16,
    lora_dropout=0.05,
    bias="none",
    task_type="CAUSAL_LM",
    target_modules="all-linear",
)
model = get_peft_model(model, lora_config)
```

**Common mistake:** setting `bnb_4bit_compute_dtype=torch.float32`. This negates the memory benefit and slows training. Always use `bfloat16` (or `float16` on older GPUs without bfloat16 support).

---

## Dataset Preparation

Data quality is the highest-leverage variable in fine-tuning. A model trained on 500 clean, diverse examples will outperform one trained on 5k noisy examples.

### Instruction / Chat Formats

**Alpaca format** (older, still common for SFT):

```json
{"instruction": "Summarize the following text.", "input": "...", "output": "..."}
```

**ChatML format** (OpenAI, Mistral, many others):

```text
<|im_start|>system
You are a helpful assistant.<|im_end|>
<|im_start|>user
What is LoRA?<|im_end|>
<|im_start|>assistant
LoRA stands for...<|im_end|>
```

#### Llama-3 format

```text
<|begin_of_text|><|start_header_id|>system<|end_header_id|>
You are a helpful assistant.<|eot_id|>
<|start_header_id|>user<|end_header_id|>
What is LoRA?<|eot_id|>
<|start_header_id|>assistant<|end_header_id|>
LoRA stands for...<|eot_id|>
```

**Always use the template the base model was trained with.** Mixing templates is a top cause of degraded performance and strange refusal behavior.

### Applying the Chat Template Programmatically

```python
from transformers import AutoTokenizer

tokenizer = AutoTokenizer.from_pretrained(model_id)

messages = [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Explain LoRA in one sentence."},
    {"role": "assistant", "content": "LoRA fine-tunes large models by training only small low-rank matrices added to frozen weights."},
]

# apply_chat_template handles the correct special tokens for this model
text = tokenizer.apply_chat_template(
    messages,
    tokenize=False,
    add_generation_prompt=False,  # False when training; True at inference
)
```

### Quality Filtering Pipeline

```python
from datasets import load_dataset, Dataset

def quality_filter(example):
    text = example.get("output", "")
    # Drop very short responses
    if len(text.split()) < 10:
        return False
    # Drop responses with known contamination patterns
    if any(pat in text.lower() for pat in ["as an ai", "i cannot", "i'm sorry, but"]):
        return False
    return True

def dedup_by_input(dataset):
    seen = set()
    deduped = []
    for ex in dataset:
        key = ex["instruction"][:100]
        if key not in seen:
            seen.add(key)
            deduped.append(ex)
    return Dataset.from_list(deduped)

raw = load_dataset("your-dataset", split="train")
filtered = raw.filter(quality_filter)
filtered = dedup_by_input(filtered)
# Shuffle and split 80/20
filtered = filtered.shuffle(seed=42)
split = filtered.train_test_split(test_size=0.2)
split["train"].to_json("train.jsonl", orient="records", lines=True)
split["test"].to_json("eval.jsonl", orient="records", lines=True)
```

---

## SFT with TRL

TRL's `SFTTrainer` handles chat template application, sequence packing, PEFT integration, and distributed training.

### Minimal SFTTrainer Setup

```python
from trl import SFTTrainer, SFTConfig
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import LoraConfig
from datasets import load_dataset
import torch

model_id = "meta-llama/Meta-Llama-3.1-8B-Instruct"
tokenizer = AutoTokenizer.from_pretrained(model_id)
tokenizer.pad_token = tokenizer.eos_token
tokenizer.padding_side = "right"  # important for causal LM

model = AutoModelForCausalLM.from_pretrained(
    model_id,
    torch_dtype=torch.bfloat16,
    device_map="auto",
    attn_implementation="flash_attention_2",  # requires flash-attn installed
)

lora_config = LoraConfig(
    r=16, lora_alpha=16, lora_dropout=0.05,
    bias="none", task_type="CAUSAL_LM",
    target_modules="all-linear",
)

dataset = load_dataset("json", data_files={"train": "train.jsonl", "test": "eval.jsonl"})

training_args = SFTConfig(
    output_dir="./sft-output",
    num_train_epochs=3,
    per_device_train_batch_size=4,
    per_device_eval_batch_size=4,
    gradient_accumulation_steps=4,         # effective batch = 4*4 = 16
    gradient_checkpointing=True,
    gradient_checkpointing_kwargs={"use_reentrant": False},
    learning_rate=2e-4,
    lr_scheduler_type="cosine",
    warmup_ratio=0.05,
    bf16=True,
    logging_steps=25,
    eval_strategy="steps",
    eval_steps=100,
    save_steps=200,
    save_total_limit=3,
    max_seq_length=2048,
    packing=True,                          # pack multiple short sequences together
    dataset_text_field="text",             # field containing the formatted prompt
    push_to_hub=False,
)

trainer = SFTTrainer(
    model=model,
    args=training_args,
    train_dataset=dataset["train"],
    eval_dataset=dataset["test"],
    peft_config=lora_config,
    tokenizer=tokenizer,
)

trainer.train()
trainer.save_model("./sft-adapter")
```

### Multi-GPU with Accelerate / DeepSpeed

```bash
# accelerate_config.yaml (ZeRO-2 for single-node multi-GPU):
accelerate launch \
  --config_file accelerate_configs/deepspeed_zero2.yaml \
  --num_processes 4 \
  train_sft.py

# For 70B+ models, use ZeRO-3 with CPU offload:
accelerate launch \
  --config_file accelerate_configs/deepspeed_zero3_cpu_offload.yaml \
  --num_processes 8 \
  train_sft.py
```

### Key SFT Hyperparameters

| Param | Conservative | Aggressive | Notes |
| ------- | ------------- | ------------ | ------- |
| `learning_rate` | 1e-4 | 3e-4 | Cosine decay from this peak |
| `num_epochs` | 1–2 | 3–5 | More epochs → more overfit risk |
| `max_seq_length` | 1024 | 4096 | Quadratic memory cost; use packing |
| `warmup_ratio` | 0.03 | 0.10 | Scale with dataset size |
| `lora_r` | 8 | 64 | Start low; increase if loss plateaus |

---

## DPO / RLHF

### Preference Datasets

DPO requires `(prompt, chosen, rejected)` triples. The `chosen` response is the preferred completion; `rejected` is the worse one.

```json
{
  "prompt": "Explain gradient descent.",
  "chosen": "Gradient descent minimizes a loss by iteratively moving weights in the direction of the negative gradient...",
  "rejected": "It's an optimization algorithm that makes the model better."
}
```

Public datasets: `HuggingFaceH4/ultrafeedback_binarized`, `Anthropic/hh-rlhf`, `Intel/orca_dpo_pairs`.

### DPOTrainer

DPO uses a contrastive loss that pushes the policy toward `chosen` and away from `rejected` relative to a frozen reference model, without needing an explicit reward model:

```text
L_DPO = -E[log σ(β · (log π(chosen|x)/π_ref(chosen|x) − log π(rejected|x)/π_ref(rejected|x)))]
```

`β` controls how tightly the policy stays close to the reference. Smaller β = more conservative; larger β = more aggressive preference learning.

```python
from trl import DPOTrainer, DPOConfig
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import LoraConfig, PeftModel
from datasets import load_dataset
import torch

model_id = "./sft-adapter"  # start from your SFT checkpoint

tokenizer = AutoTokenizer.from_pretrained(model_id)
model = AutoModelForCausalLM.from_pretrained(
    model_id, torch_dtype=torch.bfloat16, device_map="auto"
)

# Reference model: frozen copy of the SFT model
ref_model = AutoModelForCausalLM.from_pretrained(
    model_id, torch_dtype=torch.bfloat16, device_map="auto"
)
# Alternatively, pass ref_model=None and use implicit reference via PEFT

lora_config = LoraConfig(
    r=16, lora_alpha=16, lora_dropout=0.05,
    bias="none", task_type="CAUSAL_LM",
    target_modules="all-linear",
)

dataset = load_dataset("HuggingFaceH4/ultrafeedback_binarized", split="train_prefs")

dpo_args = DPOConfig(
    output_dir="./dpo-output",
    num_train_epochs=1,
    per_device_train_batch_size=2,
    gradient_accumulation_steps=8,
    learning_rate=5e-5,               # lower than SFT; DPO is sensitive
    beta=0.1,                          # KL regularization weight
    bf16=True,
    gradient_checkpointing=True,
    gradient_checkpointing_kwargs={"use_reentrant": False},
    logging_steps=10,
    save_steps=100,
    max_length=1024,
    max_prompt_length=512,
)

trainer = DPOTrainer(
    model=model,
    ref_model=ref_model,
    args=dpo_args,
    train_dataset=dataset,
    peft_config=lora_config,
    tokenizer=tokenizer,
)

trainer.train()
trainer.save_model("./dpo-adapter")
```

### RLHF / PPO Basics

Full PPO is rarely necessary today. Use it when you need online exploration (model generates, reward scores, policy updates). The TRL stack:

1. **SFT** the base model on demonstrations.
2. Train a **reward model** on preference pairs (same architecture, replace LM head with scalar output).
3. Run **PPO** with the SFT model as policy, the reward model scoring completions, and the SFT model as KL reference.

```python
from trl import PPOTrainer, PPOConfig, AutoModelForCausalLMWithValueHead

ppo_config = PPOConfig(
    model_name=model_id,
    learning_rate=1.41e-5,
    batch_size=16,
    mini_batch_size=4,
    ppo_epochs=4,
    kl_penalty="kl",      # penalize divergence from SFT model
    init_kl_coef=0.2,
    target_kl=6.0,        # adaptive KL controller target
)
```

In practice, DPO or ORPO achieve similar alignment results without the PPO instability, and are preferred for most production workflows.

---

## Evaluation

### Perplexity (sanity check)

```python
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

def compute_perplexity(model, tokenizer, texts, device="cuda"):
    model.eval()
    total_loss, total_tokens = 0.0, 0
    with torch.no_grad():
        for text in texts:
            ids = tokenizer(text, return_tensors="pt").input_ids.to(device)
            out = model(ids, labels=ids)
            total_loss += out.loss.item() * ids.size(1)
            total_tokens += ids.size(1)
    return torch.exp(torch.tensor(total_loss / total_tokens)).item()
```

Lower perplexity on your held-out eval set = better language modeling. A spike in eval perplexity early in training signals too high a learning rate.

### Task-Specific Benchmarks

Use `lm-evaluation-harness` (EleutherAI) for standardized evals:

```bash
pip install lm-eval

# Evaluate on GSM8K (math reasoning):
lm_eval --model hf \
  --model_args pretrained=./merged-model \
  --tasks gsm8k_cot \
  --num_fewshot 8 \
  --output_path ./eval-results/gsm8k.json

# MMLU (knowledge breadth):
lm_eval --model hf \
  --model_args pretrained=./merged-model \
  --tasks mmlu \
  --num_fewshot 5

# IFEval (instruction following):
lm_eval --model hf \
  --model_args pretrained=./merged-model \
  --tasks ifeval
```

### LLM-as-Judge

For open-ended generation where ground truth doesn't exist:

```python
import openai

def llm_judge(instruction, response_a, response_b):
    prompt = f"""You are evaluating two model responses.

Instruction: {instruction}
Response A: {response_a}
Response B: {response_b}

Which response is better? Answer with just "A", "B", or "Tie" and a one-sentence reason."""
    result = openai.chat.completions.create(
        model="gpt-4o",
        messages=[{"role": "user", "content": prompt}],
        temperature=0,
    )
    return result.choices[0].message.content

# Run pairwise: fine-tuned model vs baseline
wins, losses, ties = 0, 0, 0
for item in eval_set:
    verdict = llm_judge(item["instruction"], item["ft_response"], item["base_response"])
    if "A" in verdict: wins += 1
    elif "B" in verdict: losses += 1
    else: ties += 1

win_rate = wins / (wins + losses + ties)
print(f"Win rate: {win_rate:.2%}")
```

---

## Deployment

### Merging LoRA Weights

Always merge before serving for zero-latency overhead:

```python
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel
import torch

base = AutoModelForCausalLM.from_pretrained(
    "meta-llama/Meta-Llama-3.1-8B-Instruct",
    torch_dtype=torch.bfloat16,
    device_map="cpu",   # merge on CPU to avoid VRAM OOM
)
model = PeftModel.from_pretrained(base, "./lora-adapter")
merged = model.merge_and_unload()
merged.save_pretrained("./merged-model", safe_serialization=True)
AutoTokenizer.from_pretrained("./lora-adapter").save_pretrained("./merged-model")
```

### GGUF Export for llama.cpp

```bash
# Clone llama.cpp and install deps
git clone https://github.com/ggerganov/llama.cpp && cd llama.cpp
pip install -r requirements.txt

# Convert merged HF model to GGUF float16
python convert_hf_to_gguf.py ../merged-model \
  --outfile ../llama3-8b-ft.gguf \
  --outtype f16

# Quantize to Q4_K_M for best quality/size tradeoff
./llama-quantize ../llama3-8b-ft.gguf ../llama3-8b-ft-q4km.gguf Q4_K_M

# Run inference
./llama-cli -m ../llama3-8b-ft-q4km.gguf -p "Explain LoRA:" -n 200
```

Common GGUF quantization levels (Q4_K_M is the standard production choice):

- `Q2_K`: smallest, noticeable quality loss
- `Q4_K_M`: good balance (~4.5 GB for 7B)
- `Q5_K_M`: near-lossless (~5.5 GB for 7B)
- `Q8_0`: nearly identical to FP16 (~8 GB for 7B)

### vLLM Serving

```bash
pip install vllm

# Serve the merged model
python -m vllm.entrypoints.openai.api_server \
  --model ./merged-model \
  --dtype bfloat16 \
  --tensor-parallel-size 1 \
  --max-model-len 4096 \
  --port 8000

# Query it
curl http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "merged-model", "messages": [{"role": "user", "content": "Hello"}]}'
```

For LoRA adapters without merging (dynamic adapter loading):

```bash
python -m vllm.entrypoints.openai.api_server \
  --model meta-llama/Meta-Llama-3.1-8B-Instruct \
  --enable-lora \
  --lora-modules my-adapter=./lora-adapter \
  --max-lora-rank 64
```

---

## Common Failures

### Catastrophic Forgetting

The model loses general capabilities while learning the target task.

**Signs:** General benchmark scores (MMLU, ARC) drop significantly; the model refuses to answer out-of-domain questions.

#### Fixes

- Mix ~20–30% general-purpose data (e.g., a slice of OpenHermes, Dolly) into your training set.
- Lower the learning rate — 1e-4 is safer than 3e-4.
- Reduce number of epochs.
- Use smaller LoRA rank, which constrains the degree of adaptation.

### Overfitting

**Signs:** Training loss continues to fall but eval loss starts to rise or plateaus. Generated text repeats training phrases verbatim.

#### Fixes (Overfitting)

- Increase `lora_dropout` to 0.1.
- Reduce epochs; use early stopping on eval loss.
- Increase dataset size or add augmentation.
- Regularize with weight decay on the adapter parameters.

### Learning Rate Too High

**Signs:** Loss spikes on first epoch, then recovers erratically. Validation perplexity is worse than the base model.

**Fix:** Start at 1e-4 with cosine decay and 5% warmup. If still unstable, halve the LR. Never start DPO above 5e-5.

---

## Critical Rules / Gotchas

**EOS token in training data:** Every training example must end with the model's EOS token (`<|eot_id|>` for Llama-3, `</s>` for Mistral). Without it, the model learns to generate indefinitely and never stops at inference. In TRL's `SFTTrainer`, this is handled automatically if you use `apply_chat_template` — do not strip it.

**Chat template consistency:** The template applied at training time must be identical to the one used at inference. Switching from ChatML to Llama-3 format between training and serving is a guaranteed way to get garbled outputs. Pin `tokenizer_config.json` alongside your adapter weights.

**`padding_side = "right"`** for causal LM training. Left-padding causes the model to attend to pad tokens at the wrong positions. Set `tokenizer.padding_side = "right"` before any training.

**`prepare_model_for_kbit_training`** must be called after loading a quantized model with `BitsAndBytesConfig` and before applying LoRA. Skipping it silently breaks gradient computation.

**`use_cache = False`** must be set on the model when gradient checkpointing is enabled. These two features are mutually exclusive. Re-enable at inference time.

**`use_reentrant=False`** in `gradient_checkpointing_kwargs` — the default reentrant mode has known stability issues with PEFT models. Always pass this explicitly.

### GPU memory calculation before training

```text
VRAM ≈ model_params × bytes_per_param
      + activations (≈ batch × seq_len × hidden × layers × 2 bytes)
      + optimizer states (Adam: 8 bytes/trainable_param)
      + gradients (2 bytes/trainable_param)
```

For QLoRA on 7B with r=16, all-linear targets (~83M trainable params), batch=4, seq=2048:

- Base model: 3.5 GB (4-bit)
- Activations: ~3 GB
- Optimizer + gradients for adapter: ~1 GB
- **Total: ~8 GB** — comfortably fits on a 24GB GPU

**Do not train on padding tokens.** Ensure your data collator or SFTTrainer is masking pad token labels to -100 so they don't contribute to the loss.

---

## Key APIs / Libraries

| Library | Version (stable as of 2025) | Purpose |
| --------- | ---------------------------- | --------- |
| `transformers` | ≥4.46 | Model loading, tokenizers, generation |
| `peft` | ≥0.13 | LoRA, QLoRA, IA3, prompt tuning |
| `trl` | ≥0.12 | SFTTrainer, DPOTrainer, PPOTrainer |
| `bitsandbytes` | ≥0.44 | 4-bit and 8-bit quantization |
| `accelerate` | ≥1.1 | Multi-GPU / DeepSpeed / FSDP launcher |
| `datasets` | ≥3.0 | Dataset loading, streaming, processing |
| `flash-attn` | ≥2.6 | 2× throughput, reduced memory |
| `liger-kernel` | ≥0.4 | Fused kernels (RMSNorm, RoPE, SwiGLU) |
| `lm-eval` | ≥0.4 | Standardized benchmark evaluation |
| `llama.cpp` | latest | GGUF conversion and CPU inference |
| `vllm` | ≥0.6 | High-throughput GPU serving |

---

## References

- Hu et al., "LoRA: Low-Rank Adaptation of Large Language Models" — <https://arxiv.org/abs/2106.09685>
- Dettmers et al., "QLoRA: Efficient Finetuning of Quantized LLMs" — <https://arxiv.org/abs/2305.14314>
- Rafailov et al., "Direct Preference Optimization" — <https://arxiv.org/abs/2305.18290>
- HuggingFace PEFT docs — <https://huggingface.co/docs/peft>
- TRL docs (SFTTrainer, DPOTrainer) — <https://huggingface.co/docs/trl>
- philschmid.de, "How to fine-tune open LLMs in 2025" — <https://www.philschmid.de/fine-tune-llms-in-2025>
- EleutherAI lm-evaluation-harness — <https://github.com/EleutherAI/lm-evaluation-harness>
- llama.cpp — <https://github.com/ggerganov/llama.cpp>
- vLLM — <https://docs.vllm.ai>
