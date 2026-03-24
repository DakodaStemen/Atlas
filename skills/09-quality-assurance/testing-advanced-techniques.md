---
name: testing-advanced-techniques
description: Advanced testing techniques including mutation testing (Stryker, PITest), contract testing (Pact, PactFlow), property-based testing (Hypothesis, fast-check, QuickCheck), visual regression testing (Percy, Chromatic, Playwright), chaos engineering, and load/stress testing patterns. Use when going beyond standard unit and integration testing.
domain: testing
tags: [testing, qa, mutation, contract, property-based, visual-regression, chaos-engineering, load-testing, pact, stryker]
triggers: mutation testing, contract testing, Pact, property-based testing, Hypothesis, fast-check, visual regression, Percy, Chromatic, chaos engineering, fault injection, load stress testing
---

# Testing Advanced Techniques

## Mutation Testing

### Concept

Mutation testing measures test suite **effectiveness**, not just coverage. Small, intentional changes (mutants) are introduced to source code (changing `>` to `<`, `+` to `-`, removing conditions). A test suite that catches these mutations has high quality. Surviving mutants indicate weak tests.

### Tools

- **Stryker** (JavaScript/TypeScript): `npx stryker run` -- supports Jest, Mocha, Vitest
- **PITest** (Java/Kotlin): `mvn org.pitest:pitest-maven:mutationCoverage`
- **mutmut** (Python): `mutmut run`
- **cargo-mutants** (Rust): `cargo mutants`

### Key Practices

- Run mutation testing on critical business logic, not on UI or boilerplate
- Target mutation score > 80% for critical paths; 60% is acceptable for utilities
- Focus on surviving mutants in decision logic and boundary conditions
- Incremental mutation testing (changed files only) keeps CI feasible

### Mutant Categories

| Category | Example | What It Tests |
| --- | --- | --- |
| Arithmetic | `+` to `-` | Calculation logic |
| Conditional | `>` to `>=` | Boundary conditions |
| Negation | `true` to `false` | Boolean logic |
| Return value | `return x` to `return 0` | Return value usage |
| Void method | Remove call | Side effect verification |

---

## Contract Testing

### Consumer-Driven Contracts (Pact)

1. **Consumer writes test**: Defines expected request/response for each interaction
2. **Pact file generated**: JSON contract from consumer test execution
3. **Provider verifies**: Runs against real provider using contract as input
4. **Broker manages**: PactFlow or Pact Broker stores contracts and verification results

### When to Use

- Microservices communicating via REST or messaging
- Multiple consumers of the same API
- Teams developing and deploying services independently
- Preventing breaking API changes without E2E tests

### Implementation Pattern

```text
Consumer:
  1. Define interaction (given state, request, expected response)
  2. Run consumer test -> generates Pact file
  3. Publish Pact to broker

Provider:
  1. Fetch Pact from broker
  2. Set up provider states (test data)
  3. Replay requests, verify responses match contract
  4. Publish verification results

CI:
  - can-i-deploy check before deployment
  - Matrix: consumer version x provider version compatibility
```

### Key Rules

- Consumer defines the contract; provider must satisfy it
- Keep contracts minimal: test the interface, not internal behavior
- Version contracts alongside code; use tags for environments (prod, staging)
- Run `can-i-deploy` as a deployment gate

---

## Property-Based Testing

### Core Concept

Instead of testing specific examples, define **properties** (invariants) that must hold for all valid inputs. The framework generates hundreds of random inputs and finds counterexamples.

### Properties to Test

- **Idempotency**: `f(f(x)) == f(x)` (formatting, normalization)
- **Round-trip**: `decode(encode(x)) == x` (serialization)
- **Invariants**: `sort(list).length == list.length` (length preservation)
- **Commutativity**: `f(a, b) == f(b, a)` (where applicable)
- **Monotonicity**: If `a <= b` then `f(a) <= f(b)` (where applicable)

### Tools and Patterns

