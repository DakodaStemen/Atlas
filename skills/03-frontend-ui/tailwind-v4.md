---
name: tailwind-v4
description: "Tailwind CSS v4 patterns, breaking changes from v3, CSS-first configuration with @theme, migration path, dark mode, performance, and new engine capabilities."
domain: frontend
category: css
tags: [Tailwind, CSS, v4, utility-first, CSS-variables, dark-mode, migration, "@theme", Oxide, PostCSS]
triggers: [tailwind v4, tailwindcss v4, tailwind css 4, "@theme directive", tailwind migration, tailwind upgrade, css-first config, tailwind CSS variables, tailwind dark mode v4, tailwind container queries]
---

# Tailwind CSS v4

## Overview

Tailwind v4 is a ground-up rewrite. The Oxide engine replaces the old JS-based pipeline with a Rust-powered CSS processor. The configuration model shifts from `tailwind.config.js` to CSS-first `@theme` blocks. The result: 3–10x faster full builds, up to 182x faster incremental rebuilds when no new classes appear, and a dramatically reduced installation surface.

Browser requirements shifted upward: Safari 16.4+, Chrome 111+, Firefox 128+ are required due to reliance on `@property`, `color-mix()`, and cascade layers. Stick with v3.4 for projects that must support older browsers.

---

## Installation

### Vite (preferred)

```bash
npm install tailwindcss @tailwindcss/vite
```

```typescript
// vite.config.ts
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
});
```

### PostCSS

```bash
npm install tailwindcss @tailwindcss/postcss
```

```javascript
// postcss.config.mjs
export default {
  plugins: {
    "@tailwindcss/postcss": {},
  },
};
```

### CLI

```bash
npm install @tailwindcss/cli
npx @tailwindcss/cli -i input.css -o output.css --watch
```

### CSS entry point

```css
/* All you need — no @tailwind directives */
@import "tailwindcss";
```

---

## CSS-First Configuration: @theme

The `tailwind.config.js` file is gone. All design tokens live in your CSS via `@theme`. A theme variable does two things simultaneously: it defines a CSS custom property on `:root` and instructs Tailwind to generate utility classes for it.

```css
@import "tailwindcss";

@theme {
  --font-display: "Satoshi", sans-serif;
  --breakpoint-3xl: 1920px;
  --color-brand-500: oklch(0.62 0.19 250);
  --color-brand-600: oklch(0.54 0.21 250);
  --ease-fluid: cubic-bezier(0.3, 0, 0, 1);
  --radius-card: 0.75rem;
}
```

This gives you `font-display`, `3xl:` breakpoint variant, `bg-brand-500`, `text-brand-600`, `ease-fluid`, and `rounded-card` automatically.

### Namespace → utility mapping

| Namespace | Utilities generated |
| --- | --- |
| `--color-*` | `bg-*`, `text-*`, `border-*`, `ring-*`, `shadow-*`, etc. |
| `--font-*` | `font-*` (font-family) |
| `--text-*` | `text-*` (font-size) |
| `--font-weight-*` | `font-*` (weight) |
| `--spacing-*` | `p-*`, `m-*`, `w-*`, `h-*`, `gap-*`, etc. |
| `--breakpoint-*` | Responsive variants (`sm:`, `md:`, custom) |
| `--radius-*` | `rounded-*` |
| `--shadow-*` | `shadow-*` |
| `--blur-*` | `blur-*` |
| `--animate-*` | `animate-*` |
| `--ease-*` | `ease-*` |
| `--perspective-*` | `perspective-*` |
| `--aspect-*` | `aspect-*` |

### Extending vs. overriding vs. resetting

```css
/* Extend: add to defaults */
@theme {
  --color-mint-400: oklch(0.78 0.11 170);
}

/* Override a single default */
@theme {
  --breakpoint-sm: 30rem;
}

/* Nuclear reset: wipe all defaults, define everything yourself */
@theme {
  --*: initial;

  --spacing: 4px;
  --color-primary: oklch(0.62 0.19 250);
  --color-neutral-900: oklch(0.15 0.01 250);
}
```

### @theme inline — for variable references

When a theme variable must point at another CSS variable (common with shadcn/ui, design-token systems), use `inline` so Tailwind embeds the reference rather than resolving it prematurely:

```css
@theme inline {
  --color-primary: var(--ds-color-primary);
  --font-sans: var(--font-inter);
}
```

Without `inline`, the variable resolves at the wrong cascade point and utility classes get the wrong value.

### @theme static — guarantee output of unused variables

By default v4 tree-shakes unused CSS variables. Use `static` when you need all variables in output (e.g., for runtime JS access):

```css
@theme static {
  --color-primary: oklch(0.62 0.19 250);
}
```

### Custom keyframes inside @theme

```css
@theme {
  --animate-fade-up: fade-up 0.4s ease-out;

  @keyframes fade-up {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }
}
```

---

## Content Detection

No `content` array needed. The Oxide engine scans your project automatically, skipping anything in `.gitignore` and binary files. Add explicit sources when scanning node_modules or external libraries:

```css
@import "tailwindcss";
@source "../node_modules/@acme/ui-lib";
```

---

## Breaking Changes from v3

### Import syntax

```css
/* v3 */
@tailwind base;
@tailwind components;
@tailwind utilities;

/* v4 */
@import "tailwindcss";
```

### Removed deprecated opacity utilities

```html
<!-- v3 -->
<div class="bg-black bg-opacity-50 text-white text-opacity-75">

<!-- v4 -->
<div class="bg-black/50 text-white/75">
```

### Renamed utilities (scale shift)

| v3 | v4 |
| --- | --- |
| `shadow-sm` | `shadow-xs` |
| `shadow` (default) | `shadow-sm` |
| `blur-sm` | `blur-xs` |
| `rounded-sm` | `rounded-xs` |
| `outline-none` | `outline-hidden` |
| `ring` (default 3px) | `ring-3` |
| `flex-shrink-*` | `shrink-*` |
| `flex-grow-*` | `grow-*` |
| `overflow-ellipsis` | `text-ellipsis` |

### Ring defaults changed

v3 `ring` produced a 3px blue ring. v4 `ring` produces a 1px `currentColor` ring.

```html
<!-- Preserve v3 ring behavior explicitly -->
<button class="focus:ring-3 focus:ring-blue-500">Submit</button>
```

Or via theme:

```css
@theme {
  --default-ring-width: 3px;
  --default-ring-color: var(--color-blue-500);
}
```

### Default border color

Changed from `gray-200` to `currentColor`. Add to base layer if needed:

```css
@layer base {
  *, ::after, ::before, ::backdrop, ::file-selector-button {
    border-color: var(--color-gray-200, currentColor);
  }
}
```

### space-*and divide-* selector change

v3 used `> :not([hidden]) ~ :not([hidden])`. v4 uses `:not(:last-child)` with margin on the opposite side. Visual output is mostly identical but differs in edge cases with hidden elements or flex reordering. Prefer `gap` for new code:

```html
<!-- prefer this in v4 -->
<div class="flex flex-col gap-4">...</div>
```

### Important modifier moved to suffix

```html
<!-- v3 -->
<div class="!flex !text-red-500">

<!-- v4 -->
<div class="flex! text-red-500!">
```

### Arbitrary CSS variables use parentheses

```html
<!-- v3 -->
<div class="bg-[--brand-color]">

<!-- v4 -->
<div class="bg-(--brand-color)">
```

### Variant stacking order reversed

```html
<!-- v3: right-to-left -->
<ul class="first:*:pt-0">

<!-- v4: left-to-right -->
<ul class="*:first:pt-0">
```

### Commas in arbitrary grid values

```html
<!-- v3 -->
<div class="grid-cols-[max-content,auto]">

<!-- v4: use underscore -->
<div class="grid-cols-[max-content_auto]">
```

### Outline shorthand changed

```html
<!-- v3 -->
<input class="outline outline-2">

<!-- v4: just the width -->
<input class="outline-2">
```

### Hover on touch devices

v4 wraps all `hover:` styles in `@media (hover: hover)`, so they no longer fire on mobile tap. Revert if needed:

```css
@custom-variant hover (&:hover);
```

### Button cursor default

Preflight changed `button` from `cursor: pointer` to `cursor: default`. Restore:

```css
@layer base {
  button:not(:disabled), [role="button"]:not(:disabled) {
    cursor: pointer;
  }
}
```

