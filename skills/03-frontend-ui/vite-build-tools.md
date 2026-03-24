---
name: vite-build-tools
description: Modern JavaScript build tooling — Vite (dev server + Rollup build), esbuild, SWC, Rollup config, plugin authoring, and bundler optimization.
domain: frontend
category: build-tools
tags: [Vite, esbuild, Rollup, SWC, bundler, build-tools, HMR, code-splitting, tree-shaking, TypeScript]
triggers: Vite config, Vite plugin, vite.config.ts, esbuild, Rollup config, SWC transpile, HMR, code splitting Vite, Rollup plugin, build optimization Vite
---

# Modern JavaScript Build Tools

## Vite: Why and When

Vite was created to solve the dev-server startup wall that webpack and CRA hit as codebases grow. Traditional bundlers had to process every module upfront before the browser could show anything. Vite skips that.

**Dev server model:** Vite splits modules into two buckets:

- **Dependencies** (node_modules) — pre-bundled once at startup using esbuild (Go, ~100x faster than JS bundlers). Result is cached and served as ESM.
- **Source code** — served raw via native ESM. The browser requests only what it needs; Vite transforms on demand.

This means cold-start time is near-instant regardless of app size.

**HMR model:** Changes propagate only the invalidated module graph, not a full rebundle. Because it's native ESM, there's no serialization overhead.

**vs webpack:** Webpack bundles everything before serving. Huge config surface, slow cold starts on large apps, requires loaders for every file type. Vite is zero-config for common setups, uses native browser features, and is orders of magnitude faster in dev.

**vs CRA (Create React App):** CRA was webpack under a thin abstraction, slow, and effectively abandoned. Vite replaced it as the community default. `npm create vite@latest` is the current starting point.

