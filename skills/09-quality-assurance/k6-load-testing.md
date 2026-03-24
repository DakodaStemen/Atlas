---
name: k6-load-testing
description: Comprehensive patterns and best practices for writing, structuring, and running k6 load tests — covering script lifecycle, executor types, thresholds, checks, parameterized data, modules, and output integrations.
domain: testing
category: performance
tags: [k6, load-testing, performance, scenarios, thresholds, Grafana, executors, SharedArray, stress-test, soak-test]
triggers: [k6, load test, performance test, stress test, soak test, spike test, thresholds, executors, SharedArray, grafana k6, vus, virtual users, arrival rate]
---

# k6 Load Testing

k6 is a developer-centric load testing tool written in Go, scripted in JavaScript/TypeScript. Tests are plain code — version-controlled, composable, and executable locally or in CI. The runtime is Go (fast, low overhead), but the scripting surface is ES2015+ JavaScript with a curated set of built-in modules.

---

## Script Lifecycle

A k6 script has four distinct phases that run in order:

```javascript
// 1. INIT — runs once per VU before the test starts.
//    Import modules, load files, define options.
import http from 'k6/http';
import { check, sleep } from 'k6';
import { SharedArray } from 'k6/data';

export const options = { /* ... */ };

const users = new SharedArray('users', function () {
  return JSON.parse(open('./data/users.json'));
});

// 2. SETUP — runs once before VUs start. Return value is passed to default and teardown.
export function setup() {
  const res = http.post('https://api.example.com/auth/login', { username: 'admin', password: 'secret' });
  return { token: res.json('token') };
}

// 3. DEFAULT (VU function) — runs repeatedly for each VU/iteration.
export default function (data) {
  http.get('https://api.example.com/items', {
    headers: { Authorization: `Bearer ${data.token}` },
  });
  sleep(1);
}

// 4. TEARDOWN — runs once after all VUs finish.
export function teardown(data) {
  // cleanup, logout, etc.
}
```

Key rules:

- Only the init phase can call `open()` to read local files.
- `setup()` and `teardown()` each run exactly once, regardless of VU count.
- The default function runs for every iteration of every VU.

---

## Options

Options control execution behavior and can be set in three places, in ascending priority: script code → config file → CLI flags.

```javascript
export const options = {
  vus: 50,
  duration: '2m',

  // OR use stages (equivalent to ramping-vus executor)
  stages: [
    { duration: '30s', target: 20 },
    { duration: '1m',  target: 50 },
    { duration: '30s', target: 0  },
  ],

  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1500'],
    http_req_failed:   ['rate<0.01'],
    checks:            ['rate>0.99'],
  },

  tags: { env: 'staging', team: 'platform' },
};
```

Common option keys: `vus`, `duration`, `iterations`, `stages`, `scenarios`, `thresholds`, `tags`, `ext` (for cloud/output config), `noConnectionReuse`, `userAgent`, `httpDebug`.

---

## Scenarios and Executors

Scenarios let you run multiple independent workloads in a single test. Each scenario picks an executor that controls how VUs and iterations are scheduled.

```javascript
export const options = {
  scenarios: {
    browse: {
      executor: 'ramping-vus',
      stages: [
        { duration: '1m', target: 100 },
        { duration: '3m', target: 100 },
        { duration: '1m', target: 0   },
      ],
      gracefulRampDown: '30s',
    },
    checkout: {
      executor: 'constant-arrival-rate',
      rate: 20,
      timeUnit: '1s',
      duration: '5m',
      preAllocatedVUs: 30,
      maxVUs: 60,
      startTime: '1m', // start 1 minute after test begins
    },
  },
};
```

### Executor Reference

| Executor | Model | Best for |
| --- | --- | --- |
| `constant-vus` | Closed | Baseline / steady-state load at fixed concurrency |
| `ramping-vus` | Closed | Ramp-up/ramp-down, stress, soak |
| `shared-iterations` | Closed | Run exactly N total iterations across all VUs |
| `per-vu-iterations` | Closed | Each VU runs exactly N iterations |
| `constant-arrival-rate` | Open | Maintain a fixed RPS regardless of response time |
| `ramping-arrival-rate` | Open | Gradually change RPS over time |
| `externally-controlled` | — | Real-time VU scaling via REST API or CLI |

**Closed vs. open model:** Closed model VUs wait for a response before starting the next iteration — concurrency is capped. Open model (arrival-rate) spawns iterations on a schedule regardless of pending responses — better models real internet traffic.

#### constant-vus

```javascript
{
  executor: 'constant-vus',
  vus: 50,
  duration: '5m',
}
```

#### ramping-vus

```javascript
{
  executor: 'ramping-vus',
  startVUs: 0,
  stages: [
    { duration: '2m', target: 100 },
    { duration: '5m', target: 100 },
    { duration: '2m', target: 0   },
  ],
  gracefulRampDown: '30s',
}
```

