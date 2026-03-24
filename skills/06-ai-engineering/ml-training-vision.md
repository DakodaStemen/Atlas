---
name: ml-training-vision
description: Machine learning training and computer vision covering TensorFlow/Keras (model APIs, tf.data, custom training, callbacks, TF-Serving, TFLite), PyTorch (DataLoader, training loops, AMP, DDP, Lightning), and computer vision (preprocessing, augmentation, transfer learning, YOLO, segmentation, ViT/CLIP). Use when training ML models or building vision systems.
domain: ai-engineering
tags: [tensorflow, keras, pytorch, computer-vision, training, deep-learning, yolo, vit, clip, transfer-learning]
triggers: tensorflow, keras, pytorch, training loop, computer vision, image classification, object detection, YOLO, transfer learning, DDP, mixed precision
---


# TensorFlow 2.x / Keras — Best Practices Reference

## 1. Three Model-Building APIs: When to Use Which

Keras exposes three distinct APIs. Choosing the right one early prevents painful rewrites.

### Sequential API

Use only for strictly linear stacks with a single input and single output. Fast to write; zero flexibility beyond that.

```python
import keras
from keras import layers

model = keras.Sequential([
    layers.Input(shape=(784,)),
    layers.Dense(256, activation="relu"),
    layers.Dropout(0.3),
    layers.Dense(128, activation="relu"),
    layers.Dense(10),               # no activation — use from_logits=True in loss
])
model.summary()
```

**Limits:** Cannot express skip connections, shared layers, multiple inputs/outputs, or dynamic branching.


### Model Subclassing API

Use when you need dynamic control flow (conditionals on tensor values, variable-length recursion, tree-structured computation, custom gradient behavior). Hardest to serialize correctly.

```python
class ResBlock(keras.layers.Layer):
    def __init__(self, filters, **kwargs):
        super().__init__(**kwargs)
        self.conv1 = layers.Conv2D(filters, 3, padding="same", activation="relu")
        self.conv2 = layers.Conv2D(filters, 3, padding="same")
        self.proj  = layers.Conv2D(filters, 1, padding="same")  # projection shortcut
        self.add   = layers.Add()
        self.relu  = layers.Activation("relu")

    def call(self, x, training=False):
        residual = self.proj(x)
        x = self.conv1(x)
        x = self.conv2(x)
        return self.relu(self.add([x, residual]))

    def get_config(self):
        base = super().get_config()
        return {**base, "filters": self.conv1.filters}


class MyModel(keras.Model):
    def __init__(self, num_classes, **kwargs):
        super().__init__(**kwargs)
        self.block1 = ResBlock(32)
        self.block2 = ResBlock(64)
        self.gap    = layers.GlobalAveragePooling2D()
        self.head   = layers.Dense(num_classes)

    def call(self, x, training=False):
        x = self.block1(x, training=training)
        x = self.block2(x, training=training)
        x = self.gap(x)
        return self.head(x)               # logits, no softmax

    def get_config(self):
        base = super().get_config()
        return {**base, "num_classes": self.head.units}
```

#### Critical subclassing rules

- Always pass `training` to sub-layers that have different train/inference behavior (Dropout, BatchNormalization).
- Implement `get_config` + `from_config` or the model cannot be saved as `.keras` / loaded without source code.
- Do not create new `tf.Variable` objects inside `call` — place them in `__init__` or `build`.


## 3. Training with model.fit + Callbacks

For the majority of use cases `model.fit` is the right choice. Add callbacks rather than writing a custom loop.

```python
callbacks = [
    keras.callbacks.ModelCheckpoint(
        filepath="checkpoints/epoch_{epoch:02d}_val{val_loss:.4f}.keras",
        monitor="val_loss",
        save_best_only=True,
        save_weights_only=False,   # saves full model, not just weights
    ),
    keras.callbacks.EarlyStopping(
        monitor="val_loss",
        patience=7,
        restore_best_weights=True,  # rolls back to best checkpoint on stop
        min_delta=1e-4,
    ),
    keras.callbacks.ReduceLROnPlateau(
        monitor="val_loss",
        factor=0.5,
        patience=3,
        min_lr=1e-7,
    ),
    keras.callbacks.TensorBoard(
        log_dir="logs/run_001",
        histogram_freq=1,          # weight histograms every epoch
        profile_batch="5,10",      # profile steps 5-10 for bottleneck analysis
    ),
    keras.callbacks.CSVLogger("training_log.csv"),
    keras.callbacks.TerminateOnNaN(),
]

history = model.fit(
    train_dataset,
    epochs=100,
    validation_data=val_dataset,
    callbacks=callbacks,
    verbose=1,
)
```

### Custom callback skeleton

