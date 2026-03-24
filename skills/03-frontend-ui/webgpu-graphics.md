---
name: webgpu-graphics
description: WebGPU and WGSL — GPU pipeline setup, render passes, compute shaders, buffers, textures, and cross-platform GPU compute in browsers.
domain: frontend
category: graphics
tags: [WebGPU, WGSL, GPU, compute-shaders, render-pipeline, canvas, graphics, web-platform]
triggers: WebGPU, WGSL shader, GPUDevice, GPUBuffer, compute shader browser, WebGPU pipeline, WebGPU render pass
---

# WebGPU and WGSL

## When to Use

### Use raw WebGPU when

- You need direct control over GPU pipelines, compute shaders, or memory layout.
- You're building GPU compute tasks: image processing, physics simulation, ML inference, particle systems, neural rendering.
- You need features unavailable in Three.js/Babylon.js: explicit buffer management, custom compute passes, GPU timing queries, pipeline cache control.
- You're porting a native Vulkan/Metal/D3D12 renderer to the browser.

#### Prefer WebGL2 when

- You need maximum browser coverage today (Safari <17, older Chrome/Firefox).
- Your workload is purely rasterization with no compute requirements.
- You're integrating with a mature library that targets WebGL2 (Three.js r150+, Babylon.js).

#### Prefer Three.js / Babylon.js over raw WebGPU when

- You need a scene graph, asset loading, PBR materials, and lighting out of the box.
- Both Three.js (r163+) and Babylon.js now support WebGPU backends, so you get acceleration without writing WGSL yourself.

---

## Initialization

```javascript
async function initWebGPU() {
  if (!navigator.gpu) throw new Error('WebGPU not supported');

  // Pick adapter — 'high-performance' prefers discrete GPU
  const adapter = await navigator.gpu.requestAdapter({
    powerPreference: 'high-performance',
  });
  if (!adapter) throw new Error('No suitable GPU adapter found');

  // Request device — specify only the features/limits you actually need
  const device = await adapter.requestDevice({
    requiredFeatures: [],          // e.g. 'timestamp-query' for GPU timing
    requiredLimits: {},            // e.g. { maxStorageBufferBindingSize: ... }
  });

  // Always handle device loss — it can happen any time
  device.lost.then((info) => {
    console.error('GPU device lost:', info.message);
    if (info.reason !== 'destroyed') initWebGPU(); // attempt recovery
  });

  // Canvas setup
  const canvas = document.querySelector('canvas');
  const context = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat(); // 'rgba8unorm' or 'bgra8unorm'
  context.configure({ device, format });

  return { adapter, device, context, format };
}
```

---

## Buffers

### Usage flags

| Flag | Purpose |
| ------ | --------- |
| `VERTEX` | Vertex attribute data |
| `INDEX` | Index buffer |
| `UNIFORM` | Small read-only uniform structs |
| `STORAGE` | Large read/write storage in compute/fragment |
| `COPY_SRC` | Source for buffer-to-buffer or buffer-to-texture copies |
| `COPY_DST` | Destination for queue.writeBuffer or copies |
| `MAP_READ` | CPU readback (must pair with COPY_DST) |
| `MAP_WRITE` | CPU upload (must pair with COPY_SRC) |

### Upload via writeBuffer (most common)

```javascript
const uniformBuffer = device.createBuffer({
  size: 64,                                     // bytes, must be multiple of 4
  usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
});
const data = new Float32Array([1, 0, 0, 0,  0, 1, 0, 0,  0, 0, 1, 0,  0, 0, 0, 1]);
device.queue.writeBuffer(uniformBuffer, 0, data);
```

### Upload via mappedAtCreation (zero-copy initial data)

```javascript
const vertexBuffer = device.createBuffer({
  size: vertexData.byteLength,
  usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  mappedAtCreation: true,
});
new Float32Array(vertexBuffer.getMappedRange()).set(vertexData);
vertexBuffer.unmap();
```

### CPU readback (async)

```javascript
const readbackBuffer = device.createBuffer({
  size: outputBuffer.size,
  usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
});

const encoder = device.createCommandEncoder();
encoder.copyBufferToBuffer(outputBuffer, 0, readbackBuffer, 0, outputBuffer.size);
device.queue.submit([encoder.finish()]);

await readbackBuffer.mapAsync(GPUMapMode.READ);
const result = new Float32Array(readbackBuffer.getMappedRange()).slice(); // copy before unmap
readbackBuffer.unmap();
```

