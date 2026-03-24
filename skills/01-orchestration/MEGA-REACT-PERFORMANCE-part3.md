---
name: "Use Activity Component for Show/Hide (Part 3)"
description: "## Use Activity Component for Show/Hide - Part 3"
---


## Put Interaction Logic in Event Handlers

If a side effect is triggered by a specific user action (submit, click, drag), run it in that event handler. Do not model the action as state + effect; it makes effects re-run on unrelated changes and can duplicate the action.

### Incorrect (event modeled as state + effect)

```tsx
function Form() {
  const [submitted, setSubmitted] = useState(false)
  const theme = useContext(ThemeContext)

  useEffect(() => {
    if (submitted) {
      post('/api/register')
      showToast('Registered', theme)
    }
  }, [submitted, theme])

  return <button onClick={() => setSubmitted(true)}>Submit</button>
}
```

#### Correct (do it in the handler)

```tsx
function Form() {
  const theme = useContext(ThemeContext)

  function handleSubmit() {
    post('/api/register')
    showToast('Registered', theme)
  }

  return <button onClick={handleSubmit}>Submit</button>
}
```

Reference: [Should this code move to an event handler?](https://react.dev/learn/removing-effect-dependencies#should-this-code-move-to-an-event-handler)

## When to use

Use when the user asks about or needs: Put Interaction Logic in Event Handlers.
﻿---
name: State Must Represent Ground Truth
description: ## State Must Represent Ground Truth
 
 State variables—both React `useState` and Reanimated shared values—should
tags: state, derived-state, reanimated, hooks
---

## State Must Represent Ground Truth

State variables—both React `useState` and Reanimated shared values—should
represent the actual state of something (e.g., `pressed`, `progress`, `isOpen`),
not derived visual values (e.g., `scale`, `opacity`, `translateY`). Derive
visual values from state using computation or interpolation.

### Incorrect (storing the visual output)

```tsx
const scale = useSharedValue(1)

const tap = Gesture.Tap()
  .onBegin(() => {
    scale.set(withTiming(0.95))
  })
  .onFinalize(() => {
    scale.set(withTiming(1))
  })

const animatedStyle = useAnimatedStyle(() => ({
  transform: [{ scale: scale.get() }],
}))
```

#### Correct (storing the state, deriving the visual)

```tsx
const pressed = useSharedValue(0) // 0 = not pressed, 1 = pressed

const tap = Gesture.Tap()
  .onBegin(() => {
    pressed.set(withTiming(1))
  })
  .onFinalize(() => {
    pressed.set(withTiming(0))
  })

const animatedStyle = useAnimatedStyle(() => ({
  transform: [{ scale: interpolate(pressed.get(), [0, 1], [1, 0.95]) }],
}))
```

#### Why this matters

State variables should represent real "state", not necessarily a desired end
result.

1. **Single source of truth** — The state (`pressed`) describes what's
   happening; visuals are derived
2. **Easier to extend** — Adding opacity, rotation, or other effects just
   requires more interpolations from the same state
3. **Debugging** — Inspecting `pressed = 1` is clearer than `scale = 0.95`
4. **Reusable logic** — The same `pressed` value can drive multiple visual
   properties

#### Same principle for React state

```tsx
// Incorrect: storing derived values
const [isExpanded, setIsExpanded] = useState(false)
const [height, setHeight] = useState(0)

useEffect(() => {
  setHeight(isExpanded ? 200 : 0)
}, [isExpanded])

// Correct: derive from state
const [isExpanded, setIsExpanded] = useState(false)
const height = isExpanded ? 200 : 0
```

State is the minimal truth. Everything else is derived.

## When to use

Use when the user asks about or needs: State Must Represent Ground Truth.
﻿---
name: Store Event Handlers in Refs
description: ## Store Event Handlers in Refs
 
 Store callbacks in refs when used in effects that shouldn't re-subscribe on callback changes.
tags: advanced, hooks, refs, event-handlers, optimization
---

## Store Event Handlers in Refs

Store callbacks in refs when used in effects that shouldn't re-subscribe on callback changes.

### Incorrect (re-subscribes on every render)

```tsx
function useWindowEvent(event: string, handler: (e) => void) {
  useEffect(() => {
    window.addEventListener(event, handler)
    return () => window.removeEventListener(event, handler)
  }, [event, handler])
}
```

#### Correct (stable subscription)

```tsx
function useWindowEvent(event: string, handler: (e) => void) {
  const handlerRef = useRef(handler)
  useEffect(() => {
    handlerRef.current = handler
  }, [handler])

  useEffect(() => {
    const listener = (e) => handlerRef.current(e)
    window.addEventListener(event, listener)
    return () => window.removeEventListener(event, listener)
  }, [event])
}
```

#### Alternative: use `useEffectEvent` if you're on latest React

```tsx
import { useEffectEvent } from 'react'

function useWindowEvent(event: string, handler: (e) => void) {
  const onEvent = useEffectEvent(handler)

  useEffect(() => {
    window.addEventListener(event, onEvent)
    return () => window.removeEventListener(event, onEvent)
  }, [event])
}
```

`useEffectEvent` provides a cleaner API for the same pattern: it creates a stable function reference that always calls the latest version of the handler.

## When to use

Use when the user asks about or needs: Store Event Handlers in Refs.
﻿---
name: Strategic Suspense Boundaries
description: ## Strategic Suspense Boundaries
 
 Instead of awaiting data in async components before returning JSX, use Suspense boundaries to show the wrapper UI faster while data loads.
tags: async, suspense, streaming, layout-shift
---

## Strategic Suspense Boundaries

Instead of awaiting data in async components before returning JSX, use Suspense boundaries to show the wrapper UI faster while data loads.

### Incorrect (wrapper blocked by data fetching)

```tsx
async function Page() {
  const data = await fetchData() // Blocks entire page
  
  return (
    <div>
      <div>Sidebar</div>
      <div>Header</div>
      <div>
        <DataDisplay data={data} />
      </div>
      <div>Footer</div>
    </div>
  )
}
```

The entire layout waits for data even though only the middle section needs it.

#### Correct (wrapper shows immediately, data streams in)

```tsx
function Page() {
  return (
    <div>
      <div>Sidebar</div>
      <div>Header</div>
      <div>
        <Suspense fallback={<Skeleton />}>
          <DataDisplay />
        </Suspense>
      </div>
      <div>Footer</div>
    </div>
  )
}

async function DataDisplay() {
  const data = await fetchData() // Only blocks this component
  return <div>{data.content}</div>
}
```

Sidebar, Header, and Footer render immediately. Only DataDisplay waits for data.

#### Alternative (share promise across components)

```tsx
function Page() {
  // Start fetch immediately, but don't await
  const dataPromise = fetchData()
  
  return (
    <div>
      <div>Sidebar</div>
      <div>Header</div>
      <Suspense fallback={<Skeleton />}>
        <DataDisplay dataPromise={dataPromise} />
        <DataSummary dataPromise={dataPromise} />
      </Suspense>
      <div>Footer</div>
    </div>
  )
}

function DataDisplay({ dataPromise }: { dataPromise: Promise<Data> }) {
  const data = use(dataPromise) // Unwraps the promise
  return <div>{data.content}</div>
}

function DataSummary({ dataPromise }: { dataPromise: Promise<Data> }) {
  const data = use(dataPromise) // Reuses the same promise
  return <div>{data.summary}</div>
}
```

Both components share the same promise, so only one fetch occurs. Layout renders immediately while both components wait together.

#### When NOT to use this pattern

- Critical data needed for layout decisions (affects positioning)
- SEO-critical content above the fold
- Small, fast queries where suspense overhead isn't worth it
- When you want to avoid layout shift (loading → content jump)

**Trade-off:** Faster initial paint vs potential layout shift. Choose based on your UX priorities.

## When to use

Use when the user asks about or needs: Strategic Suspense Boundaries.
﻿---
name: Subscribe to Derived State
description: ## Subscribe to Derived State
 
 Subscribe to derived boolean state instead of continuous values to reduce re-render frequency.
tags: rerender, derived-state, media-query, optimization
---

## Subscribe to Derived State

Subscribe to derived boolean state instead of continuous values to reduce re-render frequency.

### Incorrect (re-renders on every pixel change)

```tsx
function Sidebar() {
  const width = useWindowWidth()  // updates continuously
  const isMobile = width < 768
  return <nav className={isMobile ? 'mobile' : 'desktop'} />
}
```

#### Correct (re-renders only when boolean changes)

```tsx
function Sidebar() {
  const isMobile = useMediaQuery('(max-width: 767px)')
  return <nav className={isMobile ? 'mobile' : 'desktop'} />
}
```

## When to use

Use when the user asks about or needs: Subscribe to Derived State.
﻿---
name: Suppress Expected Hydration Mismatches
description: ## Suppress Expected Hydration Mismatches
 
 In SSR frameworks (e.g., Next.js), some values are intentionally different on server vs client (random IDs, dates, locale/timezone formatting). For these *expected* mismatches, wrap the dynamic text in an element with `suppressHydrationWarning` to prevent noisy warnings. Do not use this to hide real bugs. Don’t overuse it.
tags: rendering, hydration, ssr, nextjs
---

## Suppress Expected Hydration Mismatches

In SSR frameworks (e.g., Next.js), some values are intentionally different on server vs client (random IDs, dates, locale/timezone formatting). For these *expected* mismatches, wrap the dynamic text in an element with `suppressHydrationWarning` to prevent noisy warnings. Do not use this to hide real bugs. Don’t overuse it.

### Incorrect (known mismatch warnings)

```tsx
function Timestamp() {
  return <span>{new Date().toLocaleString()}</span>
}
```

#### Correct (suppress expected mismatch only)

```tsx
function Timestamp() {
  return (
    <span suppressHydrationWarning>
      {new Date().toLocaleString()}
    </span>
  )
}
```

## When to use

Use when the user asks about or needs: Suppress Expected Hydration Mismatches.
﻿---
name: Initialize App Once, Not Per Mount
description: ## Initialize App Once, Not Per Mount
 
 Do not put app-wide initialization that must run once per app load inside `useEffect([])` of a component. Components can remount and effects will re-run. Use a module-level guard or top-level init in the entry module instead.
tags: initialization, useEffect, app-startup, side-effects
---

## Initialize App Once, Not Per Mount

Do not put app-wide initialization that must run once per app load inside `useEffect([])` of a component. Components can remount and effects will re-run. Use a module-level guard or top-level init in the entry module instead.

### Incorrect (runs twice in dev, re-runs on remount)

```tsx
function Comp() {
  useEffect(() => {
    loadFromStorage()
    checkAuthToken()
  }, [])

  // ...
}
```

#### Correct (once per app load)

```tsx
let didInit = false

function Comp() {
  useEffect(() => {
    if (didInit) return
    didInit = true
    loadFromStorage()
    checkAuthToken()
  }, [])

  // ...
}
```

Reference: [Initializing the application](https://react.dev/learn/you-might-not-need-an-effect#initializing-the-application)

## When to use

Use when the user asks about or needs: Initialize App Once, Not Per Mount.
﻿---
name: Defer State Reads to Usage Point
description: ## Defer State Reads to Usage Point
 
 Don't subscribe to dynamic state (searchParams, localStorage) if you only read it inside callbacks.
tags: rerender, searchParams, localStorage, optimization
---

## Defer State Reads to Usage Point

Don't subscribe to dynamic state (searchParams, localStorage) if you only read it inside callbacks.

### Incorrect (subscribes to all searchParams changes)

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const searchParams = useSearchParams()

  const handleShare = () => {
    const ref = searchParams.get('ref')
    shareChat(chatId, { ref })
  }

  return <button onClick={handleShare}>Share</button>
}
```

#### Correct (reads on demand, no subscription)

```tsx
function ShareButton({ chatId }: { chatId: string }) {
  const handleShare = () => {
    const params = new URLSearchParams(window.location.search)
    const ref = params.get('ref')
    shareChat(chatId, { ref })
  }

  return <button onClick={handleShare}>Share</button>
}
```

## When to use

Use when the user asks about or needs: Defer State Reads to Usage Point.
﻿---
name: Preload Based on User Intent
description: ## Preload Based on User Intent
 
 Preload heavy bundles before they're needed to reduce perceived latency.
tags: bundle, preload, user-intent, hover
---

## Preload Based on User Intent

Preload heavy bundles before they're needed to reduce perceived latency.

### Example (preload on hover/focus)

```tsx
function EditorButton({ onClick }: { onClick: () => void }) {
  const preload = () => {
    if (typeof window !== 'undefined') {
      void import('./monaco-editor')
    }
  }

  return (
    <button
      onMouseEnter={preload}
      onFocus={preload}
      onClick={onClick}
    >
      Open Editor
    </button>
  )
}
```

#### Example (preload when feature flag is enabled)

```tsx
function FlagsProvider({ children, flags }: Props) {
  useEffect(() => {
    if (flags.editorEnabled && typeof window !== 'undefined') {
      void import('./monaco-editor').then(mod => mod.init())
    }
  }, [flags.editorEnabled])

  return <FlagsContext.Provider value={flags}>
    {children}
  </FlagsContext.Provider>
}
```

The `typeof window !== 'undefined'` check prevents bundling preloaded modules for SSR, optimizing server bundle size and build speed.

## When to use

Use when the user asks about or needs: Preload Based on User Intent.


---

<!-- merged from: create-explicit-component-variants.md -->

﻿---
name: Create Explicit Component Variants
description: ## Create Explicit Component Variants
 
 Instead of one component with many boolean props, create explicit variant
tags: composition, variants, architecture
---

## Create Explicit Component Variants

Instead of one component with many boolean props, create explicit variant
components. Each variant composes the pieces it needs. The code documents
itself.

### Incorrect (one component, many modes)

```tsx
// What does this component actually render?
<Composer
  isThread
  isEditing={false}
  channelId='abc'
  showAttachments
  showFormatting={false}
/>
```

#### Correct (explicit variants)

```tsx
// Immediately clear what this renders
<ThreadComposer channelId="abc" />

// Or
<EditMessageComposer messageId="xyz" />

// Or
<ForwardMessageComposer messageId="123" />
```

Each implementation is unique, explicit and self-contained. Yet they can each
use shared parts.

#### Implementation

```tsx
function ThreadComposer({ channelId }: { channelId: string }) {
  return (
    <ThreadProvider channelId={channelId}>
      <Composer.Frame>
        <Composer.Input />
        <AlsoSendToChannelField channelId={channelId} />
        <Composer.Footer>
          <Composer.Formatting />
          <Composer.Emojis />
          <Composer.Submit />
        </Composer.Footer>
      </Composer.Frame>
    </ThreadProvider>
  )
}

function EditMessageComposer({ messageId }: { messageId: string }) {
  return (
    <EditMessageProvider messageId={messageId}>
      <Composer.Frame>
        <Composer.Input />
        <Composer.Footer>
          <Composer.Formatting />
          <Composer.Emojis />
          <Composer.CancelEdit />
          <Composer.SaveEdit />
        </Composer.Footer>
      </Composer.Frame>
    </EditMessageProvider>
  )
}

function ForwardMessageComposer({ messageId }: { messageId: string }) {
  return (
    <ForwardMessageProvider messageId={messageId}>
      <Composer.Frame>
        <Composer.Input placeholder="Add a message, if you'd like." />
        <Composer.Footer>
          <Composer.Formatting />
          <Composer.Emojis />
          <Composer.Mentions />
        </Composer.Footer>
      </Composer.Frame>
    </ForwardMessageProvider>
  )
}
```

Each variant is explicit about:

- What provider/state it uses
- What UI elements it includes
- What actions are available

No boolean prop combinations to reason about. No impossible states.


---

<!-- merged from: prevent-hydration-mismatch-without-flickering.md -->

﻿---
name: Prevent Hydration Mismatch Without Flickering
description: ## Prevent Hydration Mismatch Without Flickering
 
 When rendering content that depends on client-side storage (localStorage, cookies), avoid both SSR breakage and post-hydration flickering by injecting a synchronous script that updates the DOM before React hydrates.
tags: rendering, ssr, hydration, localStorage, flicker
---

## Prevent Hydration Mismatch Without Flickering

When rendering content that depends on client-side storage (localStorage, cookies), avoid both SSR breakage and post-hydration flickering by injecting a synchronous script that updates the DOM before React hydrates.

### Incorrect (breaks SSR)

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  // localStorage is not available on server - throws error
  const theme = localStorage.getItem('theme') || 'light'
  
  return (
    <div className={theme}>
      {children}
    </div>
  )
}
```

Server-side rendering will fail because `localStorage` is undefined.

#### Incorrect (visual flickering)

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState('light')
  
  useEffect(() => {
    // Runs after hydration - causes visible flash
    const stored = localStorage.getItem('theme')
    if (stored) {
      setTheme(stored)
    }
  }, [])
  
  return (
    <div className={theme}>
      {children}
    </div>
  )
}
```

Component first renders with default value (`light`), then updates after hydration, causing a visible flash of incorrect content.

#### Correct (no flicker, no hydration mismatch)

```tsx
function ThemeWrapper({ children }: { children: ReactNode }) {
  return (
    <>
      <div id="theme-wrapper">
        {children}
      </div>
      <script
        dangerouslySetInnerHTML={{
          __html: `
            (function() {
              try {
                var theme = localStorage.getItem('theme') || 'light';
                var el = document.getElementById('theme-wrapper');
                if (el) el.className = theme;
              } catch (e) {}
            })();
          `,
        }}
      />
    </>
  )
}
```

The inline script executes synchronously before showing the element, ensuring the DOM already has the correct value. No flickering, no hydration mismatch.

This pattern is especially useful for theme toggles, user preferences, authentication states, and any client-only data that should render immediately without flashing default values.


---

<!-- merged from: dynamic-imports-for-heavy-components.md -->

﻿---
name: Dynamic Imports for Heavy Components
description: ## Dynamic Imports for Heavy Components
 
 Use `next/dynamic` to lazy-load large components not needed on initial render.
tags: bundle, dynamic-import, code-splitting, next-dynamic
---

## Dynamic Imports for Heavy Components

Use `next/dynamic` to lazy-load large components not needed on initial render.

### Incorrect (Monaco bundles with main chunk ~300KB)

```tsx
import { MonacoEditor } from './monaco-editor'

