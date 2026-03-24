---
name: MEGA-JS-PERFORMANCE
description: Consolidated JavaScript/TypeScript performance patterns, async optimization, advanced types, and streaming patterns.
domain: frontend
triggers: js performance, typescript, promise.all, array iteration, event listeners, parallelization, advanced types, streaming
---

# MEGA-JS-PERFORMANCE

Consolidated JavaScript/TypeScript performance patterns, async optimization, advanced types, and streaming.


---

<!-- merged from: combine-multiple-array-iterations.md -->

﻿---
name: Combine Multiple Array Iterations
description: ## Combine Multiple Array Iterations
 
 Multiple `.filter()` or `.map()` calls iterate the array multiple times. Combine into one loop.
tags: javascript, arrays, loops, performance
---

## Combine Multiple Array Iterations

Multiple `.filter()` or `.map()` calls iterate the array multiple times. Combine into one loop.

### Incorrect (3 iterations)

```typescript
const admins = users.filter(u => u.isAdmin)
const testers = users.filter(u => u.isTester)
const inactive = users.filter(u => !u.isActive)
```

#### Correct (1 iteration)

```typescript
const admins: User[] = []
const testers: User[] = []
const inactive: User[] = []

for (const user of users) {
  if (user.isAdmin) admins.push(user)
  if (user.isTester) testers.push(user)
  if (!user.isActive) inactive.push(user)
}
```


---

<!-- merged from: promiseall-for-independent-operations.md -->

﻿---
name: Promise.all() for Independent Operations
description: ## Promise.all() for Independent Operations
 
 When async operations have no interdependencies, execute them concurrently using `Promise.all()`.
tags: async, parallelization, promises, waterfalls
---

## Promise.all() for Independent Operations

When async operations have no interdependencies, execute them concurrently using `Promise.all()`.

### Incorrect (sequential execution, 3 round trips)

```typescript
const user = await fetchUser()
const posts = await fetchPosts()
const comments = await fetchComments()
```

#### Correct (parallel execution, 1 round trip)

```typescript
const [user, posts, comments] = await Promise.all([
  fetchUser(),
  fetchPosts(),
  fetchComments()
])
```


---

<!-- merged from: deduplicate-global-event-listeners.md -->

﻿---
name: Deduplicate Global Event Listeners
description: ## Deduplicate Global Event Listeners
 
 Use `useSWRSubscription()` to share global event listeners across component instances.
tags: client, swr, event-listeners, subscription
---

## Deduplicate Global Event Listeners

Use `useSWRSubscription()` to share global event listeners across component instances.

### Incorrect (N instances = N listeners)

```tsx
function useKeyboardShortcut(key: string, callback: () => void) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === key) {
        callback()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [key, callback])
}
```

When using the `useKeyboardShortcut` hook multiple times, each instance will register a new listener.

#### Correct (N instances = 1 listener)

```tsx
import useSWRSubscription from 'swr/subscription'

// Module-level Map to track callbacks per key
const keyCallbacks = new Map<string, Set<() => void>>()

function useKeyboardShortcut(key: string, callback: () => void) {
  // Register this callback in the Map
  useEffect(() => {
    if (!keyCallbacks.has(key)) {
      keyCallbacks.set(key, new Set())
    }
    keyCallbacks.get(key)!.add(callback)

    return () => {
      const set = keyCallbacks.get(key)
      if (set) {
        set.delete(callback)
        if (set.size === 0) {
          keyCallbacks.delete(key)
        }
      }
    }
  }, [key, callback])

  useSWRSubscription('global-keydown', () => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey && keyCallbacks.has(e.key)) {
        keyCallbacks.get(e.key)!.forEach(cb => cb())
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  })
}

function Profile() {
  // Multiple shortcuts will share the same listener
  useKeyboardShortcut('p', () => { /* ... */ }) 
  useKeyboardShortcut('k', () => { /* ... */ })
  // ...
}
```


---

<!-- merged from: dependency-based-parallelization.md -->

﻿---
name: Dependency-Based Parallelization
description: ## Dependency-Based Parallelization
 
 For operations with partial dependencies, use `better-all` to maximize parallelism. It automatically starts each task at the earliest possible moment.
tags: async, parallelization, dependencies, better-all
---

## Dependency-Based Parallelization