`mapAsync` resolves only after the submitted GPU work completes — no explicit fence needed.

---

## Render Pipeline

### WGSL vertex + fragment shader

```wgsl
struct Uniforms {
  modelViewProj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) uv: vec2<f32>,
};

struct VertexOutput {
  @builtin(position) clipPosition: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
  var out: VertexOutput;
  out.clipPosition = uniforms.modelViewProj * vec4<f32>(in.position, 1.0);
  out.uv = in.uv;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return vec4<f32>(in.uv, 0.0, 1.0);
}
```

### Pipeline creation

```javascript
const shaderModule = device.createShaderModule({ code: wgslSource });

const pipeline = device.createRenderPipeline({
  label: 'main-render-pipeline',
  layout: 'auto',                // derive bind group layout from shader reflection
  vertex: {
    module: shaderModule,
    entryPoint: 'vs_main',
    buffers: [
      {
        arrayStride: 5 * 4,      // (vec3 position + vec2 uv) * 4 bytes
        attributes: [
          { shaderLocation: 0, offset: 0,      format: 'float32x3' },
          { shaderLocation: 1, offset: 3 * 4,  format: 'float32x2' },
        ],
      },
    ],
  },
  fragment: {
    module: shaderModule,
    entryPoint: 'fs_main',
    targets: [{ format: presentationFormat }],
  },
  primitive: {
    topology: 'triangle-list',
    cullMode: 'back',
  },
  depthStencil: {
    format: 'depth24plus',
    depthWriteEnabled: true,
    depthCompare: 'less',
  },
});
```

### Render pass and draw call

```javascript
function render() {
  const encoder = device.createCommandEncoder();

  const pass = encoder.beginRenderPass({
    colorAttachments: [{
      view: context.getCurrentTexture().createView(),
      clearValue: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
      loadOp: 'clear',
      storeOp: 'store',
    }],
    depthStencilAttachment: {
      view: depthTextureView,
      depthClearValue: 1.0,
      depthLoadOp: 'clear',
      depthStoreOp: 'store',
    },
  });

  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bindGroup);
  pass.setVertexBuffer(0, vertexBuffer);
  pass.setIndexBuffer(indexBuffer, 'uint16');
  pass.drawIndexed(indexCount);
  pass.end();

  device.queue.submit([encoder.finish()]);
}
```

---

## Compute Pipeline

### WGSL compute shader

```wgsl
@group(0) @binding(0) var<storage, read>       inputData:  array<f32>;
@group(0) @binding(1) var<storage, read_write> outputData: array<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= arrayLength(&inputData)) { return; }
  outputData[i] = inputData[i] * 2.0;
}
```

Always bounds-check with `arrayLength()` — the dispatch count is rounded up and trailing threads will be out of range.

### Pipeline creation and dispatch

```javascript
const computePipeline = device.createComputePipeline({
  label: 'double-values',
  layout: 'auto',
  compute: {
    module: device.createShaderModule({ code: wgslSource }),
    entryPoint: 'cs_main',
  },
});

const bindGroup = device.createComputeBindGroup(computePipeline, [inputBuffer, outputBuffer]);

const encoder = device.createCommandEncoder();
const pass = encoder.beginComputePass();
pass.setPipeline(computePipeline);
pass.setBindGroup(0, bindGroup);
pass.dispatchWorkgroups(Math.ceil(elementCount / 64)); // one workgroup per 64 elements
pass.end();

// Copy results to readback buffer, then submit
encoder.copyBufferToBuffer(outputBuffer, 0, readbackBuffer, 0, outputBuffer.size);
device.queue.submit([encoder.finish()]);
```

### Shared workgroup memory + barrier

```wgsl
const CHUNK = 256u;
var<workgroup> localSum: array<atomic<u32>, CHUNK>;

@compute @workgroup_size(256)
fn reduce(@builtin(local_invocation_index) lid: u32,
          @builtin(global_invocation_id)   gid: vec3<u32>) {
  atomicStore(&localSum[lid], 0u);
  workgroupBarrier();                          // all threads reach this before continuing

  let val = u32(inputData[gid.x]);
  atomicAdd(&localSum[lid % CHUNK], val);

  workgroupBarrier();
  if (lid == 0u) {
    var sum = 0u;
    for (var k = 0u; k < CHUNK; k++) {
      sum += atomicLoad(&localSum[k]);
    }
    outputData[gid.x / CHUNK] = f32(sum);
  }
}
```

