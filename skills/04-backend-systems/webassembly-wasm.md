---
name: webassembly-wasm
description: Patterns, workflows, and hard-won lessons for writing, integrating, and deploying WebAssembly modules — covering Rust/wasm-bindgen, Emscripten for C/C++, JS/WASM interop, WASI, the Component Model, and server-side runtimes.
domain: frontend
category: wasm
tags: [WebAssembly, WASM, Rust, wasm-bindgen, WASI, Emscripten, wasm-pack, Wasmtime, WasmEdge, Component Model]
triggers: [webassembly, wasm, wasm-bindgen, wasm-pack, emscripten, wasi, wasmtime, wasmedge, "compile to wasm", "rust to wasm", "c++ to wasm", "wasm interop", "wasm memory", "component model"]
---

# WebAssembly (WASM) — Patterns and Practices

## When to reach for WASM

WASM is worth the toolchain cost only when one of these conditions holds:

- **CPU-bound hot path** — image/video codecs, cryptography, compression, physics engines, signal processing, ML inference. The threshold is roughly: if a pure-JS implementation takes >16 ms and runs on every frame or every request, WASM is worth profiling.
- **Porting an existing C/C++/Rust library** — compiling battle-tested native code beats rewriting it in JS. AutoCAD, Figma, Photoshop web, and ffmpeg.wasm all follow this pattern.
- **Deterministic, sandboxed execution** — WASM's capability-based security model makes it a strong choice for plugin systems, user-submitted code runners, and edge functions.
- **Language portability on the server** — WASI lets the same binary run in a browser, a Node process, a serverless function, and an edge node without modification.

Do **not** reach for WASM for I/O-bound work, thin glue code, or simple DOM manipulation — the JS↔WASM boundary has overhead that will dwarf any gains.

---

## Rust + wasm-bindgen workflow

### Toolchain setup

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

`wasm-pack` wraps `wasm-bindgen-cli`, runs `wasm-opt`, and produces an npm-compatible package in one command. Prefer it over calling `wasm-bindgen` directly for browser targets.

### Cargo.toml essentials

```toml
[lib]
crate-type = ["cdylib"]   # required — produces a .wasm shared library

[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"            # bindings to JS built-ins (Array, Promise, Date…)
web-sys = { version = "0.3", features = ["Window", "Document", …] }

[profile.release]
opt-level = "z"           # minimize binary size
lto = true
codegen-units = 1
panic = "abort"           # removes unwinding code, shrinks binary
```

### Annotating the API

```rust
use wasm_bindgen::prelude::*;

// Export to JS — callable as a normal JS function
#[wasm_bindgen]
pub fn crunch(input: &[u8]) -> Vec<u8> { … }

// Import from JS — call JS functions from Rust
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Export a struct with methods
#[wasm_bindgen]
pub struct Encoder { … }

#[wasm_bindgen]
impl Encoder {
    #[wasm_bindgen(constructor)]
    pub fn new(quality: u8) -> Encoder { … }

    pub fn encode(&mut self, frame: &[u8]) -> Vec<u8> { … }
}
```

The `#[wasm_bindgen]` macro generates the JS glue that handles type marshaling automatically for the supported types: primitives, `String`, `Vec<u8>`, `JsValue`, `Option<T>`, and `Result<T, JsValue>`.

### Complex data — use serde + serde-wasm-bindgen

For structs crossing the boundary, `serde-wasm-bindgen` avoids a full JSON round-trip while still handling arbitrary shapes:

```rust
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct Config { pub width: u32, pub quality: f32 }

#[wasm_bindgen]
pub fn process(config: JsValue) -> Result<JsValue, JsValue> {
    let cfg: Config = serde_wasm_bindgen::from_value(config)?;
    let result = do_work(cfg);
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
```

### Building

```bash
wasm-pack build --target web --release   # browser ES module
wasm-pack build --target bundler         # for webpack/vite
wasm-pack build --target nodejs          # for Node.js
```

`wasm-pack` automatically runs `wasm-opt -O3 -Oz` on the output unless you pass `--dev`.

---

## Emscripten for C/C++

