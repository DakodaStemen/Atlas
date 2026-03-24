---
name: commercial-observability
description: Commercial observability platforms covering Datadog (APM, metrics, logs, monitors, dashboards, RUM, synthetics) and New Relic/Dynatrace (agent configuration, custom instrumentation, alert policies, SLO management, deployment tracking). Use when integrating or configuring Datadog, New Relic, or Dynatrace.
domain: infrastructure
tags: [datadog, newrelic, dynatrace, apm, monitoring, observability, rum, synthetics]
triggers: datadog, new relic, dynatrace, APM, commercial observability, datadog monitor, newrelic agent
---


# Datadog Observability

Reference patterns for instrumenting services with Datadog end-to-end. The three pillars—metrics, logs, and traces—only deliver full value when they are linked by consistent tags and trace IDs injected at the source.


## 2. Datadog Agent Configuration

The Agent is the local collector. Key `datadog.yaml` sections to configure explicitly:

```yaml
# /etc/datadog-agent/datadog.yaml

api_key: <DD_API_KEY>
site: datadoghq.com   # or datadoghq.eu, us3.datadoghq.com, etc.

# Unified tags applied to all telemetry from this host
env: production
tags:
  - team:platform
  - region:us-east-1

# APM / tracing
apm_config:
  enabled: true
  # Ingestion sampling — head-based, agent side
  max_traces_per_second: 50   # default; tune per host load
  # Remote configuration can override this via Ingestion Controls UI

# DogStatsD
dogstatsd_config:
  enabled: true
  socket_path: /var/run/datadog/dsd.socket   # Unix Domain Socket (lower overhead than UDP)
  # UDP fallback: port 8125

# Log collection
logs_enabled: true
logs_config:
  container_collect_all: false   # opt-in per integration; avoids log explosion
  processing_rules:
    - type: exclude_at_match
      name: exclude_health_checks
      pattern: "GET /healthz"   # drop noisy low-value lines before they leave the host

# Process monitoring (optional but useful for infra correlation)
process_config:
  enabled: true
```

For containerized deployments, most of the above maps to Helm chart values or the Datadog Operator CRD. The Helm chart key path mirrors the YAML structure (`datadog.apm.enabled`, `datadog.logs.enabled`, etc.).


## 4. Log Correlation with Trace IDs

Trace-log correlation lets you jump from a slow span directly to the exact log lines emitted during that request.

### Automatic injection (preferred)

Enable in the tracer; the library patches the logging formatter to inject fields automatically.

```python
# Python (ddtrace >= 0.41)
import logging
from ddtrace import patch_all
patch_all()   # patches logging among other libraries

# Your log format must include the injected fields
logging.basicConfig(
    format="%(asctime)s %(levelname)s [%(dd.service)s] [%(dd.env)s] "
           "[%(dd.version)s] [%(dd.trace_id)s %(dd.span_id)s] %(message)s"
)
```

```typescript
// Node.js (dd-trace)
const tracer = require('dd-trace').init({ logInjection: true });
// pino / winston automatically receive dd.trace_id, dd.span_id, dd.env, dd.service, dd.version
```

### Injected log attributes

| Attribute | Purpose |
| ----------- | --------- |
| `dd.trace_id` | Links log line to the exact trace in APM |
| `dd.span_id` | Links to the specific span within the trace |
| `dd.env` | Environment (from `DD_ENV`) |
| `dd.service` | Service name (from `DD_SERVICE`) |
| `dd.version` | Version (from `DD_VERSION`) |

In the Datadog log explorer, clicking "View Trace" on a log line navigates directly to the correlated flame graph, and vice versa.

### Log pipeline configuration

In Datadog > Logs > Pipelines, configure:

1. **Grok parser** — parse your structured log format into attributes. Use the built-in parsers for common formats (JSON, CLF, ELF) where possible.
2. **Remapper** — map `dd.trace_id` → `dd.trace_id` (official reserved attribute) so correlation works.
3. **Status remapper** — map your log level field to the reserved `status` attribute so severity filtering works across all sources.
4. **Service remapper** — ensure `dd.service` maps to the reserved `service` attribute.


## 6. Monitors and Alert Conditions

### Monitor types

| Type | When to use |
| ------ | ------------- |
| **Metric** | Threshold on any numeric metric (CPU, error rate, queue depth) |
| **APM** | Latency, error rate, throughput on a service or resource directly |
| **Log** | Alert on log pattern count or absence |
| **Composite** | AND/OR across multiple monitors to reduce noise |
| **Anomaly** | ML-detected deviation from learned baseline |
| **Forecast** | Alert before a threshold is breached (disk space, quota) |
| **Synthetics** | Uptime and SLA validation from external checkpoints |