---

## WGSL Essentials

### Scalar and vector types

| WGSL type | Description |
| ----------- | ------------- |
| `f32` | 32-bit float |
| `i32` / `u32` | signed / unsigned 32-bit int |
| `bool` | boolean |
| `vec2<f32>` / `vec2f` | 2-component float vector |
| `vec3<f32>` / `vec3f` | 3-component float vector |
| `vec4<f32>` / `vec4f` | 4-component float vector |
| `mat4x4<f32>` / `mat4x4f` | 4×4 float matrix |
| `array<f32, N>` | fixed-size array |
| `array<f32>` | runtime-sized array (storage only) |
| `atomic<u32>` | atomic integer (storage or workgroup) |

### Address spaces

```wgsl
var<uniform>   u: MyStruct;          // read-only, small, tightly packed
var<storage, read>       r: array<f32>;   // large read-only storage buffer
var<storage, read_write> rw: array<f32>;  // large read/write storage buffer
var<workgroup> shared: array<f32, 64>;    // per-workgroup shared memory (compute only)
var<private>   local: f32;               // per-invocation private variable
```

### Binding attributes

```wgsl
@group(0) @binding(0) var<uniform>         uniforms: MyUniforms;
@group(0) @binding(1) var<storage, read>   positions: array<vec4f>;
@group(1) @binding(0) var mySampler:        sampler;
@group(1) @binding(1) var myTexture:        texture_2d<f32>;
```

Group 0 typically holds per-material or per-pass data; group 1 per-object data. Keep bind group changes as infrequent as possible.

### Built-in values

```wgsl
// Vertex stage
@builtin(vertex_index)    vi: u32           // current vertex index
@builtin(position)        pos: vec4<f32>    // output clip position

// Fragment stage
@builtin(position)        fragCoord: vec4<f32>   // input pixel position
@builtin(front_facing)    front: bool

// Compute stage
@builtin(global_invocation_id)   gid: vec3<u32>
@builtin(local_invocation_id)    lid: vec3<u32>
@builtin(local_invocation_index) lidx: u32
@builtin(workgroup_id)           wid: vec3<u32>
@builtin(num_workgroups)         nwg: vec3<u32>
```

---

## Textures and Samplers

### Create and upload a texture

```javascript
const texture = device.createTexture({
  size: [width, height],
  mipLevelCount: Math.floor(Math.log2(Math.max(width, height))) + 1,
  format: 'rgba8unorm',
  usage: GPUTextureUsage.TEXTURE_BINDING
       | GPUTextureUsage.COPY_DST
       | GPUTextureUsage.RENDER_ATTACHMENT, // required for mip generation via render pass
});

// Upload base level from an ImageBitmap
device.queue.copyExternalImageToTexture(
  { source: imageBitmap },
  { texture, mipLevel: 0 },
  [width, height],
);

// Or upload raw pixel data
device.queue.writeTexture(
  { texture, mipLevel: 0 },
  pixelData,                           // ArrayBuffer / TypedArray
  { bytesPerRow: width * 4 },
  [width, height],
);
```

WebGPU does not generate mipmaps automatically. Use a render-pass blit loop or a compute shader to downsample each level.

### Sampler

```javascript
const sampler = device.createSampler({
  addressModeU: 'repeat',
  addressModeV: 'repeat',
  magFilter: 'linear',
  minFilter: 'linear',
  mipmapFilter: 'linear',
  maxAnisotropy: 16,                   // requires 'anisotropic-filtering' feature on adapter
});
```

Address modes: `'repeat'`, `'mirror-repeat'`, `'clamp-to-edge'`.

### WGSL texture sampling

```wgsl
@group(1) @binding(0) var mySampler: sampler;
@group(1) @binding(1) var myTexture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
  return textureSample(myTexture, mySampler, in.uv);
}
```

For compute shaders (no sampler needed for direct texel load):

