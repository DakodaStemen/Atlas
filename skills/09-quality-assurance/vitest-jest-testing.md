---
name: vitest-jest-testing
description: Comprehensive reference for Vitest and Jest testing in JavaScript/TypeScript projects. Covers test structure, mocking APIs, async patterns, React component and hook testing, snapshot testing, coverage configuration, and migration from Jest to Vitest.
domain: testing
category: unit
tags: [Vitest, Jest, unit-testing, mocking, TypeScript, coverage, React, async, snapshots, CI]
triggers: [vitest, jest, vi.mock, vi.fn, vi.spyOn, unit test, snapshot, coverage, testing hooks, test setup, mock module, test React]
---

# Vitest & Jest Testing Patterns

## Vitest vs Jest — Quick Comparison

| Feature | Vitest | Jest |
| --- | --- | --- |
| TypeScript support | Native, zero config | Requires `ts-jest` or Babel |
| ESM support | Native | Needs extra plugins/config |
| Config file | `vitest.config.ts` (shares Vite config) | Separate `jest.config.js` |
| Globals (`describe`, `it`, etc.) | Opt-in via `globals: true` | Auto-injected by default |
| Watch mode | HMR-based, reruns only affected tests | Full re-run unless `--watch` optimized |
| Browser testing | Built-in browser mode (v4+) | Requires Playwright/JSDOM separately |
| CSS handling | Native (`css: true`) | Needs `identity-obj-proxy` |
| Performance (large suite) | Up to 28× faster watch re-runs | Baseline |
| Mocking API | `vi.*` namespace | `jest.*` namespace |
| Ecosystem default | Nuxt, SvelteKit, Astro, Angular | Legacy / broad npm ecosystem |

**Rule of thumb:** New projects default to Vitest. Migrate an existing Jest suite if watch mode or CI times are a real pain point — not for its own sake.

---

## 1. Project Setup

### Vitest (`vitest.config.ts`)

```typescript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',          // 'node' for non-DOM code
    globals: true,                  // optional: avoids import boilerplate
    setupFiles: ['./vitest.setup.ts'],
    coverage: {
      provider: 'v8',               // or 'istanbul'
      reporter: ['text', 'html', 'lcov'],
      reportsDirectory: './coverage',
      thresholds: {
        lines: 80,
        branches: 75,
        functions: 80,
        statements: 80,
      },
      exclude: ['**/*.d.ts', 'src/generated/**', 'src/main.tsx'],
    },
  },
});
```

### Setup file (`vitest.setup.ts`)

```typescript
import '@testing-library/jest-dom/vitest'; // replaces @testing-library/jest-dom
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// React Testing Library cleanup after each test
afterEach(() => {
  cleanup();
});
```

### Jest (`jest.config.ts`)

```typescript
import type { Config } from 'jest';

const config: Config = {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  setupFilesAfterEach: ['<rootDir>/jest.setup.ts'],
  moduleNameMapper: {
    '\\.(css|less|scss)$': 'identity-obj-proxy',
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  collectCoverageFrom: ['src/**/*.{ts,tsx}', '!src/**/*.d.ts'],
  coverageThreshold: {
    global: { lines: 80, branches: 75 },
  },
};

export default config;
```

---

## 2. Test Structure (describe / it / expect)

Follow the **Arrange–Act–Assert** pattern. One logical assertion per test; group related tests in `describe` blocks.

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest'; // omit if globals: true

import { formatCurrency } from './formatCurrency';

describe('formatCurrency', () => {
  it('formats whole numbers without decimal places', () => {
    // Arrange
    const amount = 1000;
    // Act
    const result = formatCurrency(amount, 'USD');
    // Assert
    expect(result).toBe('$1,000.00');
  });

  it('handles negative values', () => {
    expect(formatCurrency(-50, 'USD')).toBe('-$50.00');
  });

  it('throws on non-numeric input', () => {
    expect(() => formatCurrency('abc' as any, 'USD')).toThrow(TypeError);
  });
});
```

### Parameterized tests (`test.each`)

```typescript
it.each([
  [1, 1, 2],
  [2, 3, 5],
  [0, -1, -1],
])('add(%i, %i) === %i', (a, b, expected) => {
  expect(add(a, b)).toBe(expected);
});
```

---

## 3. Mocking

### 3a. `vi.fn()` — create a mock function

```typescript
const mockCallback = vi.fn();