Emscripten compiles C/C++ to WASM and generates the JS glue that wires up memory, imports, and the module lifecycle.

### Basic compilation

```bash
# Install: https://emscripten.org/docs/getting_started/downloads.html
emcc src/main.cpp -O3 -o out/app.js \
  -s WASM=1 \
  -s EXPORTED_FUNCTIONS='["_my_func","_malloc","_free"]' \
  -s EXPORTED_RUNTIME_METHODS='["ccall","cwrap"]'
```

Key flags:

| Flag | Effect |
| --- | --- |
| `-O3` / `-Oz` | Release optimizations / size-first |
| `--closure 1` | Minifies the JS glue — important for production |
| `-flto` | Link-time optimization across all TUs |
| `--emit-tsd` | Generate TypeScript declarations |
| `-s ALLOW_MEMORY_GROWTH=1` | Dynamic heap; slight overhead vs fixed |
| `-s INITIAL_MEMORY=64MB` | Set heap size explicitly |
| `-s ASSERTIONS=0` | Disable runtime checks in production |
| `--pre-js` / `--post-js` | Inject JS before/after the generated glue |

### Exposing C functions to JS

```c
#include <emscripten.h>

EMSCRIPTEN_KEEPALIVE
int compress(uint8_t* input, int len, uint8_t* output) { … }
```

From JS:

```js
const compress = Module.cwrap('compress', 'number', ['number','number','number']);
```

### Streaming instantiation in the browser

```js
// The only correct pattern — never use fetch+arrayBuffer+instantiate
const { instance } = await WebAssembly.instantiateStreaming(
  fetch('/app.wasm'),
  importObject
);
```

`instantiateStreaming` compiles while the binary downloads (Firefox: 30–60 MB/s compilation throughput) and is required for V8's code caching to kick in. Non-streaming load is a common and avoidable performance mistake.

---

## JS/WASM interop — memory, strings, and the boundary cost

### The linear memory model

WASM has a single flat `ArrayBuffer` (`WebAssembly.Memory`). JS and WASM share it directly — no copy needed if you work with typed array views into that buffer.

```js
// Read output from WASM without copying
const ptr = instance.exports.get_result_ptr();
const len = instance.exports.get_result_len();
const view = new Uint8Array(instance.exports.memory.buffer, ptr, len);
// view is a live window into WASM memory — zero copy
```

**When memory grows** (via `memory.grow()`), the `ArrayBuffer` is replaced. Any cached `TypedArray` views over it become detached and will throw. Re-create views after any call that might grow the heap.

### Strings — the expensive case

WASM has no string type. Crossing the boundary requires encoding to bytes, allocating in WASM linear memory, passing the pointer, and freeing after use. wasm-bindgen automates this for Rust, but the underlying cost is real.

Manual pattern (C/Emscripten):

```js
function passString(instance, str) {
  const bytes = new TextEncoder().encode(str + '\0');
  const ptr = instance.exports.malloc(bytes.length);
  new Uint8Array(instance.exports.memory.buffer, ptr, bytes.length).set(bytes);
  return ptr;  // caller must call instance.exports.free(ptr) after use
}
```

Avoid frequent string passing in hot loops. For bulk data, prefer typed binary buffers with a fixed header format — write the entire array as a packed struct, pass one pointer, let WASM read in bulk.

### Boundary call overhead

Each WASM function call from JS has a small but non-zero overhead (~tens of nanoseconds). In tight loops, batch work on the WASM side and return a result rather than calling WASM per-element.

### Offloading to a Worker

Put the WASM module in a `Worker` to keep the main thread free. Compile the module once on the main thread, transfer the `WebAssembly.Module` (structured-cloneable) to the worker, and instantiate there. The worker only bears instantiation cost, not compilation cost.

```js
// main thread
const mod = await WebAssembly.compileStreaming(fetch('/heavy.wasm'));
worker.postMessage({ type: 'init', mod }, []);  // transfer, not copy

// worker
self.onmessage = async ({ data }) => {
  if (data.type === 'init') {
    instance = await WebAssembly.instantiate(data.mod, imports);
  }
};
```