### Removed features

- `corePlugins` option — no equivalent, use CSS overrides
- `safelist` — use `@source` with glob or explicit class in source files
- `separator` option — the `-` separator is hardcoded
- `resolveConfig` JS API — use `getComputedStyle` at runtime instead:

  ```javascript
  const shadow = getComputedStyle(document.documentElement).getPropertyValue("--shadow-xl");
  ```

- CSS preprocessor support (Sass/Less/Stylus) — Tailwind is the preprocessor now
- `theme()` function in CSS — use CSS variables instead:

  ```css
  /* v3 */
  color: theme(colors.red.500);
  /* v4 */
  color: var(--color-red-500);
  ```

### Custom container config

```javascript
// v3 tailwind.config.js
container: { center: true, padding: "2rem" }
```

```css
/* v4 */
@utility container {
  margin-inline: auto;
  padding-inline: 2rem;
}
```

---

## Custom Utilities: @utility

Replace `@layer utilities` with `@utility` for user-defined utilities that participate in Tailwind's variant system (hover, responsive, dark, etc.):

```css
/* v3 */
@layer utilities {
  .tab-4 { tab-size: 4; }
}

/* v4 */
@utility tab-4 {
  tab-size: 4;
}
```

`@layer components` still works for component-level styles. Use `@utility` for anything that should compose with variants.

---

## Dark Mode

v4 ships with `@media (prefers-color-scheme: dark)` as the default. The class-based strategy requires one line:

```css
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));
```

Then use `dark:` as normal in markup:

```html
<div class="bg-white dark:bg-zinc-900 text-zinc-900 dark:text-zinc-100">
```

### Multi-theme with CSS variables

The idiomatic v4 approach for light/dark or multiple themes is to define semantic tokens and swap them per variant:

```css
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));

@layer base {
  :root {
    --color-background: var(--color-white);
    --color-foreground: var(--color-zinc-900);
    --color-surface: var(--color-zinc-100);
  }

  .dark {
    --color-background: var(--color-zinc-950);
    --color-foreground: var(--color-zinc-50);
    --color-surface: var(--color-zinc-900);
  }
}

@theme inline {
  --color-background: var(--color-background);
  --color-foreground: var(--color-foreground);
  --color-surface: var(--color-surface);
}
```

Classes `bg-background`, `text-foreground`, `bg-surface` now respond to the `.dark` class automatically.

---

## Container Queries (built-in)

Previously required `@tailwindcss/container-queries` plugin. Now first-class:

```html
<div class="@container">
  <div class="grid grid-cols-1 @sm:grid-cols-2 @lg:grid-cols-4">
    <!-- layout responds to parent width, not viewport -->
  </div>
</div>
```

Max-width queries:

```html
<div class="@container">
  <p class="text-lg @max-sm:text-base">Responsive to container</p>
</div>
```

Range queries:

```html
<div class="flex @min-md:@max-xl:hidden">...</div>
```

Named containers:

```html
<div class="@container/sidebar">
  <nav class="@lg/sidebar:block hidden">...</nav>
</div>
```

---

## New Capabilities in v4

### 3D transforms

```html
<div class="perspective-distant">
  <div class="rotate-x-12 rotate-z-6 transform-3d hover:rotate-x-0 transition-transform">
    Card content
  </div>
</div>
```

### Expanded gradients

```html
<!-- Angle-based linear -->
<div class="bg-linear-45 from-indigo-500 to-pink-500"></div>

<!-- Interpolation color space -->
<div class="bg-linear-to-r/oklch from-indigo-500 to-teal-400"></div>
<div class="bg-linear-to-r/srgb from-indigo-500 to-teal-400"></div>

<!-- Conic -->
<div class="bg-conic from-blue-500 to-purple-500"></div>

<!-- Radial with position -->
<div class="bg-radial-[at_top_left] from-white to-slate-900"></div>
```

### @starting-style — animate on mount

```html
<div
  popover
  id="toast"
  class="transition-all duration-300 starting:opacity-0 starting:translate-y-2"
>
  Notification content
</div>
```

No JavaScript needed to animate elements appearing for the first time.

### not-* variant

```html
<li class="opacity-50 not-last:border-b">Item</li>
<div class="not-hover:opacity-75">Hover me</div>
<div class="not-supports-grid:flex">Fallback</div>
```

