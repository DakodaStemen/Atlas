---
name: testing-strategy-comprehensive
description: Comprehensive testing strategy covering test planning, risk-based prioritization, coverage analysis, shift-left practices, regression strategy, test suite maintenance at scale, test result interpretation, and unit test generation patterns. Use when planning testing efforts, analyzing coverage, managing regression suites, or establishing testing standards.
domain: testing
tags: [testing, qa, strategy, planning, coverage, regression, shift-left, maintenance, interpretation]
triggers: test plan, test strategy, coverage analysis, shift-left, regression suite, test maintenance, test results, risk-based testing, unit test generation
---

# Testing Strategy Comprehensive

## Test Planning

### Risk Assessment

- Begin every test plan with a risk assessment matrix: likelihood of failure x impact of failure = priority
- Classify features into risk tiers (critical, high, medium, low) and allocate depth proportionally:
  - **Critical**: Exploratory + automated + manual testing
  - **High**: Automated + targeted manual
  - **Medium**: Automated smoke + regression
  - **Low**: Automated smoke only

### Entry and Exit Criteria

- **Entry criteria**: Code complete, environment stable, test data available, build passes smoke tests
- **Exit criteria** (must be measurable): All P0/P1 bugs resolved, code coverage above threshold, no open blockers, regression suite green

### Planning Best Practices

- Scope must explicitly list what is **out of scope** to prevent scope creep
- Resource allocation should account for test environment setup (typically 15-30% of total test effort)
- Include risk mitigation: unavailable environments, absent testers, mid-cycle scope changes
- Time-box exploratory testing (60-90 minutes) with charter documents defining mission and boundaries
- Maintain a living test plan that updates as requirements evolve

---

## Coverage Analysis

### Coverage Types

- **Line/branch coverage**: Percentage of code paths executed by tests. Necessary but not sufficient.
- **Requirement coverage**: Mapping between requirements and tests that verify them. Ensures nothing is untested by design.
- **Risk coverage**: Whether high-risk areas have proportionally deeper testing.

### Effective Coverage Practices

- Set coverage targets that are meaningful: 80% line coverage is a reasonable floor; 100% is usually not worth the marginal effort
- **Mutation testing** is the true measure of coverage quality: high line coverage with low mutation score means tests execute code but do not verify behavior
- Focus coverage investment on: error handling paths, boundary conditions, state transitions, and integration points
- Use coverage diffs on PRs to ensure new code meets standards without requiring retroactive coverage of legacy code
- Coverage tools: Istanbul/nyc (JS), coverage.py (Python), JaCoCo (Java), tarpaulin (Rust), gcov/llvm-cov (C/C++)

### Anti-Patterns

- Do not chase coverage numbers without mutation testing validation
- Do not count tests that assert nothing (no assertions = no coverage value)
- Do not exclude hard-to-test code from coverage reports; refactor it instead

---

## Shift-Left Testing

### Core Principles

- Move testing activities **earlier** in the development lifecycle to catch defects sooner
- Developers own testing for their code; QA validates cross-cutting concerns
- Static analysis (linters, type checkers) catches defects at write-time, not test-time

### Shift-Left Practices

| Practice | When | What It Catches |
| --- | --- | --- |
| Type checking | At write-time | Type errors, null references |
| Linting | Pre-commit | Style issues, common bugs |
| Unit tests | Pre-push | Logic errors, regressions |
| Contract tests | CI | API incompatibilities |
| SAST | CI | Security vulnerabilities |
| Integration tests | CI | Cross-component issues |

### Implementation

- Pre-commit hooks: linting, formatting, secret scanning
- Pre-push hooks: unit test suite
- CI gates: integration tests, contract tests, SAST, coverage thresholds
- Feature branch testing: deploy preview environments with automated smoke tests

---

## Regression Testing Strategy

### Test Selection Methods

- **Risk-based selection**: Run tests proportional to the risk of the changed area
- **Impact analysis**: Map code changes to affected test suites using dependency analysis
- **Selective regression**: Tools like Jest `--changedSince`, pytest `--lf` (last failed), or Bazel test caching

### Automation vs Manual

- Automate: Stable features, happy paths, data-driven scenarios, cross-browser checks
- Keep manual: Exploratory testing, UX validation, edge cases requiring human judgment
- **Critical rule**: Any bug found manually more than once should be automated

### Regression Cycle Optimization

- Tag tests by priority (P0/P1/P2) and run P0 on every commit, P0+P1 on PR merge, full suite nightly
- Parallelize test execution across multiple workers/containers
- Use test impact analysis to skip unaffected tests
- Track test execution time; investigate and fix tests exceeding baseline by 2x

---

## Test Maintenance at Scale

### Naming Conventions

- Test names should describe the behavior being verified, not the implementation
- Format: `[unit]_[condition]_[expected behavior]` or `should [do X] when [condition Y]`
- Bad: `test1`, `testFunction`, `testBugFix123`
- Good: `calculateTax_withNegativeIncome_returnsZero`

### Reducing Test Debt

- Delete tests that verify deleted features (zombie tests)
- Consolidate tests that verify the same behavior through different paths
- Extract shared setup into fixtures/factories; avoid copy-paste test setup
- Flag tests with TODO/FIXME and schedule cleanup sprints

### Organization

- Mirror source directory structure in test directories
- Group by feature, not by test type (integration tests for feature X live near unit tests for feature X)
- Use consistent file naming: `*.test.ts`, `*_test.go`, `test_*.py`
- Keep test files under 300 lines; split by logical grouping

### Flaky Test Management

- Track flaky tests in a dashboard; quarantine persistent flakers
- Root causes (ordered by frequency): shared mutable state, time-dependent logic, network calls, race conditions, non-deterministic data
- Fixes: isolate state per test, use deterministic clocks, mock network calls, add proper synchronization, use factories instead of shared fixtures
- If a test is flaky for > 2 weeks without a fix, delete it and file a ticket to rewrite

---

## Test Result Interpretation

### Triage Framework

- **True failure**: Code change broke expected behavior. Fix the code.
- **False failure (flaky)**: Test failed but code is correct. Fix or quarantine the test.
- **Environment failure**: Infrastructure issue (timeout, OOM, missing service). Retry once; investigate if recurs.
- **Known failure**: Test for a known bug. Should be marked `@skip` with ticket reference.

### Metrics That Matter

- **Pass rate over time**: Trend matters more than absolute number. Declining pass rate = accumulating debt.
- **Mean time to fix**: How long from red build to green. Target: < 1 hour for CI, < 1 day for nightly.
- **Flaky rate**: Percentage of tests that flip without code changes. Target: < 1%.
- **Test execution time**: Track p50 and p99. Investigate sudden jumps.

---

## Unit Test Generation

### What to Generate

- Happy path for each public function/method
- Edge cases: null/empty input, boundary values, overflow
- Error paths: invalid input, exceptions, error returns
- State transitions for stateful components

### Generation Patterns

- **Arrange-Act-Assert**: Setup state, execute the action, verify the outcome. Every test follows this structure.
- **Given-When-Then**: For BDD-style tests. Equivalent to AAA but reads as a specification.
- **One assertion per test**: Each test verifies one behavior. Multiple assertions in one test hide failures.
- **Factory pattern**: Use factories (FactoryBot, Fishery, Faker) to generate test data with sensible defaults and targeted overrides.

### Anti-Patterns

- Testing implementation details (private methods, internal state) instead of behavior
- Excessive mocking that tests the mock, not the code
- Tests that duplicate production logic (calculating expected value the same way as production)
- Assertions on exact error messages (brittle; assert on error type or code instead)