function CodePanel({ code }: { code: string }) {
  return <MonacoEditor value={code} />
}
```

#### Correct (Monaco loads on demand)

```tsx
import dynamic from 'next/dynamic'

const MonacoEditor = dynamic(
  () => import('./monaco-editor').then(m => m.MonacoEditor),
  { ssr: false }
)

function CodePanel({ code }: { code: string }) {
  return <MonacoEditor value={code} />
}
```


---

<!-- merged from: conditional-module-loading.md -->

﻿---
name: Conditional Module Loading
description: ## Conditional Module Loading
 
 Load large data or modules only when a feature is activated.
tags: bundle, conditional-loading, lazy-loading
---

## Conditional Module Loading

Load large data or modules only when a feature is activated.

### Example (lazy-load animation frames)

```tsx
function AnimationPlayer({ enabled, setEnabled }: { enabled: boolean; setEnabled: React.Dispatch<React.SetStateAction<boolean>> }) {
  const [frames, setFrames] = useState<Frame[] | null>(null)

  useEffect(() => {
    if (enabled && !frames && typeof window !== 'undefined') {
      import('./animation-frames.js')
        .then(mod => setFrames(mod.frames))
        .catch(() => setEnabled(false))
    }
  }, [enabled, frames, setEnabled])

  if (!frames) return <Skeleton />
  return <Canvas frames={frames} />
}
```

The `typeof window !== 'undefined'` check prevents bundling this module for SSR, optimizing server bundle size and build speed.


---

<!-- merged from: parallel-data-fetching-with-component-composition.md -->

﻿---
name: Parallel Data Fetching with Component Composition
description: ## Parallel Data Fetching with Component Composition
 
 React Server Components execute sequentially within a tree. Restructure with composition to parallelize data fetching.
tags: server, rsc, parallel-fetching, composition
---

## Parallel Data Fetching with Component Composition

React Server Components execute sequentially within a tree. Restructure with composition to parallelize data fetching.

### Incorrect (Sidebar waits for Page's fetch to complete)

```tsx
export default async function Page() {
  const header = await fetchHeader()
  return (
    <div>
      <div>{header}</div>
      <Sidebar />
    </div>
  )
}

