---
name: threejs-webgl
description: Use this skill when building, optimizing, or debugging 3D scenes with Three.js, React Three Fiber (R3F), or migrating from WebGL to WebGPU. Covers scene/camera/renderer setup, geometry and materials, lighting, animation loops, R3F and Drei patterns, performance (instancing, LOD, frustum culling, dispose), post-processing, GLTF/GLB loading, custom shaders (ShaderMaterial and TSL), and the WebGPU migration path introduced in Three.js r171+.
domain: frontend
category: 3d
tags: [Three.js, WebGL, WebGPU, React-Three-Fiber, 3D, canvas, shaders, TSL, Drei, GLTF, instancing, post-processing]
triggers: Three.js setup, WebGL scene, WebGPU migration, React Three Fiber, R3F, Drei helpers, 3D canvas, GLTF loading, shader material, instanced mesh, LOD, post-processing, requestAnimationFrame loop, 3D performance
---

# Three.js, WebGL, and WebGPU Patterns

A practical reference for building production-quality 3D scenes on the web. Covers vanilla Three.js and the React Three Fiber (R3F) abstraction, with a section on the WebGPU migration path available since r171.

---

## When to Use This Skill

- Building or debugging a Three.js scene from scratch.
- Integrating 3D into a React application with R3F.
- Hitting performance walls (draw calls, memory leaks, frame drops).
- Loading and displaying GLTF/GLB models.
- Writing custom vertex/fragment shaders.
- Migrating an existing WebGL codebase to WebGPU.

---

## 1. Scene / Camera / Renderer Setup

### Minimal boilerplate

```javascript
import * as THREE from 'three';

const scene = new THREE.Scene();

const camera = new THREE.PerspectiveCamera(
  75,                                    // fov
  window.innerWidth / window.innerHeight, // aspect
  0.1,                                   // near — keep as large as possible
  100                                    // far  — keep as small as possible
);
camera.position.set(0, 1, 5);

const renderer = new THREE.WebGLRenderer({
  antialias: true,
  powerPreference: 'high-performance',
  alpha: false,        // transparent canvas background — disable unless needed
  stencil: false,      // saves memory when stencil buffer is unused
  depth: true,         // almost always required
});
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); // cap at 2; devices go up to 5
renderer.outputColorSpace = THREE.SRGBColorSpace; // r152+ replaces outputEncoding
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1;
document.body.appendChild(renderer.domElement);
```

### Key setup rules

- One Three.js unit = one meter. Keep scene dimensions logical.
- Never move the `Scene` object itself. Move objects within it.
- Center the scene around the origin to avoid floating-point drift at large coordinates.
- Keep the camera's `near`/`far` frustum as tight as the content allows — this directly improves depth buffer precision and culling efficiency.
- Set a non-black background color during development to confirm the renderer is producing output.

### Resize handling

```javascript
window.addEventListener('resize', () => {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
});
```

---

## 2. Geometry and Materials

### Geometry

Always use `BufferGeometry`-based constructors (the default in Three.js r125+):

```javascript
const geo = new THREE.BoxGeometry(1, 1, 1);
const sphere = new THREE.SphereGeometry(0.5, 32, 32);
```

Merge static geometry that shares a material into a single mesh using `BufferGeometryUtils.mergeGeometries()`. This collapses multiple draw calls into one.

```javascript
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
const merged = mergeGeometries([geo1, geo2, geo3]);
```

### Materials (cheapest to most expensive)

| Material | Cost | Use when |
| --- | --- | --- |
| `MeshBasicMaterial` | Lowest | No lighting needed; debugging |
| `MeshLambertMaterial` | Low | Diffuse-only; mobile/low-end |
| `MeshPhongMaterial` | Medium | Specular highlights |
| `MeshStandardMaterial` | High | PBR — default for production |
| `MeshPhysicalMaterial` | Highest | Clearcoat, transmission, iridescence |

- Prefer `MeshLambertMaterial` for large numbers of non-reflective objects.
- Share material instances across meshes whenever possible — changing a uniform on a shared material affects all users.
- Use `alphaTest` (e.g. `0.5`) rather than `transparent: true` for cutout effects; transparent objects sort and draw twice.

### Texture rules