```python
class LRWarmupCallback(keras.callbacks.Callback):
    def __init__(self, warmup_steps, target_lr):
        super().__init__()
        self.warmup_steps = warmup_steps
        self.target_lr    = target_lr
        self._step        = 0

    def on_train_batch_begin(self, batch, logs=None):
        if self._step < self.warmup_steps:
            lr = self.target_lr * (self._step + 1) / self.warmup_steps
            self.model.optimizer.learning_rate.assign(lr)
        self._step += 1
```


## 5. Mixed Precision Training

Mixed precision runs forward/backward passes in float16 while accumulating gradients in float32. Typical speedups: 2-3x on Volta/Ampere GPUs, 1.5-2x on TPUs.

```python
# Set policy before building the model
keras.mixed_precision.set_global_policy("mixed_float16")

# Build model normally — internal computations use float16
# but the final output layer must stay float32 for numerical stability
outputs = layers.Dense(num_classes, dtype="float32", name="logits")(x)
```

**With model.fit:** Loss scaling is automatic — nothing else needed.

**With a custom training loop:** Must wrap optimizer manually.

```python
optimizer = keras.mixed_precision.LossScaleOptimizer(
    keras.optimizers.Adam(1e-3)
)

@tf.function
def train_step(x, y):
    with tf.GradientTape() as tape:
        logits     = model(x, training=True)
        loss_value = loss_fn(y, logits)
        # Scale the loss to prevent float16 underflow
        scaled_loss = optimizer.get_scaled_loss(loss_value)
    scaled_grads = tape.gradient(scaled_loss, model.trainable_weights)
    grads        = optimizer.get_unscaled_gradients(scaled_grads)
    optimizer.apply(grads, model.trainable_weights)
    return loss_value
```

### Rules (5. Mixed Precision Training)

- Keep the output layer `dtype="float32"` regardless of global policy.
- Tensor dimensions should be multiples of 8 (ideally 64) to unlock Tensor Core throughput.
- Do not use mixed precision with models that are numerically unstable in float32 to begin with.


## 7. TensorFlow Serving (Production Inference)

### Start server with Docker (recommended)

```bash
docker run -p 8500:8500 -p 8501:8501 \
  -v "$(pwd)/model_base:/models/my_model" \
  -e MODEL_NAME=my_model \
  tensorflow/serving:latest
```

- Port 8500: gRPC
- Port 8501: REST

### REST prediction request

```python
import json, requests

payload = json.dumps({
    "signature_name": "serving_default",
    "instances": x_batch.tolist(),          # list of lists, not numpy
})
resp   = requests.post("http://localhost:8501/v1/models/my_model:predict", data=payload)
preds  = resp.json()["predictions"]
```

### Target a specific version

```python
url = "http://localhost:8501/v1/models/my_model/versions/2:predict"
```

### gRPC (lower latency, better for high-throughput)

```python
import grpc
from tensorflow_serving.apis import predict_pb2, prediction_service_pb2_grpc

channel = grpc.insecure_channel("localhost:8500")
stub    = prediction_service_pb2_grpc.PredictionServiceStub(channel)

request = predict_pb2.PredictRequest()
request.model_spec.name           = "my_model"
request.model_spec.signature_name = "serving_default"
request.inputs["inputs"].CopyFrom(tf.make_tensor_proto(x_batch))

response = stub.Predict(request, timeout=10.0)
```

#### Inspect signature before serving

```bash
saved_model_cli show --dir saved_model_dir/1 \
  --tag_set serve --signature_def serving_default
```


## 9. Common Pitfalls

### training=False forgotten at inference

Dropout stays active and BatchNorm uses batch statistics instead of running statistics. Every `model.predict`, `model.evaluate`, and manual inference call must not pass `training=True`.

### Softmax in output + from_logits=False (double softmax)

If the model's last layer has `activation="softmax"`, use `from_logits=False` in the loss. If there is no activation (logits), use `from_logits=True`. Mixing these produces silently wrong gradients. Prefer logits + `from_logits=True` for numerical stability.

```python
# Correct
outputs = layers.Dense(10)(x)                          # no activation
loss    = keras.losses.SparseCategoricalCrossentropy(from_logits=True)
```

### model.losses not added in custom training loops

Layers with `kernel_regularizer` or `activity_regularizer` push penalty terms onto `model.losses`. Forgetting `+= sum(model.losses)` inside the tape silently ignores regularization.

### HDF5 for new projects

`.h5` doesn't support TensorFlow ops added outside standard Keras (e.g. custom `@tf.function` preprocessing inside the model). Use `.keras` for in-Keras workflows, `SavedModel` for deployment.

### Shuffle buffer too small

`dataset.shuffle(1000)` on a 100k-sample dataset is almost no shuffle. Set `buffer_size` to at least the dataset size, or use a pre-shuffle step on the file list level.

### Mixed precision output in float16

