---
name: react-comprehensive-reference
description: Comprehensive reference for React, Next.js (App Router), React Native, and modern frontend patterns (shadcn/ui, Tailwind, state management).
domain: frontend
category: performance
tags: [react, nextjs, tailwind, ssr, bundle-size, performance, react-native, shadcn, framer-motion, state-management]
triggers: React best practices, Next.js optimization, Tailwind patterns, React Native skills, shadcn/ui, Framer Motion, TanStack Query, Zustand
---

# React Comprehensive Reference

This document is a unified, high-density reference for React, Next.js, React Native, and modern frontend architecture. It consolidates multiple fragmented skill files into a single source of truth for AI agents and developers.

---

## Table of Contents

1. [Next.js App Router](#1-nextjs-app-router)
2. [React Core Performance & Best Practices](#2-react-core-performance--best-practices)
3. [React 19 & Modern APIs](#3-react-19--modern-apis)
4. [React Composition Patterns](#4-react-composition-patterns)
5. [State Management (TanStack Query & Zustand)](#5-state-management)
6. [UI, Styling & Components (shadcn/ui, Radix, Tailwind)](#6-ui-styling--components)
7. [Animations (Framer Motion)](#7-animations)
8. [React Native Specifics](#8-react-native-specifics)
9. [Resource Hints & Preloading](#9-resource-hints--preloading)

---

## 1. Next.js App Router

Comprehensive reference for the Next.js App Router (introduced in v13, stable in v13.4). Covers all major patterns and gotchas as of Next.js 15/16.

### App Router vs Pages Router

#### When to migrate to App Router
- You want server-first rendering with zero client JS by default.
- You need nested, persistent layouts without re-mounting on navigation.
- You want co-location of data fetching, UI, and loading/error states in the same folder.
- You need streaming and Suspense-based loading UX out of the box.
- You want Server Actions for form mutations instead of separate API routes.

### File System Conventions

| File | Purpose |
| ------ | --------- |
| `page.tsx` | Renders the UI for a route segment. Makes the route publicly accessible. |
| `layout.tsx` | Wraps the segment and all children. Persists across navigation (no remount). |
| `loading.tsx` | Instant loading UI via Suspense boundary. Shown while `page.tsx` is streaming. |
| `error.tsx` | Error boundary for the segment. Must be a Client Component (`"use client"`). |
| `not-found.tsx` | Rendered when `notFound()` is called in the segment. |
| `route.ts` | Route handler (API endpoint). Cannot coexist with `page.tsx` in the same segment. |
| `template.tsx` | Like layout but remounts on every navigation. |

### React Server Components (RSC)

By default, every component in `app/` is a Server Component (RSC).
- **Server:** Code never sent to browser. Can be `async`. Access to server env vars. No hooks.
- **Client:** Opt-in with `"use client"`. SSR + hydration. All hooks + browser APIs.

#### Prop serialization checklist
| Type | Passable as prop? |
| ------ | ------------------- |
| string, number, boolean, null | Yes |
| Plain object, Array | Yes |
| Date | Yes |
| Promise | Yes (use `use()` in client) |
| Function | Only if it's a Server Action |
| React element / JSX | No (use children pattern) |

### Server Actions
Async functions marked with `"use server"` that run on the server and can be called from the client.
- Always check authentication **inside** each Server Action.
- Use `useActionState` for pending states and form handling.
- Cache invalidation via `revalidatePath` and `revalidateTag`.

---

## 2. React Core Performance & Best Practices

### Eliminating Waterfalls (CRITICAL)
Waterfalls are the #1 performance killer.
- **Defer Await:** Move `await` into branches where actually used.
- **Parallelization:** Use `Promise.all()` for independent operations.
- **Strategic Suspense:** Use Suspense boundaries to show wrapper UI faster while data streams.

### Bundle Size Optimization (CRITICAL)
- **Avoid Barrel Files:** Import directly from source files (e.g., `import Button from '@mui/material/Button'`).
- **Dynamic Imports:** Use `next/dynamic` for heavy components not needed on initial render.
- **Defer Third-Party Libs:** Load analytics and logging after hydration.

### Server-Side Performance (HIGH)
- **Minimize Serialization:** Only pass fields the client actually uses across the RSC boundary.
- **React.cache():** Use for per-request deduplication of non-fetch data (DB queries, etc.).
- **after():** Use Next.js's `after()` for non-blocking operations like logging after response.

### Re-render Optimization (MEDIUM)
- **Calculate Derived State During Rendering:** Avoid `useEffect` to sync state. Derive it directly.
- **Don't Define Components Inside Components:** Prevents remount on every render.
- **Functional setState:** Use `setState(prev => ...)` for state that depends on current value.
- **useRef for Transient Values:** Use for values that change often but don't need UI updates.

---

## 3. React 19 & Modern APIs

> **⚠️ React 19+ only.**

- **Ref as Prop:** `ref` is now a regular prop. No `forwardRef` needed.
- **use() Hook:** Replaces `useContext()` and can unwrap Promises.
- **Action State:** `useActionState` handles form actions and state.
- **Optimistic UI:** `useOptimistic` for immediate UI feedback.

---

## 4. React Composition Patterns

### Avoid Boolean Prop Proliferation
Don't add `isThread`, `isEditing` props. Use composition to create explicit variants.
```tsx
// Instead of <Composer isThread={true} />
function ThreadComposer() {
  return (
    <Composer.Frame>
      <Composer.Header />
      <Composer.Input />
      <AlsoSendToChannelField />
    </Composer.Frame>
  )
}
```

### Compound Components
Structure complex components with shared context. Consumers compose pieces they need.
```tsx
const Composer = {
  Provider: ComposerProvider,
  Input: ComposerInput,
  Submit: ComposerSubmit,
}
```

---

## 5. State Management

### TanStack Query v5 (Server State)
Owns data that lives on a remote server.
- **QueryKey:** Every variable used in `queryFn` must be in `queryKey`.
- **staleTime:** Controls how long data is "fresh".
- **gcTime:** Controls garbage collection of unused cache entries.

### Zustand (Client State)
Owns UI-local data (modals, filters, preferences).
- **Atomic Selectors:** `const count = useStore(s => s.count)`.
- **useShallow:** Use for object/array selectors to prevent unnecessary re-renders.
- **Slice Pattern:** Split large stores into manageable pieces.

---

## 6. UI, Styling & Components

### shadcn/ui & Radix UI
- **Copy, Not Install:** You own the component code in `components/ui/`.
- **asChild:** Radix primitives merge behavior into your custom components.
- **Theming:** Uses CSS variables (`globals.css`) and Tailwind v4 `@theme inline`.

### Tailwind Best Practices
- **Design Tokens:** Use theme variables for colors/spacing. Avoid arbitrary hex codes.
- **Utility First:** Prefer utility classes over custom CSS.
- **Responsive:** Use `sm:`, `md:`, `lg:` modifiers instead of manual media queries.

---

## 7. Animations (Framer Motion)

- **motion.* components:** HTML/SVG elements augmented with animation props.
- **GPU Accelerated:** Animate `transform` and `opacity` only. Avoid `width`, `height`, `top`, `left`.
- **Variants:** Declarative states defined outside JSX.
- **AnimatePresence:** Handles exit animations when components unmount.
- **Layout Prop:** Animates size/position changes automatically.

---

## 8. React Native Specifics

### Core Rendering
- **Never use && with falsy values:** `{count && <Text>}` crashes if `count=0`. Use ternary.
- **Wrap Strings in Text:** RN crashes if a string is a direct child of `<View>`.

### List Performance
- **FlashList / LegendList:** Always virtualize lists.
- **Lightweight Items:** Minimize hooks and context access inside `renderItem`.
- **Stable References:** Avoid inline objects/arrays in props to prevent re-renders.

### Animation (Reanimated)
- **UI Thread:** All animations and gesture handling should run on the UI thread.
- **.get() and .set():** Use for shared values when using React Compiler.
- **Transform/Opacity:** Only animate GPU-accelerated properties.

---

## 9. Resource Hints & Preloading

React DOM APIs to hint the browser about resources:
- `prefetchDNS(href)`
- `preconnect(href)`
- `preload(href, options)`
- `preloadModule(href)`
- `preinit(href, options)`

Use these in Server Components to start loading critical fonts, styles, and scripts before the client receives HTML.

---

## 10. Code-Level Anti-Patterns & Fixes

### Don't Wrap Simple Primitives in useMemo
When an expression is simple and returns a primitive (boolean, number, string), `useMemo` overhead exceeds the expression cost.
```tsx
// BAD
const isLoading = useMemo(() => user.isLoading || notifications.isLoading, [user.isLoading, notifications.isLoading])

// GOOD
const isLoading = user.isLoading || notifications.isLoading
```

### Build Index Maps for Repeated Lookups
Multiple `.find()` calls by the same key should use a Map.
```typescript
// BAD: O(n) per lookup
const user = users.find(u => u.id === order.userId)

// GOOD: O(1) per lookup
const userById = new Map(users.map(u => [u.id, u]))
const user = userById.get(order.userId)
```

### Decouple State Management from UI
The provider component should be the only place that knows how state is managed. UI components consume the context interface -- they do not know if state comes from useState, Zustand, or a server sync.
```tsx
// Provider handles all state management details
function ChannelProvider({ channelId, children }) {
  const { state, update, submit } = useGlobalChannel(channelId)
  return (
    <Composer.Provider state={state} actions={{ update, submit }}>
      {children}
    </Composer.Provider>
  )
}
// UI component only knows about the context interface
function ChannelComposer() {
  return <Composer.Frame><Composer.Input /><Composer.Submit /></Composer.Frame>
}
```

### Minimize Serialization at RSC Boundaries
Only pass fields the client actually uses. The RSC boundary serializes all object properties into strings embedded in HTML.
```tsx
// BAD: serializes all 50 fields
async function Page() {
  const user = await fetchUser()
  return <Profile user={user} />
}
// GOOD: serializes only 1 field
async function Page() {
  const user = await fetchUser()
  return <Profile name={user.name} />
}
```

### Prevent Waterfall Chains in API Routes
Start independent operations immediately, even if you don't await them yet.
```typescript
// GOOD: auth and config start immediately
export async function GET(request: Request) {
  const sessionPromise = auth()
  const configPromise = fetchConfig()
  const session = await sessionPromise
  const [config, data] = await Promise.all([configPromise, fetchData(session.user.id)])
  return Response.json({ data, config })
}
```

### CSS content-visibility for Long Lists
Apply `content-visibility: auto` to defer off-screen rendering (10x faster initial render for 1000+ items).
```css
.message-item { content-visibility: auto; contain-intrinsic-size: 0 80px; }
```

### Optimize SVG with SVGO
Reduce coordinate precision to decrease file size.
```bash
npx svgo --precision=1 --multipass icon.svg
```

### Expo Font Loading (React Native)
Use the `expo-font` config plugin to embed fonts at build time instead of `useFonts` or `Font.loadAsync`.
```json
{ "expo": { "plugins": [["expo-font", { "fonts": ["./assets/fonts/Geist-Bold.otf"] }]] } }
```

---

## Checklist Before Ship

- [ ] No async waterfalls; data fetched on server or in parallel.
- [ ] Bundle size audited; heavy components lazy-loaded.
- [ ] React 19 patterns used (ref as prop, use()).
- [ ] No falsy `&&` in JSX (especially for React Native).
- [ ] Lists are virtualized and items are memoized.
- [ ] Server Actions are authenticated and inputs validated.
- [ ] Design tokens used for all styling; no magic numbers.
