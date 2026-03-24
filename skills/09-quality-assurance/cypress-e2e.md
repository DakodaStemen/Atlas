---
name: cypress-e2e
description: Patterns, commands, and decision-making for Cypress end-to-end and component testing. Covers test structure, network stubbing with cy.intercept, fixture-driven data, custom commands, page object pattern, component vs E2E testing, Cypress Cloud parallelization, retry and flakiness reduction, CI setup, and a decision matrix against Playwright.
domain: testing
category: e2e
tags: [Cypress, E2E, component-testing, cy.intercept, fixtures, page-object, custom-commands, CI, Playwright]
triggers: [cypress, e2e testing, end-to-end test, component test, cy.intercept, cypress cloud, test flakiness, playwright vs cypress]
---

# Cypress E2E Testing

## Test Structure and Commands

Tests follow the standard Mocha describe/it structure. Each `it` block must be fully independent — change any single test to `it.only` and it must pass on its own. Never rely on execution order or shared mutable state between tests.

```js
describe('Checkout flow', () => {
  beforeEach(() => {
    // Reset state before each test, not after
    cy.task('db:seed')
    cy.login('user@example.com', 'password')
    cy.visit('/cart')
  })

  it('completes a purchase', () => {
    cy.getBySel('checkout-button').click()
    cy.getBySel('order-confirmation').should('be.visible')
    cy.getBySel('order-id').should('not.be.empty')
  })
})
```

**Why `beforeEach` not `afterEach` for cleanup:** Cypress can refresh the browser mid-test, so `afterEach` is not guaranteed to run. Put teardown logic in `beforeEach` so the next test always starts from a clean state.

**Combine assertions in one test.** The unit-test habit of one assertion per `it` block is an anti-pattern in Cypress. Resetting state between tests is expensive; if multiple assertions share the same setup they belong in the same test.

---

## Selector Strategy

Selectors tied to CSS classes, IDs, or DOM structure break when the UI changes. Use `data-*` attributes so test selectors are decoupled from styling and behavior.

### Priority order (best first)

1. `cy.get('[data-cy="submit"]')` — dedicated test attribute, immune to style changes
2. `cy.contains('Submit')` — only when the text itself is the contract
3. `cy.get('[name="submit"]')` — acceptable for form inputs
4. `cy.get('#submit')` — fragile; IDs repurposed during refactors
5. `cy.get('.btn-submit')` — breaks on any CSS refactor
6. `cy.get('button')` — useless without additional context

Add `getBySel` and `getBySelLike` as custom commands (see Custom Commands section) to enforce this convention across the whole suite.

---

## cy.intercept for Network Stubbing

`cy.intercept()` replaces the old `cy.route()` and gives full control over request/response at the network layer.

```js
// Stub a GET with fixture data
cy.intercept('GET', '/api/users', { fixture: 'users.json' }).as('getUsers')

// Stub with inline response
cy.intercept('POST', '/api/orders', {
  statusCode: 201,
  body: { id: 'ord_123', status: 'pending' },
}).as('createOrder')

// Modify a real response (pass-through with transformation)
cy.intercept('GET', '/api/config', (req) => {
  req.reply((res) => {
    res.body.featureFlag = true
  })
}).as('getConfig')

// Assert the request was made
cy.get('[data-cy="load-users"]').click()
cy.wait('@getUsers').its('request.url').should('include', '/api/users')

// Assert request payload
cy.wait('@createOrder').its('request.body').should('deep.include', {
  productId: 'prod_42',
})
```

**Always alias intercepts before the action that triggers the request.** `cy.wait('@alias')` waits for the matched request; it is not a timer and will not introduce flakiness.

**Never use `cy.wait(milliseconds)`.** It is a hard sleep that makes tests slow and still flaky. The only valid argument to `cy.wait()` is a route alias.

---

## Fixtures for Test Data

Fixtures live in `cypress/fixtures/` and are plain JSON (or other formats). Load them with `cy.fixture()` to keep test data out of test code.