The final Dense/output layer must be `dtype="float32"`. float16 has limited range (~65504 max); softmax/cross-entropy computed in float16 overflows or loses resolution. Keras does not enforce this automatically.

### @tf.function tracing loops

Calling a `@tf.function`-decorated function with Python scalars that change value each call triggers a re-trace every call. Pass tensors, not Python ints, as arguments.

```python
# Bad — retraces on every new value of n
@tf.function
def f(x, n):
    for _ in range(n): x = x + 1
    return x

# Good — pass tensor or use tf.while_loop for dynamic counts
@tf.function
def f(x, n):
    return x + tf.cast(n, tf.float32)
```

### Not calling reset_state() on metrics

`keras.metrics.Mean` etc. accumulate across calls. In a manual loop, failing to call `.reset_state()` at the end of each epoch causes metrics to be the running average across all epochs, not the current epoch.


---


# PyTorch Training — Best Practices

## 1. Dataset and DataLoader

Define a `Dataset` subclass, then wrap it in `DataLoader`. The loader is where most performance tuning happens.

```python
from torch.utils.data import Dataset, DataLoader

class MyDataset(Dataset):
    def __init__(self, data, labels):
        self.data = data
        self.labels = labels

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx):
        return self.data[idx], self.labels[idx]

loader = DataLoader(
    dataset,
    batch_size=64,
    shuffle=True,
    num_workers=4,        # rule of thumb: 4 * num_GPUs; 0 = synchronous (debug only)
    pin_memory=True,      # enables faster async host-to-device transfers on CUDA
    drop_last=True,       # avoids uneven last batch disrupting BatchNorm stats
    persistent_workers=True,  # keeps workers alive between epochs (PyTorch >= 1.7)
)
```

### Key settings

- `num_workers=0` is fine for debugging; use `num_workers >= 2` in production so data loading overlaps with GPU compute.
- `pin_memory=True` only helps when training on GPU. Has no effect on CPU-only runs.
- Create tensors directly on device when possible: `torch.rand(size, device="cuda")` instead of `.cuda()` afterward.
- Don't call `.item()` inside the training loop unless necessary — it forces a CPU-GPU sync that stalls the pipeline.

## 2. Training Loop Structure

A correct, minimal raw PyTorch training loop:

```python
model.train()
for epoch in range(num_epochs):
    for batch_idx, (inputs, targets) in enumerate(train_loader):
        inputs, targets = inputs.to(device, non_blocking=True), targets.to(device, non_blocking=True)

        optimizer.zero_grad(set_to_none=True)   # faster than zero_grad(); sets grads to None instead of 0

        outputs = model(inputs)
        loss = criterion(outputs, targets)

        loss.backward()

        # gradient clipping before optimizer step (optional but recommended)
        torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)

        optimizer.step()
        scheduler.step()   # for per-step schedulers like OneCycleLR; otherwise step per epoch

    # validation
    model.eval()
    with torch.no_grad():
        for val_inputs, val_targets in val_loader:
            val_inputs, val_targets = val_inputs.to(device), val_targets.to(device)
            val_outputs = model(val_inputs)
            # compute metrics
    model.train()
```

### Order matters

1. `zero_grad` first (before forward pass)
2. Forward + loss
3. `loss.backward()`
4. Gradient clipping (if used)
5. `optimizer.step()`
6. `scheduler.step()` (position depends on scheduler type — see section 4)

Use `non_blocking=True` with `pin_memory=True` for async host-to-device transfers.

## 3. Optimizer Setup

```python
import torch.optim as optim

optimizer = optim.AdamW(
    model.parameters(),
    lr=3e-4,
    weight_decay=0.01,   # built-in L2 regularization; use AdamW, not Adam + manual decay
    betas=(0.9, 0.999),
    eps=1e-8,
)
```

### Practical notes

- Prefer `AdamW` over `Adam` for almost all cases. Adam with manual weight decay is mathematically incorrect; AdamW fixes this.
- Use separate param groups if layers need different learning rates (common in fine-tuning):

  ```python
  optimizer = optim.AdamW([
      {"params": model.backbone.parameters(), "lr": 1e-5},
      {"params": model.head.parameters(), "lr": 1e-3},
  ])
  ```

- `SGD` with momentum is still competitive for vision models, often beating Adam in final accuracy with proper LR tuning.

## 4. Learning Rate Scheduler