```wgsl
@group(0) @binding(0) var srcTexture: texture_2d<f32>;

@compute @workgroup_size(8, 8)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(srcTexture, 0);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let color = textureLoad(srcTexture, gid.xy, 0);  // mip level 0
  // ...
}
```

---

## Bind Groups

### Explicit bind group layout (preferred for reuse)

```javascript
const bindGroupLayout = device.createBindGroupLayout({
  entries: [
    { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
      buffer: { type: 'uniform' } },
    { binding: 1, visibility: GPUShaderStage.FRAGMENT,
      sampler: { type: 'filtering' } },
    { binding: 2, visibility: GPUShaderStage.FRAGMENT,
      texture: { sampleType: 'float', viewDimension: '2d' } },
    { binding: 3, visibility: GPUShaderStage.COMPUTE,
      buffer: { type: 'storage' } },
  ],
});

const pipelineLayout = device.createPipelineLayout({
  bindGroupLayouts: [bindGroupLayout],
});
```

Pass `pipelineLayout` instead of `'auto'` when multiple pipelines share the same layout — this avoids redundant bind group recreation.

### Creating a bind group

```javascript
const bindGroup = device.createBindGroup({
  layout: bindGroupLayout,              // or pipeline.getBindGroupLayout(0) if layout:'auto'
  entries: [
    { binding: 0, resource: { buffer: uniformBuffer } },
    { binding: 1, resource: sampler },
    { binding: 2, resource: texture.createView() },
    { binding: 3, resource: { buffer: storageBuffer } },
  ],
});
```

### Dynamic offsets

For uniform buffers that store per-object data packed together:

```javascript
const bindGroup = device.createBindGroup({
  layout: bindGroupLayout,
  entries: [{ binding: 0, resource: { buffer: packedUniformBuffer, size: 256 } }],
});

// In render loop:
pass.setBindGroup(0, bindGroup, [objectIndex * 256]); // dynamic offset in bytes
```

Dynamic offsets must be multiples of `minUniformBufferOffsetAlignment` (typically 256).

---

## Performance

### Avoid per-frame pipeline or bind group creation

Pipeline compilation is expensive and async. Create all pipelines at startup with `createRenderPipelineAsync` / `createComputePipelineAsync` to avoid blocking the main thread.

```javascript
// Prefer the async variant to avoid stalls
const pipeline = await device.createRenderPipelineAsync(descriptor);
```

### Buffer reuse over recreation

Never call `device.createBuffer` in the render loop. Pre-allocate with `mappedAtCreation` or `writeBuffer` for dynamic data.

### GPU timing queries (requires 'timestamp-query' feature)

```javascript
const querySet = device.createQuerySet({ type: 'timestamp', count: 2 });
const queryBuffer = device.createBuffer({
  size: 2 * 8,                          // 2 timestamps × 8 bytes (BigUint64)
  usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC,
});

const pass = encoder.beginComputePass({
  timestampWrites: {
    querySet,
    beginningOfPassWriteIndex: 0,
    endOfPassWriteIndex: 1,
  },
});
// ...
pass.end();
encoder.resolveQuerySet(querySet, 0, 2, queryBuffer, 0);
encoder.copyBufferToBuffer(queryBuffer, 0, readbackBuffer, 0, 16);
device.queue.submit([encoder.finish()]);

await readbackBuffer.mapAsync(GPUMapMode.READ);
const times = new BigInt64Array(readbackBuffer.getMappedRange());
const gpuMs = Number(times[1] - times[0]) / 1e6;
readbackBuffer.unmap();
```

### Minimize bind group switches

Sort draw calls by pipeline first, then by bind group. Switching bind groups mid-pass is cheap relative to pipeline switches but still carries cost.

### Batch writeBuffer calls

`device.queue.writeBuffer` is synchronous on the CPU side but incurs a staging buffer copy. Batch multiple uniform updates before submitting the command encoder.

### Workgroup size

For 1D compute: use 64. For 2D image processing: use 8×8 (= 64 threads total). Avoid workgroup sizes that are not multiples of 32 (the warp/wave size on most hardware).

---

## Critical Rules / Gotchas

### Browser support (as of early 2026)

- Chrome 113+ (desktop and Android): stable
- Edge 113+: stable
- Firefox: behind a flag (`dom.webgpu.enabled`), not yet stable by default
- Safari 18+ (macOS 15 / iOS 18): stable
- Feature detect with `if (!navigator.gpu)` and always provide a fallback

