---
name: prometheus-comprehensive
description: Comprehensive Prometheus reference covering PromQL query patterns, metric types (counter/gauge/histogram/summary), recording and alerting rules, label cardinality, scrape configuration, federation, long-term storage (Thanos/VictoriaMetrics), and Alertmanager configuration (routing trees, inhibition, silences, receivers, HA clustering, amtool).
domain: observability
tags: [prometheus, promql, alertmanager, metrics, alerting, recording-rules, histogram, thanos, routing, inhibition, silences]
triggers: prometheus, promql, alertmanager, alerting rules, recording rules, metric types, histogram, counter, gauge, routing tree, inhibition, silences, thanos, victoriametrics
---


# Prometheus & PromQL Best Practices

## 1. Metric Types

### Counter

A counter only increases (or resets to zero on process restart). Use for totals: requests served, bytes sent, errors encountered.

```text
http_requests_total{method="GET", status="200"}
process_cpu_seconds_total
```

Never expose the raw counter in a dashboard or alert. Always derive a rate:

```promql
rate(http_requests_total[5m])      # per-second average over 5m window
increase(http_requests_total[1h])  # total increase over 1 hour (rate * range)
```

`increase()` is syntactic sugar for `rate() * range_seconds`. Both extrapolate, so values are estimates, not exact integer counts.

`irate()` uses only the last two samples — reactive to spikes but noisy over longer windows. Use `rate()` for dashboards, `irate()` only when you need to catch short-lived spikes.

### Gauge

A gauge can go up or down. Use for current state: memory in use, active connections, queue depth, temperature.

```text
node_memory_MemAvailable_bytes
go_goroutines
```

Never take `rate()` of a gauge. Use it directly, or with `deriv()` to estimate slope:

```promql
deriv(node_memory_MemAvailable_bytes[15m])  # rate of change in bytes/sec
```

### Histogram

A histogram samples observations into pre-configured buckets and exposes three series:

| Suffix | Meaning |
| --- | --- |
| `_bucket{le="0.1"}` | Count of observations ≤ 0.1 |
| `_sum` | Sum of all observed values |
| `_count` | Total number of observations |

Use `histogram_quantile()` to estimate percentiles server-side:

```promql
# 95th percentile latency, aggregated across all instances
histogram_quantile(0.95,
  sum by (le) (
    rate(http_request_duration_seconds_bucket[5m])
  )
)

# Per-service 99th percentile
histogram_quantile(0.99,
  sum by (le, service) (
    rate(http_request_duration_seconds_bucket[5m])
  )
)
```

`histogram_quantile()` interpolates linearly within the bucket containing the percentile. Accuracy depends on bucket width — narrower buckets near expected values give tighter estimates. Always include `le` in the `sum by ()` clause; leaving it out breaks the function.

**Bucket selection:** cover the expected range with a geometric or linear sequence. For HTTP latency in seconds:

```yaml
buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]
```

**Native histograms** (stable in Prometheus 3.x) eliminate the need to pre-configure buckets by using a sparse, high-resolution bucket representation. Prefer them for new instrumentation when the client library supports it.

### Summary

A summary computes quantiles client-side and exposes them as `{quantile="0.95"}` labels. Observations are expensive (streaming quantile algorithm). The critical limitation: **summary quantiles cannot be aggregated across instances**. Averaging the 95th percentile of two instances gives a statistically meaningless number.

Use summaries only when:

- You need accurate quantiles on a single instance with a known distribution
- You cannot predict the value range for histogram buckets

In nearly every other case, prefer histograms.


## 3. Label Cardinality

Every unique combination of label values creates a separate time series. Cardinality is multiplicative:

```text
5 methods × 10 status codes × 200 endpoints = 10,000 series per metric
```

High cardinality degrades ingestion throughput, query latency, and RAM usage. Prometheus stores all active series in memory.

### Hard Rules

- Never use user IDs, session IDs, email addresses, UUIDs, or request IDs as labels
- Never use unbounded string values (full URL paths, SQL queries, error messages)
- Normalize dynamic URL segments before labeling: `/user/12345` → `/user/{id}`
- Investigate anything above 10,000 series per metric

### Acceptable Cardinality