---

## WASI — server-side and portable WASM

WASI (WebAssembly System Interface) gives a WASM module access to OS primitives (filesystem, sockets, clocks, env vars) through a capability-based API, without needing a browser. The same binary runs everywhere a WASI-compliant runtime exists.

### WASI versions

| Version | Status | Key addition |
| --- | --- | --- |
| WASI Preview 1 (0.1) | Stable, widely supported | Basic POSIX-like syscalls |
| WASI Preview 2 (0.2, Jan 2024) | Stable — current target | Component Model, typed interfaces (WIT), worlds |
| WASI 0.3 | In progress (late 2025) | Native async, typed streams, composable futures |

Target WASI 0.2 for new code. WASI 0.1 is still required for some runtimes and toolchains that haven't caught up.

### WASI "worlds"

WASI 0.2 groups APIs into *worlds* — sets of interfaces for a specific environment:

- `wasi:cli/imports` — command-line apps (stdio, env, args, filesystem, sockets)
- `wasi:http/proxy` — HTTP handler world for edge/serverless use (Spin, Cloudflare Workers WASM)
- Custom worlds are composable via the Component Model

### Running a WASI binary

```bash
# Wasmtime
wasmtime run --dir /data my-module.wasm -- --flag value

# WasmEdge
wasmedge --dir /data:/ my-module.wasm

# Node.js (WASI Preview 1 only)
node --experimental-wasi-unstable-preview1 runner.mjs
```

---

## The Component Model

The Component Model (stabilized in WASI 0.2) solves language-agnostic interop between WASM binaries. Two components written in different languages can call each other through a shared WIT (WASM Interface Types) interface — no shared memory, no raw pointer passing.

### WIT interface definition

```wit
// math.wit
package example:math@0.1.0;

interface operations {
  add: func(a: f64, b: f64) -> f64;
  sqrt: func(x: f64) -> f64;
}

world math-lib {
  export operations;
}
```

### Generating bindings

```bash
# Rust
cargo add wit-bindgen
# generates src/bindings.rs from the .wit file at build time

# JS/TS — jco (the JS component toolchain)
npx @bytecodealliance/jco transpile math.component.wasm -o ./bindings
```

### Composition

`wac` (WASM Composition tool) can link components at build time into a single composite component — a Rust codec component composed with a JS orchestration component, for example.

Key benefit: a Python component and a Rust component can call each other with no shared-memory hacks, no C ABI, no serialization to JSON. The runtime (Wasmtime, WasmEdge, Spin) handles the translation.

---

## Server-side runtimes

### Wasmtime

- Written in Rust; developed by Bytecode Alliance.
- Primary WASI 0.2 reference implementation; best Component Model support.
- JIT (Cranelift backend) plus AOT (`wasmtime compile`).
- Benchmarks: 85–90% of native performance in steady state; best-in-class multi-tenant memory density.
- Best for: server processes, embedding in Rust applications (`wasmtime` crate), CI sandbox runners.

```rust
// Embed in a Rust host
use wasmtime::*;
let engine = Engine::default();
let module = Module::from_file(&engine, "plugin.wasm")?;
let mut store = Store::new(&engine, ());
let instance = Instance::new(&mut store, &module, &[])?;
let func = instance.get_typed_func::<(i32,), i32>(&mut store, "process")?;
let result = func.call(&mut store, (42,))?;
```

### WasmEdge

- Written in C++; strong OCI/container and cloud-native integration.
- AOT compilation closes the gap with Wasmtime on throughput benchmarks.
- Purpose-built for edge deployment: AWS Lambda, Google Cloud Run, Azure, and Docker Desktop all support it.
- Best for: edge functions, AI inference workloads (has ONNX and PyTorch WASM backends), IoT, and scenarios where you need Kubernetes-style orchestration of WASM.

### Wasmer

- Supports multiple compiler backends (Cranelift, LLVM, Singlepass).
- Singlepass is intentionally fast to compile, low-quality output — useful for short-lived executions where startup cost > runtime cost.
- `wasmer compile` produces `.wasmu` AOT artifacts for repeated fast loading.