- Dimensions must be powers of two: 128, 256, 512, 1024, 2048. Non-POT textures disable mipmapping and require clamped wrapping.
- Set `texture.colorSpace = THREE.SRGBColorSpace` for color/albedo maps only. Normal, roughness, metalness, and AO maps stay in linear space.
- Use the smallest viable texture. A 256×256 tiling diffuse can outperform a 4096×4096 unique map.
- Use `KTX2` / `Basis` compressed textures for large atlases — the `KTX2Loader` and `EXRLoader` ship with Three.js addons.

---

## 3. Lighting

### Light types and cost

| Light | Shadows | Draw cost | Notes |
| --- | --- | --- | --- |
| `AmbientLight` | No | Negligible | Global fill only |
| `HemisphereLight` | No | Negligible | Sky/ground gradient fill |
| `DirectionalLight` | Yes (1 depth pass) | Medium | Sun; shadow map is orthographic |
| `SpotLight` | Yes (1 depth pass) | Medium-high | Cone; needs target |
| `PointLight` | Yes (6 depth passes) | Very high | Avoid with shadows when possible |
| `RectAreaLight` | No | High | Area lights; requires `RectAreaLightUniformsLib` |

Rules:

- Direct lights are expensive. Use as few as possible — two or three at most.
- Prefer HDRI environment maps (`PMREMGenerator` + `RGBELoader`) over multiple dynamic lights for ambient/reflective lighting. They look better and cost far less per frame.
- Toggle visibility with `light.visible = false` or `light.intensity = 0` — do not add/remove lights at runtime, as that forces shader recompilation.
- Shadow maps: call `renderer.shadowMap.needsUpdate = true` only on frames where the scene changes for static setups. Use `CameraHelper` during development to visualize shadow frustum and trim it tightly.
- `PointLight` shadows render the scene six times (cube map faces). Use only when necessary.

---

## 4. Animation Loop

```javascript
const clock = new THREE.Clock();

function animate() {
  requestAnimationFrame(animate);

  const delta = clock.getDelta(); // seconds since last frame

  // update logic
  mesh.rotation.y += delta * 0.5;

  renderer.render(scene, camera);
}
animate();
```

Rules:

- Never create new objects (geometries, materials, vectors) inside the render loop. Allocate outside and mutate in place: `vec.set(x, y, z)`.
- Update uniforms only when their value actually changes, not unconditionally every frame.
- For static scenes (e.g. a product viewer with `OrbitControls`), render on demand:

```javascript
controls.addEventListener('change', () => renderer.render(scene, camera));
// no continuous requestAnimationFrame loop needed
```

- Pause rendering when the tab is hidden:

```javascript
document.addEventListener('visibilitychange', () => {
  if (document.hidden) renderer.setAnimationLoop(null);
  else renderer.setAnimationLoop(animate);
});
```

- Animate vertex positions and particle movement on the GPU via shaders, not by mutating `BufferAttribute` data each frame.

---

## 5. React Three Fiber (R3F) Patterns

R3F maps Three.js constructors to JSX. Every Three.js class is available as a lowercase tag; `args` maps to the constructor arguments.

### Basic scene

```jsx
import { Canvas, useFrame } from '@react-three/fiber';
import { useRef } from 'react';

function Box() {
  const ref = useRef();
  useFrame((state, delta) => {
    ref.current.rotation.y += delta * 0.5;
  });
  return (
    <mesh ref={ref}>
      <boxGeometry args={[1, 1, 1]} />
      <meshStandardMaterial color="hotpink" />
    </mesh>
  );
}

export default function App() {
  return (
    <Canvas
      gl={{
        powerPreference: 'high-performance',
        antialias: false,   // disable when using post-processing
        stencil: false,
        depth: true,
      }}
      camera={{ fov: 75, near: 0.1, far: 100, position: [0, 1, 5] }}
      dpr={[1, 2]}          // responsive pixel ratio, capped at 2
    >
      <Box />
    </Canvas>
  );
}
```

### Key R3F rules

- `useFrame` runs inside the render loop — keep it lightweight. Avoid state updates inside `useFrame`; mutate refs directly.
- Use `invalidate()` from `useThree` to trigger a single frame render in demand-mode canvases.
- Access the renderer, scene, camera, and clock via `useThree()`:

```javascript
const { gl, scene, camera, size } = useThree();
```

- `frameloop="demand"` on `<Canvas>` disables the continuous loop; call `invalidate()` when you need a repaint.

### Drei helpers (most commonly used)

`@react-three/drei` provides ready-made abstractions:

```jsx
import {
  OrbitControls, Environment, useGLTF,
  useTexture, Html, Text, Billboard,
  Instances, Instance, useInstances,
  Lod, DetailLevels, Detail,
  ContactShadows, MeshReflectorMaterial,
  useProgress, Loader,
  Stats, Perf,
} from '@react-three/drei';
```

- `<OrbitControls />` — camera orbit, pan, zoom; add `makeDefault` to register as the default control.
- `<Environment preset="city" />` — loads an HDRI and sets scene background + env map in one line.
- `<Html>` — renders DOM elements anchored to a 3D position (overlays, labels).
- `<Text>` — GPU-rendered SDF text; no DOM overhead.
- `<Instances>` / `<Instance>` — R3F-idiomatic instancing.
- `<ContactShadows>` — cheap baked-style shadow on a plane beneath objects.
- `<Stats>` / `<Perf>` — FPS and draw call overlays for development.

---

## 6. GLTF / GLB Loading

### Vanilla Three.js

```javascript
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { DRACOLoader } from 'three/addons/loaders/DRACOLoader.js';

const draco = new DRACOLoader();
draco.setDecoderPath('/draco/'); // or CDN: https://www.gstatic.com/draco/versioned/decoders/1.5.7/

const loader = new GLTFLoader();
loader.setDRACOLoader(draco);

loader.load('/model.glb', (gltf) => {
  scene.add(gltf.scene);
}, undefined, (err) => console.error(err));
```

### R3F with Drei

```jsx
import { useGLTF } from '@react-three/drei';

function Model({ url }) {
  const { scene, animations } = useGLTF(url);
  return <primitive object={scene} />;
}

// Preload outside component to avoid waterfall
useGLTF.preload('/model.glb');
```

### Asset pipeline rules

- Export models as GLB (binary GLTF), not OBJ or COLLADA. GLB is compact, self-contained, and designed for the web.
- Apply Draco mesh compression at export time — reduces geometry to under 10% of original size.
- Use `gltfjsx` to convert a GLB into a JSX component with automatic splitting and typed access:

```bash
npx gltfjsx model.glb -S -T -t
# -S simplify, -T transform/optimize, -t typescript
```

- Dispose of loaded models when unmounting to prevent GPU memory leaks (see Section 8).

---

## 7. Performance: Instancing, LOD, Frustum Culling

### Instanced meshes

Use `InstancedMesh` when rendering hundreds to thousands of identical geometries. A single draw call replaces N draw calls.

```javascript
const geometry = new THREE.SphereGeometry(0.1, 8, 8);
const material = new THREE.MeshStandardMaterial({ color: 'white' });
const count = 10000;
const mesh = new THREE.InstancedMesh(geometry, material, count);

const dummy = new THREE.Object3D();
for (let i = 0; i < count; i++) {
  dummy.position.set(
    (Math.random() - 0.5) * 20,
    (Math.random() - 0.5) * 20,
    (Math.random() - 0.5) * 20
  );
  dummy.updateMatrix();
  mesh.setMatrixAt(i, dummy.matrix);
}
mesh.instanceMatrix.needsUpdate = true;
scene.add(mesh);
```

With R3F + Drei:

```jsx
<Instances limit={10000}>
  <sphereGeometry args={[0.1, 8, 8]} />
  <meshStandardMaterial color="white" />
  {positions.map((pos, i) => (
    <Instance key={i} position={pos} />
  ))}
</Instances>
```

### Level of Detail (LOD)

```javascript
const lod = new THREE.LOD();
lod.addLevel(highDetailMesh, 0);    // shown when camera < 10 units
lod.addLevel(medDetailMesh, 10);
lod.addLevel(lowDetailMesh, 30);
lod.addLevel(billboardMesh, 80);
scene.add(lod);

// in the render loop:
lod.update(camera);
```

### Frustum culling

Three.js performs frustum culling automatically per object. Maximize its effectiveness:

- Set tight `near`/`far` values on the camera.
- Use `object.frustumCulled = true` (default). Only disable it for objects you know are always visible.
- For large instanced meshes, frustum culling operates on the bounding sphere of the entire `InstancedMesh`. Consider spatial partitioning (BVH) for very large instance counts — the `three-mesh-bvh` library adds per-instance culling.

### Draw call budget

- Target under 100 draw calls per frame on mobile; 200–300 is manageable on desktop.
- Check the count with `renderer.info.render.calls` at runtime.
- Instancing, geometry merging, and material sharing are the primary levers.

