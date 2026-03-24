---
name: css-in-js
description: Use this skill when writing, reviewing, or migrating component styles with CSS-in-JS libraries (Styled Components, Emotion, Linaria, Vanilla Extract). Covers runtime vs zero-runtime trade-offs, SSR setup, theming, dynamic props, bundle impact, and decision guidance for choosing CSS-in-JS vs Tailwind vs CSS Modules.
domain: frontend
category: styling
tags: [CSS-in-JS, Styled-Components, Emotion, Linaria, Vanilla-Extract, theming, SSR, performance, zero-runtime]
triggers: styled-components, emotion, css-in-js, linaria, vanilla-extract, ThemeProvider, css prop, styled template, zero-runtime CSS, critical CSS, component styling, dynamic styles, SSR styles
---

# CSS-in-JS

Practical guidance for runtime and zero-runtime CSS-in-JS: when to use each library, how to configure it correctly, and when to abandon the approach entirely in favor of Tailwind or CSS Modules.

## When to Use This Skill

- Adding or refactoring styles in a React application using Styled Components or Emotion.
- Evaluating whether to adopt Linaria or Vanilla Extract for a performance-sensitive project.
- Diagnosing SSR hydration mismatches or style-flicker caused by runtime style injection.
- Deciding between CSS-in-JS, Tailwind, and CSS Modules for a new project or migration.

---

## Runtime vs Zero-Runtime: The Fundamental Split

**Runtime CSS-in-JS** (Styled Components, Emotion) parses style strings and injects `<style>` tags in the browser during render. This gives you full access to JavaScript values at render time but carries measurable overhead:

- Style serialization happens on every render where styles depend on props or state. Serializing a CSS object to a string inside a render cycle repeats work on every re-render.
- React's reconciliation pauses while the browser recalculates CSS rules against all DOM nodes. Sebastian Markbåge (React core team) described this as "VERY slow" because it forces layout recalculations every frame during React rendering.
- Measured cost: in one production migration from Emotion to Sass Modules, the affected component dropped from 54.3ms to 27.7ms render time on an M1 Max — roughly 48% faster — with the gap widening further on lower-end devices.

**Zero-runtime CSS-in-JS** (Linaria, Vanilla Extract) extracts all styles to `.css` files at build time. The browser receives static CSS; no JavaScript runs to inject styles. Dynamic values that depend on runtime state fall back to CSS custom properties (variables), not inline JavaScript.

**The rule:** If you're writing a new component library, design system, or any project where SSR performance and bundle size are first-class concerns, default to zero-runtime. Choose runtime only when you need deeply dynamic styles that cannot be expressed as CSS variable swaps.

---

## Styled Components

### Core API

```tsx
import styled from 'styled-components';

// Static component
const Card = styled.div`
  border-radius: 8px;
  padding: 16px;
  background: white;
`;

// Dynamic via props
const Button = styled.button<{ $primary?: boolean }>`
  background: ${({ $primary }) => ($primary ? '#0070f3' : 'transparent')};
  color: ${({ $primary }) => ($primary ? 'white' : '#0070f3')};
  border: 2px solid #0070f3;
  padding: 8px 16px;
  border-radius: 4px;
`;

// Extending an existing component
const LargeButton = styled(Button)`
  padding: 12px 24px;
  font-size: 1.1rem;
`;
```

Use the `$` prefix convention (transient props) for props that should not be forwarded to the DOM element. Without it, React will emit unknown-attribute warnings.

### Theming with ThemeProvider

```tsx
import { ThemeProvider, DefaultTheme } from 'styled-components';

const theme: DefaultTheme = {
  colors: {
    primary: '#0070f3',
    text: '#111',
    background: '#fff',
  },
  spacing: {
    sm: '8px',
    md: '16px',
    lg: '32px',
  },
};

// Wrap at app root
function App() {
  return (
    <ThemeProvider theme={theme}>
      <RootLayout />
    </ThemeProvider>
  );
}

// Consume in any styled component
const Heading = styled.h1`
  color: ${({ theme }) => theme.colors.primary};
  margin-bottom: ${({ theme }) => theme.spacing.md};
`;
```

### SSR Setup (Next.js)

Styled Components requires a Babel plugin or the SWC compiler plugin for SSR. Without it, the server renders unstyled HTML and the client re-injects styles, causing a flash of unstyled content (FOUC).

#### Next.js App Router (`next.config.js`)

```js
// next.config.js
const nextConfig = {
  compiler: {
    styledComponents: true,
  },
};
module.exports = nextConfig;
```

#### Pages Router (legacy `_document.tsx`)