async function Sidebar() {
  const items = await fetchSidebarItems()
  return <nav>{items.map(renderItem)}</nav>
}
```

#### Correct (both fetch simultaneously)

```tsx
async function Header() {
  const data = await fetchHeader()
  return <div>{data}</div>
}

async function Sidebar() {
  const items = await fetchSidebarItems()
  return <nav>{items.map(renderItem)}</nav>
}

export default function Page() {
  return (
    <div>
      <Header />
      <Sidebar />
    </div>
  )
}
```

#### Alternative with children prop

```tsx
async function Header() {
  const data = await fetchHeader()
  return <div>{data}</div>
}

async function Sidebar() {
  const items = await fetchSidebarItems()
  return <nav>{items.map(renderItem)}</nav>
}

function Layout({ children }: { children: ReactNode }) {
  return (
    <div>
      <Header />
      {children}
    </div>
  )
}

export default function Page() {
  return (
    <Layout>
      <Sidebar />
    </Layout>
  )
}
```


---

<!-- merged from: never-use-with-potentially-falsy-values.md -->

﻿---
name: Never Use && with Potentially Falsy Values
description: ## Never Use && with Potentially Falsy Values
 
 Never use `{value && <Component />}` when `value` could be an empty string or
tags: rendering, conditional, jsx, crash
---

## Never Use && with Potentially Falsy Values

Never use `{value && <Component />}` when `value` could be an empty string or
`0`. These are falsy but JSX-renderable—React Native will try to render them as
text outside a `<Text>` component, causing a hard crash in production.

### Incorrect (crashes if count is 0 or name is "")

```tsx
function Profile({ name, count }: { name: string; count: number }) {
  return (
    <View>
      {name && <Text>{name}</Text>}
      {count && <Text>{count} items</Text>}
    </View>
  )
}
// If name="" or count=0, renders the falsy value → crash
```

#### Correct (ternary with null)

```tsx
function Profile({ name, count }: { name: string; count: number }) {
  return (
    <View>
      {name ? <Text>{name}</Text> : null}
      {count ? <Text>{count} items</Text> : null}
    </View>
  )
}
```

#### Correct (explicit boolean coercion)

```tsx
function Profile({ name, count }: { name: string; count: number }) {
  return (
    <View>
      {!!name && <Text>{name}</Text>}
      {!!count && <Text>{count} items</Text>}
    </View>
  )
}
```

#### Best (early return)

```tsx
function Profile({ name, count }: { name: string; count: number }) {
  if (!name) return null

  return (
    <View>
      <Text>{name}</Text>
      {count > 0 ? <Text>{count} items</Text> : null}
    </View>
  )
}
```

Early returns are clearest. When using conditionals inline, prefer ternary or
explicit boolean checks.

**Lint rule:** Enable `react/jsx-no-leaked-render` from
[eslint-plugin-react](https://github.com/jsx-eslint/eslint-plugin-react/blob/master/docs/rules/jsx-no-leaked-render.md)
to catch this automatically.


---

<!-- merged from: import-from-design-system-folder.md -->

﻿---
name: Import from Design System Folder
description: ## Import from Design System Folder
 
 Re-export dependencies from a design system folder. App code imports from there,
tags: imports, architecture, design-system
---

## Import from Design System Folder

Re-export dependencies from a design system folder. App code imports from there,
not directly from packages. This enables global changes and easy refactoring.

### Incorrect (imports directly from package)

```tsx
import { View, Text } from 'react-native'
import { Button } from '@ui/button'