### Threshold configuration

Set thresholds from measured baselines, not arbitrary numbers. Procedure:

1. Run the service for 2 weeks in production.
2. Open the metric in Metrics Explorer; examine P95 and max over the period.
3. Set the warning threshold at 1–1.5× the P95; critical at 2×P95 or the point at which user impact begins.

For error rates: warning at 1%, critical at 5% is a common starting point—but validate against your actual SLO target, not a generic number.

### Anomaly and composite monitors

Anomaly detection reduces alert fatigue for metrics with daily or weekly seasonality (traffic, latency). Use it when you cannot set a static threshold that holds across business hours and off-peak.

Composite monitors combine signals before paging. A latency spike alone may not warrant a page; a latency spike AND an elevated error rate together reliably indicates a real incident.

```yaml
# Composite monitor example
monitor_a: p99 latency > 2000ms
monitor_b: error rate > 2%
composite: monitor_a AND monitor_b
```

### Notification routing

```text
# Monitor message template
{{#is_alert}}
Service {{service.name}} in {{env.name}} is degraded.
Runbook: https://runbooks.internal/{{service.name}}-degraded
@pagerduty-platform-oncall
{{/is_alert}}

{{#is_warning}}
@slack-platform-alerts
{{/is_warning}}
```

Use `@pagerduty-<integration-name>` for critical; `@slack-<channel>` for warning. Avoid using personal `@mentions`—route to services and channels only.

### Monitor tagging (organizational)

Every monitor must carry at minimum:

- `team:<name>` — for ownership filtering
- `service:<name>` — for service-level aggregation
- `env:<name>` — to suppress staging alerts from production dashboards
- `sli:<type>` when the monitor is used as an SLI (e.g., `sli:availability`, `sli:latency`)

Monitor-based SLOs inherit constituent monitor tags, so consistent monitor tagging directly enables SLO filtering.

### Downtime scheduling

Use Datadog Manage Downtime to suppress alerts during planned maintenance. Target by tag query (`service:payments-api AND env:production`) rather than by individual monitor name—tag-based targeting auto-includes new monitors that match the query.


## 8. Dashboard Best Practices

### Structure: one service, one overview dashboard

Every production service should have a single overview dashboard with three sections:

1. **Golden signals** — requests/sec, error rate, P95/P99 latency, saturation (CPU, memory).
2. **SLO status** — SLO widget showing current status and remaining error budget for the 7d, 30d, 90d windows.
3. **Infrastructure** — host/container CPU, memory, network I/O correlated with service metrics via shared template variables.

### Template variables

```text
$env      = tag: env         (default: production)
$service  = tag: service     (default: payments-api)
$version  = tag: version     (default: *)
```

All widgets use `$env` and `$service` as scope variables. This allows a single dashboard to serve all environments and services without duplication.

### RED method widget layout

| Widget | Metric | Query pattern |
| -------- | -------- | --------------- |
| Timeseries — request rate | `trace.<service>.hits` | `sum:trace.payments-api.request.hits{env:$env}.as_rate()` |
| Timeseries — error rate % | `trace.<service>.errors` | `(sum:trace.payments-api.request.errors{env:$env} / sum:trace.payments-api.request.hits{env:$env}) * 100` |
| Timeseries — P99 latency | `trace.<service>.duration` | `p99:trace.payments-api.request.duration{env:$env}` |

### Dashboard sharing and access

- Production dashboards should be shared read-only with the wider engineering org.
- Executive-facing dashboards should show only SLO status and error budget—not raw metrics.
- Use the SLO List widget for multi-service status pages.


## 10. Integration Patterns

### CI/CD deployment markers

Push a deployment event on every production deploy to overlay it on all dashboards:

```bash
curl -X POST "https://api.datadoghq.com/api/v1/events" \
  -H "DD-API-KEY: ${DD_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Deploy: payments-api v1.4.2",
    "text": "Deployed by CI pipeline. Commit: abc1234.",
    "tags": ["service:payments-api", "env:production", "version:1.4.2"],
    "alert_type": "info",
    "source_type_name": "MY_APPS"
  }'
```

### Kubernetes cluster monitoring

Minimal Helm values for full cluster coverage:

```yaml
datadog:
  apiKey: <DD_API_KEY>
  site: datadoghq.com
  apm:
    portEnabled: true
  logs:
    enabled: true
    containerCollectAll: false   # opt-in per namespace annotation
  processAgent:
    enabled: true
  clusterChecks:
    enabled: true
agents:
  tolerations:
    - operator: Exists   # ensure Agent runs on all nodes including taints
```