| Label | Approximate Cardinality | Safe? |
| --- | --- | --- |
| `status` (HTTP) | ~10 | Yes |
| `method` (HTTP) | ~5 | Yes |
| `service` | ~100 | Usually |
| `instance` | ~1000 | With care |
| `user_id` | Millions | Never |

### Detecting Cardinality Problems

```promql
# Top 10 metrics by series count
topk(10, count by (__name__)({__name__=~".+"}))

# Series count for a specific metric
count(http_requests_total)
```


## 5. Recording Rules

Recording rules precompute expensive or frequently-used expressions and store results as new time series, evaluated at the group's interval (default: global `evaluation_interval`).

### When to Use

- Dashboard panels that run the same heavy aggregation on every render
- Alerting rules that aggregate across thousands of series
- Ratio queries used in multiple places
- Any query taking >1 second to evaluate

### Naming Convention: `level:metric:operations`

```text
level      — aggregation level and retained labels (e.g., instance_path, job, cluster)
metric     — original metric name; strip _total when rate/irate is applied
operations — list of operations applied, newest first
```

```yaml
groups:
  - name: request_rates
    interval: 30s
    rules:
      # Raw per-instance, per-path rate
      - record: instance_path:requests:rate5m
        expr: |
          rate(http_requests_total[5m])

      # Aggregated to path level
      - record: path:requests:rate5m
        expr: |
          sum without (instance) (instance_path:requests:rate5m)

      # Job-level failure ratio
      - record: job:request_failures_per_requests:ratio_rate5m
        expr: |
          sum without (instance, path) (
            instance_path:request_failures:rate5m
          )
          /
          sum without (instance, path) (
            instance_path:requests:rate5m
          )
```

### Key Rules

- Aggregate ratios by summing numerator and denominator separately, then dividing. Never average ratios.
- Use `without` rather than `by` to preserve future label additions.
- Strip `_total` suffix when applying `rate()` — the result is a per-second rate, not a count.
- For summaries, use `mean` in the operations field, not `rate`.


## 7. Scrape Configuration

### Basic Job

```yaml
scrape_configs:
  - job_name: "api"
    scrape_interval: 15s
    scrape_timeout: 10s
    metrics_path: /metrics
    scheme: http
    static_configs:
      - targets: ["api-1:8080", "api-2:8080"]
        labels:
          env: production
```

### Service Discovery

```yaml
  - job_name: "kubernetes-pods"
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      # Only scrape pods with annotation prometheus.io/scrape: "true"
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: "true"
      # Use annotation for custom path
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      # Attach namespace as label
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace
```

### Relabeling Tips

- `keep` and `drop` actions filter targets before scraping — use them to avoid scraping irrelevant pods
- `__address__` and `__metrics_path__` are special labels that control the scrape target
- `honor_labels: true` on federation jobs preserves the origin labels from source servers


## 9. Remote Write and Long-Term Storage

Prometheus's local TSDB is optimized for 15 days to a few months of retention. For longer retention or global query views across multiple Prometheus instances, use remote write.

### Remote Write Configuration

```yaml
remote_write:
  - url: "https://thanos-receiver:10908/api/v1/receive"
    queue_config:
      capacity: 20000
      max_shards: 30
      max_samples_per_send: 10000
      batch_send_deadline: 5s
    write_relabel_configs:
      # Drop high-cardinality debug metrics before sending
      - source_labels: [__name__]
        regex: "go_gc_.*|debug_.*"
        action: drop
```

Remote write increases Prometheus memory usage by roughly 25%. Tune `max_shards` and `capacity` based on ingestion rate.

### Solution Comparison

| Solution | Architecture | Object Storage | Global Query | Complexity |
| --- | --- | --- | --- | --- |
| **Thanos** | Sidecar + Store Gateway + Querier | S3, GCS, Azure | Yes | High |
| **Cortex / Grafana Mimir** | Fully distributed microservices | S3, GCS, Azure | Yes | Very High |
| **VictoriaMetrics** | Single binary or cluster | Local / EBS | With vmselect | Low–Medium |
| **Grafana Cloud** | Managed | Managed | Yes | Low (managed) |

**Thanos** attaches a sidecar to each Prometheus pod that uploads TSDB blocks to object storage. The Querier fan-outs queries across Store Gateways and live Prometheus instances. Good choice when you already have object storage and want minimal changes to existing Prometheus deployments.