function Profile() {
  return (
    <View>
      <Text>Hello</Text>
      <Button>Save</Button>
    </View>
  )
}
```

#### Correct (imports from design system)

```tsx
// components/view.tsx
import { View as RNView } from 'react-native'

// ideal: pick the props you will actually use to control implementation
export function View(
  props: Pick<React.ComponentProps<typeof RNView>, 'style' | 'children'>
) {
  return <RNView {...props} />
}
```

```tsx
// components/text.tsx
export { Text } from 'react-native'
```

```tsx
// components/button.tsx
export { Button } from '@ui/button'
```

```tsx
import { View } from '@/components/view'
import { Text } from '@/components/text'
import { Button } from '@/components/button'

function Profile() {
  return (
    <View>
      <Text>Hello</Text>
      <Button>Save</Button>
    </View>
  )
}
```

Start by simply re-exporting. Customize later without changing app code.


---

<!-- merged from: measuring-view-dimensions.md -->

﻿---
name: Measuring View Dimensions
description: ## Measuring View Dimensions
 
 Use both `useLayoutEffect` (synchronous) and `onLayout` (for updates). The sync
tags: layout, measurement, onLayout, useLayoutEffect
---

## Measuring View Dimensions

Use both `useLayoutEffect` (synchronous) and `onLayout` (for updates). The sync
measurement gives you the initial size immediately; `onLayout` keeps it current
when the view changes. For non-primitive states, use a dispatch updater to
compare values and avoid unnecessary re-renders.

### Height only

```tsx
import { useLayoutEffect, useRef, useState } from 'react'
import { View, LayoutChangeEvent } from 'react-native'