Annotate namespaces to opt specific workloads into log collection:

```yaml
# namespace annotation
annotations:
  ad.datadoghq.com/logs: '[{"source": "python", "service": "payments-api"}]'
```

### Infrastructure-as-code: Terraform

Use the `DataDog/datadog` Terraform provider to codify monitors and SLOs. Drift in monitor configuration is a common source of silent alert degradation.

```hcl
resource "datadog_monitor" "payments_error_rate" {
  name    = "payments-api error rate high"
  type    = "metric alert"
  message = "Error rate above threshold. Runbook: https://runbooks.internal/payments-degraded @pagerduty-platform"

  query = "sum(last_5m):sum:trace.payments-api.request.errors{env:production}.as_rate() / sum:trace.payments-api.request.hits{env:production}.as_rate() * 100 > 5"

  monitor_thresholds {
    critical = 5
    warning  = 1
  }

  tags = ["team:platform", "service:payments-api", "env:production", "sli:availability"]
}

resource "datadog_service_level_objective" "payments_availability" {
  name        = "platform/payments-api/availability"
  type        = "metric"
  description = "99.9% of payment requests return a non-5xx response over 30 days."

  query {
    numerator   = "sum:trace.payments-api.request.hits{http.status_code < 500,env:production}.as_count()"
    denominator = "sum:trace.payments-api.request.hits{env:production}.as_count()"
  }

  thresholds {
    timeframe = "30d"
    target    = 99.9
    warning   = 99.95
  }

  tags = ["team:platform", "service:payments-api", "env:production", "journey:checkout", "sli:availability"]
}
```


## References

- Unified Service Tagging: <https://docs.datadoghq.com/getting_started/tagging/unified_service_tagging/>
- APM Tracing: <https://docs.datadoghq.com/tracing/>
- Log-Trace Correlation: <https://docs.datadoghq.com/tracing/other_telemetry/connect_logs_and_traces/>
- Log Management Best Practices: <https://docs.datadoghq.com/logs/guide/best-practices-for-log-management/>
- DogStatsD: <https://docs.datadoghq.com/developers/dogstatsd/>
- Service Level Objectives: <https://docs.datadoghq.com/service_management/service_level_objectives/>
- SLO Best Practices (Datadog blog): <https://www.datadoghq.com/blog/define-and-manage-slos/>
- Monitor Tagging Best Practices: <https://www.datadoghq.com/blog/tagging-best-practices-monitors/>
- Agent Environment Variables: <https://docs.datadoghq.com/agent/guide/environment-variables/>

---


# New Relic and Dynatrace APM

## When to Use

### New Relic shines when

- You need a developer-first, code-level APM with broad language agent coverage (Node, Python, Java, Go, Ruby, .NET, PHP).
- Your team lives in NRQL — the query language is powerful, approachable, and consistent across all telemetry types (metrics, events, logs, traces).
- You want flexible, cost-visible pricing tied to data ingestion (100 GB/month free tier).
- You're building dashboards and alert conditions from scratch without relying on AI-driven auto-detection; you want explicit control.
- You need tightly integrated browser monitoring, synthetic monitoring, and mobile APM in one account hierarchy.

#### Dynatrace shines when

- You want zero-configuration auto-instrumentation across the full stack — OneAgent deploys once per host and discovers every process, JVM, .NET runtime, Node process, and container automatically.
- You need causal AI (Davis) to reduce alert noise: hundreds of individual signals collapse into a single actionable Problem card with a pinpointed root cause.
- Your environment is heavily Kubernetes/containerized — Dynatrace's topology map (Smartscape) builds entity relationships automatically from observed traffic.
- SLO management and error budget burn rates need to tie directly into automated problem prioritization.
- You need OpenTelemetry ingestion without giving up enriched auto-instrumentation where it matters.

**Key tradeoff:** New Relic gives you more query flexibility and lower initial complexity; Dynatrace gives you deeper auto-discovery and smarter noise reduction at the cost of higher licensing price and more vendor lock-in on the OneAgent model.


## New Relic: NRQL Patterns

NRQL operates on four data types: `Transaction`, `Metric`, `Log`, `Span`. All use the same syntax.

### Golden signals

#### Latency — p50/p95/p99

```sql
FROM Transaction
SELECT percentile(duration, 50, 95, 99)
WHERE appName = 'prod-checkout-node'
SINCE 1 HOUR AGO TIMESERIES
```

#### Error rate