#### Async device loss

`requestDevice()` never rejects. If adapter creation fails internally the returned `GPUDevice` is already lost. Always attach a `.lost` handler.

#### Validation errors are async and silent by default

Errors appear in the browser console but do not throw. Wrap suspect calls in `pushErrorScope` / `popErrorScope` during development:

```javascript
device.pushErrorScope('validation');
const buf = device.createBuffer({ size: 0, usage: GPUBufferUsage.VERTEX }); // size 0 is invalid
const err = await device.popErrorScope();
if (err) console.error('Validation error:', err.message);
```

#### `vec3f` alignment trap

In a struct, `vec3<f32>` occupies 16 bytes (not 12) due to WGSL alignment rules. A struct `{ a: vec3f, b: f32 }` is 16 bytes, not 16. But `{ a: vec3f }` used as an array element gives stride 16, not 12. Always verify with a struct-size calculation or use `vec4f` and ignore the w component.

#### COPY_DST required for writeBuffer

Any buffer that will receive data via `queue.writeBuffer` or `encoder.copyBufferToBuffer` must declare `COPY_DST`.

#### mapAsync and GPU work ordering

`mapAsync` implicitly waits for all previously submitted GPU work that writes to the buffer. You do not need an explicit fence.

#### No implicit synchronization between compute and render passes

If a compute pass writes a buffer that a subsequent render pass reads, WebGPU handles the barrier automatically within a single `submit()`. Splitting across two `submit()` calls loses that guarantee.

#### Canvas texture is only valid during the frame

`context.getCurrentTexture()` returns a new texture each frame and becomes invalid after `submit()`. Never hold a reference across frames.

#### Pipeline layout reuse

When two pipelines share the same bind group layout, creating them with an explicit `GPUPipelineLayout` lets you reuse the same `GPUBindGroup` for both — avoids redundant object creation.

---

## Key APIs

| API | Purpose |
| ----- | --------- |
| `navigator.gpu.requestAdapter(opts)` | Select GPU adapter |
| `adapter.requestDevice(desc)` | Create logical device |
| `device.createBuffer(desc)` | Allocate GPU buffer |
| `device.queue.writeBuffer(buf, offset, data)` | Upload CPU data to buffer |
| `device.createShaderModule({ code })` | Compile WGSL source |
| `device.createRenderPipeline(desc)` | Build raster pipeline |
| `device.createComputePipeline(desc)` | Build compute pipeline |
| `device.createRenderPipelineAsync(desc)` | Non-blocking pipeline build |
| `device.createBindGroupLayout(desc)` | Define resource binding layout |
| `device.createBindGroup(desc)` | Bind resources to layout |
| `device.createTexture(desc)` | Allocate GPU texture |
| `device.createSampler(desc)` | Create texture sampler |
| `device.createCommandEncoder()` | Begin recording commands |
| `encoder.beginRenderPass(desc)` | Start render pass |
| `encoder.beginComputePass(desc)` | Start compute pass |
| `pass.dispatchWorkgroups(x, y, z)` | Launch compute work |
| `device.queue.submit([cmdbuf])` | Send commands to GPU |
| `buf.mapAsync(mode)` | Map buffer for CPU access |
| `buf.getMappedRange()` | Get ArrayBuffer view of mapped region |
| `device.pushErrorScope(filter)` / `popErrorScope()` | Capture validation errors |
| `device.createQuerySet({ type: 'timestamp' })` | GPU timing queries |

---

## References

- [WebGPU Fundamentals](https://webgpufundamentals.org/webgpu/lessons/webgpu-fundamentals.html)
- [WebGPU Compute Shaders Basics](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)
- [WebGPU Compute Shaders — Image Histogram](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders-histogram.html)
- [Using WebGPU Compute Shaders with Vertex Data — toji.dev](https://toji.dev/webgpu-best-practices/compute-vertex-data.html)
- [WGSL Specification — W3C](https://www.w3.org/TR/WGSL/)
- [WebGPU Explainer — gpuweb.github.io](https://gpuweb.github.io/gpuweb/explainer/)
- [Reaction-Diffusion Compute Shader in WebGPU — Codrops](https://tympanus.net/codrops/2024/05/01/reaction-diffusion-compute-shader-in-webgpu/)