function MeasuredBox({ children }: { children: React.ReactNode }) {
  const ref = useRef<View>(null)
  const [height, setHeight] = useState<number | undefined>(undefined)

  useLayoutEffect(() => {
    // Sync measurement on mount (RN 0.82+)
    const rect = ref.current?.getBoundingClientRect()
    if (rect) setHeight(rect.height)
    // Pre-0.82: ref.current?.measure((x, y, w, h) => setHeight(h))
  }, [])

  const onLayout = (e: LayoutChangeEvent) => {
    setHeight(e.nativeEvent.layout.height)
  }

  return (
    <View ref={ref} onLayout={onLayout}>
      {children}
    </View>
  )
}
```

#### Both dimensions

```tsx
import { useLayoutEffect, useRef, useState } from 'react'
import { View, LayoutChangeEvent } from 'react-native'

type Size = { width: number; height: number }

function MeasuredBox({ children }: { children: React.ReactNode }) {
  const ref = useRef<View>(null)
  const [size, setSize] = useState<Size | undefined>(undefined)

  useLayoutEffect(() => {
    const rect = ref.current?.getBoundingClientRect()
    if (rect) setSize({ width: rect.width, height: rect.height })
  }, [])

  const onLayout = (e: LayoutChangeEvent) => {
    const { width, height } = e.nativeEvent.layout
    setSize((prev) => {
      // for non-primitive states, compare values before firing a re-render
      if (prev?.width === width && prev?.height === height) return prev
      return { width, height }
    })
  }

  return (
    <View ref={ref} onLayout={onLayout}>
      {children}
    </View>
  )
}
```

Use functional setState to compare—don't read state directly in the callback.


---

<!-- merged from: frontend-design.md -->

﻿---
name: frontend-design
description: Create distinctive, production-grade frontend interfaces with high design quality. Use this skill when the user asks to build web components, pages, artifacts, posters, or applications (examples include websites, landing pages, dashboards, React components, HTML/CSS layouts, or when styling/beautifying any web UI). Generates creative, polished code and UI design that avoids generic AI aesthetics.
---

This skill guides creation of distinctive, production-grade frontend interfaces that avoid generic "AI slop" aesthetics. Implement real working code with exceptional attention to aesthetic details and creative choices.

The user provides frontend requirements: a component, page, application, or interface to build. They may include context about the purpose, audience, or technical constraints.

## Design Thinking

Before coding, understand the context and commit to a BOLD aesthetic direction:

- **Purpose**: What problem does this interface solve? Who uses it?
- **Tone**: Pick an extreme: brutally minimal, maximalist chaos, retro-futuristic, organic/natural, luxury/refined, playful/toy-like, editorial/magazine, brutalist/raw, art deco/geometric, soft/pastel, industrial/utilitarian, etc. There are so many flavors to choose from. Use these for inspiration but design one that is true to the aesthetic direction.
- **Constraints**: Technical requirements (framework, performance, accessibility).
- **Differentiation**: What makes this UNFORGETTABLE? What's the one thing someone will remember?

**CRITICAL**: Choose a clear conceptual direction and execute it with precision. Bold maximalism and refined minimalism both work - the key is intentionality, not intensity.

Then implement working code (HTML/CSS/JS, React, Vue, etc.) that is:

- Production-grade and functional
- Visually striking and memorable
- Cohesive with a clear aesthetic point-of-view
- Meticulously refined in every detail

## Frontend Aesthetics Guidelines

Focus on:

- **Typography**: Choose fonts that are beautiful, unique, and interesting. Avoid generic fonts like Arial and Inter; opt instead for distinctive choices that elevate the frontend's aesthetics; unexpected, characterful font choices. Pair a distinctive display font with a refined body font.
- **Color & Theme**: Commit to a cohesive aesthetic. Use CSS variables for consistency. Dominant colors with sharp accents outperform timid, evenly-distributed palettes.
- **Motion**: Use animations for effects and micro-interactions. Prioritize CSS-only solutions for HTML. Use Motion library for React when available. Focus on high-impact moments: one well-orchestrated page load with staggered reveals (animation-delay) creates more delight than scattered micro-interactions. Use scroll-triggering and hover states that surprise.
- **Spatial Composition**: Unexpected layouts. Asymmetry. Overlap. Diagonal flow. Grid-breaking elements. Generous negative space OR controlled density.
- **Backgrounds & Visual Details**: Create atmosphere and depth rather than defaulting to solid colors. Add contextual effects and textures that match the overall aesthetic. Apply creative forms like gradient meshes, noise textures, geometric patterns, layered transparencies, dramatic shadows, decorative borders, custom cursors, and grain overlays.

NEVER use generic AI-generated aesthetics like overused font families (Inter, Roboto, Arial, system fonts), cliched color schemes (particularly purple gradients on white backgrounds), predictable layouts and component patterns, and cookie-cutter design that lacks context-specific character.

Interpret creatively and make unexpected choices that feel genuinely designed for the context. No design should be the same. Vary between light and dark themes, different fonts, different aesthetics. NEVER converge on common choices (Space Grotesk, for example) across generations.

**IMPORTANT**: Match implementation complexity to the aesthetic vision. Maximalist designs need elaborate code with extensive animations and effects. Minimalist or refined designs need restraint, precision, and careful attention to spacing, typography, and subtle details. Elegance comes from executing the vision well.

Remember: Claude is capable of extraordinary creative work. Don't hold back, show what can truly be created when thinking outside the box and committing fully to a distinctive vision.


---

<!-- merged from: interactive-state-sync-patterns.md -->

﻿---
name: Interactive State Sync Patterns
description: # Interactive State Sync Patterns
 
 Use this reference when building ChatGPT apps with long-lived widget state, repeated interactions, or component-initiated tool calls (for example: games, boards, maps, dashboards, editors, or realtime-ish UIs).
---

# Interactive State Sync Patterns

Use this reference when building ChatGPT apps with long-lived widget state, repeated interactions, or component-initiated tool calls (for example: games, boards, maps, dashboards, editors, or realtime-ish UIs).

Do not load this file for simple read-only render apps unless state sync behavior is part of the task.

## When This Reference Helps

Read this file when the app needs one or more of these patterns:

- Repeated actions that may return similar data (retry, refresh, reset, reroll)
- UI controls that trigger tool calls after the initial render
- Local widget behavior that should also work outside ChatGPT during development
- Multiple tool calls updating one mounted widget over time
- Clear separation between model-visible state and widget-only state

## Reusable Patterns

### 1. Snapshot + Event Token

Return a stable state snapshot in `structuredContent` and add a monotonic event token for repeated actions that may not change other fields.

Examples:

- `stateVersion`
- `refreshCount`
- `resetCount`
- `lastMutationId`

Use this when the widget must detect "same shape, new event" updates reliably.

### 2. Intent-Focused Tool Surface

Prefer small, explicit tools that map to user-visible actions or data operations.

- Keep names action-oriented
- Use enums and bounded schemas where possible
- Avoid kitchen-sink tools that mix unrelated reads and writes

This improves model tool selection and reduces malformed calls.

### 3. Idempotent Handlers (or Explicitly Non-Idempotent)

Design handlers to tolerate retries. If a tool is not idempotent, make the side effect explicit and confirm intent in the flow.

- Reads and pure transforms should usually be idempotent
- Writes should include clear impact hints and current-turn confirmation where needed
- Repeated calls with the same input should not corrupt widget state

### 4. `structuredContent` / `_meta` Partitioning

Partition payloads intentionally:

- `structuredContent`: concise model-visible state the widget also uses
- `content`: short narration/status text
- `_meta`: large maps, caches, or sensitive widget-only hydration data

Keep `structuredContent` small enough for follow-up reasoning and chaining.

### 5. MCP Apps Bridge First, `window.openai` Second

For new scaffolds:

- Prefer MCP Apps bridge notifications and `tools/call` (portable across hosts)
- Use `window.openai` as a compatibility layer plus optional ChatGPT extensions

This keeps the app portable while still enabling ChatGPT-specific capabilities when helpful.

### 6. Component-Initiated Tool Calls Without Remounting

For interactive widgets, allow the UI to call data/action tools directly and update the existing widget state instead of forcing a full re-render/remount every time.

This is especially useful for:

- Refresh
- Retry
- Rerun
- Toggle/filter actions
- Incremental interactions inside one widget session

### 7. Standalone / No-Host Fallback Mode

When feasible, make the widget usable without ChatGPT during development:

- If host APIs are unavailable, apply local state directly
- Preserve basic interactions in a normal browser

This speeds up front-end iteration and reduces dependence on connector setup for every UI tweak.

### 8. Decouple Data Tools from Render Tools (When Complexity Grows)

Use separate data and render tools when the app has multi-step reasoning or frequent updates.

- Data tools fetch/compute/mutate and return reusable `structuredContent`
- Render tools attach the widget template and focus on presentation

This reduces unnecessary remounts and gives the model a chance to refine data before rendering.

## Common Anti-Patterns

- Putting large widget-only blobs into `structuredContent`
- Attaching a widget template to every tool when only one render tool needs it
- Using hidden client-side state as the source of truth for critical actions
- Depending only on `window.openai` APIs for baseline app behavior
- Using ambiguous tool names that do not match user intent

## Example App Types That Benefit From These Patterns

- Multiplayer or turn-based games
- Collaborative boards / task views
- Maps with filters and repeated searches
- Dashboards with refresh and drill-down actions
- Editors or builders with iterative tool calls


---

<!-- merged from: defer-non-critical-third-party-libraries.md -->

﻿---
name: Defer Non-Critical Third-Party Libraries
description: ## Defer Non-Critical Third-Party Libraries
 
 Analytics, logging, and error tracking don't block user interaction. Load them after hydration.
tags: bundle, third-party, analytics, defer
---

## Defer Non-Critical Third-Party Libraries

Analytics, logging, and error tracking don't block user interaction. Load them after hydration.

### Incorrect (blocks initial bundle)

```tsx
import { Analytics } from '@vercel/analytics/react'