For operations with partial dependencies, use `better-all` to maximize parallelism. It automatically starts each task at the earliest possible moment.

### Incorrect (profile waits for config unnecessarily)

```typescript
const [user, config] = await Promise.all([
  fetchUser(),
  fetchConfig()
])
const profile = await fetchProfile(user.id)
```

#### Correct (config and profile run in parallel)

```typescript
import { all } from 'better-all'

const { user, config, profile } = await all({
  async user() { return fetchUser() },
  async config() { return fetchConfig() },
  async profile() {
    return fetchProfile((await this.$.user).id)
  }
})
```

#### Alternative without extra dependencies

We can also create all the promises first, and do `Promise.all()` at the end.

```typescript
const userPromise = fetchUser()
const profilePromise = userPromise.then(user => fetchProfile(user.id))

const [user, config, profile] = await Promise.all([
  userPromise,
  fetchConfig(),
  profilePromise
])
```

Reference: [https://github.com/shuding/better-all](https://github.com/shuding/better-all)


---

<!-- merged from: usestate-dispatch-updaters-for-state-that-depends-on-current-val.md -->

﻿---
name: useState Dispatch updaters for State That Depends on Current Value
description: ## Use Dispatch Updaters for State That Depends on Current Value
 
 When the next state depends on the current state, use a dispatch updater
tags: state, hooks, useState, callbacks
---

## Use Dispatch Updaters for State That Depends on Current Value

When the next state depends on the current state, use a dispatch updater
(`setState(prev => ...)`) instead of reading the state variable directly in a
callback. This avoids stale closures and ensures you're comparing against the
latest value.

### Incorrect (reads state directly)

```tsx
const [size, setSize] = useState<Size | undefined>(undefined)

const onLayout = (e: LayoutChangeEvent) => {
  const { width, height } = e.nativeEvent.layout
  // size may be stale in this closure
  if (size?.width !== width || size?.height !== height) {
    setSize({ width, height })
  }
}
```

#### Correct (dispatch updater)

```tsx
const [size, setSize] = useState<Size | undefined>(undefined)

const onLayout = (e: LayoutChangeEvent) => {
  const { width, height } = e.nativeEvent.layout
  setSize((prev) => {
    if (prev?.width === width && prev?.height === height) return prev
    return { width, height }
  })
}
```

Returning the previous value from the updater skips the re-render.

For primitive states, you don't need to compare values before firing a
re-render.

#### Incorrect (unnecessary comparison for primitive state)

```tsx
const [size, setSize] = useState<Size | undefined>(undefined)

const onLayout = (e: LayoutChangeEvent) => {
  const { width, height } = e.nativeEvent.layout
  setSize((prev) => (prev === width ? prev : width))
}
```

#### Correct (sets primitive state directly)

```tsx
const [size, setSize] = useState<Size | undefined>(undefined)

const onLayout = (e: LayoutChangeEvent) => {
  const { width, height } = e.nativeEvent.layout
  setSize(width)
}
```

However, if the next state depends on the current state, you should still use a
dispatch updater.

#### Incorrect (reads state directly from the callback)

```tsx
const [count, setCount] = useState(0)

const onTap = () => {
  setCount(count + 1)
}
```

#### Correct (dispatch updater) (2)

```tsx
const [count, setCount] = useState(0)

const onTap = () => {
  setCount((prev) => prev + 1)
}
```


---

<!-- merged from: typescript-advanced-types.md -->

# TypeScript Advanced Type System

## Conditional Types

Conditional types select one of two types based on a type relationship test:

```typescript
type IsString<T> = T extends string ? true : false;

type A = IsString<string>;  // true
type B = IsString<number>;  // false
```

The full syntax is `T extends U ? X : Y`. When `T` is assignable to `U`, the result is `X`; otherwise `Y`.

### Distributive Behavior

When a **naked** type parameter (bare `T`, not `T[]` or `Array<T>`) is used in a conditional type, TypeScript distributes the condition over each member of a union:

```typescript
type ToArray<T> = T extends any ? T[] : never;

type StrOrNumArray = ToArray<string | number>;
// Expands to: string[] | number[]
// NOT: (string | number)[]
```

To suppress distribution, wrap `T` in a tuple:

```typescript
type NoDistribute<T> = [T] extends [any] ? T[] : never;

type Together = NoDistribute<string | number>;
// (string | number)[]
```

### Conditional Types Over Unions

```typescript
type NonNullable<T> = T extends null | undefined ? never : T;

type Clean = NonNullable<string | null | undefined | number>;
// string | number
```

`never` in a union is dropped automatically, which makes it the standard way to filter union members.

---

## infer Keyword

`infer` declares a type variable that TypeScript infers when the condition matches. It only works inside the `extends` branch of a conditional type.

### Extracting Return Types

```typescript
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;

type Fn = (x: number) => string;
type R = ReturnType<Fn>;  // string
```

### Extracting Parameters

```typescript
type Parameters<T> = T extends (...args: infer P) => any ? P : never;

type P = Parameters<(a: string, b: number) => void>;
// [a: string, b: number]
```

### Promise Unwrapping (Awaited)

```typescript
type Awaited<T> =
  T extends null | undefined
    ? T
    : T extends object & { then(onfulfilled: infer F, ...args: any[]): any }
      ? F extends (value: infer V, ...args: any[]) => any
        ? Awaited<V>
        : never
      : T;

type Resolved = Awaited<Promise<Promise<string>>>;  // string
```

### Recursive Inference

```typescript
// Extract the element type of an arbitrarily nested array
type DeepElement<T> = T extends (infer E)[] ? DeepElement<E> : T;

type D = DeepElement<string[][][]>;  // string
```

### infer in Template Literal

```typescript
type TrimLeft<S extends string> =
  S extends ` ${infer Rest}` ? TrimLeft<Rest> : S;

type T = TrimLeft<"   hello">;  // "hello"
```

### Multiple infer in One Condition

```typescript
type Head<T extends any[]> = T extends [infer H, ...any[]] ? H : never;
type Tail<T extends any[]> = T extends [any, ...infer Rest] ? Rest : never;

type H = Head<[1, 2, 3]>;    // 1
type Tl = Tail<[1, 2, 3]>;   // [2, 3]
```

---

## Mapped Types

Mapped types iterate over a union of keys to produce a new object type.

```typescript
// Basic form
type Readonly<T> = {
  readonly [K in keyof T]: T[K];
};

type Partial<T> = {
  [K in keyof T]?: T[K];
};

type Required<T> = {
  [K in keyof T]-?: T[K];  // -? removes optionality
};
```

The `-?` modifier strips optional (`?`) from each property. Similarly `-readonly` strips `readonly`.

### Pick and Record

```typescript
type Pick<T, K extends keyof T> = {
  [P in K]: T[P];
};

type Record<K extends keyof any, V> = {
  [P in K]: V;
};
```

### as Clause — Key Remapping

The `as` clause remaps keys during iteration (TypeScript 4.1+):

```typescript
type Getters<T> = {
  [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

type G = Getters<{ name: string; age: number }>;
// { getName: () => string; getAge: () => number }
```

Filtering keys by returning `never` from the `as` clause:

```typescript
type OmitFunctions<T> = {
  [K in keyof T as T[K] extends Function ? never : K]: T[K];
};
```

### Mapped Type Over Union (not keyof)

```typescript
type EventMap<Events extends string> = {
  [E in Events]: { type: E; payload: unknown };
};

type AppEvents = EventMap<"click" | "focus" | "blur">;
```

---

## Template Literal Types

Template literal types construct string literal types using the same interpolation syntax as JavaScript template literals:

```typescript
type Direction = "North" | "South" | "East" | "West";
type EventName = `on${Direction}`;
// "onNorth" | "onSouth" | "onEast" | "onWest"
```

When multiple union types appear in interpolation positions, the result is the cross-product:

```typescript
type Row = "top" | "bottom";
type Col = "left" | "right";
type Corner = `${Row}-${Col}`;
// "top-left" | "top-right" | "bottom-left" | "bottom-right"
```

### Intrinsic String Manipulation Types

```typescript
type U = Uppercase<"hello">;       // "HELLO"
type L = Lowercase<"HELLO">;       // "hello"
type C = Capitalize<"hello">;      // "Hello"
type Uc = Uncapitalize<"Hello">;   // "hello"
```

These are compiler built-ins — they cannot be expressed with user-land types alone.

### Parsing Strings with infer

```typescript
type GetEventName<T extends string> =
  T extends `on${infer E}` ? Uncapitalize<E> : never;

type E = GetEventName<"onClick">;   // "click"
```

```typescript
// Split a dot-separated path into a tuple
type Split<S extends string, D extends string> =
  S extends `${infer Head}${D}${infer Tail}`
    ? [Head, ...Split<Tail, D>]
    : [S];

type Parts = Split<"a.b.c", ".">;  // ["a", "b", "c"]
```

---

## Recursive Types

TypeScript supports recursive type aliases (TS 3.7+, fully general in 4.1+).

### JSON Type

```typescript
type JSONPrimitive = string | number | boolean | null;
type JSONObject = { [key: string]: JSONValue };
type JSONArray = JSONValue[];
type JSONValue = JSONPrimitive | JSONObject | JSONArray;
```

### Deep Readonly

```typescript
type DeepReadonly<T> =
  T extends (infer E)[]
    ? ReadonlyArray<DeepReadonly<E>>
    : T extends object
      ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
      : T;
```

### Linked List

```typescript
type List<T> = { head: T; tail: List<T> } | null;

type NumberList = List<number>;
```

### Recursive Mapped Types

```typescript
type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K];
};
```

### Tuple Length and Arithmetic

```typescript
type Length<T extends any[]> = T["length"];
type L = Length<[1, 2, 3]>;  // 3

// Build a tuple of N elements (used for compile-time arithmetic)
type BuildTuple<N extends number, Acc extends unknown[] = []> =
  Acc["length"] extends N ? Acc : BuildTuple<N, [...Acc, unknown]>;

type Five = BuildTuple<5>;  // [unknown, unknown, unknown, unknown, unknown]
```

---

## Type Predicates

A type predicate narrows the type of a parameter in the true branch of a type guard.

```typescript
function isString(value: unknown): value is string {
  return typeof value === "string";
}

function process(value: string | number) {
  if (isString(value)) {
    value.toUpperCase();  // value: string here
  }
}
```

### Array Filter with Type Predicate

```typescript
function isDefined<T>(value: T | undefined | null): value is T {
  return value != null;
}

const items = [1, null, 2, undefined, 3];
const defined = items.filter(isDefined);  // number[]
```

### Assertion Functions

`asserts` throws at runtime if the condition fails and narrows the type for subsequent code:

```typescript
function assert(condition: unknown, msg?: string): asserts condition {
  if (!condition) throw new Error(msg ?? "Assertion failed");
}

function assertIsString(val: unknown): asserts val is string {
  if (typeof val !== "string") throw new TypeError("Expected string");
}

function example(x: unknown) {
  assertIsString(x);
  x.toUpperCase();  // x: string — narrowed after assertion
}
```

---

## Variance

Variance describes how subtyping relationships on type parameters relate to subtyping on the container type.

### Covariance

A type `F<T>` is **covariant** in `T` if `A extends B` implies `F<A> extends F<B>`. Most read-only positions are covariant.

```typescript
// Array<T> is covariant — string[] is assignable to readonly unknown[]
const strs: string[] = ["a"];
const vals: readonly unknown[] = strs;  // ok
```

### Contravariance

A type `F<T>` is **contravariant** in `T` if `A extends B` implies `F<B> extends F<A>`. Function parameters are contravariant.

```typescript
type Handler<T> = (val: T) => void;

// Handler<unknown> is assignable to Handler<string>
// because unknown is wider: a handler that accepts anything can handle strings
const handleUnknown: Handler<unknown> = (v) => console.log(v);
const handleStr: Handler<string> = handleUnknown;  // ok
```

### Function Parameter Bivariance

Without `--strictFunctionTypes`, TypeScript treats method parameters bivariantly (both co- and contravariant) for backward compatibility. With `--strictFunctionTypes`, function-typed properties written as `prop: (x: T) => void` are checked contravariantly; methods written with shorthand `method(x: T): void` are still bivariant.

```typescript
// strictFunctionTypes = true
type Fn = (x: string) => void;
const fn: Fn = (x: unknown) => {};  // ok (contravariant: wider param)
// const fn2: Fn = (x: number) => {};  // error
```

### Explicit Variance Annotations (TypeScript 4.7+)

```typescript
type Provider<out T> = () => T;      // covariant
type Consumer<in T> = (val: T) => void;  // contravariant
type Transformer<in T, out U> = (val: T) => U;  // contravariant in T, covariant in U
```

These are hints to the checker — TypeScript still verifies correctness.

---

## Utility Types Deep Dive

### ReturnType

```typescript
type ReturnType<T extends (...args: any) => any> =
  T extends (...args: any) => infer R ? R : any;
```

### Parameters

```typescript
type Parameters<T extends (...args: any) => any> =
  T extends (...args: infer P) => any ? P : never;
```

### ConstructorParameters and InstanceType

```typescript
type ConstructorParameters<T extends abstract new (...args: any) => any> =
  T extends abstract new (...args: infer P) => any ? P : never;

type InstanceType<T extends abstract new (...args: any) => any> =
  T extends abstract new (...args: any) => infer R ? R : any;
```

### Awaited

```typescript
// Recursively unwraps Promises (built-in since TS 4.5)
type Awaited<T> =
  T extends null | undefined ? T :
  T extends object & { then(onfulfilled: infer F, ...args: any[]): any }
    ? F extends (value: infer V, ...args: any[]) => any
      ? Awaited<V>
      : never
    : T;
```

### UnionToIntersection

This is a classic trick: function parameter positions are contravariant, so distributing over a union and collecting into a single infer forces an intersection.

```typescript
type UnionToIntersection<U> =
  (U extends any ? (x: U) => void : never) extends (x: infer I) => void
    ? I
    : never;

type UI = UnionToIntersection<{ a: 1 } | { b: 2 }>;
// { a: 1 } & { b: 2 }
```

### Exclude, Extract, Omit

```typescript
type Exclude<T, U> = T extends U ? never : T;
type Extract<T, U> = T extends U ? T : never;
type Omit<T, K extends keyof any> = Pick<T, Exclude<keyof T, K>>;
```

---

## const Type Parameters (TypeScript 5.0+)

Before TS 5.0, generic inference widened array and object literals:

```typescript
function identity<T>(val: T): T { return val; }

const v = identity(["a", "b"]);  // string[]  — widened
```

With `const` on the type parameter, TypeScript infers the narrowest literal type:

```typescript
function identity<const T>(val: T): T { return val; }

const v = identity(["a", "b"]);  // readonly ["a", "b"]  — literal
```

This is equivalent to adding `as const` at the call site but without requiring callers to remember.

```typescript
function makeRoute<const T extends { path: string; method: string }>(route: T): T {
  return route;
}

const r = makeRoute({ path: "/users", method: "GET" });
// r.path: "/users"   r.method: "GET"  — not widened to string
```

---

## satisfies Operator (TypeScript 4.9+)

`satisfies` validates that a value matches a type **without widening** the inferred type of the variable. The value retains its literal / more specific type while still being checked against the constraint.

```typescript
type ColorMap = Record<string, [number, number, number] | string>;

const palette = {
  red: [255, 0, 0],
  green: "#00ff00",
  blue: [0, 0, 255],
} satisfies ColorMap;

// palette.green is still `string`, not `[number,number,number] | string`
// palette.red is still `[number, number, number]`
palette.green.toUpperCase();  // ok — type preserved as string
palette.red.map(x => x * 2); // ok — type preserved as number[]
```

Without `satisfies`, typing `palette: ColorMap` would widen every value to `[number,number,number] | string`.

---

## NoInfer Utility (TypeScript 5.4+)

`NoInfer<T>` prevents TypeScript from using a given argument position as an inference site for a type parameter. The type must be inferred from other positions first, then the `NoInfer` position is only checked for compatibility.

```typescript
function createStore<T>(initial: T, fallback: NoInfer<T>): T {
  return initial ?? fallback;
}

// T is inferred as `string` from `initial`, NOT from `fallback`
createStore("hello", "world");  // ok
// createStore("hello", 42);    // error — 42 not assignable to string
```

Without `NoInfer`, TypeScript could widen `T` by unifying the types of both arguments.

```typescript
// Classic use case: default value must match inferred element type
function getOrDefault<T>(arr: T[], fallback: NoInfer<T>): T {
  return arr.length > 0 ? arr[0] : fallback;
}

getOrDefault([1, 2, 3], 0);      // ok, T = number
// getOrDefault([1, 2, 3], "x"); // error
```

---

## Discriminated Unions

A discriminated union has a common **literal** property (the discriminant) that uniquely identifies each variant.

```typescript
type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "rect"; width: number; height: number }
  | { kind: "triangle"; base: number; height: number };

function area(s: Shape): number {
  switch (s.kind) {
    case "circle":   return Math.PI * s.radius ** 2;
    case "rect":     return s.width * s.height;
    case "triangle": return 0.5 * s.base * s.height;
  }
}
```

### Exhaustiveness Checking with never

```typescript
function assertNever(x: never): never {
  throw new Error(`Unhandled case: ${JSON.stringify(x)}`);
}

function area(s: Shape): number {
  switch (s.kind) {
    case "circle":   return Math.PI * s.radius ** 2;
    case "rect":     return s.width * s.height;
    case "triangle": return 0.5 * s.base * s.height;
    default:         return assertNever(s);  // error if a variant is missing
  }
}
```

Adding a new variant to `Shape` without handling it in `area` causes a compile error at `assertNever(s)` because `s` is no longer `never`.

---

## Critical Rules / Gotchas

### Distributive Conditional Requires a Naked Type Parameter

Distribution only happens when the type parameter appears bare — not wrapped:

```typescript
type Dist<T>   = T extends string ? "yes" : "no";
type NoDist<T> = [T] extends [string] ? "yes" : "no";

type A = Dist<string | number>;    // "yes" | "no"   (distributed)
type B = NoDist<string | number>;  // "no"            (not distributed)
```

### infer Scope is Local to the Branch

The type variable introduced by `infer` is only in scope in the true branch:

```typescript
type First<T> = T extends [infer H, ...any[]] ? H : never;
// H is not accessible outside ? H : never
```

### Circular Mapped Types and Depth Limits

TypeScript has an internal recursion limit (around 100 levels). Recursive types that are too deep produce a `Type instantiation is excessively deep` error. Tail-recursive patterns using tuple accumulation (as shown in `BuildTuple` above) can often sidestep this.

### type alias vs interface for Extension

Interfaces can be augmented (declaration merging); type aliases cannot:

```typescript
interface Animal { name: string; }
interface Animal { age: number; }  // ok — merges

type Creature = { name: string; };
// type Creature = { age: number; };  // error — duplicate identifier
```

Use `interface` for public API shapes that consumers may need to extend; use `type` for unions, intersections, conditional types, and mapped types.

### keyof on Union vs Intersection

`keyof (A | B)` gives keys common to both (intersection of key sets).
`keyof (A & B)` gives keys present in either (union of key sets).

```typescript
type A = { x: number; y: number };
type B = { y: string; z: boolean };

type KU = keyof (A | B);  // "y"          — only shared key
type KI = keyof (A & B);  // "x" | "y" | "z"
```

### Excess Property Checking Only at Assignment

Excess properties are only flagged when assigning an object literal directly. Widening via a variable bypasses the check:

```typescript
type Point = { x: number; y: number };

const p: Point = { x: 1, y: 2, z: 3 };  // error — excess 'z'

const obj = { x: 1, y: 2, z: 3 };
const p2: Point = obj;  // ok — structural subtype, not a fresh literal
```

---

## References

- TypeScript Handbook — Conditional Types: <https://www.typescriptlang.org/docs/handbook/2/conditional-types.html>
- TypeScript Handbook — Mapped Types: <https://www.typescriptlang.org/docs/handbook/2/mapped-types.html>
- TypeScript Handbook — Template Literal Types: <https://www.typescriptlang.org/docs/handbook/2/template-literal-types.html>
- TypeScript Handbook — Typeof / Keyof: <https://www.typescriptlang.org/docs/handbook/2/typeof-types.html>
- TypeScript 4.7 Release Notes (variance annotations): <https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-7.html>
- TypeScript 4.9 Release Notes (satisfies): <https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-9.html>
- TypeScript 5.0 Release Notes (const type parameters): <https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-0.html>
- TypeScript 5.4 Release Notes (NoInfer): <https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-4.html>


---

<!-- merged from: streaming-typescript.md -->

﻿---
name: Streaming — TypeScript
description: # Streaming — TypeScript
 
 ## Quick Start
---

# Streaming — TypeScript

## Quick Start (Streaming — TypeScript)

```typescript
const stream = client.messages.stream({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [{ role: "user", content: "Write a story" }],
});

for await (const event of stream) {
  if (
    event.type === "content_block_delta" &&
    event.delta.type === "text_delta"
  ) {
    process.stdout.write(event.delta.text);
  }
}
```

---

## Handling Different Content Types

> **Opus 4.6:** Use `thinking: {type: "adaptive"}`. On older models, use `thinking: {type: "enabled", budget_tokens: N}` instead.

```typescript
const stream = client.messages.stream({
  model: "claude-opus-4-6",
  max_tokens: 16000,
  thinking: { type: "adaptive" },
  messages: [{ role: "user", content: "Analyze this problem" }],
});

for await (const event of stream) {
  switch (event.type) {
    case "content_block_start":
      switch (event.content_block.type) {
        case "thinking":
          console.log("\n[Thinking...]");
          break;
        case "text":
          console.log("\n[Response:]");
          break;
      }
      break;
    case "content_block_delta":
      switch (event.delta.type) {
        case "thinking_delta":
          process.stdout.write(event.delta.thinking);
          break;
        case "text_delta":
          process.stdout.write(event.delta.text);
          break;
      }
      break;
  }
}
```

---

## Streaming with Tool Use (Tool Runner)

Use the tool runner with `stream: true`. The outer loop iterates over tool runner iterations (messages), the inner loop processes stream events:

```typescript
import Anthropic from "@anthropic-ai/sdk";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/beta/zod";
import { z } from "zod";

const client = new Anthropic();

const getWeather = betaZodTool({
  name: "get_weather",
  description: "Get current weather for a location",
  inputSchema: z.object({
    location: z.string().describe("City and state, e.g., San Francisco, CA"),
  }),
  run: async ({ location }) => `72°F and sunny in ${location}`,
});

const runner = client.beta.messages.toolRunner({
  model: "claude-opus-4-6",
  max_tokens: 4096,
  tools: [getWeather],
  messages: [
    { role: "user", content: "What's the weather in Paris and London?" },
  ],
  stream: true,
});

// Outer loop: each tool runner iteration
for await (const messageStream of runner) {
  // Inner loop: stream events for this iteration
  for await (const event of messageStream) {
    switch (event.type) {
      case "content_block_delta":
        switch (event.delta.type) {
          case "text_delta":
            process.stdout.write(event.delta.text);
            break;
          case "input_json_delta":
            // Tool input being streamed
            break;
        }
        break;
    }
  }
}
```

---

## Getting the Final Message

```typescript
const stream = client.messages.stream({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [{ role: "user", content: "Hello" }],
});

for await (const event of stream) {
  // Process events...
}

const finalMessage = await stream.finalMessage();
console.log(`Tokens used: ${finalMessage.usage.output_tokens}`);
```

---

## Stream Event Types

| Event Type | Description | When it fires |
| --------------------- | --------------------------- | --------------------------------- |
| `message_start` | Contains message metadata | Once at the beginning |
| `content_block_start` | New content block beginning | When a text/tool_use block starts |
| `content_block_delta` | Incremental content update | For each token/chunk |
| `content_block_stop` | Content block complete | When a block finishes |
| `message_delta` | Message-level updates | Contains `stop_reason`, usage |
| `message_stop` | Message complete | Once at the end |

## Best Practices

1. **Always flush output** — Use `process.stdout.write()` for immediate display
2. **Handle partial responses** — If the stream is interrupted, you may have incomplete content
3. **Track token usage** — The `message_delta` event contains usage information
4. **Use `finalMessage()`** — Get the complete `Anthropic.Message` object even when streaming. Don't wrap `.on()` events in `new Promise()` — `finalMessage()` handles all completion/error/abort states internally
5. **Buffer for web UIs** — Consider buffering a few tokens before rendering to avoid excessive DOM updates
6. **Use `stream.on("text", ...)` for deltas** — The `text` event provides just the delta string, simpler than manually filtering `content_block_delta` events
7. **For agentic loops with streaming** — See the [Streaming Manual Loop](./tool-use.md#streaming-manual-loop) section in tool-use.md for combining `stream()` + `finalMessage()` with a tool-use loop

## Raw SSE Format

If using raw HTTP (not SDKs), the stream returns Server-Sent Events:

```text
event: message_start
data: {"type":"message_start","message":{"id":"msg_...","type":"message",...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":12}}

event: message_stop
data: {"type":"message_stop"}
```