```python
# Option A: cosine annealing (most common for fine-tuning)
scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=num_epochs)
# Call scheduler.step() once per epoch, after validation

# Option B: OneCycleLR (trains faster, call per batch)
scheduler = optim.lr_scheduler.OneCycleLR(
    optimizer,
    max_lr=3e-3,
    steps_per_epoch=len(train_loader),
    epochs=num_epochs,
    pct_start=0.3,
)
# Call scheduler.step() after every optimizer.step()

# Option C: warmup then decay (transformers)
from torch.optim.lr_scheduler import LinearLR, CosineAnnealingLR, SequentialLR
warmup = LinearLR(optimizer, start_factor=0.1, end_factor=1.0, total_iters=warmup_steps)
decay = CosineAnnealingLR(optimizer, T_max=total_steps - warmup_steps)
scheduler = SequentialLR(optimizer, schedulers=[warmup, decay], milestones=[warmup_steps])
```

When to call `scheduler.step()`:

- Per-epoch schedulers (`CosineAnnealingLR`, `StepLR`, `ReduceLROnPlateau`): after each epoch's validation.
- Per-step schedulers (`OneCycleLR`, `CyclicLR`): after each `optimizer.step()`.

## 5. Mixed Precision Training (AMP)

Mixed precision is the single highest-leverage optimization for GPU training. It typically gives 1.5–3× speedup and halves memory usage, with negligible accuracy impact.

```python
from torch.amp import autocast, GradScaler

scaler = GradScaler("cuda")   # manages loss scaling to prevent FP16 underflow

for inputs, targets in train_loader:
    inputs, targets = inputs.to(device), targets.to(device)
    optimizer.zero_grad(set_to_none=True)

    # autocast wraps the forward pass only
    with autocast(device_type="cuda", dtype=torch.float16):
        outputs = model(inputs)
        loss = criterion(outputs, targets)

    # scale loss → backward in FP16 → unscale → step in FP32
    scaler.scale(loss).backward()

    # optional: unscale before gradient clipping so norms are in real units
    scaler.unscale_(optimizer)
    torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)

    scaler.step(optimizer)
    scaler.update()
    scheduler.step()
```

### FP16 vs BF16

- `torch.float16`: works on Volta, Turing, Ampere. Requires `GradScaler` because its dynamic range is narrow (max ~65,504). Vulnerable to NaN from gradient underflow.
- `torch.bfloat16`: same dynamic range as FP32, less precision. Available on Ampere (A100) and newer, and all TPUs. Does **not** require `GradScaler`. Much more stable during large-model training.
- Rule: use BF16 on A100/H100; use FP16 on older GPUs (V100, T4, consumer RTX).

#### When autocast doesn't help

- Increase batch size or model size until GPU is saturated. Autocast on a tiny model shows no speedup.
- Set layer dimensions to multiples of 8 to activate Tensor Cores.
- Enable `torch.backends.cudnn.benchmark = True` once per run (picks fastest conv algorithm for your input size).

## 6. Gradient Clipping

Gradient clipping prevents exploding gradients and is essential for RNNs and Transformers.

```python
# Clip by global norm (most common)
torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)

# Clip by value (less common, use only if you have a specific reason)
torch.nn.utils.clip_grad_value_(model.parameters(), clip_value=0.5)
```

Always clip **after** `scaler.unscale_(optimizer)` when using AMP, so the gradient norms are in real (unscaled) units before being checked against `max_norm`.

Typical values: `max_norm=1.0` for Transformers, `max_norm=5.0` for RNNs. Monitor gradient norms during early training to calibrate.

## 7. Checkpointing

Save both model and optimizer state. Without optimizer state, resuming training diverges.

```python
# Saving
checkpoint = {
    "epoch": epoch,
    "model_state_dict": model.state_dict(),
    "optimizer_state_dict": optimizer.state_dict(),
    "scheduler_state_dict": scheduler.state_dict(),
    "scaler_state_dict": scaler.state_dict(),   # required if using AMP
    "best_val_loss": best_val_loss,
}
torch.save(checkpoint, "checkpoint.pt")

# Loading
checkpoint = torch.load("checkpoint.pt", map_location=device)
model.load_state_dict(checkpoint["model_state_dict"])
optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
scheduler.load_state_dict(checkpoint["scheduler_state_dict"])
scaler.load_state_dict(checkpoint["scaler_state_dict"])
start_epoch = checkpoint["epoch"] + 1
```

**Activation checkpointing** (gradient checkpointing) trades compute for memory — useful when batch size is memory-limited:

```python
from torch.utils.checkpoint import checkpoint

# Inside a model forward, wrap expensive layers:
x = checkpoint(self.expensive_block, x)
# Recomputes the block during backward instead of storing activations
```

## 8. PyTorch Lightning vs Raw Training

### Use raw PyTorch when

- You need full control over every training detail.
- The research is highly experimental (custom backward passes, weird loop structures).
- The codebase is small enough that boilerplate isn't painful.

#### Use PyTorch Lightning when

- You want automatic handling of: multi-GPU, mixed precision, gradient clipping, logging, checkpointing, early stopping.
- The team is larger and consistency matters.
- You want to switch hardware (CPU/GPU/TPU) with zero code changes.