```sql
FROM Transaction
SELECT percentage(count(*), WHERE error IS TRUE) AS 'Error %'
WHERE appName = 'prod-checkout-node'
SINCE 30 MINUTES AGO TIMESERIES
```

#### Throughput (RPM)

```sql
FROM Transaction
SELECT rate(count(*), 1 minute) AS 'RPM'
WHERE appName = 'prod-checkout-node'
SINCE 1 HOUR AGO TIMESERIES
```

#### Saturation — error trend across all apps

```sql
FROM Transaction
SELECT percentage(count(*), WHERE error IS TRUE) AS 'Errors'
SINCE 2 DAYS AGO
FACET appName
```

#### SLI — success baseline over a week

```sql
FROM Transaction
SELECT percentage(count(*), WHERE error IS FALSE) AS 'Success SLI'
WHERE appName = 'prod-checkout-node'
SINCE 1 WEEK AGO
```

#### Percentile latency as SLI

```sql
FROM Transaction
SELECT percentile(duration, 95) AS 'p95 Latency SLI'
WHERE appName = 'prod-checkout-node'
SINCE 1 WEEK AGO
```

#### APM metric timeslice data

```sql
FROM Metric
SELECT average(apm.service.transaction.duration)
WHERE appName = 'prod-checkout-node'
FACET transactionName
SINCE 1 HOUR AGO TIMESERIES
```

#### Slow transaction investigation

```sql
FROM Transaction
SELECT average(duration), max(duration), count(*)
WHERE appName = 'prod-checkout-node' AND duration > 2.0
FACET request.uri
SINCE 1 HOUR AGO
LIMIT 20
```

#### Prometheus histogram percentile (when using Prometheus remote write)

```sql
FROM Metric
SELECT bucketPercentile(prometheus_http_request_duration_seconds_bucket, 95)
SINCE 1 HOUR AGO TIMESERIES
```


## New Relic: Distributed Tracing

### Modes

- **Standard distributed tracing** (head-based sampling, default) — agent samples a representative subset based on throughput. Adaptive: sampling rate increases at low traffic, scales back at high traffic.
- **Infinite Tracing** (tail-based sampling) — 100% of spans are forwarded to a New Relic-managed trace observer; only anomalous or slow traces are retained. Requires `NEW_RELIC_INFINITE_TRACING_TRACE_OBSERVER_HOST`. Adds ~1–2 minute latency to trace availability in UI.

### W3C Trace Context

All agents (Node ≥ 7.3.0, Java ≥ 6.4.0, Python ≥ 5.22.0) propagate both `traceparent`/`tracestate` W3C headers and the legacy `newrelic` header by default. To use only W3C headers for interop with non-New Relic OTel services:

```python
NEW_RELIC_DISTRIBUTED_TRACING_EXCLUDE_NEWRELIC_HEADER=true
```

Distributed tracing and legacy Cross-Application Tracing (CAT) are mutually exclusive. Disable CAT explicitly:

```text
NEW_RELIC_CROSS_APPLICATION_TRACER_ENABLED=false
```

### Custom span attributes

```js
// Node.js — add business context to spans
const newrelic = require('newrelic');
newrelic.addCustomSpanAttribute('db.instance', dbName);
newrelic.addCustomSpanAttribute('customer.id', customerId);
newrelic.addCustomSpanAttribute('order.id', orderId);
```

Then query spans directly:

```sql
FROM Span
SELECT average(duration.ms)
WHERE customer.id = '12345'
SINCE 1 HOUR AGO
```

### Sampling gotcha

Head-based sampling will miss the specific slow or erroring request you need in production. Use Infinite Tracing for critical low-volume workflows (batch jobs, async workers, payment flows) where every span matters. Be aware of the extra outbound gRPC port 443 requirement to the trace observer host — firewall rules block this silently.


## Dynatrace: DQL

DQL runs on Grail, Dynatrace's columnar data lakehouse. All signal types — logs, metrics, traces, events — are queryable through a unified pipe-based syntax.

### Syntax fundamentals

Commands are chained with `|`. Each command returns tabular output (records and fields).

```bash
fetch <data-type>
| filter <condition>
| summarize <aggregation>, by: {<field>}
| sort <field> [asc|desc]
| limit <n>
```

Data types: `logs`, `spans`, `metrics`, `events`, `bizevents`

Always include a time scope — without one, `fetch logs` defaults to the last 2 hours:

```dql
fetch logs
| filter timestamp >= now() - 24h
```

### Log queries

#### Error logs for a service

```dql
fetch logs
| filter dt.entity.service == "SERVICE-ABC123"
| filter loglevel == "ERROR"
| summarize count(), by: {bin(timestamp, 1m)}
| sort timestamp desc
```