**VictoriaMetrics** is a drop-in Prometheus-compatible backend. Single-node handles tens of millions of active series on modest hardware. Cluster mode provides horizontal scale. Simpler operationally than Thanos. Accepts Prometheus remote write natively.

**Grafana Mimir** (successor to Cortex) is designed for massive multi-tenant deployments. Highest operational complexity but most feature-complete.


## 11. Instrumentation Checklist

For any online-serving system, expose at minimum:

| Metric | Type | What to Measure |
| --- | --- | --- |
| `<app>_requests_total` | Counter | All requests |
| `<app>_request_errors_total` | Counter | Failed requests, labeled by error type |
| `<app>_request_duration_seconds` | Histogram | Full request latency |
| `<app>_requests_in_flight` | Gauge | Currently active requests |
| `<app>_build_info` | Gauge (info) | Version, commit, build date as labels |

For offline/batch systems, also expose:

| Metric | Type | What to Measure |
| --- | --- | --- |
| `<app>_last_success_timestamp_seconds` | Gauge | Unix time of last successful run |
| `<app>_items_processed_total` | Counter | Items processed per stage |
| `<app>_processing_duration_seconds` | Histogram | Per-stage duration |

Export a Unix timestamp of the last success rather than "seconds since last success" — the latter requires the process to be running to stay accurate.

For every log line that indicates a warning or error condition, also increment a counter. This enables alerting on error rates without log parsing.


---


# Prometheus Alertmanager Configuration

## Overview

Alertmanager sits between Prometheus (or any compatible client) and your notification channels. Its job is to deduplicate alerts that fire across multiple replicas, group related alerts into coherent notifications, route them to the right team, and suppress noise via inhibition and silences. Every alert Prometheus sends is evaluated against a routing tree; when a matching route is found the alert is batched with its group, timing rules applied, and the group is dispatched to a receiver.

Key pipeline: **receive → deduplicate → group → route → inhibit/silence check → notify**


## Matchers Syntax

Alertmanager supports two matcher syntaxes; the modern UTF-8 syntax is preferred.

```text
label = "exact-value"        # equality
label != "exact-value"       # inequality
label =~ "regex.*"           # regex match
label !~ "regex.*"           # regex non-match
```

Multiple matchers in a list use AND logic — all must match.

```yaml
matchers:
  - severity =~ "warning|critical"
  - env = "production"
  - team != "infra"
```

Quote values containing special characters. Enable UTF-8 strict mode with `--enable-feature="utf8-strict-mode"` to enforce this globally; without it, Alertmanager defaults to fallback mode (tries UTF-8, falls back to classic).


## Silences

Silences are manual, time-bounded suppressions applied on top of the routing tree. They do not affect routing; matched alerts still traverse routes but notifications are dropped.

### Silence via amtool

```bash
# Silence a specific host for 4 hours during maintenance
amtool silence add instance="db-01.prod" \
  --duration=4h \
  --comment="Planned maintenance, DB schema migration" \
  --author="ops-team"

# Silence a regex pattern of alert names
amtool silence add 'alertname=~"DiskSpace.*"' severity="warning" \
  --duration=2h \
  --comment="Investigating disk issue"

# List active silences
amtool silence query

# List all silences including expired
amtool silence query --expired

# Expire a silence immediately
amtool silence expire <silence-id>

# Bulk expire all active silences (use carefully)
amtool silence query -q | xargs -I {} amtool silence expire {}
```

### Silence via REST API

```bash
# Create a silence (POST /api/v2/silences)
curl -X POST http://alertmanager:9093/api/v2/silences \
  -H "Content-Type: application/json" \
  -d '{
    "matchers": [
      {"name": "instance", "value": "db-01.prod", "isRegex": false},
      {"name": "severity", "value": "warning", "isRegex": false}
    ],
    "startsAt": "2024-01-15T10:00:00Z",
    "endsAt": "2024-01-15T14:00:00Z",
    "comment": "Maintenance window",
    "createdBy": "ops-team"
  }'

# List active silences (GET /api/v2/silences)
curl http://alertmanager:9093/api/v2/silences

# Delete/expire a silence (DELETE /api/v2/silence/{id})
curl -X DELETE http://alertmanager:9093/api/v2/silence/<id>
```

### Silence Storage