**When NOT to use Vite:** If you need webpack's mature ecosystem for unusual loaders/legacy integration, or you're in a micro-frontend setup with Module Federation (Vite has a plugin but it's not as mature). For production-grade SSR with complex routing, Next.js (which manages its own build layer) may be a better fit.

**Production build:** Vite uses Rollup (or Rolldown — the Rust rewrite) to produce optimized static assets. The dev/prod split is intentional: unbundled ESM in dev, tree-shaken and chunked bundle in prod.

---

## vite.config.ts

```typescript
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react-swc'
import tsconfigPaths from 'vite-tsconfig-paths'
import { resolve } from 'path'

export default defineConfig(({ command, mode }) => {
  // .env files are NOT auto-injected into process.env inside vite.config
  const env = loadEnv(mode, process.cwd(), '')

  return {
    plugins: [
      react(),
      tsconfigPaths(),
    ],

    resolve: {
      alias: {
        '@': resolve(__dirname, 'src'),
        '@components': resolve(__dirname, 'src/components'),
      },
    },

    server: {
      port: 3000,
      host: true,           // expose on LAN (0.0.0.0)
      open: false,
      https: false,
      proxy: {
        '/api': {
          target: env.VITE_API_URL || 'http://localhost:8080',
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api/, ''),
        },
      },
      hmr: {
        overlay: true,      // show error overlay on HMR failures
      },
    },

    build: {
      target: 'es2020',
      outDir: 'dist',
      sourcemap: mode === 'development',
      minify: 'oxc',        // default in recent Vite; 'esbuild' or 'terser' also valid
      cssMinify: 'lightningcss', // default; 'esbuild' also works
      rollupOptions: {
        output: {
          manualChunks: {
            vendor: ['react', 'react-dom'],
            router: ['react-router-dom'],
          },
        },
      },
    },

    define: {
      __APP_VERSION__: JSON.stringify(process.env.npm_package_version),
    },
  }
})
```

The `command` parameter is `'serve'` during dev and `'build'` during production — use it for conditional logic (e.g., enabling devtools only in dev).

---

## Vite: Dev Server

Key server config options:

| Option | Default | Notes |
| --- | --- | --- |
| `server.port` | `5173` | Errors if port is taken unless `strictPort: false` |
| `server.host` | `'localhost'` | Set `true` or `'0.0.0.0'` to expose on network |
| `server.https` | `false` | Pass `{ key, cert }` object or use `@vitejs/plugin-basic-ssl` |
| `server.proxy` | — | Per-path proxy rules; supports ws for WebSocket proxying |
| `server.hmr` | `true` | `{ port, host, overlay, protocol }` |
| `server.open` | `false` | Auto-open browser on start |
| `server.fs.allow` | project root | Whitelist directories accessible via `/@fs/` |

### WebSocket proxy example (for socket.io)

```typescript
server: {
  proxy: {
    '/socket.io': {
      target: 'ws://localhost:3001',
      ws: true,
    },
  },
},
```

#### HTTPS with custom cert

```typescript
import fs from 'fs'
server: {
  https: {
    key: fs.readFileSync('./certs/key.pem'),
    cert: fs.readFileSync('./certs/cert.pem'),
  },
},
```

HMR preserves component state across edits when using React Fast Refresh (`@vitejs/plugin-react` or `-swc` variant). Full reload only happens when the module has no accepted HMR boundary (e.g., edited file is not a component).

---

## Vite: Build (Production)

Vite's build pipeline is Rollup (or Rolldown in newer versions). The entry point is `index.html` by default.

```typescript
build: {
  target: ['es2020', 'chrome111', 'firefox114', 'safari16.4'],
  outDir: 'dist',
  assetsDir: 'assets',
  assetsInlineLimit: 4096,  // files < 4KB inlined as base64
  sourcemap: false,
  minify: 'oxc',            // oxc (default, fastest) | esbuild | terser | false
  cssMinify: 'lightningcss',
  cssCodeSplit: true,
  chunkSizeWarningLimit: 500, // kB

  rollupOptions: {
    input: {
      main: resolve(__dirname, 'index.html'),
      // admin: resolve(__dirname, 'admin/index.html'), // multi-page
    },
    output: {
      entryFileNames: 'assets/[name]-[hash].js',
      chunkFileNames: 'assets/[name]-[hash].js',
      assetFileNames: 'assets/[name]-[hash][extname]',
    },
    external: ['some-peer-dep'], // exclude from bundle
  },
},
```

**Browser targets:** Vite defaults to supporting "Baseline Widely Available" browsers (Chrome 111+, Firefox 114+, Safari 16.4+). Use `@vitejs/plugin-legacy` for IE11/older browser support — it generates a second bundle with `<script nomodule>` fallback.

**Load error handling** for stale assets after deploys:

```typescript
window.addEventListener('vite:preloadError', () => {
  window.location.reload()
})
```

---

## Vite: Code Splitting

Vite / Rollup splits code automatically at dynamic import boundaries. Manual control via `manualChunks`:

```typescript
build: {
  rollupOptions: {
    output: {
      manualChunks(id) {
        // All of node_modules → vendor chunk
        if (id.includes('node_modules')) {
          return 'vendor'
        }
        // Group specific packages
        if (id.includes('react') || id.includes('react-dom')) {
          return 'react-vendor'
        }
        if (id.includes('@radix-ui')) {
          return 'radix'
        }
      },
    },
  },
},
```

**Dynamic imports** create split points automatically:

```typescript
// Route-level code splitting
const Dashboard = React.lazy(() => import('./pages/Dashboard'))

// Manual async load
const module = await import('./heavy-util')
```

**Gotcha:** `manualChunks` as an object (static map) is simpler but less flexible — circular dependencies between chunks can cause runtime errors. The function form handles this better because you can inspect the full module ID.

**Lib mode** for library output:

```typescript
build: {
  lib: {
    entry: resolve(__dirname, 'src/index.ts'),
    name: 'MyLib',           // UMD global name
    fileName: 'my-lib',      // output: my-lib.es.js, my-lib.umd.js
    formats: ['es', 'cjs', 'umd'],
  },
  rollupOptions: {
    external: ['react', 'react-dom'],
    output: {
      globals: {
        react: 'React',
        'react-dom': 'ReactDOM',
      },
    },
  },
},
```

---

## Vite: Plugins

Plugins go in the `plugins` array in `vite.config.ts`. Order matters — use `enforce: 'pre'` or `'post'` when a plugin needs to run before/after core transforms.

### Official / common plugins

```typescript
// React with Babel (slower, needed for custom Babel plugins)
import react from '@vitejs/plugin-react'

// React with SWC (faster, recommended default)
import react from '@vitejs/plugin-react-swc'

// Vue 3
import vue from '@vitejs/plugin-vue'

// Vue JSX
import vueJsx from '@vitejs/plugin-vue-jsx'

// Legacy browser support
import legacy from '@vitejs/plugin-legacy'
```

#### Popular ecosystem plugins

```typescript
import { VitePWA } from 'vite-plugin-pwa'
import tsconfigPaths from 'vite-tsconfig-paths'
import checker from 'vite-plugin-checker'   // TypeScript/ESLint checking in dev
import { visualizer } from 'rollup-plugin-visualizer' // bundle analysis

plugins: [
  react(),
  tsconfigPaths(),
  checker({ typescript: true }),
  VitePWA({
    registerType: 'autoUpdate',
    workbox: { globPatterns: ['**/*.{js,css,html,ico,png,svg}'] },
  }),
  visualizer({ open: true }),  // opens stats.html after build
]
```

---

## Vite: Plugin Authoring

A Vite plugin is a factory function returning an object. It extends the Rollup/Rolldown plugin interface with Vite-specific hooks.

```typescript
import type { Plugin } from 'vite'

export function myPlugin(options: { prefix: string }): Plugin {
  return {
    name: 'vite-plugin-my',   // required; used in error messages and ordering
    enforce: 'pre',           // 'pre' | 'post' | undefined
    apply: 'build',           // 'serve' | 'build' | function

    // Modify raw Vite config before resolution
    config(config, { command }) {
      if (command === 'build') {
        return { build: { sourcemap: true } }
      }
    },

    // Store resolved config for later hooks
    configResolved(resolvedConfig) {
      // resolvedConfig is the final merged config
    },

    // Custom module resolution
    resolveId(id, importer) {
      if (id === 'virtual:my-module') {
        return '\0virtual:my-module'  // \0 prefix hides from other plugins
      }
    },

    // Load module source
    load(id) {
      if (id === '\0virtual:my-module') {
        return `export const prefix = "${options.prefix}"`
      }
    },

    // Transform source code
    transform(code, id) {
      if (!id.endsWith('.ts')) return
      // Return transformed code + optional source map
      return {
        code: code.replace(/DEBUG_/g, options.prefix),
        map: null,
      }
    },

    // Access the dev server instance
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url === '/__my-route__') {
          res.end('hello from plugin middleware')
          return
        }
        next()
      })
      // Return a function to run middleware AFTER internal middlewares
      return () => {
        server.middlewares.use(/* post middleware */)
      }
    },

    // Transform index.html
    transformIndexHtml(html) {
      return html.replace(/<title>/, '<title>Prefixed: ')
    },

    buildStart() { /* called once at build start */ },
    buildEnd()   { /* called when build completes or errors */ },
    closeBundle() { /* after all output written; good for cleanup */ },
  }
}
```

**Virtual modules pattern** — expose runtime data from config:

```typescript
resolveId(id) {
  if (id === 'virtual:routes') return '\0virtual:routes'
},
load(id) {
  if (id === '\0virtual:routes') {
    const routes = scanRoutes()
    return `export default ${JSON.stringify(routes)}`
  }
},
```

**Path normalization** — on Windows, `id` may use backslashes. Always normalize:

```typescript
import { normalizePath } from 'vite'
const normalId = normalizePath(id)
```

---

## Vite: Environment Variables

Vite exposes env vars at `import.meta.env` (statically replaced at build time).

### Rules

- Only vars prefixed with `VITE_` are exposed to client code.
- `import.meta.env.MODE` — `'development'` | `'production'` | custom mode
- `import.meta.env.DEV` — boolean
- `import.meta.env.PROD` — boolean
- `import.meta.env.BASE_URL` — the configured `base` path
- `import.meta.env.SSR` — boolean; true when running in SSR context

**.env file precedence** (highest to lowest):

```text
.env.[mode].local   ← gitignored, machine-specific
.env.[mode]         ← committed, mode-specific
.env.local          ← gitignored, all modes
.env                ← committed, all modes baseline
```

```dotenv
# .env
VITE_API_URL=https://api.example.com
VITE_FEATURE_FLAG=true
SECRET_KEY=not-exposed   # no VITE_ prefix → server-side only
```

```typescript
// Client code
const apiUrl = import.meta.env.VITE_API_URL  // string
const isDev  = import.meta.env.DEV           // boolean
```

**`define`** injects global constants (not scoped to VITE_ prefix, works in all files including non-module scripts):

```typescript
define: {
  __APP_VERSION__: JSON.stringify('1.2.3'),
  __FEATURE_X__: true,
}
```

**Inside vite.config.ts**, env files are NOT auto-loaded. Use `loadEnv`:

```typescript
import { defineConfig, loadEnv } from 'vite'
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '') // '' = load all prefixes
  return {
    server: { port: Number(env.PORT) || 3000 },
  }
})
```

#### TypeScript types for custom env vars

```typescript
// src/vite-env.d.ts
/// <reference types="vite/client" />
interface ImportMetaEnv {
  readonly VITE_API_URL: string
  readonly VITE_FEATURE_FLAG: string
}
interface ImportMeta {
  readonly env: ImportMetaEnv
}
```

---

## esbuild

Go-based bundler/transformer. The fastest JS tooling in existence (10–100x faster than webpack/Babel for the same operations). Used inside Vite for dependency pre-bundling and optionally for production minification.

### Two modes

**Transform API** — single-file transpilation, no bundling:

```typescript
import { transform } from 'esbuild'

const result = await transform(`const x: number = 1`, {
  loader: 'ts',
  target: 'es2020',
  minify: false,
})
console.log(result.code) // 'const x = 1;\n'
```

**Build API** — full bundler:

```typescript
import { build } from 'esbuild'

await build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  outfile: 'dist/bundle.js',
  platform: 'browser',     // 'browser' | 'node' | 'neutral'
  target: ['chrome111', 'firefox114'],
  format: 'esm',           // 'esm' | 'cjs' | 'iife'
  minify: true,
  sourcemap: true,
  splitting: true,         // code splitting (ESM only)
  outdir: 'dist',          // required when splitting = true
  loader: {
    '.png': 'file',
    '.svg': 'dataurl',
    '.txt': 'text',
  },
  define: {
    'process.env.NODE_ENV': '"production"',
  },
  external: ['react', 'react-dom'],
  treeShaking: true,
})
```

**Loaders:** `js`, `jsx`, `ts`, `tsx`, `css`, `json`, `text`, `base64`, `file`, `dataurl`, `binary`, `copy`, `empty`.

**Minification flags:** `minify: true` is shorthand for `minifyWhitespace + minifyIdentifiers + minifySyntax` all true. Can set individually.

#### esbuild does NOT do

- Type checking (use `tsc --noEmit` separately)
- Decorator metadata (without plugins)
- CSS modules (without plugins)

---

## esbuild: Plugins

esbuild plugins intercept module resolution and loading. They run in Node.js (not the browser).

```typescript
import { build, Plugin } from 'esbuild'

const svgPlugin: Plugin = {
  name: 'svg-loader',
  setup(build) {
    // Intercept .svg imports
    build.onLoad({ filter: /\.svg$/ }, async (args) => {
      const fs = await import('fs/promises')
      const svg = await fs.readFile(args.path, 'utf8')
      const escaped = JSON.stringify(svg)
      return {
        contents: `export default ${escaped}`,
        loader: 'js',
      }
    })
  },
}

// Virtual module via namespace
const virtualPlugin: Plugin = {
  name: 'virtual',
  setup(build) {
    // Mark virtual: imports as belonging to this namespace
    build.onResolve({ filter: /^virtual:/ }, (args) => ({
      path: args.path,
      namespace: 'virtual-ns',
    }))

    build.onLoad({ filter: /.*/, namespace: 'virtual-ns' }, (args) => ({
      contents: `export const name = "${args.path}"`,
      loader: 'js',
    }))
  },
}

await build({ plugins: [svgPlugin, virtualPlugin], /* ... */ })
```

### `onResolve` and `onLoad` hooks

- `onResolve` — intercept module path resolution; return `{ path, namespace, external }`
- `onLoad` — load file contents; return `{ contents, loader, resolveDir }`
- Both accept `{ filter: RegExp, namespace?: string }` as selector

---

## SWC

Rust-based Babel replacement. Near-native speed for JS/TS transpilation. Default in Next.js since v12. Used in Vite via `@vitejs/plugin-react-swc`.

### @swc/core Node.js API

```typescript
import { transform, transformFile } from '@swc/core'

const result = await transform(`
  const greet = (name: string) => \`Hello, \${name}!\`
`, {
  jsc: {
    parser: { syntax: 'typescript' },
    target: 'es2020',
    transform: {
      react: {
        runtime: 'automatic',
        development: false,
        refresh: false,
      },
    },
  },
  module: { type: 'es6' },
  sourceMaps: true,
})
```

#### .swcrc configuration

```json
{
  "$schema": "https://json.schemastore.org/swcrc",
  "jsc": {
    "parser": {
      "syntax": "typescript",
      "tsx": true,
      "decorators": true
    },
    "target": "es2020",
    "loose": false,
    "minify": {
      "compress": true,
      "mangle": true
    },
    "transform": {
      "react": {
        "runtime": "automatic",
        "refresh": true
      },
      "legacyDecorator": true,
      "decoratorMetadata": true
    },
    "externalHelpers": true
  },
  "module": {
    "type": "es6",
    "strict": false,
    "noInterop": false
  },
  "env": {
    "targets": "chrome >= 111, firefox >= 114"
  },
  "sourceMaps": true
}
```

**SWC does NOT do type checking** — it strips types without validating them. Run `tsc --noEmit` in CI separately.

#### In Vite

```typescript
// vite.config.ts
import react from '@vitejs/plugin-react-swc'