### Static object optimization

```javascript
// Disable auto matrix recalculation for objects that never move
mesh.matrixAutoUpdate = false;
mesh.updateMatrix(); // call once after positioning
```

### Device pixel ratio

```javascript
// Cap at 2; never expose raw devicePixelRatio
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
```

For adaptive quality, monitor frame time and reduce DPR by 20% when under budget:

```javascript
if (frameTime > 20) renderer.setPixelRatio(renderer.getPixelRatio() * 0.8);
```

---

## 8. Memory Management and Dispose

Three.js does not garbage-collect GPU resources. Every geometry, material, and texture must be explicitly disposed.

```javascript
function disposeMesh(mesh) {
  mesh.geometry.dispose();

  const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
  for (const mat of materials) {
    // dispose all texture maps
    for (const key of Object.keys(mat)) {
      if (mat[key] && typeof mat[key].dispose === 'function') {
        mat[key].dispose();
      }
    }
    mat.dispose();
  }
}

function disposeScene(scene) {
  scene.traverse((child) => {
    if (child.isMesh) disposeMesh(child);
  });
}
```

### R3F: use cleanup in useEffect

```jsx
useEffect(() => {
  return () => {
    geometry.dispose();
    material.dispose();
    texture.dispose();
  };
}, []);
```

Rules:

- Do not remove and re-add objects repeatedly without disposing them.
- Textures are the largest GPU memory consumers — track them and dispose when the component unmounts.
- Use `renderer.info.memory` to monitor textures and geometries in development.

---

## 9. Custom Shaders (ShaderMaterial)

```javascript
const material = new THREE.ShaderMaterial({
  uniforms: {
    uTime: { value: 0 },
    uColor: { value: new THREE.Color('#ff6030') },
  },
  vertexShader: /* glsl */`
    uniform float uTime;
    varying vec2 vUv;

    void main() {
      vUv = uv;
      vec3 pos = position;
      pos.y += sin(pos.x * 4.0 + uTime) * 0.1;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(pos, 1.0);
    }
  `,
  fragmentShader: /* glsl */`
    uniform vec3 uColor;
    varying vec2 vUv;

    void main() {
      gl_FragColor = vec4(uColor * vUv.x, 1.0);
    }
  `,
});

// In the render loop — only update when value changes:
material.uniforms.uTime.value = clock.getElapsedTime();
```

### RawShaderMaterial

Use `RawShaderMaterial` when you need full control over precision qualifiers and do not want Three.js to inject its built-in uniforms. You must declare `precision`, `projectionMatrix`, `modelViewMatrix`, etc. manually.

### Shader tips

- Use `/* glsl */` template literal tag comment for syntax highlighting in VS Code with the glsl-literal extension.
- Prefer uniforms over recompiling shaders. Changing `defines` forces a recompile; changing `uniforms.value` does not.
- For per-instance variation in `InstancedMesh`, use `InstancedBufferAttribute` to pass per-instance data to the vertex shader.

---

## 10. Post-Processing

### Vanilla (three/addons EffectComposer)

```javascript
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { SMAAPass } from 'three/addons/postprocessing/SMAAPass.js';

const composer = new EffectComposer(renderer);
composer.addPass(new RenderPass(scene, camera));
composer.addPass(new UnrealBloomPass(
  new THREE.Vector2(window.innerWidth, window.innerHeight),
  1.5, 0.4, 0.85  // strength, radius, threshold
));
composer.addPass(new SMAAPass(window.innerWidth, window.innerHeight));

// In render loop — replace renderer.render():
composer.render();
```

### R3F: @react-three/postprocessing

```jsx
import { EffectComposer, Bloom, SMAA, Vignette } from '@react-three/postprocessing';

<EffectComposer>
  <Bloom luminanceThreshold={0.9} intensity={1.5} />
  <Vignette eskil={false} offset={0.1} darkness={1.1} />
  <SMAA />
</EffectComposer>
```

### Rules

- Disable the renderer's built-in antialias (`antialias: false`) when using post-processing — they conflict in WebGL 1.
- Use SMAA over FXAA: SMAA is faster and produces better results.
- Combine multiple simple passes into a single custom `ShaderPass` when possible to reduce full-screen blit count.
- Disable post-processing entirely on low-end devices (monitor frame time and adapt).

---

## 11. WebGPU Migration Path (Three.js r171+)