export default function RootLayout({ children }) {
  return (
    <html>
      <body>
        {children}
        <Analytics />
      </body>
    </html>
  )
}
```

#### Correct (loads after hydration)

```tsx
import dynamic from 'next/dynamic'

const Analytics = dynamic(
  () => import('@vercel/analytics/react').then(m => m.Analytics),
  { ssr: false }
)

export default function RootLayout({ children }) {
  return (
    <html>
      <body>
        {children}
        <Analytics />
      </body>
    </html>
  )
}
```


---

<!-- merged from: install-native-dependencies-in-app-directory.md -->

﻿---
name: Install Native Dependencies in App Directory
description: ## Install Native Dependencies in App Directory
 
 In a monorepo, packages with native code must be installed in the native app's
tags: monorepo, native, autolinking, installation
---

## Install Native Dependencies in App Directory

In a monorepo, packages with native code must be installed in the native app's
directory directly. Autolinking only scans the app's `node_modules`—it won't
find native dependencies installed in other packages.

### Incorrect (native dep in shared package only)

```text
packages/
  ui/
    package.json  # has react-native-reanimated
  app/
    package.json  # missing react-native-reanimated
```

Autolinking fails—native code not linked.

#### Correct (native dep in app directory)

```text
packages/
  ui/
    package.json  # has react-native-reanimated
  app/
    package.json  # also has react-native-reanimated
```

```json
// packages/app/package.json
{
  "dependencies": {
    "react-native-reanimated": "3.16.1"
  }
}
```

Even if the shared package uses the native dependency, the app must also list it
for autolinking to detect and link the native code.