```python
# Hypothesis (Python)
from hypothesis import given
import hypothesis.strategies as st

@given(st.lists(st.integers()))
def test_sort_preserves_length(lst):
    assert len(sorted(lst)) == len(lst)

@given(st.text())
def test_encode_decode_roundtrip(s):
    assert decode(encode(s)) == s
```

```typescript
// fast-check (TypeScript)
import fc from 'fast-check';

fc.assert(
  fc.property(fc.array(fc.integer()), (arr) => {
    const sorted = arr.slice().sort((a, b) => a - b);
    return sorted.length === arr.length;
  })
);
```

### Shrinking

When a counterexample is found, frameworks automatically **shrink** it to the minimal failing case. This makes debugging much easier than finding a complex failing input.

---

## Visual Regression Testing

### Tools

- **Percy** (cloud): Automates capture and comparison. Approving in Percy UI sets new baseline.
- **Chromatic** (Storybook): Captures every component per branch, manages baselines.
- **Playwright** (native): `toHaveScreenshot()` matcher stores baseline images in repo.

### Playwright VRT Pattern

```javascript
import { test, expect } from '@playwright/test';

test('homepage visual', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveScreenshot('homepage.png', {
    maxDiffPixelRatio: 0.01,  // Allow 1% pixel difference
  });
});
```

### Best Practices

- Run VRT on a single browser/viewport first; expand coverage gradually
- Use threshold-based comparison (1-2% pixel diff) to avoid false positives from anti-aliasing
- Exclude dynamic content (timestamps, ads, animations) from comparison regions
- Review and approve intentional visual changes in CI before merge
- Store baselines in the repository (Playwright) or cloud service (Percy/Chromatic)

---

## Chaos Engineering

### Principles

1. **Define steady state**: Latency p95, error rate, throughput metrics under normal conditions
2. **Hypothesize**: "If [fault X occurs], the system will [expected behavior]"
3. **Introduce fault**: Network partition, service crash, disk full, high latency
4. **Observe**: Did the system behave as hypothesized?
5. **Learn**: Fix gaps between hypothesis and reality

### Fault Types

| Fault | Tool | What It Tests |
| --- | --- | --- |
| Kill process | `kill -9`, Chaos Monkey | Restart recovery |
| Network latency | tc, toxiproxy | Timeout handling |
| Network partition | iptables, Chaos Mesh | Failover, split-brain |
| Disk full | fallocate | Graceful degradation |
| CPU saturation | stress-ng | Queue depth, backpressure |
| DNS failure | Block DNS | Fallback resolution |

### Safety Rules

- **Start small**: Single instance, single AZ, non-production
- **Define abort condition**: Error rate > X%, latency > Y, customer impact detected
- **Use feature flags**: Target specific users or traffic percentage
- **Schedule game days**: Planned chaos with all hands on deck
- **Never run in production** without abort conditions and rollback plan

---

## Load & Stress Testing

### Tool Selection

- **k6**: Developer-centric, JavaScript scripting, lightweight, built-in metrics
- **Gatling**: JVM-based, Scala DSL, detailed HTML reports
- **Locust**: Python-based, distributed, event-driven
- **Artillery**: YAML config + JS, good for quick API load tests

### Test Design

- Model realistic user flows with think time between requests
- Mix operations proportionally to production traffic patterns
- Use ramp-up periods (do not spike to full load immediately)
- Include data variation (different user IDs, query parameters)

### Targets and SLOs

- Define pass/fail thresholds before running:
  - p95 latency < X ms
  - Error rate < Y%
  - Throughput >= Z RPS
- Report: actual vs target for each metric
- Run at target load for sustained period (10-30 minutes minimum)
- Run stress test beyond target to find breaking point

### CI Integration

- Smoke load test (5 VUs, 1 minute) on every PR
- Full load test on release branches or nightly
- Store results in time-series database for trend analysis
- Alert on regression: latency increase > 10%, throughput decrease > 5%