Since r171 (September 2025), Three.js ships a production-ready `WebGPURenderer` that falls back to WebGL 2 automatically. WebGPU is now supported in Chrome, Edge, Firefox, and Safari 26 (including iOS).

### Import swap

```javascript
// Before
import * as THREE from 'three';
const renderer = new THREE.WebGLRenderer({ antialias: true });

// After
import * as THREE from 'three/webgpu';
const renderer = new THREE.WebGPURenderer({ antialias: true });
await renderer.init(); // REQUIRED — WebGPU init is async; skipping this causes a blank canvas
renderer.setSize(window.innerWidth, window.innerHeight);
```

The `three/webgpu` entry point is a drop-in replacement for most scenes. Standard materials, loaders, and helpers work unchanged.

### TSL — Three Shader Language

TSL is a JavaScript-based shader authoring system that compiles to WGSL (WebGPU) and GLSL (WebGL) from a single source. Use it instead of raw GLSL or WGSL to keep one codebase that runs on both renderers.

```javascript
import { Fn, uv, vec4, uniform, sin, timerLocal } from 'three/tsl';
import * as THREE from 'three/webgpu';

const uTime = uniform(0); // replaces uniforms.uTime.value

const colorNode = Fn(() => {
  const t = sin(uTime.add(uv().x.mul(4.0)));
  return vec4(uv(), t, 1.0);
});

const material = new THREE.MeshBasicNodeMaterial();
material.colorNode = colorNode();

// Update in render loop:
uTime.value = performance.now() / 1000;
```

### Node materials

WebGPU materials use a node-based composition system. Standard class names become `*NodeMaterial`:

```javascript
// WebGL
new THREE.MeshStandardMaterial({ color: '#ff6030' })

// WebGPU (node material, direct property assignment)
const mat = new THREE.MeshStandardNodeMaterial();
mat.colorNode = vec4(1.0, 0.376, 0.188, 1.0);
```

### Compute shaders

WebGPU unlocks GPU-parallel compute — not possible in WebGL:

```javascript
import { ComputeNode, instanceIndex, storage } from 'three/tsl';
import * as THREE from 'three/webgpu';

// million-particle position update on GPU
const positions = storage(new THREE.StorageBufferAttribute(new Float32Array(count * 3), 3), 'vec3', count);

const computeUpdate = Fn(() => {
  const pos = positions.element(instanceIndex);
  pos.y = pos.y.add(0.01);
  positions.element(instanceIndex).assign(pos);
})().compute(count);

renderer.computeAsync(computeUpdate);
```

### Migration effort estimates

| Project type | Effort |
| --- | --- |
| Standard materials, no custom shaders | 1–2 hours |
| Custom GLSL shaders — convert to TSL | 1–2 days |
| Heavy post-processing pipeline | 1–2 weeks |

### When to migrate

Migrating is justified when you are hitting concrete performance walls: scene complexity beyond WebGL's draw call limits, needing compute shaders (particles, physics, fluid simulation), or requiring multi-threaded command encoding for very large scenes. If the application runs acceptably on WebGL, the migration overhead rarely pays off immediately.

---

## 12. Quick Reference Checklist

### Scene setup

- [ ] `outputColorSpace = THREE.SRGBColorSpace`
- [ ] `powerPreference: 'high-performance'`
- [ ] `setPixelRatio(Math.min(devicePixelRatio, 2))`
- [ ] Tight `near`/`far` on camera

#### Assets

- [ ] GLB with Draco compression
- [ ] Power-of-two textures
- [ ] Correct color space per texture type
- [ ] `gltfjsx` for R3F component generation

#### Performance

- [ ] `InstancedMesh` for repeated geometry
- [ ] `LOD` for distant objects
- [ ] `matrixAutoUpdate = false` for static meshes
- [ ] `renderer.info.render.calls` < 200 per frame
- [ ] `renderer.info.memory` monitored in dev

#### Memory

- [ ] `geometry.dispose()` on unmount
- [ ] `material.dispose()` on unmount
- [ ] `texture.dispose()` on unmount
- [ ] No object creation inside render loop

#### Post-processing

- [ ] `antialias: false` on renderer when using composer
- [ ] SMAA pass instead of built-in AA
- [ ] Post-processing disabled on low-end devices

#### WebGPU

- [ ] `await renderer.init()` before first render
- [ ] TSL for all custom shaders
- [ ] `three/webgpu` import for automatic fallback