mockCallback('hello');

expect(mockCallback).toHaveBeenCalledTimes(1);
expect(mockCallback).toHaveBeenCalledWith('hello');

// Control return values
const mockFetch = vi.fn()
  .mockResolvedValueOnce({ data: 'first' })
  .mockResolvedValueOnce({ data: 'second' });
```

### 3b. `vi.mock()` — replace an entire module

`vi.mock` is **hoisted** to the top of the file automatically — it always runs before imports.

```typescript
import { describe, it, expect, vi } from 'vitest';
import { getUserProfile } from './userService';
import { fetchUser } from './api'; // will be replaced

vi.mock('./api', () => ({
  fetchUser: vi.fn().mockResolvedValue({ id: 1, name: 'Alice' }),
}));

describe('getUserProfile', () => {
  it('returns the user name', async () => {
    const result = await getUserProfile(1);
    expect(result.name).toBe('Alice');
    expect(fetchUser).toHaveBeenCalledWith(1);
  });
});
```

**Partial mock** — keep real exports, override only what you need:

```typescript
vi.mock('./utils', async (importOriginal) => {
  const real = await importOriginal<typeof import('./utils')>();
  return {
    ...real,
    expensiveOperation: vi.fn().mockReturnValue('cheap result'),
  };
});
```

### 3c. `vi.spyOn()` — observe a real method

Use spyOn when you want to track calls but preserve the original behaviour, or surgically override one method on an object.

```typescript
import * as fs from 'fs';

const spy = vi.spyOn(fs, 'readFileSync').mockReturnValue('mocked content');

myFunction(); // internally calls fs.readFileSync

expect(spy).toHaveBeenCalledOnce();

vi.restoreAllMocks(); // restore original readFileSync
```

#### TypeScript — use `vi.mocked()` for type inference

```typescript
import { vi, expect } from 'vitest';
import { fetchUser } from './api';

vi.mock('./api');

const mockedFetch = vi.mocked(fetchUser);
mockedFetch.mockResolvedValue({ id: 2, name: 'Bob' });
```

### 3d. `vi.stubGlobal()` — stub globals

```typescript
vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
  ok: true,
  json: async () => ({ result: 42 }),
}));

// auto-reset in afterEach with:
afterEach(() => vi.unstubAllGlobals());
```

### 3e. `vi.stubEnv()` — stub environment variables

```typescript
vi.stubEnv('NODE_ENV', 'production');
expect(import.meta.env.NODE_ENV).toBe('production');

afterEach(() => vi.unstubAllEnvs());
```

### Mock cleanup strategy

```typescript
// vitest.config.ts
test: {
  clearMocks: true,    // clears call history between tests (recommended)
  resetMocks: false,   // also resets implementation (use carefully)
  restoreMocks: false, // restores spied originals (use in afterEach instead)
}
```

Or explicitly in hooks:

```typescript
afterEach(() => {
  vi.clearAllMocks();
});

afterAll(() => {
  vi.restoreAllMocks();
});
```

---

## 4. Setup and Teardown

```typescript
describe('DatabaseService', () => {
  let db: DatabaseService;

  beforeAll(async () => {
    // runs once before the describe block
    db = await DatabaseService.connect(':memory:');
  });

  afterAll(async () => {
    await db.disconnect();
  });

  beforeEach(async () => {
    // runs before every test — seed clean state
    await db.seed(fixtures);
  });

  afterEach(async () => {
    await db.truncate();
  });

  it('inserts a record', async () => {
    const id = await db.insert({ name: 'test' });
    expect(id).toBeGreaterThan(0);
  });
});
```

---

## 5. Async Testing

### Promises and async/await

```typescript
it('resolves with user data', async () => {
  const user = await fetchUser(1);
  expect(user.name).toBe('Alice');
});

it('rejects when user not found', async () => {
  await expect(fetchUser(999)).rejects.toThrow('User not found');
});
```

### Fake timers

```typescript
import { vi, it, expect } from 'vitest';