```tsx
import Document, { DocumentContext } from 'next/document';
import { ServerStyleSheet } from 'styled-components';

export default class MyDocument extends Document {
  static async getInitialProps(ctx: DocumentContext) {
    const sheet = new ServerStyleSheet();
    const originalRenderPage = ctx.renderPage;
    try {
      ctx.renderPage = () =>
        originalRenderPage({
          enhanceApp: (App) => (props) =>
            sheet.collectStyles(<App {...props} />),
        });
      const initialProps = await Document.getInitialProps(ctx);
      return {
        ...initialProps,
        styles: [initialProps.styles, sheet.getStyleElement()],
      };
    } finally {
      sheet.seal();
    }
  }
}
```

**Bundle size:** ~12.7 kB minzipped. Styled Components does not support React Server Components (RSC) — any component importing it must be a Client Component.

---

## Emotion

### Two Modes

Emotion ships two packages for React:

- `@emotion/styled` — API-compatible with Styled Components; drop-in replacement for most use cases.
- `@emotion/react` — exposes the `css` prop, which attaches styles directly to JSX elements without creating named wrapper components.

```tsx
// @emotion/styled — identical surface to styled-components
import styled from '@emotion/styled';

const Box = styled.div<{ color: string }>`
  color: ${({ color }) => color};
  padding: 16px;
`;

// @emotion/react — css prop
/** @jsxImportSource @emotion/react */
import { css } from '@emotion/react';

const dynamicStyle = (color: string) => css`
  color: ${color};
  padding: 16px;
`;

function Component({ color }: { color: string }) {
  return <div css={dynamicStyle(color)}>content</div>;
}
```

### Performance Rule: Serialize Outside the Render Cycle

The single biggest performance win with Emotion is moving static `css()` calls outside the component body. Serializing inside `render` re-runs the template literal parser on every render.

```tsx
// BAD — serialized on every render
function Badge({ label }: { label: string }) {
  return (
    <span css={css`background: gold; border-radius: 4px; padding: 2px 6px;`}>
      {label}
    </span>
  );
}

// GOOD — serialized once at module load
const badgeStyle = css`
  background: gold;
  border-radius: 4px;
  padding: 2px 6px;
`;

function Badge({ label }: { label: string }) {
  return <span css={badgeStyle}>{label}</span>;
}
```

For dynamic styles, separate the static part from the dynamic part:

```tsx
const baseStyle = css`padding: 8px 16px; border-radius: 4px;`;

const variantStyle = (primary: boolean) =>
  primary ? css`background: #0070f3; color: white;` : css`background: transparent; color: #0070f3;`;

// Emotion composes these efficiently — one serialization per variant value, not per render
```

### SSR Setup (Emotion)

Emotion's React integration supports SSR with zero configuration when using the default `@emotion/react` cache. For advanced cases (custom caches, streaming):

```tsx
import createCache from '@emotion/cache';
import { CacheProvider } from '@emotion/react';

const cache = createCache({ key: 'css', prepend: true });

function App() {
  return (
    <CacheProvider value={cache}>
      <Root />
    </CacheProvider>
  );
}
```

For Next.js App Router, Emotion also does not support RSC natively. Components using the `css` prop or `styled` API must be Client Components.

**Bundle size:** ~7.9 kB minzipped — meaningfully smaller than Styled Components.

### Styled Components vs Emotion: When to Pick Which

| Concern | Styled Components | Emotion |
| --- | --- | --- |
| Bundle size | 12.7 kB | 7.9 kB |
| API flexibility | Tagged templates only | Templates + css prop + object styles |
| SSR zero-config | No (plugin required) | Yes (basic cases) |
| React DevTools noise | Minimal | `css` prop adds wrapper nodes |
| Migration from SC | — | `@emotion/styled` is a drop-in |
| Community / ecosystem | Larger; more examples | Smaller but growing |

If you are starting fresh and performance matters, prefer Emotion for its smaller footprint. If you are migrating an existing Styled Components codebase and want minimal churn, `@emotion/styled` is API-compatible.

---

## Linaria (Zero-Runtime)

Linaria extracts styles to static `.css` files at build time. It parses tagged template literals and CSS property objects, evaluates JavaScript expressions that are statically knowable, and writes the results to CSS. At runtime, components receive class names — no style injection occurs.

### API

```tsx
import { css, styled } from '@linaria/react';

// css tag — produces a class name string at build time
const container = css`
  display: flex;
  gap: 16px;
  padding: 24px;
`;

// styled — like styled-components but extracted at build time
const Heading = styled.h1<{ $size: 'sm' | 'lg' }>`
  font-size: ${({ $size }) => ($size === 'lg' ? '2rem' : '1.25rem')};
  color: #111;
`;