export default defineConfig({
  plugins: [react()],  // replaces @vitejs/plugin-react (Babel)
})
```

Use `@vitejs/plugin-react` (Babel) only if you need custom Babel plugins (e.g., `babel-plugin-styled-components`, `babel-plugin-macros`). For everything else, SWC is faster with no downside.

---

## Rollup

Purpose-built library bundler. Excellent tree-shaking, clean ESM output, minimal runtime overhead. Best choice for publishing packages to npm.

### Full config example (`rollup.config.js`)

```javascript
import { defineConfig } from 'rollup'
import typescript from '@rollup/plugin-typescript'
import resolve from '@rollup/plugin-node-resolve'
import commonjs from '@rollup/plugin-commonjs'
import terser from '@rollup/plugin-terser'
import dts from 'rollup-plugin-dts'

export default defineConfig([
  // Main bundle
  {
    input: 'src/index.ts',
    external: ['react', 'react-dom', /^react\//],
    plugins: [
      resolve({ browser: true }),
      commonjs(),
      typescript({ tsconfig: './tsconfig.json' }),
      terser(),
    ],
    output: [
      {
        file: 'dist/index.cjs',
        format: 'cjs',
        sourcemap: true,
        exports: 'named',
      },
      {
        file: 'dist/index.mjs',
        format: 'es',
        sourcemap: true,
      },
    ],
    treeshake: {
      moduleSideEffects: false,  // assume no side effects → more aggressive pruning
      propertyReadSideEffects: false,
    },
  },
  // Type declarations
  {
    input: 'src/index.ts',
    plugins: [dts()],
    output: { file: 'dist/index.d.ts', format: 'es' },
  },
])
```

#### Key Rollup concepts

- `external` — mark deps as peer deps; they won't be bundled. Accepts array of strings or a function `(id) => boolean`.
- `treeshake` — Rollup has the most precise tree-shaking of any bundler because it operates on ES module static imports.
- Multiple output formats — generate CJS + ESM + UMD from one build pass.
- `preserveModules: true` — output one file per input module (useful for component libraries that want per-component tree-shaking without a bundler).

#### Essential Rollup plugins

- `@rollup/plugin-node-resolve` — resolve npm packages from node_modules
- `@rollup/plugin-commonjs` — convert CJS dependencies to ESM for bundling
- `@rollup/plugin-typescript` — TypeScript compilation via tsc API
- `@rollup/plugin-alias` — path aliases
- `@rollup/plugin-replace` — string replacement (env vars)
- `rollup-plugin-dts` — bundle `.d.ts` files into one declaration file
- `@rollup/plugin-terser` — minification

---

## Turbopack

Rust-based webpack successor built by Vercel. Default bundler in Next.js 15 for development.

### Enable in Next.js

```bash
# package.json dev script
next dev --turbopack
```

Or in `next.config.ts`:

```typescript
const nextConfig = {
  turbopack: {
    resolveAlias: {
      '@': './src',
    },
    resolveExtensions: ['.tsx', '.ts', '.jsx', '.js'],
    rules: {
      '*.svg': {
        loaders: ['@svgr/webpack'],
        as: '*.js',
      },
    },
  },
}
```

**Current status (2025):** Turbopack dev is stable in Next.js 15. Production builds (`next build --turbopack`) are in beta. It is not a general-purpose bundler — it's Next.js-specific. Do not use it outside Next.js.

**Performance:** Benchmarks show 10x faster cold start and 96% faster Fast Refresh than webpack on large Next.js apps. Incrementally compiles only changed modules.

**Plugin system:** Turbopack does not have a general plugin API yet. It supports webpack loaders via the `rules` config (compatibility shim).

---

## Critical Rules / Gotchas

### CommonJS interop

Vite serves native ESM in dev. If a dependency uses `require()` or `module.exports`, Vite pre-bundles it with esbuild to convert it to ESM. If a CJS dep is not pre-bundled (e.g., excluded via `optimizeDeps.exclude`), you'll get "require is not defined" errors at runtime. Add misbehaving packages to `optimizeDeps.include`.

```typescript
optimizeDeps: {
  include: ['some-cjs-package'],
  exclude: ['@my/local-package'],
},
```

#### SSR mode

In SSR, modules run in Node.js, not the browser. Vite switches to a different module graph. Use `import.meta.env.SSR` to guard browser-only code. Never import browser globals (window, document) at module top level without SSR guard.

```typescript
if (!import.meta.env.SSR) {
  // safe to use browser APIs here
}
```

#### Top-level await

Supported in Vite with `target: 'esnext'` or `target: 'es2022'+`. With older targets, top-level await is downcompiled and may cause issues with code-splitting. Avoid it in shared utility modules if you need broad browser support.

#### Dynamic imports with variables

Rollup cannot statically analyze `import(variable)`. Use template literals with a static prefix:

```typescript
// BAD — Rollup can't split this
const mod = await import(modulePath)

// GOOD — Rollup sees the pattern and can chunk correctly
const mod = await import(`./pages/${pageName}.tsx`)
```

#### JSON imports

Vite supports `import data from './data.json'` natively. Rollup needs `@rollup/plugin-json`.

#### CSS in lib mode

When building a library with `build.lib`, CSS is extracted to a separate file. Consumers must import it manually. Alternatively, inject styles via JS with a plugin or document this explicitly in your library's README.

#### `define` vs `import.meta.env`

`define` does raw string replacement at build time (think C preprocessor). The value must be a valid JS expression. `import.meta.env` is the typed, prefix-filtered layer on top. Prefer `import.meta.env.VITE_*` for app code; use `define` only for low-level constants or cross-framework compatibility.

#### Vite config file is not processed by the app's build pipeline

You can use TypeScript in `vite.config.ts`, but it runs in Node.js via Vite's own bundler (Rolldown by default). Do not import browser-only code or use browser APIs in config files.

#### esbuild does not emit type declarations

esbuild strips TypeScript types silently. For libraries, use `tsc --emitDeclarationOnly` or Rollup + `rollup-plugin-dts` to generate `.d.ts` files.

#### Rollup `external` with scoped packages

String matching in `external` is exact. Use a regex for org-scoped packages:

```javascript
external: [/^@my-org\//]  // excludes all @my-org/* packages
```

---

## References

- Vite docs: <https://vite.dev/guide/>
- Vite build options: <https://vite.dev/config/build-options>
- Vite plugin API: <https://vite.dev/guide/api-plugin>
- esbuild API: <https://esbuild.github.io/api/>
- esbuild plugins: <https://esbuild.github.io/plugins/>
- Rollup docs: <https://rollupjs.org/introduction/>
- SWC docs: <https://swc.rs/docs/configuration/swcrc>
- @vitejs/plugin-react-swc: <https://github.com/vitejs/vite-plugin-react-swc>
- Turbopack docs: <https://turbo.build/pack/docs>
- rollup-plugin-visualizer: <https://github.com/btd/rollup-plugin-visualizer>
- vite-plugin-pwa: <https://vite-pwa-org.netlify.app/>