#### constant-arrival-rate

```javascript
{
  executor: 'constant-arrival-rate',
  rate: 50,          // 50 iterations
  timeUnit: '1s',    // per second = 50 RPS
  duration: '5m',
  preAllocatedVUs: 60,
  maxVUs: 100,
}
```

#### ramping-arrival-rate

```javascript
{
  executor: 'ramping-arrival-rate',
  startRate: 10,
  timeUnit: '1s',
  preAllocatedVUs: 50,
  maxVUs: 200,
  stages: [
    { duration: '2m', target: 50  },
    { duration: '3m', target: 50  },
    { duration: '2m', target: 200 },
    { duration: '1m', target: 0   },
  ],
}
```

---

## Thresholds

Thresholds are the pass/fail SLOs for your test. If any threshold is breached, k6 exits with a non-zero code — CI pipelines fail automatically.

```javascript
thresholds: {
  // 95th percentile response time under 500ms, 99th under 1.5s
  http_req_duration: ['p(95)<500', 'p(99)<1500'],

  // Fewer than 1% of requests fail
  http_req_failed: ['rate<0.01'],

  // 99% of checks pass
  checks: ['rate>0.99'],

  // Custom metric
  'my_custom_metric': ['avg<200'],

  // Abort the test early if the threshold is violated
  http_req_duration: [{
    threshold: 'p(99)<2000',
    abortOnFail: true,
    delayAbortEval: '10s', // wait 10s of data before evaluating
  }],

  // Scope to a specific scenario via tags
  'http_req_duration{scenario:checkout}': ['p(95)<800'],
}
```

Built-in metric names: `http_reqs`, `http_req_duration`, `http_req_failed`, `http_req_blocked`, `http_req_connecting`, `http_req_tls_handshaking`, `http_req_sending`, `http_req_waiting`, `http_req_receiving`, `iteration_duration`, `iterations`, `vus`, `vus_max`, `data_sent`, `data_received`.

Threshold aggregation methods: `avg`, `min`, `max`, `med`, `p(N)`, `count`, `rate`.

---

## Checks

Checks are inline assertions. Unlike thresholds they never fail the test by themselves — they record a pass/fail rate that you can then threshold against.

```javascript
import { check } from 'k6';
import http from 'k6/http';

export default function () {
  const res = http.get('https://api.example.com/users/1');

  check(res, {
    'status is 200':        (r) => r.status === 200,
    'response time < 400ms': (r) => r.timings.duration < 400,
    'body contains id':      (r) => r.json('id') !== undefined,
  });
}
```

To make check failures actually fail the test, add a threshold on the `checks` metric:

```javascript
thresholds: {
  checks: ['rate>0.99'], // fail if more than 1% of checks fail
}
```

`check()` returns `true` if all conditions pass. You can combine it with `fail()` from `k6` to abort a single iteration:

```javascript
import { check, fail } from 'k6';

const ok = check(res, { 'status 200': (r) => r.status === 200 });
if (!ok) fail('unexpected status: ' + res.status);
```

---

## Environment Variables

Pass runtime configuration without touching the script:

```bash
k6 run -e BASE_URL=https://staging.example.com -e API_KEY=secret script.js
```

Access in script via `__ENV`:

```javascript
const BASE_URL = __ENV.BASE_URL || 'https://localhost:3000';

export default function () {
  http.get(`${BASE_URL}/health`);
}
```

Use env vars to select test data files, toggle feature flags, or point at different environments from the same script. Never hardcode secrets — pass them as env vars and keep them out of VCS.

---

## Parameterized Data with SharedArray

`SharedArray` loads data once in the init phase and shares it across all VUs without copying it into each VU's memory. Essential for large datasets (thousands of users, product IDs, etc.).

```javascript
import { SharedArray } from 'k6/data';
import { scenario } from 'k6/execution';

// Loaded once, shared read-only across all VUs
const users = new SharedArray('users', function () {
  return JSON.parse(open('./data/users.json'));
  // or: return papaparse.parse(open('./data/users.csv'), { header: true }).data;
});

export default function () {
  // Map iteration index to a user, wrapping around with modulo
  const user = users[scenario.iterationInTest % users.length];

  const res = http.post('https://api.example.com/login', JSON.stringify({
    username: user.username,
    password: user.password,
  }), { headers: { 'Content-Type': 'application/json' } });

  check(res, { 'login ok': (r) => r.status === 200 });
}
```

For CSV data, use the `papaparse` library (bundled with k6):

```javascript
import papaparse from 'https://jslib.k6.io/papaparse/5.1.1/index.js';
const data = new SharedArray('records', () =>
  papaparse.parse(open('./data/records.csv'), { header: true, skipEmptyLines: true }).data
);
```

---

## Modules and Code Reuse

