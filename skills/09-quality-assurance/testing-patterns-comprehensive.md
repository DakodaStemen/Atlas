---
name: testing-patterns-comprehensive
description: Comprehensive testing patterns covering integration testing, performance baselines, security testing in SDLC, accessibility testing, flaky test debugging, test data generation, browser automation, and review checklists. Use when implementing specific testing types or debugging test infrastructure issues.
domain: testing
tags: [testing, qa, integration, performance, security, accessibility, flaky-tests, test-data, e2e, browser-automation]
triggers: integration testing, performance baselines, security testing, accessibility testing, flaky tests, test data, browser automation, e2e testing, test doubles
---

# Testing Patterns Comprehensive

## Integration Testing

### Boundary Definition

- Define the integration boundary explicitly for each test: what is real, what is simulated
- Use the test pyramid: many unit tests, fewer integration tests, fewest e2e tests
- Integration tests cover gaps unit tests cannot: serialization, SQL queries, HTTP behavior, message formatting

### Test Doubles Hierarchy

Use the simplest double that validates the behavior:

| Double | Use When |
| --- | --- |
| **Stub** | Need canned responses; behavior doesn't matter |
| **Fake** | Need lightweight implementation (in-memory cache, SQLite) |
| **Mock** | Interaction verification IS the point of the test |
| **Spy** | Need to verify calls without changing behavior |

- Avoid mocking what you do not own; wrap third-party libraries behind your own interface
- Prefer real databases over mocks: use testcontainers (Docker ephemeral instances) or SQLite in-memory

### HTTP Dependencies

- Use recorded/replayed approach (WireMock, VCR) for stability
- Run periodic live integration tests to detect contract drift
- Contract testing (Pact, Specmatic) for service-to-service API validation

### Error Path Testing

- Test timeouts, connection refused, malformed responses, rate limiting
- These paths are where production incidents hide
- Each test creates its own data with unique identifiers; no shared test databases

---

## Performance Testing Baselines

### Establishing Baselines

- Run baseline tests on dedicated hardware (not shared CI runners) for consistency
- Collect p50, p95, p99 latency, throughput (RPS), error rate, and resource utilization
- Run minimum 30 iterations per configuration for statistical stability
- Document exact configuration: hardware, OS, runtime version, concurrency settings

### Regression Detection

- Compare new results against baseline using statistical tests (t-test for means, Mann-Whitney for distributions)
- Alert on: p99 latency increase > 10%, throughput decrease > 5%, error rate increase > 0.1%
- Store historical results for trend analysis (Prometheus, InfluxDB, or simple CSV)

### Load Test Profiles

| Profile | Pattern | Goal |
| --- | --- | --- |
| **Smoke** | 1-5 VUs, 1 minute | Verify test works |
| **Load** | Target VUs, 10-30 min | Validate SLA at expected load |
| **Stress** | Ramp beyond target, 15 min | Find breaking point |
| **Soak** | Target VUs, 2-8 hours | Detect memory leaks, connection exhaustion |
| **Spike** | Sudden jump then drop | Validate auto-scaling and recovery |

### CI Performance Gates

- Run smoke tests on every PR
- Run load tests nightly or on release branches
- Fail the build if latency exceeds baseline by threshold
- Store results as build artifacts for post-hoc analysis

---

## Security Testing in SDLC

### Integration Points

| Phase | Test Type | Tools |
| --- | --- | --- |
| Pre-commit | Secret scanning | gitleaks, truffleHog |
| CI | SAST | Semgrep, CodeQL, Snyk Code |
| CI | Dependency scanning | npm audit, cargo audit, Snyk |
| Deploy | DAST | OWASP ZAP, Burp Suite |
| Pre-release | Pentest | Manual + automated |

### Balancing Security and Velocity

- Start SAST with low false-positive rule sets; expand gradually
- Use `@security-ignore` annotations with mandatory justification comments
- Run full DAST scans nightly, not on every PR (too slow)
- Dependency scanning on every PR with auto-PR for patches
- Track security debt separately from feature work

---

## Accessibility Testing

### Beyond Automated Scanning

Automated tools (axe, Lighthouse) catch ~40% of issues. Manual testing is required for:

- Screen reader announcement order for dynamic content
- Keyboard navigation flow correctness (tab order, focus management)
- Cognitive accessibility (reading level, information density)
- Custom ARIA patterns for complex widgets (data grids, drag-and-drop, live regions)

### Testing Checklist

- [ ] Tab through every interactive element; verify logical order
- [ ] Activate every interactive element with keyboard only (Enter, Space, Arrow keys)
- [ ] Test with screen reader (NVDA on Windows, VoiceOver on Mac, ORCA on Linux)
- [ ] Verify live region announcements for dynamic content updates
- [ ] Check color contrast ratios (4.5:1 for normal text, 3:1 for large text)
- [ ] Test at 200% zoom; verify no content is lost or overlapped
- [ ] Verify all images have meaningful alt text (or empty alt for decorative)
- [ ] Test form error announcements and focus management

---

## Flaky Test Debugging

### Root Cause Categories

1. **Shared mutable state** (most common): Tests modify global state without cleanup
2. **Time-dependent logic**: Tests rely on wall clock, sleep durations, or date calculations
3. **Network calls**: External services are slow, rate-limited, or unavailable
4. **Race conditions**: Async operations complete in unpredictable order
5. **Non-deterministic data**: Random values, auto-increment IDs, or order-dependent collections

### Diagnostic Steps

1. Run the failing test in isolation (`--only`, `it.only`). If it passes, shared state is likely.
2. Run the test 50x in a loop. If it passes every time, look for ordering or environment issues.
3. Check CI vs local environment differences: timezone, locale, file system case sensitivity, available memory.
4. Add verbose logging around the failure point; look for timing-related patterns.
5. Check if failure correlates with parallel execution (try `--runInBand` / sequential mode).

### Fixes

- Replace `sleep(N)` with explicit waits (`waitFor`, polling assertions)
- Use deterministic clocks (Sinon fake timers, `freezegun`, `tokio::time::pause`)
- Replace real HTTP calls with recorded fixtures (WireMock, VCR)
- Isolate database state per test using transactions or testcontainers
- Use factories with unique IDs instead of shared fixtures

---

## Test Data Generation

### Strategies

- **Factories** (FactoryBot, Fishery, factory_boy): Objects with sensible defaults and targeted overrides
- **Fakers** (Faker.js, Faker, fake): Generate realistic-looking data (names, emails, addresses)
- **Fixtures**: Static data files (JSON, SQL) for reproducible scenarios
- **Snapshots**: Sanitized production data dumps for realistic distributions
- **Property-based**: Generate random inputs constrained by type (Hypothesis, fast-check, QuickCheck)

### Privacy-Compliant Test Data

- Never copy production PII to test environments
- Use data generators that produce structurally valid but synthetic data
- For realistic distributions, anonymize production data using k-anonymity or differential privacy
- Document which test environments contain what class of data

### Performance Test Data

- Generate data volumes matching production scale (or 10x for stress tests)
- Use bulk insertion methods, not individual inserts
- Pre-generate and cache large datasets; do not regenerate on every test run
- Verify data distribution matches production (cardinality, skew, null rates)

---

## Browser Automation & E2E

### Tool Selection

| Tool | Best For |
| --- | --- |
| **Playwright** | Cross-browser, API mocking, network interception |
| **Cypress** | Component testing, real-time debugging, JS-heavy apps |
| **Selenium** | Legacy systems, multi-language support |

### Best Practices

- Use data-testid attributes for selectors (not CSS classes or text content)
- Implement page object pattern or component abstraction for reusable interactions
- Wait for application state, not arbitrary timeouts
- Isolate test data: each test creates its own user, data, and cleanup
- Take screenshots on failure for debugging; archive as CI artifacts
- Run E2E tests in headless mode in CI, headed mode for debugging

### Review Checklist (Pre-Merge)

- [ ] Build after each meaningful edit, not only at the end
- [ ] Run the app after changes when startup-sensitive files changed
- [ ] Verify actual launch instead of assuming success from spawned process
- [ ] All interactive elements have accessible names
- [ ] No hardcoded waits (sleep); use explicit wait conditions
- [ ] Test data cleaned up after test execution
