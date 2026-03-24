---
name: observability-stack
description: Complete observability stack covering OpenTelemetry instrumentation (traces, metrics, logs, context propagation), OTel Collector configuration (receivers, processors, exporters, pipelines), structured logging, distributed tracing, SLO/SLI alerting, Grafana dashboards, Sentry, and agent flow telemetry. Use when building or configuring observability infrastructure.
domain: infrastructure
tags: [observability, opentelemetry, otel-collector, logging, tracing, slo, sli, grafana, sentry, alerting, telemetry]
triggers: observability, opentelemetry, otel collector, structured logging, distributed tracing, SLO, SLI, grafana, sentry, alerting, telemetry
---


# Observability Comprehensive Guide

## 1. OpenTelemetry Instrumentation

### Three Signals

- **Traces**: End-to-end request flow across services. Composed of spans with parent-child relationships.
- **Metrics**: Numeric measurements over time (counters, gauges, histograms).
- **Logs**: Timestamped event records with structured context.

### Auto-Instrumentation

Most languages have auto-instrumentation packages that capture traces/metrics for common frameworks without code changes:

- **Node.js**: `@opentelemetry/auto-instrumentations-node`
- **Python**: `opentelemetry-instrumentation`
- **Java**: `opentelemetry-javaagent`
- **Go**: Manual instrumentation required (use `otel` SDK)

### Manual Spans

```typescript
const tracer = trace.getTracer('my-service');
const span = tracer.startSpan('process-order', {
  attributes: { 'order.id': orderId, 'order.total': total }
});
try {
  // ... processing
  span.setStatus({ code: SpanStatusCode.OK });
} catch (err) {
  span.setStatus({ code: SpanStatusCode.ERROR, message: err.message });
  span.recordException(err);
  throw err;
} finally {
  span.end();
}
```

### Context Propagation

- Use W3C Trace Context headers (`traceparent`, `tracestate`) for cross-service propagation.
- Inject context into outgoing HTTP requests. Extract from incoming requests.
- For message queues: embed trace context in message headers/metadata.

### Exporter Configuration