```js
// cypress/fixtures/users.json
[
  { "email": "admin@example.com", "role": "admin" },
  { "email": "viewer@example.com", "role": "viewer" }
]

// In a test
cy.fixture('users').then((users) => {
  users.forEach((user) => {
    cy.request('POST', '/api/seed/user', user)
  })
})

// With intercept
cy.intercept('GET', '/api/users', { fixture: 'users.json' })
```

Use fixtures for:

- API response stubs that would otherwise require a running backend
- Repeated input data across multiple tests
- Large, complex payloads that clutter test code

Keep fixtures small and focused. One fixture per feature area, not one global fixture for the whole suite.

---

## Custom Commands

Custom commands live in `cypress/support/commands.js` (or `.ts`) and are loaded automatically. They eliminate repetition and enforce selector conventions.

```js
// cypress/support/commands.js

// Stable selector helpers
Cypress.Commands.add('getBySel', (selector, ...args) => {
  return cy.get(`[data-cy="${selector}"]`, ...args)
})
Cypress.Commands.add('getBySelLike', (selector, ...args) => {
  return cy.get(`[data-cy*="${selector}"]`, ...args)
})

// Programmatic login — bypasses UI, hits the API directly
Cypress.Commands.add('login', (email, password) => {
  cy.request('POST', '/api/auth/login', { email, password }).then(({ body }) => {
    window.localStorage.setItem('auth_token', body.token)
  })
  cy.visit('/')
})

// Database reset via cy.task
Cypress.Commands.add('resetDb', () => {
  cy.task('db:seed')
})
```

**Never use the UI to log in inside `beforeEach`.** Every test that requires authentication would then spend time clicking through a login form. Authenticate programmatically via `cy.request()` or set a session token directly. Use `cy.session()` to cache and restore sessions across tests for further speed gains.

---

## Page Object Pattern

The page object pattern wraps selectors and actions behind a class, making large test suites maintainable when the UI changes.

```js
// cypress/pages/CheckoutPage.js
export class CheckoutPage {
  selectors = {
    addressLine1: '[data-cy="address-line-1"]',
    payButton: '[data-cy="pay-button"]',
    confirmation: '[data-cy="order-confirmation"]',
  }

  fillAddress(address) {
    cy.get(this.selectors.addressLine1).type(address)
    return this
  }

  submit() {
    cy.get(this.selectors.payButton).click()
    return this
  }

  assertConfirmed() {
    cy.get(this.selectors.confirmation).should('be.visible')
    return this
  }
}

// In a test
import { CheckoutPage } from '../pages/CheckoutPage'

it('completes checkout', () => {
  const checkout = new CheckoutPage()
  checkout.fillAddress('123 Main St').submit().assertConfirmed()
})
```

Keep page objects thin — they hold selectors and interactions, not assertions. Assertions belong in the test. Returning `this` enables method chaining but is optional.

---

## Component Testing vs E2E

Cypress supports two distinct test modes, configured separately.

| | Component Testing | E2E Testing |
| --- | --- | --- |
| What is loaded | Single component | Full application |
| Speed | Fast (no server required) | Slower (full page load) |
| Scope | Isolated UI unit | User workflow through the app |
| Network | Mock everything | Can stub or use real backend |
| Best for | Component libraries, design systems, complex interactive components | Critical user journeys, multi-page flows |

### Setting up component tests

```js
// cypress.config.js
import { defineConfig } from 'cypress'

export default defineConfig({
  component: {
    devServer: {
      framework: 'react', // or 'vue', 'angular', 'svelte'
      bundler: 'vite',    // or 'webpack'
    },
  },
})
```

```jsx
// src/components/Stepper.cy.jsx
import Stepper from './Stepper'

describe('Stepper', () => {
  it('increments the count', () => {
    const onChangeSpy = cy.spy().as('onChange')
    cy.mount(<Stepper initialValue={1} onChange={onChangeSpy} />)

    cy.getBySel('increment').click()
    cy.getBySel('count').should('have.text', '2')
    cy.get('@onChange').should('have.been.calledWith', 2)
  })
})
```