### Choosing a runtime

| Need | Runtime |
| --- | --- |
| WASI 0.2 + Component Model | Wasmtime |
| Edge / cloud-native / containers | WasmEdge |
| Fast cold starts, scripting use cases | Wasmer (Singlepass) |
| Browser | V8 (built into Chrome/Node), SpiderMonkey (Firefox) |

---

## Performance considerations

### Startup latency

- Use `WebAssembly.instantiateStreaming` (browser) — compiles while downloading.
- V8 and SpiderMonkey cache the compiled native code in IndexedDB after the first load. A 50 MB .wasm that takes 47 s cold compiles in under 1 s on cache hit. Serve the `.wasm` with immutable cache headers so the cache stays valid.
- For server runtimes, AOT-compile to a `.cwasm` / `.wasmu` artifact at deploy time and load that — skip JIT overhead entirely at runtime.

### Binary size

- For Rust: `opt-level = "z"`, `lto = true`, `panic = "abort"` in release profile. Use `wasm-opt -Oz` (included in wasm-pack).
- For C/C++: Emscripten's `-Oz --closure 1` combination. Dead-code elimination at link time via LTO.
- Gzip/Brotli compress `.wasm` at the CDN layer — WASM compresses extremely well (often 3–5×).

### Memory

- WASM linear memory starts at whatever `initial_memory` you set and grows in 64 KB pages via `memory.grow()`.
- Memory never shrinks back. Avoid growth in hot paths; pre-allocate enough up front.
- If `ALLOW_MEMORY_GROWTH=1` (Emscripten) or dynamic growth (Rust), re-acquire `TypedArray` views after every call that might grow the heap.
- Multi-threading requires `SharedArrayBuffer`, which in turn requires `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` response headers.

### Throughput

- Minimize JS↔WASM boundary crossings; batch work inside WASM.
- Pass large payloads via shared linear memory (pointer + length), not by value.
- Profile with Chrome DevTools WASM profiling or `perf` + DWARF debug info (wasm-pack builds include source maps).

---

## Common pitfalls

**Passing strings in a loop.** Each string crossing allocates, encodes, and later frees. Move the loop inside the WASM module and pass the full dataset as a binary buffer.

**Not freeing allocations.** When Emscripten or manual WASM code allocates memory (e.g., for a return value), JS must call the corresponding `free(ptr)`. wasm-bindgen handles this automatically for Rust — the pattern is not free in C.

**Forgetting `--release` / `-O3`.** Debug WASM can be 10–100× slower than release. Always benchmark release builds.

**Shipping huge binaries.** Including an entire Rust standard library because one function needs `HashMap`. Audit with `twiggy` (WASM binary size profiler) and remove unused features with `default-features = false`.

**Non-streaming instantiation.** `fetch().then(r => r.arrayBuffer()).then(buf => WebAssembly.instantiate(buf))` downloads the entire binary before compilation starts. Replace with `instantiateStreaming`.

**Stale TypedArray views after memory growth.** Cache `new Uint8Array(memory.buffer)` across calls at your peril — `memory.buffer` is replaced on growth. Create views lazily or re-acquire after every WASM call.

**Ignoring threading requirements.** `SharedArrayBuffer` is disabled by default in most origins due to Spectre mitigations. Set the required COOP/COEP headers server-side, not just in dev.

**Mixing WASI Preview 1 and Preview 2 binaries.** They are not binary-compatible. Know which world your toolchain targets before composing components.

---

## Decision checklist

Before adding WASM to a project:

1. Profile the JS baseline first — WASM is not always faster for small inputs.
2. Confirm the workload is CPU-bound, not I/O-bound.
3. Choose toolchain: Rust + wasm-pack (greenfield, best ergonomics), Emscripten (existing C/C++ code), AssemblyScript (JS-familiar, limited perf ceiling).
4. Decide browser vs. server vs. edge — pick the WASI world and runtime accordingly.
5. Plan the interop contract up front: which functions cross the boundary, what data shapes, who owns memory.
6. Budget for binary size and startup time, not just throughput.