Export to collectors (OTel Collector, Jaeger, Tempo, Datadog) via OTLP (gRPC or HTTP):

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
OTEL_SERVICE_NAME=my-service
OTEL_RESOURCE_ATTRIBUTES=deployment.environment=production
```

## 2. Structured Logging

### Rules

- Log as JSON, not free-form text. Every log entry: timestamp, level, message, and structured context fields.
- Include trace ID and span ID in every log for correlation with traces.
- Use consistent field names across services (`user_id`, not `userId` in one and `user` in another).
- Log at appropriate levels: ERROR (action needed), WARN (degraded), INFO (significant events), DEBUG (development only, disabled in prod).

### What to Log

- **Always**: Request start/end, errors, authentication events, significant state changes.
- **Never**: Passwords, tokens, PII, full request bodies with sensitive data.
- **Conditionally**: Debug-level details, full payloads (in non-production).

### Correlation

- Attach `trace_id` and `span_id` to every log line.
- Use request-scoped context (middleware) to automatically inject IDs.
- This enables: click on trace → see all related logs, and vice versa.

## 3. Distributed Tracing

### When Essential

- Microservice architectures (>3 services in a request path).
- Debugging latency issues (which service is slow?).
- Understanding request flow and dependencies.
- Identifying retry storms, cascading failures.

### Trace Design

- Create a span for every significant operation: HTTP request, DB query, cache lookup, queue publish/consume.
- Name spans descriptively: `HTTP GET /api/users`, `postgres.query`, `redis.get`.
- Add relevant attributes: `http.method`, `db.statement` (sanitized), `user.id`.

### Sampling

- **Head sampling**: Decide at trace start (fastest, may miss interesting traces). Use 1-10% for high-traffic services.
- **Tail sampling** (at collector): Decide after trace completes. Keep 100% of errors, slow requests, and flagged traces. Sample normal traces at lower rate.

## 4. SLOs, SLIs, and Alerting

### Definitions

- **SLI** (Service Level Indicator): Measured metric (e.g., "99.2% of requests completed in <200ms").
- **SLO** (Service Level Objective): Target for an SLI (e.g., "99.5% of requests under 200ms over 30 days").
- **Error budget**: `100% - SLO` = allowable failure. When budget is exhausted, prioritize reliability over features.

### Choosing SLIs

| Service Type | Primary SLI |
|-------------|-------------|
| API | Latency (p99), availability (success rate) |
| Data pipeline | Freshness (max delay), correctness |
| Background job | Completion rate, processing time |
| Storage | Durability, availability |

### Alert Design

- Alert on SLO burn rate, not individual errors. "We've consumed 50% of our 30-day error budget in 1 hour" is actionable.
- Use multi-window burn rates: fast (5m) for pages, slow (6h) for tickets.
- Route by severity: page for fast burn, ticket for slow burn, dashboard for informational.
- Include runbook links in every alert. Context: what's wrong, what to check, what to do.

### Tooling

- **Prometheus/Alertmanager**: Open-source metrics and alerting.
- **Grafana**: Dashboards for metrics, logs, and traces.
- **PagerDuty/OpsGenie**: Incident management and on-call routing.

## 5. Grafana Dashboards

### Design Principles

- **USE method** (Utilization, Saturation, Errors) for infrastructure dashboards.
- **RED method** (Rate, Errors, Duration) for service dashboards.
- Top row: health summary (green/red indicators). Detail panels below.
- Use variables for environment, service, and time range selection.

### Panel Types

| Data | Panel Type |
|------|-----------|
| Request rate | Time series (line) |
| Error rate | Time series + threshold |
| Latency percentiles | Time series (p50, p95, p99) |
| Current status | Stat or gauge |
| Log volume | Bar chart |

## 6. Sentry Error Tracking

### Integration

- Install SDK, configure DSN. Auto-captures unhandled exceptions.
- Add breadcrumbs for context leading up to errors.
- Set release version for tracking regressions.
- Use environments (production, staging) for filtering.

### Best Practices

- Set sample rate (0.1-1.0) based on traffic volume.
- Configure ignore rules for expected errors (404s, bots).
- Use Sentry issues for prioritization (group similar errors).
- Set up alerts for new issues and regression.

## 7. Agent/MCP Flow Telemetry

- One trace per user request or agent task. Spans for each logical step: RECALL, RESEARCH, tool call, EVOLUTION.
- Track token counts (input/output) per call and per session.
- Monitor retrieval quality: chunks returned, scores, source paths.
- Track tool execution: which tools called, arguments, duration.
- Respect privacy: hash or redact sensitive fields in traces.

## Checklist

- [ ] OpenTelemetry SDK configured with auto-instrumentation
- [ ] Traces, metrics, and logs exported to collector
- [ ] Structured JSON logging with trace correlation IDs
- [ ] SLIs defined for each critical service
- [ ] SLOs set with error budget tracking
- [ ] Alerts based on burn rate, not individual errors
- [ ] Grafana dashboards using RED/USE methods
- [ ] Sentry configured with appropriate sample rates
- [ ] Tail sampling keeping 100% of errors and slow traces

---


# OpenTelemetry Collector Configuration

## Architecture

The Collector is a single vendor-agnostic binary that receives, processes, and exports telemetry (traces, metrics, logs). It decouples instrumentation from backend choice and centralises cross-cutting concerns like sampling, batching, and credential management.

### Deployment topologies

**No Collector** — SDK exports directly to backend. Simplest path; no separation of concerns and no cross-cutting policy.

**Agent** — A Collector instance runs alongside every application host (DaemonSet on Kubernetes, sidecar on VMs). Applications export via OTLP to `localhost`. The agent offloads retries, batching, and host-level metadata enrichment from the application. Preferred when you need `hostmetricsreceiver` or `filelogreceiver` because those require local filesystem/kernel access.

**Gateway** — One or more Collector Deployments per cluster/region act as a centralised OTLP endpoint. Agents (or SDKs) forward to the gateway, which applies sampling, credential management, and fan-out to multiple backends. Add a load balancer (NGINX or the `loadbalancing` exporter) in front of multiple gateway instances.

**Agent + Gateway (layered)** — The standard production pattern. Agents collect host/pod telemetry and forward via OTLP to a gateway tier. Gateways handle tail sampling, routing, and backend fan-out. Keeps agent configs simple and concentrates operational complexity in the gateway.

When to use each:

- Agent only: single backend, low volume, simple ops.
- Gateway only: centralised policy, multiple backends, multi-team credential separation.
- Layered: production clusters, tail sampling at scale, >1 backend.


## Receivers

Receivers are the entry points for telemetry. They can be push-based (SDK sends to collector) or pull-based (collector scrapes a target).

### OTLP receiver (gRPC + HTTP)

The default receiver for any SDK-instrumented application. gRPC on 4317, HTTP/protobuf on 4318.

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
        max_recv_msg_size_mib: 4
      http:
        endpoint: 0.0.0.0:4318
        cors:
          allowed_origins: ["https://*.example.com"]
```