Use `cy.spy()` to assert that event callbacks fire with the right arguments. Use `cy.stub()` to replace functions (e.g., API calls) called inside the component.

**When to prefer component tests:** When the interaction is UI-only and testing it end-to-end would require complex backend state. Component tests give faster feedback during development and catch regressions in isolation.

---

## Retries and Flakiness Reduction

Cypress retries `cy.get()` and assertions automatically until `defaultCommandTimeout` is reached. Most flakiness comes from not using this correctly.

```js
// cypress.config.js
export default defineConfig({
  e2e: {
    defaultCommandTimeout: 8000,
    requestTimeout: 10000,
    responseTimeout: 10000,
    retries: {
      runMode: 2,   // retry failed tests up to 2x in CI
      openMode: 0,  // no retries in interactive mode (you want to see failures immediately)
    },
  },
})
```

### Flakiness causes and fixes

| Cause | Fix |
| --- | --- |
| `cy.wait(2000)` | Replace with `cy.wait('@alias')` or assert element state |
| Race condition on page load | Assert element visibility before interacting: `.should('be.visible')` |
| Shared state between tests | Use `beforeEach` to reset; avoid `afterEach` cleanup |
| Brittle selectors | Switch to `data-cy` attributes |
| Non-deterministic data | Seed the database in `beforeEach` via `cy.task()` |
| Animations | Add `* { transition: none !important; }` in test CSS, or assert post-animation state |
| Conditional UI (sometimes visible) | Avoid conditional testing; make the app state deterministic before asserting |

#### `cy.session()` for authentication caching

```js
Cypress.Commands.add('login', (email, password) => {
  cy.session([email, password], () => {
    cy.request('POST', '/api/auth/login', { email, password }).then(({ body }) => {
      window.localStorage.setItem('auth_token', body.token)
    })
  })
})
```

`cy.session()` caches the browser session (cookies, localStorage) and restores it in subsequent tests instead of re-authenticating every time.

---

## CI Setup

```yaml
# .github/workflows/e2e.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  cypress:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: npm ci

      - name: Start application
        run: npm run build && npm run start:ci &

      - name: Wait for server
        run: npx wait-on http://localhost:3000 --timeout 60000

      - name: Run Cypress
        uses: cypress-io/github-action@v6
        with:
          record: true
          parallel: true
          group: 'E2E - Chrome'
          browser: chrome
        env:
          CYPRESS_RECORD_KEY: ${{ secrets.CYPRESS_RECORD_KEY }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

Key points:

- Start the server independently; do not start it from within Cypress tests
- Use `wait-on` to poll until the server is ready before running tests
- Never hardcode `baseUrl` in tests — set it in `cypress.config.js` and switch via environment variable for staging vs production

```js
// cypress.config.js
export default defineConfig({
  e2e: {
    baseUrl: process.env.CYPRESS_BASE_URL || 'http://localhost:3000',
  },
})
```

---

## Cypress Cloud Parallelization

Cypress Cloud (formerly Dashboard) orchestrates parallel runs across multiple CI machines. Tests are split dynamically based on historical duration, not statically.

```bash
# Run with parallelization across N machines (each gets this flag)
cypress run --record --parallel --group "E2E Suite" --ci-build-id "$BUILD_ID"
```

Features:

- **Load balancing**: Cypress Cloud distributes spec files to available machines so all finish at roughly the same time
- **Auto-cancellation**: Kills the entire run when a configurable failure threshold is hit, saving CI minutes
- **Spec prioritization**: Failed specs from the previous run are scheduled first so you see failures faster
- **Test replay**: Full video and network logs captured in Cypress Cloud for debugging failures without local reproduction

Parallelization requires a Cypress Cloud subscription. For open-source projects, the free tier covers basic recording and a limited number of test results per month.

---

## Cypress vs Playwright Decision Matrix

| Dimension | Cypress | Playwright |
| --- | --- | --- |
| Architecture | In-browser (same run loop as app) | Out-of-process via CDP |
| Languages | JavaScript / TypeScript only | JS, TS, Python, C#, Java |
| Browser support | Chrome, Firefox, Edge | Chrome, Firefox, Safari (WebKit), Edge |
| Mobile testing | None | Browser emulation |
| Multi-tab / multi-domain | Limited; `cy.origin()` for cross-origin | Native |
| Parallel execution | Via Cypress Cloud (paid) | Built-in, free |
| Component testing | Mature, excellent DX | Experimental, less mature |
| Debugging | Time-travel, snapshots, real-time runner | Trace viewer, Inspector |
| Install size | ~500 MB | ~10 MB |
| Test speed (headless) | Baseline | ~35–45% faster in parallel |
| Community maturity | Established, large plugin ecosystem | Growing fast, Microsoft-backed |
| Weekly downloads (early 2026) | ~5–8M | ~20–30M (surpassed Cypress mid-2024) |

### Choose Cypress when

- Your app is a single-domain SPA (React, Vue, Angular)
- Developer experience and real-time debugging matter more than raw speed
- You want mature component testing alongside E2E in one tool
- The team is JavaScript/TypeScript-only

#### Choose Playwright when

- You need cross-browser coverage including Safari
- Tests span multiple domains, tabs, or windows
- Parallel execution cost at scale is a concern (Cypress Cloud is paywalled)
- Team members write tests in Python, Java, or C#
- Mobile browser emulation is required
- You have thousands of tests and need maximum throughput

---

## Environment Variables and Secrets

```js
// cypress.config.js
export default defineConfig({
  env: {
    apiKey: process.env.API_KEY,
    apiUrl: process.env.API_URL || 'http://localhost:3000',
  },
})