Break scripts into reusable modules. k6 supports ES module imports from relative paths.

```text
tests/
  load/
    options/
      thresholds.js
    lib/
      auth.js
      http-client.js
    data/
      users.json
    scenarios/
      browse.js
      checkout.js
    main.js
```

```javascript
// lib/auth.js
import http from 'k6/http';
import { check } from 'k6';

export function login(baseUrl, username, password) {
  const res = http.post(`${baseUrl}/auth/login`, JSON.stringify({ username, password }), {
    headers: { 'Content-Type': 'application/json' },
  });
  check(res, { 'login 200': (r) => r.status === 200 });
  return res.json('token');
}
```

```javascript
// main.js
import { login } from './lib/auth.js';
import { browseScenario } from './scenarios/browse.js';
```

You can also import from the k6 jslib CDN for community utilities:

```javascript
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';
import { uuidv4 }           from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';
```

---

## Test Type Patterns

### Smoke Test

Verify the system works at all before heavier tests. 1–2 VUs, short duration.

```javascript
export const options = {
  vus: 2,
  duration: '30s',
  thresholds: { http_req_failed: ['rate<0.01'] },
};
```

### Average-Load (Baseline) Test

Simulate normal production traffic. Ramp up, hold steady, ramp down.

```javascript
export const options = {
  stages: [
    { duration: '5m',  target: 100 }, // ramp to expected load
    { duration: '20m', target: 100 }, // hold
    { duration: '5m',  target: 0   }, // ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],
    http_req_failed:   ['rate<0.01'],
  },
};
```

### Stress Test

Push beyond normal load to find where degradation starts.

```javascript
export const options = {
  stages: [
    { duration: '5m',  target: 100 },
    { duration: '5m',  target: 200 },
    { duration: '5m',  target: 400 },
    { duration: '5m',  target: 600 },
    { duration: '5m',  target: 0   },
  ],
};
```

### Soak Test

Hold moderate load for hours to expose memory leaks and resource exhaustion.

```javascript
export const options = {
  stages: [
    { duration: '5m',  target: 100 }, // ramp up
    { duration: '4h',  target: 100 }, // hold — this is the soak
    { duration: '5m',  target: 0   }, // ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<1000'],
    http_req_failed:   ['rate<0.01'],
  },
};
```

### Spike Test

Sudden, sharp increase in load to test autoscaling and recovery.

```javascript
export const options = {
  stages: [
    { duration: '2m',  target: 20  }, // baseline
    { duration: '30s', target: 500 }, // spike
    { duration: '3m',  target: 500 }, // hold spike
    { duration: '30s', target: 20  }, // recover
    { duration: '2m',  target: 20  }, // verify recovery
    { duration: '1m',  target: 0   },
  ],
};
```

### Breakpoint Test

Ramp load until the system fails. Never run against production.

```javascript
export const options = {
  stages: [
    { duration: '2h', target: 10000 }, // ramp up indefinitely until something breaks
  ],
  thresholds: {
    http_req_duration: [{ threshold: 'p(99)<5000', abortOnFail: true }],
    http_req_failed:   [{ threshold: 'rate<0.10',  abortOnFail: true }],
  },
};
```

---

## Custom Metrics

```javascript
import { Counter, Gauge, Rate, Trend } from 'k6/metrics';

const errorCount    = new Counter('errors');
const activeUsers   = new Gauge('active_users');
const successRate   = new Rate('success_rate');
const checkoutTime  = new Trend('checkout_duration', true); // true = display as ms

export default function () {
  const start = Date.now();
  const res = http.post('https://api.example.com/checkout', payload);

  if (res.status !== 200) errorCount.add(1);
  successRate.add(res.status === 200);
  checkoutTime.add(Date.now() - start);
}
```

Custom metrics appear in output alongside built-in metrics and can be used in thresholds:

```javascript
thresholds: {
  checkout_duration: ['p(95)<2000'],
  success_rate:      ['rate>0.99'],
}
```

---

## Tags and Groups

**Tags** attach metadata to requests for filtering in analysis:

```javascript
http.get('https://api.example.com/items', { tags: { name: 'list-items', endpoint: 'items' } });
```

**Groups** create logical sections in results:

```javascript
import { group } from 'k6';

export default function () {
  group('browse catalog', () => {
    http.get(`${BASE_URL}/categories`);
    http.get(`${BASE_URL}/products`);
  });

  group('checkout', () => {
    http.post(`${BASE_URL}/cart`, cartPayload);
    http.post(`${BASE_URL}/order`, orderPayload);
  });
}
```

Group-level metrics and threshold scoping:

```javascript
thresholds: {
  'http_req_duration{group:::checkout}': ['p(95)<1000'],
}
```

---

## Output Destinations

### Real-Time Streaming (OSS)

Pass via CLI with `-o` / `--out`:

```bash
# InfluxDB
k6 run --out influxdb=http://localhost:8086/k6 script.js

# Prometheus remote write
k6 run --out experimental-prometheus-rw script.js

# JSON file
k6 run --out json=results.json script.js

# Datadog
K6_DATADOG_ADDR=localhost:8125 k6 run --out datadog script.js

# Multiple outputs simultaneously
k6 run --out influxdb=http://localhost:8086/k6 --out json=results.json script.js
```

Available OSS outputs: `json`, `csv`, `influxdb`, `kafka`, `statsd`, `datadog`, `experimental-prometheus-rw`, `web-dashboard`.

### Grafana Cloud k6 (managed)

```bash
k6 cloud script.js
# or run locally and stream to cloud
k6 run --out cloud script.js
```

Configure in script:

```javascript
export const options = {
  ext: {
    loadimpact: {
      projectID: 123456,
      name: 'My Test Run',
    },
  },
};
```

### Grafana Cloud k6 vs. Self-Hosted OSS

| Concern | Grafana Cloud k6 | Self-Hosted OSS |
| --- | --- | --- |
| Infra | Managed, zero-ops | You provision InfluxDB/Prometheus + Grafana |
| Distributed load generation | Built-in (geo-distributed) | Manual with k6 operator (Kubernetes) |
| Result storage | Automatic | Your responsibility |
| Dashboards | Built-in | Build in Grafana with community dashboards |
| Cost | Pay per usage | Free, but infra costs apply |
| CI integration | `k6 cloud run` or GitHub Action | Same, stream to self-hosted backend |

---

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run k6 load test
  uses: grafana/k6-action@v0.3.1
  with:
    filename: tests/load/main.js
    flags: --out json=results.json
  env:
    BASE_URL: ${{ secrets.STAGING_URL }}
    API_KEY:  ${{ secrets.API_KEY }}
```

### CLI Patterns

```bash
# Run with env vars and output
k6 run \
  -e BASE_URL=https://staging.example.com \
  -e USERS=50 \
  --out influxdb=http://influx:8086/k6 \
  --tag testrun=deploy-#123 \
  tests/load/main.js

# Run a specific scenario only
k6 run --scenario checkout tests/load/main.js
```

---

## Practical Patterns

### Dynamic base URL with fallback

```javascript
const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
```

### Request batching

```javascript
import { batch } from 'k6/http';

const responses = batch([
  ['GET', `${BASE_URL}/api/users`],
  ['GET', `${BASE_URL}/api/products`],
  ['GET', `${BASE_URL}/api/categories`],
]);
responses.forEach((r) => check(r, { 'status 200': (res) => res.status === 200 }));
```

### Per-scenario thresholds via tags

```javascript
scenarios: {
  browse:   { executor: 'constant-vus', vus: 100, duration: '5m', tags: { scenario: 'browse'   } },
  checkout: { executor: 'constant-vus', vus: 20,  duration: '5m', tags: { scenario: 'checkout' } },
},
thresholds: {
  'http_req_duration{scenario:browse}':   ['p(95)<300'],
  'http_req_duration{scenario:checkout}': ['p(95)<1000'],
}
```

### Think time and pacing

```javascript
import { sleep } from 'k6';
import { randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

export default function () {
  http.get(`${BASE_URL}/page`);
  sleep(randomIntBetween(1, 5)); // simulate user think time
}
```

### Execution context

```javascript
import { scenario, vu, instance } from 'k6/execution';

export default function () {
  console.log(`VU ${vu.idInTest} / iteration ${scenario.iterationInTest}`);
  const user = users[vu.idInTest % users.length];
}
```

---

## Common Pitfalls

- **Sleep is mandatory for closed-model tests.** Without `sleep()`, VUs hammer the system as fast as possible — this is rarely realistic behavior.
- **Checks don't fail tests.** Always pair checks with a `checks` threshold if you need them to gate CI.
- **`open()` only works in init context.** Calling it inside the default function throws an error.
- **SharedArray data is read-only.** You cannot mutate it at runtime; use VU-local variables for mutable state.
- **Avoid `console.log` in hot paths.** It serializes to stdout per-VU and will tank performance at high VU counts. Use it only during development.
- **Arrival-rate executors need sufficient `preAllocatedVUs`.** If you underestimate, k6 will warn about dropped iterations.
- **Tag your runs.** Without `--tag` or script-level tags, distinguishing runs in a shared backend (InfluxDB, Grafana) is painful.
- **Use `gracefulStop` and `gracefulRampDown`.** Without them, VUs can be killed mid-request, producing misleading error spikes at the end of a test.

```javascript
export const options = {
  scenarios: {
    main: {
      executor: 'ramping-vus',
      stages: [...],
      gracefulRampDown: '30s',
      gracefulStop: '30s',
    },
  },
};
```