it('fires callback after 1 second', async () => {
  vi.useFakeTimers();

  const callback = vi.fn();
  setTimeout(callback, 1000);

  await vi.advanceTimersByTimeAsync(1000);

  expect(callback).toHaveBeenCalledOnce();

  vi.useRealTimers();
});
```

### Mocking `Date`

```typescript
it('uses the mocked date', () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2024-01-15T12:00:00Z'));

  const result = getTodayLabel();
  expect(result).toBe('Monday, January 15');

  vi.useRealTimers();
});
```

---

## 6. Snapshot Testing

```typescript
import { it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Badge } from './Badge';

it('matches snapshot', () => {
  const { container } = render(<Badge label="New" variant="success" />);
  expect(container.firstChild).toMatchSnapshot();
});

// Inline snapshot — value stored in the test file itself
it('serializes config to JSON', () => {
  expect(buildConfig({ debug: true })).toMatchInlineSnapshot(`
    {
      "debug": true,
      "logLevel": "verbose",
    }
  `);
});
```

Update snapshots: `vitest --update-snapshots` / `jest --updateSnapshot`.

Snapshots are a safety net for unintentional changes, not a substitute for intent-driven assertions. Keep them focused on structure, not every implementation detail.

---

## 7. Component Testing with jsdom (React)

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LoginForm } from './LoginForm';

describe('LoginForm', () => {
  it('calls onSubmit with credentials', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(<LoginForm onSubmit={onSubmit} />);

    await user.type(screen.getByLabelText(/email/i), 'alice@example.com');
    await user.type(screen.getByLabelText(/password/i), 'hunter2');
    await user.click(screen.getByRole('button', { name: /sign in/i }));

    expect(onSubmit).toHaveBeenCalledWith({
      email: 'alice@example.com',
      password: 'hunter2',
    });
  });

  it('shows validation error on empty submit', async () => {
    const user = userEvent.setup();
    render(<LoginForm onSubmit={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: /sign in/i }));

    expect(screen.getByText(/email is required/i)).toBeInTheDocument();
  });
});
```

**Query priority** (prefer accessible queries):

1. `getByRole` — most robust
2. `getByLabelText`
3. `getByPlaceholderText`
4. `getByText`
5. `getByTestId` — last resort

---

## 8. Testing React Hooks

Use `@testing-library/react` `renderHook` (not the deprecated `@testing-library/react-hooks`):

```typescript
import { renderHook, act } from '@testing-library/react';
import { useCounter } from './useCounter';

describe('useCounter', () => {
  it('initialises with default value', () => {
    const { result } = renderHook(() => useCounter(10));
    expect(result.current.count).toBe(10);
  });

  it('increments the count', () => {
    const { result } = renderHook(() => useCounter(0));

    act(() => {
      result.current.increment();
    });

    expect(result.current.count).toBe(1);
  });
});
```

For hooks that fetch data, mock the network layer (not the hook internals):

```typescript
vi.mock('../api/users', () => ({
  useUsersQuery: vi.fn().mockReturnValue({
    data: [{ id: 1, name: 'Alice' }],
    isLoading: false,
  }),
}));
```

---

## 9. Coverage Configuration

### Run coverage

```bash
# Vitest
npx vitest run --coverage

# Jest
npx jest --coverage
```

### Vitest coverage config

```typescript
// vitest.config.ts
test: {
  coverage: {
    provider: 'v8',                  // 'istanbul' for more detailed branch tracking
    reporter: ['text', 'html', 'lcov'],
    reportsDirectory: './coverage',
    include: ['src/**/*.{ts,tsx}'],
    exclude: [
      'src/**/*.d.ts',
      'src/**/*.stories.tsx',
      'src/generated/**',
      'src/main.tsx',
    ],
    thresholds: {
      lines: 80,
      branches: 75,
      functions: 80,
      statements: 80,
    },
    // fail CI if thresholds not met
    thresholdAutoUpdate: false,
  },
}
```

**v8 vs Istanbul:** v8 uses V8's built-in coverage (faster, less setup), Istanbul instruments source (more accurate branch tracking, especially with complex ternaries). Istanbul is safer for strict branch coverage targets.