function Page() {
  return (
    <div className={container}>
      <Heading $size="lg">Title</Heading>
    </div>
  );
}
```

Linaria evaluates the template literal at build time. Expressions that depend on runtime values (component state, server data) are replaced with CSS custom properties, and Linaria writes the variable assignment into an inline `style` prop automatically.

### Bundler Setup

Linaria requires a Babel transform and bundler integration:

```bash
npm install @linaria/core @linaria/react @linaria/babel-preset
# + bundler plugin: @linaria/webpack-loader or @linaria/vite
```

#### Vite

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import linaria from '@linaria/vite';

export default defineConfig({
  plugins: [linaria({ sourceMap: true })],
});
```

### Limitations

- Expressions inside template literals must be statically evaluable at build time. Complex runtime logic cannot be serialized.
- Bundler configuration is more involved than dropping in a package; every new environment (Jest, Storybook) needs its own Linaria transform.
- Hot-reload performance in development can be slower than runtime solutions because the build step runs on every save.

---

## Vanilla Extract (Zero-Runtime, TypeScript-First)

Vanilla Extract writes styles in `.css.ts` files using a fully typed API. It is framework-agnostic and integrates with webpack, Vite, Parcel, esbuild, and Next.js.

### API (Vanilla Extract (Zero-Runtime, TypeScript-First))

```ts
// styles.css.ts
import { style, createTheme, createVar } from '@vanilla-extract/css';

export const [themeClass, vars] = createTheme({
  color: {
    primary: '#0070f3',
    text: '#111',
  },
  space: {
    sm: '8px',
    md: '16px',
  },
});

export const container = style({
  padding: vars.space.md,
  color: vars.color.text,
});

export const button = style({
  background: vars.color.primary,
  color: 'white',
  borderRadius: '4px',
  padding: `${vars.space.sm} ${vars.space.md}`,
});
```

```tsx
// Component.tsx
import { themeClass, container, button } from './styles.css';

export function Page() {
  return (
    <div className={themeClass}>
      <main className={container}>
        <button className={button}>Click</button>
      </main>
    </div>
  );
}
```

### Sprinkles (Utility API)

Vanilla Extract ships `@vanilla-extract/sprinkles`, which generates a Tailwind-like utility API from your design tokens — typed at compile time:

```ts
// sprinkles.css.ts
import { defineProperties, createSprinkles } from '@vanilla-extract/sprinkles';

const properties = defineProperties({
  properties: {
    display: ['none', 'flex', 'block'],
    padding: { sm: '8px', md: '16px', lg: '32px' },
    color: { primary: '#0070f3', text: '#111' },
  },
});

export const sprinkles = createSprinkles(properties);
```

```tsx
<div className={sprinkles({ display: 'flex', padding: 'md' })} />
```

### Linaria vs Vanilla Extract

| Concern | Linaria | Vanilla Extract |
| --- | --- | --- |
| Syntax | Tagged template literals (familiar) | TypeScript object API (`.css.ts` files) |
| TypeScript | Compatible | First-class, full inference |
| Dynamic values | CSS custom property fallback | CSS custom property fallback |
| Bundle impact | Zero runtime | Zero runtime |
| Ecosystem | 10.8k GitHub stars | 8.7k GitHub stars |
| Theming | Manual CSS variables | `createTheme` + typed vars |

Pick Vanilla Extract when TypeScript type safety on style properties matters to your team. Pick Linaria when your team is already comfortable with tagged template syntax and wants minimal API surface change from Emotion/Styled Components.

---

## Dynamic Styles: The Right Pattern

Regardless of library, follow this decision tree for dynamic styling:

1. **Value changes between a small set of known variants** — use prop-driven class switching, not inline logic:

   ```tsx
   const variantMap = { primary: primaryStyle, secondary: secondaryStyle };
   <div className={variantMap[variant]} />
   ```

2. **Value is a CSS custom property that changes at runtime** — use `style` prop to set the variable, keep the rule in CSS:

   ```tsx
   const dynamicColor = css`color: var(--item-color);`;
   <span className={dynamicColor} style={{ '--item-color': color } as React.CSSProperties} />
   ```

3. **Value requires JavaScript arithmetic or complex logic** — only then reach for runtime CSS-in-JS; even then, memoize the style object so it is referentially stable.

Avoid generating unique class names inside render on every keystroke or scroll event. This causes continuous style injection and StyleSheet churn.

---

## Server-Side Rendering: Critical CSS Extraction

For runtime CSS-in-JS (Emotion, Styled Components), the goal is to collect all styles used by the server-rendered HTML and embed them in `<head>` before the HTML is sent. Without this, the page renders unstyled until the JavaScript bundle loads and injects styles — a flash that wrecks Core Web Vitals.

Key principles:

- **Styled Components**: use `ServerStyleSheet.collectStyles()` in `_document.tsx` (Pages Router) or the SWC compiler option (App Router).
- **Emotion**: the default cache handles basic SSR. For streaming SSR (React 18+), use `@emotion/server`'s `renderStylesToNodeStream` or the `extractCriticalToChunks` API.
- **Zero-runtime libraries**: no SSR style collection needed. The `.css` files are served as static assets by the bundler. This is a major operational advantage.

React Server Components (RSC) are incompatible with runtime CSS-in-JS because style injection requires a React context that does not exist on the server render tree. If your project uses the Next.js App Router heavily with RSC, zero-runtime CSS-in-JS or CSS Modules are the only viable paths.

---

## Bundle Size and Build Impact

| Approach | Runtime JS cost | Build complexity | Dynamic capability |
| --- | --- | --- | --- |
| Styled Components | ~12.7 kB | Low | High |
| Emotion | ~7.9 kB | Low | High |
| Linaria | ~0 kB | Medium | Medium (CSS vars) |
| Vanilla Extract | ~0 kB | Medium | Medium (CSS vars) |
| CSS Modules | 0 kB | None | Low |
| Tailwind | 0 kB (CSS only) | Low | Low (class-based) |

Note that Tailwind's CSS output, with proper PurgeCSS/content scanning, is typically the smallest transferred payload in production — utility classes that are not referenced are eliminated entirely at build time.

---

## Choosing Between CSS-in-JS, Tailwind, and CSS Modules

### Use CSS-in-JS (Emotion or Styled Components) when

- You are building a component library or design system where styles must travel with components as a single importable unit.
- Theming requires deep runtime customization (user-configurable color schemes, white-label products with per-tenant themes).
- Your team is heavily React-centric and co-location of styles with component logic is a firm preference.

#### Use Linaria or Vanilla Extract when

- The above applies, but you need RSC compatibility or zero runtime overhead.
- You are building at a scale where the measured 48% render cost of runtime CSS-in-JS is unacceptable.
- Your project uses Next.js App Router with Server Components as the default.

#### Use Tailwind when

- You want the smallest possible CSS payload with no JavaScript overhead.
- Rapid prototyping matters more than strict style encapsulation.
- Your team finds utility-first ergonomics natural (most do, after a short ramp).
- Tailwind v4's Oxide engine (Rust-based, 10× faster builds vs v3) removes the previous build-time objection.

#### Use CSS Modules when

- You want scoped CSS with zero runtime cost and no new tooling.
- The team has traditional CSS backgrounds and wants to write standard CSS.
- You are working with a framework that has first-class CSS Modules support (Next.js, Vite, Create React App).
- Global styles are minimal and can be managed in a single `globals.css`.

#### Avoid CSS-in-JS entirely when

- Your project uses React Server Components as the primary rendering model. Runtime CSS-in-JS requires Client Components everywhere styles are used, which defeats the performance case for RSC.
- Bundle size is constrained and you cannot afford the 8–13 kB overhead.

---

## Checklist Before Ship

- [ ] Static `css()` or `styled` calls are defined outside component bodies, not inside render.
- [ ] Transient props use the `$` prefix to prevent DOM forwarding warnings.
- [ ] SSR style collection is configured (ServerStyleSheet or Emotion server extraction).
- [ ] Dynamic values that only change on user interaction use CSS custom properties, not new class names per render.
- [ ] ThemeProvider wraps the application root; theme object is defined outside the component tree so it is referentially stable.
- [ ] Components using runtime CSS-in-JS are marked `"use client"` if using Next.js App Router.
- [ ] Bundle analyzer confirms CSS-in-JS runtime cost is acceptable for the project's performance budget.
- [ ] Zero-runtime alternative (Linaria/Vanilla Extract) has been considered if RSC compatibility is needed.

---

## References

- [Emotion Docs — Performance](https://emotion.sh/docs/performance)
- [Emotion Docs — Best Practices](https://emotion.sh/docs/best-practices)
- [Why We're Breaking Up with CSS-in-JS (Sam Magura)](https://dev.to/srmagura/why-were-breaking-up-wiht-css-in-js-4g9b)
- [Styled-components vs Emotion — LogRocket](https://blog.logrocket.com/styled-components-vs-emotion-for-handling-css/)
- [Comparing Top Zero-Runtime CSS-in-JS Libraries — LogRocket](https://blog.logrocket.com/comparing-top-zero-runtime-css-js-libraries/)
- [Linaria — Zero-Runtime CSS in JS](https://linaria.dev/)
- [Vanilla Extract — Zero-Runtime Stylesheets in TypeScript](https://vanilla-extract.style/)
- [CSS Modules vs CSS-in-JS vs Tailwind — Medium](https://medium.com/@ignatovich.dm/css-modules-vs-css-in-js-vs-tailwind-css-a-comprehensive-comparison-24e7cb6f48e9)