```python
import lightning as L

class LitModel(L.LightningModule):
    def __init__(self):
        super().__init__()
        self.model = MyModel()
        self.criterion = nn.CrossEntropyLoss()

    def training_step(self, batch, batch_idx):
        inputs, targets = batch
        outputs = self.model(inputs)
        loss = self.criterion(outputs, targets)
        self.log("train_loss", loss, prog_bar=True)
        return loss

    def validation_step(self, batch, batch_idx):
        inputs, targets = batch
        outputs = self.model(inputs)
        loss = self.criterion(outputs, targets)
        self.log("val_loss", loss, prog_bar=True)

    def configure_optimizers(self):
        optimizer = torch.optim.AdamW(self.parameters(), lr=3e-4)
        scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=10)
        return {"optimizer": optimizer, "lr_scheduler": scheduler}

trainer = L.Trainer(
    max_epochs=50,
    precision="bf16-mixed",        # or "16-mixed" for FP16
    gradient_clip_val=1.0,
    accumulate_grad_batches=4,     # gradient accumulation built-in
    devices="auto",
    accelerator="auto",
)
trainer.fit(lit_model, train_loader, val_loader)
```

Lightning also provides **Fabric** for a lighter-weight abstraction that gives you manual loop control with automatic device/precision handling:

```python
import lightning as L

fabric = L.Fabric(accelerator="cuda", devices=4, precision="bf16-mixed")
fabric.launch()

model, optimizer = fabric.setup(model, optimizer)
train_loader = fabric.setup_dataloaders(train_loader)

for inputs, targets in train_loader:
    outputs = model(inputs)
    loss = criterion(outputs, targets)
    fabric.backward(loss)
    optimizer.step()
    optimizer.zero_grad()
```

Fabric measured at ~1.8 min vs ~21 min baseline (single GPU, no AMP) on a representative benchmark — roughly 11× speedup when combining 4 GPUs + BF16.

## 9. Distributed Training — DDP Basics

DistributedDataParallel (DDP) is the correct multi-GPU approach. `DataParallel` is a single-process wrapper with significant overhead; avoid it for anything serious.

```python
# launch with: torchrun --nproc_per_node=4 train.py

import torch.distributed as dist
from torch.nn.parallel import DistributedDataParallel as DDP
from torch.utils.data.distributed import DistributedSampler

def main():
    dist.init_process_group(backend="nccl")   # nccl for GPU, gloo for CPU
    rank = dist.get_rank()
    local_rank = int(os.environ["LOCAL_RANK"])
    device = torch.device(f"cuda:{local_rank}")
    torch.cuda.set_device(device)

    model = MyModel().to(device)
    model = DDP(model, device_ids=[local_rank])

    sampler = DistributedSampler(dataset, shuffle=True)
    loader = DataLoader(dataset, batch_size=64, sampler=sampler, num_workers=4, pin_memory=True)

    for epoch in range(num_epochs):
        sampler.set_epoch(epoch)   # required for correct shuffling across epochs

        for inputs, targets in loader:
            inputs, targets = inputs.to(device), targets.to(device)
            optimizer.zero_grad(set_to_none=True)
            with autocast(device_type="cuda", dtype=torch.bfloat16):
                loss = criterion(model(inputs), targets)
            scaler.scale(loss).backward()
            scaler.step(optimizer)
            scaler.update()

        # only rank 0 saves checkpoints
        if rank == 0:
            torch.save(model.module.state_dict(), f"checkpoint_epoch_{epoch}.pt")

        dist.barrier()   # all ranks wait before next epoch

    dist.destroy_process_group()
```

### DDP pitfalls

- Always call `sampler.set_epoch(epoch)` so each epoch gets a different shuffle.
- Save `model.module.state_dict()`, not `model.state_dict()` — the DDP wrapper adds an extra layer.
- Use `map_location={"cuda:0": f"cuda:{local_rank}"}` when loading checkpoints so each process loads to its own GPU.
- Unbalanced work across processes causes timeouts at synchronization barriers. Ensure batches are roughly equal-sized (`drop_last=True` helps).
- For gradient accumulation with DDP, use `model.no_sync()` on all but the last accumulation step to suppress redundant all-reduce calls.

**FSDP** (Fully Sharded Data Parallel) is the next step when model weights alone exceed a single GPU's memory. Use it via `torch.distributed.fsdp.FullyShardedDataParallel` or Lightning's `strategy="fsdp"`.

## 10. torch.compile

```python
model = torch.compile(model)
```

`torch.compile` fuses Python overhead and pointwise ops into optimized kernels. It can give 10–30% speedup on training for large models, but has meaningful warm-up cost (~minutes) on first call. Skip it for:

- Short training runs (< 10 min total) — warm-up cost dominates.
- Models with very dynamic shapes — recompilations negate the gain.
- Debugging — it makes stack traces harder to read.

Use `torch.compile(model, mode="reduce-overhead")` for models with small batch sizes; `mode="max-autotune"` for maximum throughput when shape is fixed.

## 11. Debugging NaN Loss

Work through these in order:

1. **Isolate the cause.** Disable AMP first (`enabled=False` in `autocast`) to rule out FP16 underflow. If NaN disappears, the issue is loss scaling or a numerically unstable op in low precision.

2. **Check the data.** NaN in inputs propagates everywhere.

   ```python
   assert not torch.isnan(inputs).any(), "NaN in inputs"
   assert not torch.isinf(inputs).any(), "Inf in inputs"
   ```

3. **Enable anomaly detection** (expensive, use only for debugging):

   ```python
   with torch.autograd.set_detect_anomaly(True):
       loss.backward()
   ```

   This prints the exact op that produced NaN in the backward pass.

4. **Check the learning rate.** An LR that is too high causes loss to explode. Try reducing by 10×.

5. **Gradient clipping.** Add `clip_grad_norm_` with `max_norm=1.0` if gradients are exploding (monitor with `grad_norm = sum(p.grad.norm()**2 for p in model.parameters())**0.5`).

6. **Log-softmax / cross-entropy.** Use `nn.CrossEntropyLoss` (which applies log-softmax internally and is numerically stable) rather than manual `log(softmax(x))`.

7. **Weight initialization.** Default PyTorch initialization is usually fine, but custom init with wrong scale causes vanishing/exploding gradients from the first step.

## 12. Common Mistakes

| Mistake | Correct Approach |
| --- | --- |
| `model.zero_grad()` every step | `optimizer.zero_grad(set_to_none=True)` — faster, same effect |
| `optimizer.step()` before `loss.backward()` | Always: zero_grad → forward → backward → (clip) → step |
| `scheduler.step()` before `optimizer.step()` | Step scheduler after optimizer |
| Calling `.item()` or `.numpy()` inside the training loop excessively | Accumulate raw tensors; call `.item()` only for logging |
| Saving `model.state_dict()` on DDP model | Save `model.module.state_dict()` |
| Manual weight decay with `Adam` | Use `AdamW` — Adam + manual WD is mathematically incorrect |
| `DataParallel` for multi-GPU | Use `DistributedDataParallel` via `torchrun` |
| Not calling `sampler.set_epoch(epoch)` in DDP | Required or all epochs see identical data order |
| Clipping gradients before `scaler.unscale_()` | Unscale first, then clip — otherwise you clip scaled (inflated) gradients |
| Disabling `torch.backends.cudnn.benchmark` | Enable it at training start for fixed-size inputs |
| Forgetting `model.eval()` + `torch.no_grad()` during validation | Both are required: `eval()` disables dropout/BN running stats, `no_grad()` saves memory |
| Large models with no activation checkpointing | Use `torch.utils.checkpoint.checkpoint` to trade memory for recomputation |

## 13. Performance Checklist

Before declaring a training run slow:

- [ ] `num_workers > 0` and `pin_memory=True` on DataLoader
- [ ] AMP enabled with `autocast` + `GradScaler` (or BF16 if on A100+)
- [ ] `torch.backends.cudnn.benchmark = True` (fixed input sizes only)
- [ ] Layer dimensions are multiples of 8
- [ ] `optimizer.zero_grad(set_to_none=True)`
- [ ] `non_blocking=True` on `.to(device)` calls
- [ ] No unnecessary `.item()` / `.cpu()` inside the hot loop
- [ ] `Conv2d` layers before `BatchNorm` have `bias=False`
- [ ] For multi-GPU: using DDP via `torchrun`, not `DataParallel`
- [ ] Debugging APIs (`detect_anomaly`, profilers) disabled in production runs
- [ ] Gradient accumulation if GPU memory is the bottleneck (instead of reducing batch size)

---


# Computer Vision Patterns

## Library Stack

Pick the right tool for the job rather than forcing one library across everything.

| Need | Library |
| ------ | --------- |
| Classic image ops, video I/O | OpenCV (`cv2`) |
| Deep learning backbone + training | PyTorch + torchvision |
| Modern architectures (ViT, EfficientNet, Swin) | timm |
| YOLO detection/segmentation/pose | Ultralytics |
| Vision-language, zero-shot | Hugging Face `transformers` (CLIP) |
| Annotation utilities, tracking, dataset I/O | Supervision (`supervision`) |
| Augmentation with mask support | Albumentations |

A realistic production pipeline typically combines at least three of these. OpenCV handles frame capture and classical preprocessing; PyTorch/timm or Ultralytics runs inference; Supervision handles post-processing and visualization.


## Data Augmentation Strategies