// In tests — use cy.env(), not process.env
cy.request({
  url: `${Cypress.env('apiUrl')}/data`,
  headers: { Authorization: `Bearer ${Cypress.env('apiKey')}` },
})
```

Never commit secrets to `cypress.env.json`. Add it to `.gitignore`. Use CI secret management to inject environment variables.

---

## Common Anti-Patterns

### Hard sleeps

```js
// Wrong
cy.wait(3000)

// Right
cy.wait('@apiAlias')
// or
cy.get('[data-cy="result"]').should('be.visible')
```

#### Assigning command return values

```js
// Wrong — Cypress commands are async; this is always undefined
const el = cy.get('button')
el.click()

// Right
cy.get('button').click()
// or with alias
cy.get('button').as('btn')
cy.get('@btn').click()
```

#### UI login in every test

```js
// Wrong — hits the login page on every test
beforeEach(() => {
  cy.visit('/login')
  cy.get('#email').type('user@example.com')
  cy.get('#password').type('password')
  cy.get('[type=submit]').click()
})

// Right — programmatic auth, one network round-trip
beforeEach(() => {
  cy.login('user@example.com', 'password') // custom command using cy.request + cy.session
})
```

#### Testing third-party sites

```js
// Wrong — tests sites you don't control
cy.visit('https://accounts.google.com')

// Right — stub OAuth, or use cy.origin() only for sites you own
cy.intercept('GET', 'https://accounts.google.com/o/oauth2/**', {
  statusCode: 200,
  body: { access_token: 'fake_token' },
})
```

#### Tiny single-assertion tests

```js
// Wrong — four tests for one form field means four state resets
it('has validation attr', () => { ... })
it('has active class', () => { ... })
it('formats input', () => { ... })
it('shows error', () => { ... })

// Right — one test, one state setup
it('validates and formats the first name field', () => {
  cy.getBySel('first-name')
    .type('johnny')
    .should('have.attr', 'data-validation', 'required')
    .and('have.class', 'active')
    .and('have.value', 'Johnny')
  cy.getBySel('first-name').clear()
  cy.getBySel('submit').click()
  cy.getBySel('errors').should('contain', 'First name required')
})
```

#### afterEach cleanup

```js
// Wrong — not guaranteed to run; hides debugging state
afterEach(() => { cy.task('db:reset') })

// Right — deterministic state before each test
beforeEach(() => { cy.task('db:seed') })
```
