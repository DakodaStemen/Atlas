---
name: modern-css-patterns
description: "Modern CSS — Grid, Flexbox, container queries, cascade layers, CSS custom properties, logical properties, and :has() selector patterns."
domain: frontend
category: css
tags: [CSS, Grid, Flexbox, container-queries, cascade-layers, custom-properties, ":has", logical-properties, modern-CSS]
triggers: "CSS Grid, CSS Flexbox, container queries CSS, @layer cascade, CSS custom properties, CSS variables, :has selector, logical properties CSS, subgrid, CSS nesting"
---

# Modern CSS Patterns

## Grid Layout

CSS Grid is a two-dimensional layout system. Define structure on the container; items fill it.

```css
/* Named template areas */
.layout {
  display: grid;
  grid-template-columns: 200px 1fr;
  grid-template-rows: auto 1fr auto;
  grid-template-areas:
    "sidebar header"
    "sidebar main"
    "sidebar footer";
  gap: 1rem;
}

.header  { grid-area: header; }
.sidebar { grid-area: sidebar; }
.main    { grid-area: main; }
.footer  { grid-area: footer; }
```

```css
/* Named lines */
.grid {
  grid-template-columns: [content-start] 1fr [content-end sidebar-start] 250px [sidebar-end];
}
.item { grid-column: content-start / content-end; }
```

```css
/* Auto-placement: fill rows first (default), then columns */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  grid-auto-rows: minmax(100px, auto);
  gap: 1rem;
}
```

```css
/* minmax and fit-content */
.grid {
  grid-template-columns:
    minmax(100px, 300px)          /* min 100px, max 300px */
    fit-content(200px)            /* shrinks to content, caps at 200px */
    1fr;
}
```

### auto-fill vs auto-fit

- `auto-fill`: creates as many tracks as fit, including empty ones.
- `auto-fit`: collapses empty tracks to 0, letting filled tracks stretch.

```css
/* auto-fill: preserves empty columns */
grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));

/* auto-fit: expands items to fill available space */
grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
```

---

## Subgrid

Nested grid elements can inherit their parent's track sizes using `subgrid`. Baseline since September 2023.

```css
.parent-grid {
  display: grid;
  grid-template-columns: repeat(9, 1fr);
  grid-template-rows: repeat(4, minmax(100px, auto));
  gap: 20px;
}

.card {
  display: grid;
  grid-column: 2 / 7;   /* spans 5 parent columns */
  grid-row: 2 / 4;       /* spans 2 parent rows */
  grid-template-columns: subgrid;  /* inherits the 5 column tracks */
  grid-template-rows: subgrid;     /* inherits the 2 row tracks */
}

.card-content {
  grid-column: 3 / 6;
  grid-row: 1 / 3;
}
```

```css
/* Named lines on subgrid — add custom names, inherit parent named lines too */
.card {
  grid-template-columns: subgrid [card-start] [card-mid] [card-end];
}
```

Gap is inherited from the parent; override it on the subgrid if needed:

```css
.card {
  grid-template-columns: subgrid;
  row-gap: 0;  /* override inherited gap */
}
```

**No implicit grid in subgridded dimensions** — extra items overflow into the last track rather than creating new tracks.

---

## Flexbox Patterns

Flexbox is one-dimensional. Use it for component-level layout or distributing items along a single axis.

```css
/* gap replaces margin hacks */
.flex-row {
  display: flex;
  gap: 1rem 2rem; /* row-gap column-gap */
  flex-wrap: wrap;
}
```

```css
/* flex-basis vs width
   flex-basis is the hypothetical size before flex-grow/shrink are applied.
   Use flex-basis for flex items, width for non-flex contexts. */
.item {
  flex: 1 1 200px;  /* grow shrink basis — shorthand */
}
```

```css
/* min-width: 0 gotcha
   Flex items have min-width: auto by default, which prevents shrinking
   below their content size. Override explicitly when truncation is needed. */
.truncate-item {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

```css
/* align-content vs align-items
   align-items: aligns items within each line.
   align-content: aligns lines within the container (only has effect with flex-wrap). */