Use Albumentations when working with segmentation masks, bounding boxes, or keypoints — it applies the same geometric transform to both image and label simultaneously. torchvision transforms only handle images.

```python
import albumentations as A
from albumentations.pytorch import ToTensorV2

train_transform = A.Compose([
    A.RandomResizedCrop(height=224, width=224, scale=(0.5, 1.0)),
    A.HorizontalFlip(p=0.5),
    A.ColorJitter(brightness=0.2, contrast=0.2, saturation=0.2, hue=0.05, p=0.8),
    A.GaussNoise(p=0.3),
    A.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
    ToTensorV2(),
], bbox_params=A.BboxParams(format="yolo", label_fields=["class_labels"]))
```

For segmentation, pass `mask` alongside `image`:

```python
result = train_transform(image=img_np, mask=mask_np)
img_t, mask_t = result["image"], result["mask"]
```

Key augmentation choices by task:

- **Classification:** RandomResizedCrop + HorizontalFlip + ColorJitter covers most cases. MixUp and CutMix add meaningful gains on ImageNet-scale datasets.
- **Detection:** Mosaic (YOLO's default) is the single highest-impact augmentation. Avoid aggressive geometric distortion that changes aspect ratios unpredictably.
- **Segmentation:** Elastic transforms and grid distortion help with medical imaging. Keep augmentations mild for satellite/aerial where geometry matters.


## Object Detection — YOLO Patterns

YOLOv8/YOLO11 via Ultralytics is the practical default for new detection work. The API is consistent across detection, segmentation, pose, and classification tasks.

```python
from ultralytics import YOLO

# Load a pretrained model
model = YOLO("yolo11n.pt")   # nano; also s/m/l/x variants

# Train
model.train(
    data="path/to/dataset.yaml",
    epochs=100,
    imgsz=640,
    batch=16,
    device=0,               # GPU index or "cpu"
    augment=True,           # mosaic, mixup, etc.
    cos_lr=True,
)

# Inference — single image or batch
results = model.predict(
    source="image.jpg",     # path, URL, numpy array, torch tensor, or directory
    conf=0.25,              # confidence threshold
    iou=0.45,               # NMS IoU threshold
    imgsz=640,
    device=0,
    stream=True,            # generator for large sources (video/dir)
)

for r in results:
    boxes = r.boxes.xyxy.cpu().numpy()   # [N, 4] absolute coords
    scores = r.boxes.conf.cpu().numpy()
    classes = r.boxes.cls.cpu().numpy()
    r.save(filename="out.jpg")           # annotated image
```

### Export for Production

```python
# ONNX (CPU/general)
model.export(format="onnx", dynamic=True, simplify=True)

# TensorRT (NVIDIA GPU, maximum throughput)
model.export(format="engine", half=True, device=0)

# CoreML (Apple Silicon)
model.export(format="coreml")
```

### YOLO Dataset YAML

```yaml
path: /data/mydata
train: images/train
val: images/val
test: images/test

nc: 3
names: ["cat", "dog", "car"]
```

### Architecture Notes

- **YOLOv8/YOLO11:** Anchor-free, C2f module (concatenates all bottleneck outputs for richer gradient flow), decoupled head.
- **YOLOv9:** Adds Programmable Gradient Information (PGI) and Generalized Efficient Layer Aggregation Network (GELAN) to combat information loss in deep networks. Higher mAP on COCO than v8 at equivalent parameter count.
- **NMS-free inference** is available in newer variants (YOLO26+), reducing post-processing latency on edge devices.


## Vision-Language Models — CLIP

CLIP enables zero-shot classification, image-text retrieval, and embedding-based search without task-specific training data.

### Zero-Shot Classification (pipeline API)

```python
import torch
from transformers import pipeline

clip = pipeline(
    task="zero-shot-image-classification",
    model="openai/clip-vit-base-patch32",
    dtype=torch.bfloat16,
    device=0,
)
results = clip(
    "image.jpg",
    candidate_labels=["a photo of a cat", "a photo of a dog", "a traffic scene"],
)
# returns list of {"label": ..., "score": ...} sorted by confidence
```

### Embedding Extraction (AutoModel API)

```python
from transformers import AutoProcessor, AutoModel
import torch

model = AutoModel.from_pretrained(
    "openai/clip-vit-base-patch32",
    dtype=torch.bfloat16,
    attn_implementation="sdpa",   # scaled dot-product attention — faster on modern GPUs
)
processor = AutoProcessor.from_pretrained("openai/clip-vit-base-patch32")

inputs = processor(text=text_list, images=image_list, return_tensors="pt", padding=True)
outputs = model(**inputs)

# L2-normalized embeddings for cosine similarity / vector search
image_embeds = outputs.image_embeds / outputs.image_embeds.norm(dim=-1, keepdim=True)
text_embeds  = outputs.text_embeds  / outputs.text_embeds.norm(dim=-1, keepdim=True)

# Similarity matrix [n_images, n_texts]
similarity = (image_embeds @ text_embeds.T) * model.logit_scale.exp()
probs = similarity.softmax(dim=-1)
```

### Model Variants

| Model | Patch | Params | Notes |
| ------- | ------- | -------- | ------- |
| clip-vit-base-patch32 | 32 | ~150M | Fast, good for prototyping |
| clip-vit-base-patch16 | 16 | ~150M | Better spatial resolution |
| clip-vit-large-patch14 | 14 | ~430M | Highest open-vocab accuracy |

**Text prompt engineering matters significantly.** `"a photo of a {label}"` consistently outperforms bare label strings. For fine-grained tasks, use multiple prompt templates and average the embeddings.


## Classic Computer Vision with OpenCV

OpenCV is the right tool when you need speed, don't want to load a GPU model, or are working with video streams.

```python
import cv2
import numpy as np

# Read and convert color space (OpenCV loads BGR by default)
img = cv2.imread("image.jpg")
img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)

# Resize preserving aspect ratio
def resize_aspect(img, max_dim=640):
    h, w = img.shape[:2]
    scale = max_dim / max(h, w)
    return cv2.resize(img, (int(w * scale), int(h * scale)), interpolation=cv2.INTER_LINEAR)

# Gaussian blur for noise reduction
blurred = cv2.GaussianBlur(img, ksize=(5, 5), sigmaX=1.0)

# Canny edge detection
edges = cv2.Canny(blurred, threshold1=50, threshold2=150)

# Contour detection
contours, hierarchy = cv2.findContours(edges, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
cv2.drawContours(img, contours, -1, (0, 255, 0), 2)

# Template matching
result = cv2.matchTemplate(img_gray, template, cv2.TM_CCOEFF_NORMED)
min_val, max_val, min_loc, max_loc = cv2.minMaxLoc(result)

# Video stream processing
cap = cv2.VideoCapture(0)
while cap.isOpened():
    ret, frame = cap.read()
    if not ret:
        break
    # process frame
    cv2.imshow("frame", frame)
    if cv2.waitKey(1) & 0xFF == ord("q"):
        break
cap.release()
cv2.destroyAllWindows()
```

OpenCV also runs ONNX models directly via `cv2.dnn`, which is useful for embedded deployments without a full PyTorch install.


## Inference Optimization

### Half Precision (FP16)

```python
model = model.half().eval().cuda()
with torch.no_grad(), torch.autocast(device_type="cuda", dtype=torch.float16):
    output = model(input_tensor)
```

### torch.compile (PyTorch 2.x)

```python
model = torch.compile(model, mode="reduce-overhead")  # or "max-autotune"
```

Provides 1.5–2× speedup on repeated inference with the same input shape.

### Batch Inference DataLoader Pattern

```python
from torch.utils.data import DataLoader, Dataset

class ImageFolder(Dataset):
    def __init__(self, paths, transform):
        self.paths = paths
        self.transform = transform

    def __len__(self):
        return len(self.paths)

    def __getitem__(self, i):
        img = Image.open(self.paths[i]).convert("RGB")
        return self.transform(img), self.paths[i]

loader = DataLoader(dataset, batch_size=64, num_workers=4, pin_memory=True)

model.eval()
with torch.no_grad():
    for imgs, paths in loader:
        imgs = imgs.cuda(non_blocking=True)
        outputs = model(imgs)
```

`pin_memory=True` + `non_blocking=True` overlaps CPU-GPU transfer with compute.

### ONNX Export and Runtime

```python
import torch.onnx

dummy = torch.randn(1, 3, 224, 224).cuda()
torch.onnx.export(
    model, dummy, "model.onnx",
    input_names=["input"],
    output_names=["output"],
    dynamic_axes={"input": {0: "batch_size"}, "output": {0: "batch_size"}},
    opset_version=17,
)

# Inference with ONNX Runtime
import onnxruntime as ort
sess = ort.InferenceSession("model.onnx", providers=["CUDAExecutionProvider"])
out = sess.run(None, {"input": img_np.astype("float32")})
```

### TensorRT (NVIDIA only, maximum throughput)

Via Ultralytics: `model.export(format="engine", half=True)` handles the full TRT conversion. For custom models, use `torch2trt` or Polygraphy.

### Edge/CPU Deployment

- **ONNX Runtime** with `CPUExecutionProvider`: portable, works everywhere.
- **OpenVINO**: best for Intel CPUs/iGPUs, significant speedup over vanilla PyTorch CPU.
- **CoreML**: Apple Silicon (M-series), use `coremltools` for export.
- Quantize to INT8 when latency is critical and 1–2% accuracy loss is acceptable.