In production, bind to `localhost:4317` when the collector runs on the same node as the SDK and there is no need for external access.

### hostmetrics receiver

Scrapes system-level metrics from the host OS. Requires filesystem access — must run as a DaemonSet or on-host agent.

```yaml
receivers:
  hostmetrics:
    collection_interval: 30s
    scrapers:
      cpu:       {}
      disk:      {}
      load:      {}
      filesystem: {}
      memory:    {}
      network:   {}
      paging:    {}
      processes: {}
```

### prometheus receiver

Scrapes Prometheus endpoints. Accepts standard Prometheus `scrape_configs`.

```yaml
receivers:
  prometheus:
    config:
      scrape_configs:
        - job_name: my-service
          scrape_interval: 15s
          static_configs:
            - targets: ["localhost:8080"]
```

For cluster-wide scraping with sharding use the Target Allocator (OpenTelemetry Operator).

### filelog receiver

Tails log files and emits log records.

```yaml
receivers:
  filelog:
    include: [/var/log/pods/*/*/*.log]
    start_at: beginning
    include_file_path: true
    include_file_name: false
    operators:
      - type: container
        id: container-parser
```

### k8s_events receiver

Emits Kubernetes events as log records.

```yaml
receivers:
  k8s_events:
    namespaces: [default, production]
```


## Exporters

### otlp (gRPC)

```yaml
exporters:
  otlp:
    endpoint: otelcol-gateway:4317
    tls:
      insecure: false
      ca_file: /certs/ca.crt
    retry_on_failure:
      enabled: true
      initial_interval: 5s
      max_interval: 30s
      max_elapsed_time: 300s
    sending_queue:
      enabled: true
      num_consumers: 10
      queue_size: 5000
      storage: file_storage  # persistent queue via file_storage extension
```

### otlphttp (HTTP/protobuf)

```yaml
exporters:
  otlphttp:
    endpoint: https://ingest.example.com:4318
    headers:
      Authorization: "Bearer ${env:INGEST_TOKEN}"
    compression: gzip
    retry_on_failure:
      enabled: true
      max_elapsed_time: 120s
```

### prometheus (remote write)

Exposes a Prometheus scrape endpoint on the collector itself.

```yaml
exporters:
  prometheus:
    endpoint: 0.0.0.0:8889
    namespace: otelcol
    send_timestamps: true
    metric_expiration: 180m
```

### prometheusremotewrite

Pushes metrics to a Prometheus-compatible remote write endpoint (Cortex, Thanos, Mimir, VictoriaMetrics).

```yaml
exporters:
  prometheusremotewrite:
    endpoint: https://prometheus.example.com/api/v1/write
    tls:
      insecure_skip_verify: false
```

### debug (console — development only)

```yaml
exporters:
  debug:
    verbosity: detailed  # basic | normal | detailed
```

### Jaeger / Zipkin

Jaeger is deprecated in favour of OTLP. For legacy backends:

```yaml
exporters:
  zipkin:
    endpoint: http://zipkin:9411/api/v2/spans
    format: proto
```

### Vendor exporters

Available via `otelcol-contrib`: `datadog`, `dynatrace`, `newrelic`, `splunk_hec`, `elasticsearch`, `awsxray`, `googlecloud`, and many others. Reference the [opentelemetry-collector-contrib](https://github.com/open-telemetry/opentelemetry-collector-contrib) registry for the full list.


## Extensions

Extensions add operational capabilities outside of the data pipeline.

```yaml
extensions:
  health_check:
    endpoint: 0.0.0.0:13133   # GET /health returns 200 when ready
    path: /health
    check_collector_pipeline:
      enabled: true
      interval: 5m
      exporter_failure_threshold: 5

  pprof:
    endpoint: localhost:1777   # Go pprof profiling — never expose externally

  zpages:
    endpoint: localhost:55679  # /debug/tracez, /debug/pipelinez, /debug/rpcz

  basicauth/client:
    client_auth:
      username: "${env:EXPORTER_USER}"
      password: "${env:EXPORTER_PASSWORD}"

  file_storage:
    directory: /var/lib/otelcol/storage  # used by persistent queue
    timeout: 10s
    compaction:
      on_start: true
      directory: /var/lib/otelcol/storage/compaction

service:
  extensions: [health_check, pprof, zpages, file_storage]
```

`file_storage` is required for persistent queues on exporters. Without it, the sending queue is in-memory only and data is lost on restart.

apiVersion: v1
kind: Service
metadata:
  name: otelcol-gateway
  namespace: monitoring
spec:
  selector:
    app: otelcol-gateway
  ports:
    - name: otlp-grpc
      port: 4317
      targetPort: 4317
    - name: otlp-http
      port: 4318
      targetPort: 4318
```

### RBAC for k8s metadata

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: otelcol
  namespace: monitoring
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: otelcol
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: otelcol
subjects:
  - kind: ServiceAccount
    name: otelcol
    namespace: monitoring
```

### Helm (OpenTelemetry Operator)

The OpenTelemetry Operator manages collector lifecycle via `OpenTelemetryCollector` CRDs.

```bash
helm repo add open-telemetry https://open-telemetry.github.io/opentelemetry-helm-charts
helm install otel-operator open-telemetry/opentelemetry-operator \
  --namespace monitoring --create-namespace \
  --set admissionWebhooks.certManager.enabled=true
```

```yaml
apiVersion: opentelemetry.io/v1alpha1
kind: OpenTelemetryCollector
metadata:
  name: otelcol
  namespace: monitoring
spec:
  mode: daemonset   # daemonset | deployment | statefulset | sidecar
  serviceAccount: otelcol
  config: |
    receivers:
      otlp:
        protocols:
          grpc:
            endpoint: 0.0.0.0:4317
    processors:
      memory_limiter:
        check_interval: 1s
        limit_mib: 400
        spike_limit_mib: 100
      batch:
        timeout: 5s
    exporters:
      otlp:
        endpoint: otelcol-gateway:4317
    service:
      pipelines:
        traces:
          receivers:  [otlp]
          processors: [memory_limiter, batch]
          exporters:  [otlp]
```


## Tail Sampling

Tail sampling makes the sampling decision after an entire trace is received (vs. head sampling which decides at trace start). This allows policies like "always sample errors" or "sample 5% of successful traces". Tail sampling requires all spans for a trace to reach the same collector instance — use the `loadbalancing` exporter routed by `traceID` in front of a StatefulSet of tail-sampling collectors.

```yaml
processors:
  tail_sampling:
    decision_wait: 10s       # wait this long for all spans before deciding
    num_traces: 50000        # in-memory trace buffer size
    expected_new_traces_per_sec: 10
    policies:
      # Always sample traces with errors
      - name: errors-policy
        type: status_code
        status_code: {status_codes: [ERROR]}

      # Always sample slow traces (>1s)
      - name: slow-traces-policy
        type: latency
        latency: {threshold_ms: 1000}

      # Sample 10% of everything else
      - name: probabilistic-policy
        type: probabilistic
        probabilistic: {sampling_percentage: 10}

      # Composite: combine multiple policies with AND/OR logic
      - name: composite-policy
        type: composite
        composite:
          max_total_spans_per_second: 1000
          policy_order: [errors-policy, slow-traces-policy, probabilistic-policy]
          composite_sub_policy:
            - name: errors-policy
              type: status_code
              status_code: {status_codes: [ERROR]}
            - name: slow-traces-policy
              type: latency
              latency: {threshold_ms: 500}
          rate_allocation:
            - policy: errors-policy
              percent: 50
            - policy: slow-traces-policy
              percent: 25
```

Policy types available: `always_sample`, `latency`, `numeric_attribute`, `probabilistic`, `status_code`, `string_attribute`, `rate_limiting`, `span_count`, `trace_state`, `boolean_attribute`, `ottl_condition`, `composite`.


## Collector Builder (ocb)

`ocb` (OpenTelemetry Collector Builder) compiles a custom collector binary containing only the components you need. This reduces binary size, attack surface, and startup time compared to `otelcol-contrib`.

### Install

```bash
curl --proto '=https' --tlsv1.2 -fL -o ocb \
  https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/cmd%2Fbuilder%2Fv0.147.0/ocb_0.147.0_linux_amd64
chmod +x ocb
```

### Builder manifest (builder-config.yaml)

```yaml
dist:
  name: otelcol-custom
  description: "Custom OTel Collector for production"
  output_path: ./otelcol-custom
  version: 0.147.0

receivers:
  - gomod: go.opentelemetry.io/collector/receiver/otlpreceiver v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/receiver/hostmetricsreceiver v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/receiver/filelogreceiver v0.147.0

processors:
  - gomod: go.opentelemetry.io/collector/processor/batchprocessor v0.147.0
  - gomod: go.opentelemetry.io/collector/processor/memorylimiterprocessor v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/processor/k8sattributesprocessor v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/processor/resourcedetectionprocessor v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/processor/filterprocessor v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/processor/tailsamplingprocessor v0.147.0

exporters:
  - gomod: go.opentelemetry.io/collector/exporter/otlpexporter v0.147.0
  - gomod: go.opentelemetry.io/collector/exporter/otlphttpexporter v0.147.0
  - gomod: go.opentelemetry.io/collector/exporter/debugexporter v0.147.0

extensions:
  - gomod: go.opentelemetry.io/collector/extension/zpagesextension v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/extension/healthcheckextension v0.147.0
  - gomod: github.com/open-telemetry/opentelemetry-collector-contrib/extension/storage/filestorage v0.147.0

providers:
  - gomod: go.opentelemetry.io/collector/confmap/provider/envprovider v1.53.0
  - gomod: go.opentelemetry.io/collector/confmap/provider/fileprovider v1.53.0
```

### Build

```bash
./ocb --config builder-config.yaml
# produces ./otelcol-custom binary
./otelcol-custom --config config.yaml
```


## References

- Collector configuration reference: <https://opentelemetry.io/docs/collector/configuration/>
- Deployment patterns: <https://opentelemetry.io/docs/collector/deployment/>
- Scaling: <https://opentelemetry.io/docs/collector/scaling/>
- OCB: <https://opentelemetry.io/docs/collector/extend/ocb/>
- Transforming telemetry (OTTL): <https://opentelemetry.io/docs/collector/transforming-telemetry/>
- Internal telemetry: <https://opentelemetry.io/docs/collector/internal-telemetry/>
- k8sattributes processor: <https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/processor/k8sattributesprocessor>
- Tail sampling processor: <https://github.com/open-telemetry/opentelemetry-collector-contrib/tree/main/processor/tailsamplingprocessor>
- Collector contrib registry: <https://github.com/open-telemetry/opentelemetry-collector-contrib>
- OpenTelemetry Operator (Kubernetes): <https://opentelemetry.io/docs/kubernetes/operator/>