.container {
  display: flex;
  flex-wrap: wrap;
  align-items: center;    /* items within each row */
  align-content: start;  /* rows relative to container */
}
```

```css
/* Holy grail with flex */
.page {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}
.main { flex: 1; }  /* takes remaining height */
```

---

## Container Queries

Query an element's own container size instead of the viewport. Baseline 2023.

```css
/* Establish a containment context */
.card-wrapper {
  container-type: inline-size;   /* query inline size (width in LTR) */
  container-name: card;          /* optional: name it for targeting */
}

/* Shorthand */
.card-wrapper {
  container: card / inline-size;
}
```

```css
/* Respond to the container */
@container (width > 600px) {
  .card {
    display: grid;
    grid-template-columns: 1fr 2fr;
  }
}

/* Target a named container */
@container card (width > 400px) {
  .card h2 {
    font-size: 2em;
  }
}
```

**Container query length units** — relative to the query container's dimensions:

| Unit | Meaning |
| ------- | ------------------------------------ |
| `cqi` | 1% of container's inline size |
| `cqb` | 1% of container's block size |
| `cqw` | 1% of container's width |
| `cqh` | 1% of container's height |
| `cqmin` | smaller of cqi/cqb |
| `cqmax` | larger of cqi/cqb |

```css
/* Fluid font size relative to container, not viewport */
@container (width > 700px) {
  .card h2 {
    font-size: max(1.5em, 1.23em + 2cqi);
  }
}
```

### container-type values

- `inline-size` — query inline dimension only (most common).
- `size` — query both inline and block dimensions (adds size containment in both axes).
- `normal` — not a size query container; still valid for style queries.

---

## Cascade Layers

`@layer` gives you explicit control over the cascade. Layers declared earlier have lower priority.

```css
/* Establish layer order upfront — this is what controls precedence */
@layer reset, base, theme, components, utilities;
```

```css
/* Populate layers separately, anywhere in the file */
@layer reset {
  *, *::before, *::after { box-sizing: border-box; }
  body { margin: 0; }
}

@layer base {
  h1 { font-size: 2rem; }
  p  { line-height: 1.6; }
}

@layer utilities {
  .mt-4 { margin-top: 1rem; }
  .sr-only { position: absolute; clip: rect(0 0 0 0); }
}
```

### Precedence rules

1. `utilities` > `components` > `theme` > `base` > `reset` (later layers win).
2. Unlayered styles beat all layers, regardless of specificity.
3. Specificity only matters within the same layer.

```css
/* Unlayered styles always win */
p { color: red; }           /* wins */

@layer base {
  p { color: blue; }        /* loses — even if more specific */
}
```

```css
/* !important reverses the layer order for important declarations */
@layer reset {
  a { color: blue !important; }  /* wins over utilities !important */
}
@layer utilities {
  a { color: green !important; } /* loses — earlier layer wins for !important */
}
```

**Nested layers** use dot notation:

```css
@layer framework {
  @layer layout { }
  @layer theme  { }
}

/* Append to a nested layer */
@layer framework.layout {
  .container { max-width: 1200px; }
}
```

#### Importing stylesheets into a layer

```css
@import "reset.css" layer(reset);
@import "theme.css" layer(theme);
```

**`revert-layer`** rolls a property back to what it would be in the previous layer:

```css
@layer components {
  .button {
    background: blue;
  }
  .button.plain {
    background: revert-layer;  /* falls back to base/theme layer value */
  }
}
```

---

## Custom Properties

```css
/* Basic declaration and use */
:root {
  --color-primary: #3b82f6;
  --spacing-md: 1rem;
  --font-size-lg: 1.25rem;
}