Focus coverage effort on business logic and edge cases. 100% coverage on trivial getters/setters is low value.

---

## 10. Migrating from Jest to Vitest

### Automated codemod (fastest path)

```bash
npx codemod jest/vitest
```

Converts `jest.*` → `vi.*`, updates imports, handles most boilerplate.

### Manual changes required

#### Dependencies

```bash
npm uninstall jest @types/jest ts-jest babel-jest
npm install -D vitest @vitest/coverage-v8
# if DOM testing:
npm install -D jsdom @testing-library/jest-dom
```

#### `package.json` scripts

```json
{
  "scripts": {
    "test": "vitest",
    "test:run": "vitest run",
    "test:coverage": "vitest run --coverage",
    "test:ui": "vitest --ui"
  }
}
```

#### Setup file — critical swap

```typescript
// jest.setup.ts (before)
import '@testing-library/jest-dom';

// vitest.setup.ts (after)
import '@testing-library/jest-dom/vitest';
```

#### API replacements

| Jest | Vitest |
| --- | --- |
| `jest.fn()` | `vi.fn()` |
| `jest.spyOn()` | `vi.spyOn()` |
| `jest.mock()` | `vi.mock()` |
| `jest.clearAllMocks()` | `vi.clearAllMocks()` |
| `jest.resetAllMocks()` | `vi.resetAllMocks()` |
| `jest.restoreAllMocks()` | `vi.restoreAllMocks()` |
| `jest.useFakeTimers()` | `vi.useFakeTimers()` |
| `jest.runAllTimers()` | `vi.runAllTimers()` |
| `jest.requireActual()` | `vi.importActual()` (async) |
| `jest.requireMock()` | `vi.importMock()` (async) |

**Globals** — Jest injects `describe`, `it`, `expect` automatically. Vitest requires either:

```typescript
// Option A: explicit import (preferred for clarity)
import { describe, it, expect, vi } from 'vitest';

// Option B: enable globals in config (mimics Jest behaviour)
// vitest.config.ts → test: { globals: true }
// then add to tsconfig: "types": ["vitest/globals"]
```

### Common gotchas

- `vi.mock()` factory functions cannot reference variables from outer scope (hoisting limitation). Use `vi.fn()` inline or `vi.hoisted()` for shared setup.
- CSS imports need `css: true` in config or a custom plugin — remove `identity-obj-proxy`.
- Path aliases from `tsconfig.json` must be mirrored in `resolve.alias` inside `vitest.config.ts` unless you use `vite-tsconfig-paths`.
- `@testing-library/jest-dom` matchers won't be typed correctly unless you import from the `/vitest` subpath.

---

## 11. CI Integration

### GitHub Actions

```yaml
- name: Run tests
  run: npx vitest run --coverage --reporter=junit --outputFile=test-results.xml

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v4
  with:
    files: ./coverage/lcov.info
```

### Fail CI on coverage threshold

Set `thresholds` in `vitest.config.ts` — Vitest exits with code 1 when any threshold is missed.

### Parallelism

Vitest runs test files in parallel by default (worker threads). For test suites that share heavy global state (e.g., a real database), use `pool: 'forks'` or `singleFork: true` to avoid conflicts:

```typescript
test: {
  pool: 'forks',       // process isolation vs thread isolation
  poolOptions: {
    forks: { singleFork: true },
  },
}
```

---

## 12. Performance Tips

- **Isolate slow dependencies first.** Profile with `--reporter=verbose` to find slow test files.
- **Avoid real I/O in unit tests.** Mock `fs`, `fetch`, database clients at the module boundary.
- **Use `vi.mock` sparingly.** Full module mocks add overhead and can mask coupling issues. Prefer dependency injection where practical.
- **`test.concurrent`** — marks tests inside a `describe` to run concurrently (same worker thread). Use only when tests are truly stateless.
- **`--bail=1`** — stop on first failure in CI to get fast feedback during broken builds.
- **`coverage.include`** — whitelist only your source files. Including `node_modules` or build output by accident is the most common cause of slow coverage runs.
- Keep `beforeAll` setup lightweight. Expensive DB seeding in `beforeAll` means a single flaky test blocks the whole suite.