#### HTTP status code distribution parsed from log content

```dql
fetch logs
| parse content, "LD 'HTTP/' LD ' ' INT:status_code"
| summarize count(), by: {status_code}
| sort count() desc
```

#### Define a metric from log field for alerting

Extract a numeric field (e.g., `response_time_ms`) from logs using `parse`, then create a Metric event from logs in the Dynatrace UI to back an SLO or alert.

### Span / trace queries

#### Slow spans by operation

```dql
fetch spans
| filter dt.entity.service == "SERVICE-ABC123"
| filter duration > 2000ms
| summarize avg(duration), p95(duration), count(), by: {span.name}
| sort p95(duration) desc
| limit 20
```

#### Error spans with trace IDs for cross-signal investigation

```dql
fetch spans
| filter status == "ERROR"
| fields timestamp, trace_id, span.name, error.message, duration
| sort duration desc
| limit 50
```

#### Log-trace pivot — all logs for a specific trace

```dql
fetch logs
| filter trace_id == "abc123def456"
| fields timestamp, loglevel, content, trace_id, span_id
| sort timestamp asc
```

### Metric timeseries

#### Host CPU utilization

```dql
timeseries avg(dt.host.cpu.usage), by: {host.name}
| filter host.name == "prod-web-01"
```

#### Service p95 response time across all services

```dql
timeseries p95(dt.service.response.time), by: {dt.entity.service}
```


## Dynatrace: SLOs and Burndown

### SLO types

- **Service SLO** — based on success rate or latency of a Dynatrace-monitored service.
- **Synthetic SLO** — based on synthetic monitor availability or performance.
- **Custom metric SLO** — any DQL-derived or ingested metric can back an SLO.

### Error budget burn rate

- Burn rate of **1.0** = consuming budget at exactly the rate that exhausts it at the end of the SLO window.
- Burn rate of **2.0** = consuming at double rate — SLO will breach in half the remaining window.
- Dynatrace calculates burn rate automatically from the SLO target and real-time success rate.

### Two-window alerting (Google SRE model)

- **Fast burn alert:** 1-hour evaluation window, burn rate > 14.4 → page immediately. At this rate you burn a full month's error budget in ~2 hours.
- **Slow burn alert:** 6-hour evaluation window, burn rate > 6 → create a ticket. On track to exhaust budget before end of SLO window.

Davis AI also **predicts SLO violations** by projecting burn rate trend forward and opening a Problem card before the breach occurs.

### Configuring an SLO

Settings → Service-level objectives → Create SLO:

- Type: Availability or Performance
- Target: `99.9%`
- Warning: `99.95%` (gives lead time before breach)
- Timeframe: Rolling 7d or 30d
- Filter: `dt.entity.service == "SERVICE-ABC123"`

For performance SLOs, the success metric is typically: `(requests with duration < threshold) / total requests`.


## Key APIs / CLI

### New Relic (Key APIs / CLI)

#### NerdGraph (GraphQL API)

```bash
curl -X POST https://api.newrelic.com/graphql \
  -H "API-Key: $NEW_RELIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ actor { account(id: 12345) { name } } }"}'
```

#### Custom Event API

```text
POST https://insights-collector.newrelic.com/v1/accounts/<ID>/events
```

#### Metric API

```text
POST https://metric-api.newrelic.com/metric/v1
```

#### Log API

```text
POST https://log-api.newrelic.com/log/v1
```

#### New Relic CLI — deployment marker

```bash
newrelic apm deployment create \
  --applicationId 12345 \
  --revision "v2.1.0" \
  --user "ci-pipeline"
```

### Dynatrace (Key APIs / CLI)

#### OTLP trace ingest

```text
POST https://<env>.live.dynatrace.com/api/v2/otlp/v1/traces
Authorization: Api-Token <token>
Content-Type: application/x-protobuf
```

#### Metrics ingest (line protocol)

```text
POST https://<env>.live.dynatrace.com/api/v2/metrics/ingest
Content-Type: text/plain

custom.request.count,env=prod count,delta=42 1672531200000
```

#### BizEvents API

```text
POST https://<env>.live.dynatrace.com/api/v2/bizevents
```

#### Grail DQL query API

```sql
POST https://<env>.live.dynatrace.com/platform/storage/query/v1/query:execute
Body: { "query": "fetch logs | filter loglevel == \"ERROR\" | limit 10" }
```

#### Monaco (config-as-code) deployment

```bash
monaco deploy --environment prod --project myapp
```