.button {
  background: var(--color-primary);
  padding: var(--spacing-md);
  font-size: var(--font-size-lg, 1rem);  /* fallback as second arg */
}
```

```css
/* Chained fallbacks */
.text {
  color: var(--color-accent, var(--color-primary, #000));
}
```

**Typed custom properties with `@property`** — enables animation, type checking, and explicit inheritance. Baseline July 2024.

```css
@property --hue {
  syntax: "<number>";
  inherits: false;
  initial-value: 220;
}

@property --progress {
  syntax: "<percentage>";
  inherits: false;
  initial-value: 0%;
}
```

Typed properties can be animated smoothly:

```css
@property --progress {
  syntax: "<percentage>";
  inherits: false;
  initial-value: 0%;
}

.bar {
  background: linear-gradient(to right, green var(--progress), #eee var(--progress));
  animation: fill 2s ease forwards;
}

@keyframes fill {
  to { --progress: 100%; }
}
```

Without `@property`, the gradient would snap instead of transitioning.

### Descriptor requirements

- `syntax` and `inherits` are both required.
- `initial-value` is required for any syntax other than `"*"`.
- `initial-value` must be computationally independent (`10px` is valid; `3em` is not).

---

## :has() Selector

`:has()` selects a parent (or preceding sibling) based on what it contains. Baseline December 2023.

```css
/* Select a section that contains a .featured child */
section:has(.featured) {
  border: 2px solid blue;
}

/* Select h1 immediately followed by h2 */
h1:has(+ h2) {
  margin-block-end: 0.25rem;
}

/* Any heading followed by any other heading */
:is(h1, h2, h3):has(+ :is(h2, h3, h4)) {
  margin-block-end: 0.25rem;
}
```

```css
/* Form state styling */
.field:has(input:invalid) label {
  color: red;
}

.field:has(input:focus) {
  outline: 2px solid blue;
}

.form:has(input:required:invalid) .submit-btn {
  opacity: 0.5;
  pointer-events: none;
}
```

```css
/* AND logic — chain :has() calls */
body:has(video):has(audio) {
  /* only when both video AND audio are present */
}

/* OR logic — comma list inside :has() */
body:has(video, audio) {
  /* when either video OR audio is present */
}
```

```css
/* Sibling count trick — style a list differently based on item count */
li:nth-child(4):last-child ~ li,
li:nth-child(4):last-child {
  /* at least 4 items — use :has() to do this on the parent */
}

ul:has(> li:nth-child(4)) {
  grid-template-columns: repeat(2, 1fr);
}
```

**Specificity:** `:has()` takes the specificity of its most specific argument, same as `:is()` and `:not()`.

### Limitations

- Cannot nest `:has()` inside another `:has()`.
- Pseudo-elements are not valid inside `:has()`.

**Performance:** Anchor `:has()` to specific containers, not `body`, `:root`, or `*`. Use child combinator `>` to constrain traversal depth.

```css
/* Avoid */
body:has(.sidebar) { }

/* Prefer */
.layout:has(> .sidebar-expanded) { }
```

---

## CSS Nesting

Native CSS nesting (no preprocessor required) is baseline across modern browsers.

```css
/* Descendant — implicit when no & */
.card {
  background: white;

  .title {
    font-size: 1.25rem;   /* parses as: .card .title */
  }

  &:hover {
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.1);  /* parses as: .card:hover */
  }

  &.featured {
    border: 2px solid gold;  /* parses as: .card.featured — compound selector, & required */
  }
}
```

```css
/* Reversed context with appended & */
.button {
  background: blue;

  .dark-theme & {
    background: navy;  /* parses as: .dark-theme .button */
  }
}
```

```css
/* Nesting at-rules */
.hero {
  font-size: 1rem;

  @media (width >= 768px) {
    font-size: 1.5rem;
  }

  @supports (display: grid) {
    display: grid;
  }
}
```

```css
/* Combinators work with or without & */
h2 {
  color: tomato;

  + p {
    color: inherit;   /* parses as: h2 + p */
  }

  & + p {
    color: inherit;   /* same result — & makes it explicit */
  }
}
```

### Sass differences

- No string concatenation: `&__child` does not work. Use `.parent .child` or a class directly.
- `&` represents the full selector list, not a string.
- Nesting is parsed in document order; declarations after a nested rule are wrapped in `CSSNestedDeclarations`.

```css
/* Declaration order matters */
.foo {
  color: red;           /* parsed first */
  @media screen {
    color: blue;        /* inside CSSMediaRule */
  }
  color: green;         /* CSSNestedDeclarations — applied after nested rule */
}
```

---

## Logical Properties

Logical properties are writing-mode-aware. They adapt automatically to RTL languages and vertical writing modes.

| Physical | Logical |
| ------------------- | --------------------------- |
| `width` | `inline-size` |
| `height` | `block-size` |
| `margin-top` | `margin-block-start` |
| `margin-bottom` | `margin-block-end` |
| `margin-left` | `margin-inline-start` |
| `margin-right` | `margin-inline-end` |
| `padding-top` | `padding-block-start` |
| `padding-left` | `padding-inline-start` |
| `top` | `inset-block-start` |
| `left` | `inset-inline-start` |
| `border-top` | `border-block-start` |
| `border-left` | `border-inline-start` |

Shorthands exist for both axes:

```css
/* Shorthand for both block ends */
margin-block: 1rem;          /* margin-block-start + margin-block-end */
padding-inline: 2rem 1rem;   /* inline-start inline-end */
border-block: 1px solid #ccc;
inset: 0;                    /* all four sides */
```

```css
/* Component that works in LTR and RTL without changes */
.card {
  inline-size: 100%;
  max-inline-size: 400px;
  margin-inline: auto;
  padding-block: 1.5rem;
  padding-inline: 1.25rem;
  border-inline-start: 4px solid var(--color-primary);
  border-start-start-radius: 8px;
  border-end-start-radius: 8px;
}
```

```css
/* Float and text-align also have logical values */
.pullquote {
  float: inline-end;        /* right in LTR, left in RTL */
  text-align: start;        /* left in LTR, right in RTL */
}
```

---

## Modern Color

### oklch()

Perceptually uniform, works with the P3 wide color gamut. Baseline May 2023.

```css
/* oklch(lightness chroma hue / alpha)
   lightness: 0–1 (or 0%–100%)
   chroma: 0–0.4 (roughly; theoretically unbounded)
   hue: 0–360deg — NOTE: differs from HSL (0deg ≈ magenta, 41deg ≈ red) */

:root {
  --blue-dark:  oklch(30% 0.2 240);
  --blue:       oklch(55% 0.2 240);
  --blue-light: oklch(85% 0.1 240);
}
```

```css
/* Relative color syntax — adjust lightness while keeping hue/chroma */
:root { --brand: oklch(55% 0.2 240); }

.hover-state {
  background: oklch(from var(--brand) calc(l + 0.15) c h);
}
.disabled {
  background: oklch(from var(--brand) l calc(c * 0.3) h);
}
```

### color-mix()

Mix two colors in any color space. Baseline May 2023.

```css
/* color-mix(in <space>, <color1> [%], <color2> [%]) */

.button {
  background: color-mix(in oklch, var(--color-primary) 80%, white);
}

/* Tint/shade scale */
.tint-25 { background: color-mix(in oklab, #a71e14 25%, white); }
.tint-50 { background: color-mix(in oklab, #a71e14 50%, white); }
.tint-75 { background: color-mix(in oklab, #a71e14 75%, white); }

/* Transparency from a custom property */
.overlay {
  background: color-mix(in srgb, var(--surface) 70%, transparent);
}
```

#### Color space guide

- `oklab` / `oklch` — perceptually uniform, avoids graying-out in mixes.
- `xyz` / `srgb-linear` — accurate physical light mixing.
- `srgb` — matches browser defaults but not perceptually smooth.

---

## Animation

```css
/* @keyframes with transition timing */
@keyframes slide-in {
  from {
    transform: translateX(-100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.panel {
  animation: slide-in 300ms cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
```

### Scroll-Driven Animations

Link animation progress to scroll position rather than time. Not yet Baseline — check browser support.

```css
/* Named scroll timeline */
.scroller {
  scroll-timeline-name: --main-scroll;
  scroll-timeline-axis: block;
  overflow-y: scroll;
}

.progress-bar {
  animation: grow-bar linear forwards;
  animation-timeline: --main-scroll;
}

@keyframes grow-bar {
  from { width: 0%; }
  to   { width: 100%; }
}
```

```css
/* Anonymous scroll timeline — simplest form */
.sticky-header {
  animation: shrink linear both;
  animation-timeline: scroll(block root);
  animation-range: 0px 100px;
}

@keyframes shrink {
  to { font-size: 0.875rem; padding-block: 0.5rem; }
}
```

```css
/* View progress timeline — fires as element enters/exits the viewport */
.card {
  animation: fade-in linear both;
  animation-timeline: view();
  animation-range: entry 0% entry 30%;
}

@keyframes fade-in {
  from { opacity: 0; transform: translateY(20px); }
  to   { opacity: 1; transform: none; }
}
```

`animation-timeline` must be declared **after** the `animation` shorthand (it is reset-only in the shorthand).

### View Transitions API

```css
/* Simple same-document transition */
@view-transition {
  navigation: auto;  /* opt-in to cross-document transitions */
}

/* Customize the transition */
::view-transition-old(root) {
  animation: fade-out 200ms ease-out;
}
::view-transition-new(root) {
  animation: fade-in 200ms ease-in;
}
```

```css
/* Named view transition on a specific element */
.hero-image {
  view-transition-name: hero;
}

::view-transition-old(hero),
::view-transition-new(hero) {
  animation-duration: 400ms;
}
```

```js
// Trigger from JavaScript
document.startViewTransition(() => {
  updateDOM();
});
```

---

## Performance

```css
/* will-change: hint to promote element to its own compositor layer.
   Use sparingly — each promotion consumes GPU memory. */
.animated-card {
  will-change: transform, opacity;  /* only when animation is imminent */
}

/* Remove after animation ends */
.animated-card.done {
  will-change: auto;
}
```

```css
/* contain: isolate subtree to limit layout/style/paint recalculation scope */
.widget {
  contain: layout paint;   /* prevents widget's internals from affecting outside layout */
}

.isolated {
  contain: strict;  /* layout + paint + size — strongest isolation */
}
```

```css
/* content-visibility: skip rendering off-screen content entirely */
.article-body {
  content-visibility: auto;
  contain-intrinsic-size: 0 500px;  /* estimated size to hold scroll position */
}
```

### Layer promotion gotchas

- Overusing `will-change` or `transform: translateZ(0)` creates memory pressure.
- Every promoted layer is a texture on the GPU — use only on elements with known animation.
- `contain: strict` also creates a new stacking context.

---

## Critical Rules and Gotchas

**Stacking context:** `position` + `z-index` alone does not create a stacking context. These properties also create one: `transform`, `opacity < 1`, `filter`, `isolation: isolate`, `will-change`, `contain: layout`.

```css
/* z-index has no effect on non-positioned elements (position: static) */
.broken { z-index: 99; }              /* ignored */
.fixed  { position: relative; z-index: 99; }  /* works */
```

### Grid `auto-rows` gotcha

```css
/* auto-rows defines size for implicitly created rows.
   Explicit rows from grid-template-rows are not affected. */
.grid {
  grid-template-rows: 100px;    /* first row: 100px */
  grid-auto-rows: minmax(50px, auto);  /* all additional rows */
}
```

#### Specificity with `@layer`

```css
@layer utilities {
  /* Even a low-specificity rule in a later layer wins over a high-specificity
     rule in an earlier layer. Layer order overrides specificity. */
  .mt-4 { margin-top: 1rem; }       /* wins */
}

@layer base {
  #main .container p { margin-top: 2rem; }  /* loses — earlier layer */
}
```

#### Unlayered styles always win over any layer

```css
p { color: red; }          /* unlayered — wins unconditionally */

@layer utilities {
  p { color: blue; }       /* loses */
}
```

**Flex item min-size:** Flex items default to `min-width: auto` (or `min-height: auto` for column flex). This prevents shrinking below content size. Set `min-width: 0` to allow shrinking.

**`align-content` on single-line flex has no effect** unless `flex-wrap: wrap` is set.

**Container query containment creates a new stacking context** when `container-type: size` or `inline-size` is used.

**`@property` `initial-value` must be computationally independent:** `10px` is valid; `3em` is invalid because it depends on `font-size`.

---

## References

- [CSS Grid Layout — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout)
- [CSS Grid Complete Guide — CSS-Tricks](https://css-tricks.com/snippets/css/complete-guide-grid/)
- [Subgrid — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_grid_layout/Subgrid)
- [Container Queries — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_containment/Container_queries)
- [Cascade Layers (@layer) — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/@layer)
- [:has() Pseudo-class — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/:has)
- [@property — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/@property)
- [CSS Nesting — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_nesting/Using_CSS_nesting)
- [CSS Logical Properties — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_logical_properties_and_values)
- [oklch() — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/oklch)
- [color-mix() — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/color-mix)
- [animation-timeline (Scroll-Driven) — MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timeline)
- [View Transitions API — MDN](https://developer.mozilla.org/en-US/docs/Web/API/View_Transitions_API)
- [Learn CSS — web.dev](https://web.dev/learn/css)
- [Modern CSS Layouts — Smashing Magazine](https://www.smashingmagazine.com/2024/05/modern-css-layouts-no-framework-needed/)