### in-* variant (group without the class)

```html
<nav>
  <a class="in-[nav]:underline">Link</a>
</nav>
```

### nth-* variants

```html
<li class="nth-[2n+1]:bg-zinc-100">Odd item</li>
<li class="nth-last-3:font-bold">Third from end</li>
```

### field-sizing

```html
<textarea class="field-sizing-content">Grows as user types</textarea>
```

### color-scheme

```html
<html class="color-scheme-dark">
  <!-- Dark scrollbars, form controls, date pickers -->
</html>
```

### inset-shadow and inset-ring

```html
<div class="shadow-md inset-shadow-sm ring-1 inset-ring-2">
  Up to 4 box-shadows layered
</div>
```

---

## @apply and @reference

`@apply` works in v4 but use it sparingly — it couples CSS to Tailwind's internals and makes output less predictable. Prefer composing in markup or extracting components at the framework level.

When `@apply` is genuinely appropriate (Vue `<style>`, Svelte `<style>`, CSS Modules), use `@reference` to pull in theme context without emitting duplicate CSS:

```vue
<style>
  @reference "../../app.css";

  h1 {
    @apply text-2xl font-bold text-zinc-900 dark:text-zinc-50;
  }
</style>
```

Or skip `@apply` entirely and reference CSS variables directly:

```css
h1 {
  font-size: var(--text-2xl);
  color: var(--color-zinc-900);
}
```

---

## Keeping a JS Config

If you need to migrate incrementally or have plugins that require a config file, reference it explicitly:

```css
@import "tailwindcss";
@config "../../tailwind.config.js";
```

Unsupported config options in v4: `corePlugins`, `safelist`, `separator`. Everything else works until you migrate.

---

## Prefix

Prefixes now look like variant namespaces and must come first:

```css
@import "tailwindcss" prefix(tw);
```

```html
<div class="tw:flex tw:bg-zinc-900 tw:hover:bg-zinc-800">
```

---

## Migration Path

### Automated upgrade (recommended first step)

```bash
npx @tailwindcss/upgrade
```

Requires Node 20+. Run on a clean branch. The codemod handles:

- Converts `@tailwind` directives to `@import "tailwindcss"`
- Updates PostCSS config to `@tailwindcss/postcss`
- Renames deprecated classes (`flex-shrink` → `shrink`, `bg-opacity-*` → `/` syntax, etc.)
- Migrates basic `tailwind.config.js` theme extensions to `@theme` blocks

### Manual follow-up checklist

After the codemod, audit these areas manually:

1. **Dynamic class construction** — string concatenation like `"bg-" + color` won't be detected; grep and fix.
2. **Plugin refactoring** — `addUtilities` calls become `@utility` in CSS; `matchUtilities` needs evaluation.
3. **Renamed scale classes** — verify shadow, blur, rounded, ring usage against the rename table above.
4. **Ring and border color defaults** — add explicit `border-gray-200` and `ring-blue-500` where behavior must match v3.
5. **`space-*` / `divide-*`** — test with hidden children and flex reordering; migrate to `gap` where possible.
6. **Hover on mobile** — intentional v4 behavior; revert per component if needed.
7. **PostCSS preprocessors** — Sass/Less must be removed; Tailwind handles nesting and variables natively.
8. **Browser targets** — verify your support matrix permits Safari 16.4+.
9. **`resolveConfig` usage** — replace with `getComputedStyle` reads against CSS variables.
10. **`@apply` in scoped styles** — add `@reference` to each scoped style block.

---

## Performance Notes

The engine improvements that matter most in practice:

- **Incremental builds with no new classes** complete in ~192µs vs 35ms in v3 — effectively instant hot reload.
- `@property` registration allows CSS transitions on gradient stops without JavaScript.
- `color-mix()` handles opacity at the CSS engine level, replacing the old opacity hack that required duplicate variable declarations.
- Lightning CSS handles vendor prefixing in the same pass, eliminating the need for `autoprefixer` as a separate PostCSS plugin.
- The `--spacing` multiplier (`0.25rem` base) generates all spacing utilities via `calc()` rather than emitting hundreds of static declarations, significantly reducing CSS output size.