Silences are stored in-memory with periodic disk snapshots. In a cluster they synchronize across peers via the gossip protocol. Expired silences are garbage-collected after 5 days by default.


## Time Intervals (Maintenance Windows)

```yaml
time_intervals:
  - name: business-hours
    time_intervals:
      - times:
          - start_time: "09:00"
            end_time: "17:00"
        weekdays: ["monday:friday"]
        location: "America/New_York"

  - name: weekend
    time_intervals:
      - weekdays: ["saturday", "sunday"]

route:
  receiver: default
  routes:
    - matchers:
        - team = "platform"
      receiver: platform-pagerduty
      # Only page during business hours; outside those hours use a lower-urgency receiver
      active_time_intervals:
        - business-hours

    - matchers:
        - team = "platform"
      receiver: platform-slack
      mute_time_intervals:
        - business-hours    # Slack gets it outside business hours
```

# StatefulSet so pod DNS names are stable (am-0, am-1, am-2)
# Pass peers via init container or downward API
```


## Testing with amtool

amtool ships with Alertmanager and is the primary tool for config validation and route testing.

### Validate Configuration

```bash
# Check config file syntax and references
amtool check-config /etc/alertmanager/alertmanager.yml

# Validate against a running instance
amtool --alertmanager.url=http://localhost:9093 config show
```

### Test Route Matching

```bash
# Show which route and receiver an alert would match
amtool --alertmanager.url=http://localhost:9093 config routes test \
  alertname="HighMemory" \
  severity="critical" \
  team="platform" \
  cluster="prod-us-east"

# Visualize the full routing tree
amtool --alertmanager.url=http://localhost:9093 config routes
```

### Inspect Live Alerts

```bash
# List all currently firing alerts
amtool --alertmanager.url=http://localhost:9093 alert

# Filter by label
amtool --alertmanager.url=http://localhost:9093 alert query severity="critical"

# JSON output for scripting
amtool --alertmanager.url=http://localhost:9093 alert query --output=json
```

### amtool Configuration File

Create `~/.config/amtool/config.yml` to avoid repeating `--alertmanager.url`:

```yaml
alertmanager.url: http://localhost:9093
author: ops-team
comment_required: true
output: extended
```


## Minimal Complete Example

```yaml
global:
  resolve_timeout: 5m
  slack_api_url: "https://hooks.slack.com/services/T000/B000/xxx"
  pagerduty_url: "https://events.pagerduty.com/v2/enqueue"

time_intervals:
  - name: business-hours
    time_intervals:
      - times:
          - start_time: "09:00"
            end_time: "18:00"
        weekdays: ["monday:friday"]

inhibit_rules:
  - source_matchers:
      - severity = "critical"
    target_matchers:
      - severity = "warning"
    equal:
      - alertname
      - cluster
      - service

route:
  receiver: default-slack
  group_by: [alertname, cluster, service]
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 12h

  routes:
    # Log all alerts to audit webhook before team routing
    - receiver: audit-webhook
      continue: true

    # Platform team
    - matchers:
        - team = "platform"
      receiver: platform-slack
      routes:
        - matchers:
            - severity = "critical"
            - env = "production"
          receiver: platform-pagerduty
          group_wait: 10s
          repeat_interval: 1h

    # Data team
    - matchers:
        - team = "data"
      receiver: data-slack
      mute_time_intervals:
        - business-hours   # Mute during business hours; use tickets instead

receivers:
  - name: default-slack
    slack_configs:
      - channel: "#alerts-general"
        send_resolved: true

  - name: audit-webhook
    webhook_configs:
      - url: "https://audit.internal/alertmanager"
        send_resolved: true

  - name: platform-slack
    slack_configs:
      - channel: "#platform-alerts"
        send_resolved: true
        color: '{{ if eq .Status "firing" }}danger{{ else }}good{{ end }}'
        title: '[{{ .Status | toUpper }}] {{ .CommonLabels.alertname }}'
        text: '{{ .CommonAnnotations.summary }}'

  - name: platform-pagerduty
    pagerduty_configs:
      - routing_key: "<platform-integration-key>"
        severity: critical
        description: '{{ .CommonAnnotations.summary }}'
        send_resolved: true

  - name: data-slack
    slack_configs:
      - channel: "#data-alerts"
        send_resolved: true
```

