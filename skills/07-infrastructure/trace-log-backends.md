---
name: trace-log-backends
description: Distributed tracing and log aggregation backends covering Jaeger/Tempo (trace storage, querying, sampling, trace-to-logs) and Loki (log aggregation, LogQL, label strategies, alerting, retention). Use when deploying or configuring trace and log storage infrastructure.
domain: infrastructure
tags: [jaeger, tempo, loki, tracing, logs, logql, trace-storage, log-aggregation, grafana]
triggers: jaeger, tempo, loki, logql, trace storage, log aggregation, distributed tracing backend, log backend
---


# Distributed Tracing: Jaeger and Grafana Tempo

## Core Concepts

A **trace** represents the full journey of a request through a distributed system. It is composed of **spans** — individual units of work with a start time, duration, parent-span reference, and attached metadata. Every span carries a `trace_id` (shared across the request) and a `span_id` (unique to that operation).

The three signals of observability map to different tools:

- Metrics → Prometheus / Mimir
- Logs → Loki
- Traces → Jaeger or Tempo (both consume OTLP from OpenTelemetry)


## Jaeger Architecture

### All-in-One (development only)

Single binary containing collector, query service, and in-memory storage. Accessible at `http://localhost:16686`. Data is lost on restart — not for production.

```bash
docker run --rm -p 16686:16686 -p 4317:4317 -p 4318:4318 \
  jaegertracing/all-in-one:latest
```

### Distributed (production)

Each component runs independently and scales separately:

- **Collector** — receives spans over OTLP gRPC (4317), OTLP HTTP (4318), Jaeger Thrift HTTP (14268), Jaeger gRPC (14250), Zipkin (9411). Validates, applies sampling policies, writes to storage.
- **Ingester** (optional) — buffers spans through Kafka before writing to storage. Useful for smoothing ingestion spikes.
- **Query service** — serves the API and UI from storage.
- **Storage** — Elasticsearch (good for search) or Cassandra (good for scale and high availability). Badger (embedded) is available for single-node setups.

The Agent (UDP-based sidecar) is deprecated. All new deployments should point the OTel SDK exporter directly at the Collector or route through an OpenTelemetry Collector.


## OpenTelemetry SDK Instrumentation

The OTel SDK is the standard instrumentation layer for both Jaeger and Tempo. Both backends accept OTLP — instrument once, route anywhere.

### Go — tracer setup

```go
import (
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
    "go.opentelemetry.io/otel/sdk/trace"
    "go.opentelemetry.io/otel/sdk/resource"
    semconv "go.opentelemetry.io/otel/semconv/v1.24.0"
)

func initTracer(ctx context.Context) (*trace.TracerProvider, error) {
    exporter, err := otlptracegrpc.New(ctx,
        otlptracegrpc.WithEndpoint("otel-collector:4317"),
        otlptracegrpc.WithInsecure(), // use WithTLSClientConfig in prod
    )
    if err != nil {
        return nil, err
    }

    res := resource.NewWithAttributes(
        semconv.SchemaURL,
        semconv.ServiceName("order-service"),
        semconv.ServiceVersion("1.4.2"),
        semconv.DeploymentEnvironment("production"),
    )

    tp := trace.NewTracerProvider(
        trace.WithBatcher(exporter),           // always BatchSpanProcessor, not SimpleSpanProcessor
        trace.WithResource(res),
        trace.WithSampler(trace.ParentBased(  // respect upstream sampling decision
            trace.TraceIDRatioBased(0.1),     // sample 10% of new root spans
        )),
    )
    otel.SetTracerProvider(tp)
    otel.SetTextMapPropagator(propagation.NewCompositeTextMapPropagator(
        propagation.TraceContext{}, // W3C traceparent header — prefer this
        propagation.Baggage{},
    ))
    return tp, nil
}
```

### Python — tracer setup

```python
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource, SERVICE_NAME
from opentelemetry.propagate import set_global_textmap
from opentelemetry.propagators.composite import CompositePropagator
from opentelemetry.trace.propagation.tracecontext import TraceContextTextMapPropagator
from opentelemetry.baggage.propagation import W3CBaggagePropagator

resource = Resource(attributes={SERVICE_NAME: "payment-service"})
exporter = OTLPSpanExporter(endpoint="http://otel-collector:4317", insecure=True)

provider = TracerProvider(resource=resource)
provider.add_span_processor(BatchSpanProcessor(exporter))
trace.set_tracer_provider(provider)
set_global_textmap(CompositePropagator([
    TraceContextTextMapPropagator(),
    W3CBaggagePropagator(),
]))

tracer = trace.get_tracer(__name__)
```

### Creating spans with attributes and events

```go
tracer := otel.Tracer("checkout")

ctx, span := tracer.Start(ctx, "process-payment",
    oteltrace.WithSpanKind(oteltrace.SpanKindInternal),
    oteltrace.WithAttributes(
        attribute.String("payment.provider", "stripe"),
        attribute.String("payment.method", "card"),
        attribute.Float64("payment.amount_usd", 49.99),
        attribute.String("user.id", userID),
    ),
)
defer span.End()

// Record a point-in-time event (not a log line — a timestamped annotation on the span)
span.AddEvent("payment-authorized", oteltrace.WithAttributes(
    attribute.String("authorization.code", authCode),
))

// Mark a span as failed
if err != nil {
    span.RecordError(err)
    span.SetStatus(codes.Error, err.Error())
    return err
}
```


## Context Propagation

Context propagation carries the `trace_id` and `span_id` across process boundaries so spans can be stitched into a single trace.

### W3C TraceContext (recommended)

The `traceparent` HTTP header encodes four fields:

```text
traceparent: 00-{trace_id_hex_32}-{parent_span_id_hex_16}-{flags}
```

Example: `traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`

The trailing `01` means the sampling flag is set (trace is being sampled). A value of `00` means the parent did not sample. With `ParentBased` sampler, the child service respects this flag — it will not sample a trace that was not sampled at the root.

The `tracestate` header carries vendor-specific key-value pairs without affecting the core propagation.

### B3 (legacy)

Used by Zipkin and older Jaeger deployments. Single-header form: `b3: {trace_id}-{span_id}-{sampling_flag}`. Multi-header form uses `X-B3-TraceId`, `X-B3-SpanId`, `X-B3-Sampled`. Use W3C TraceContext for all new systems; add B3 only when interoperating with legacy services.

### Propagation across async boundaries

For message queues, inject the propagation context into message headers at the producer:

```go
// Producer
carrier := propagation.MapCarrier{}
otel.GetTextMapPropagator().Inject(ctx, carrier)
msg.Headers = mapToKafkaHeaders(carrier)

// Consumer
ctx = otel.GetTextMapPropagator().Extract(context.Background(), propagation.MapCarrier(msgHeaders))
ctx, span := tracer.Start(ctx, "process-order-event", oteltrace.WithSpanKind(oteltrace.SpanKindConsumer))
```


## OpenTelemetry Collector as Universal Router

The Collector decouples instrumentation from backend selection. Applications send OTLP once; the Collector fans out, filters, and transforms.

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318
  jaeger:
    protocols:
      grpc:
        endpoint: 0.0.0.0:14250
      thrift_http:
        endpoint: 0.0.0.0:14268

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024
  resource:
    attributes:
      - key: deployment.environment
        value: production
        action: upsert
  filter/drop-health:
    spans:
      exclude:
        match_type: strict
        attributes:
          - key: http.target
            value: "/health"

exporters:
  otlp/tempo:
    endpoint: tempo-distributor:4317
    tls:
      insecure: false
      cert_file: /certs/client.crt
      key_file: /certs/client.key
  otlp/jaeger:
    endpoint: jaeger-collector:4317
    tls:
      insecure: true

service:
  pipelines:
    traces:
      receivers: [otlp, jaeger]
      processors: [batch, resource, filter/drop-health]
      exporters: [otlp/tempo]
```


## Tempo Metrics Generator

The metrics-generator reads spans as they arrive and writes RED metrics to a Prometheus-compatible remote-write endpoint.

```yaml
metrics_generator:
  registry:
    external_labels:
      cluster: prod-us-east
  storage:
    path: /tmp/tempo/generator
    remote_write:
      - url: http://prometheus:9090/api/v1/write
  processors:
    - service-graphs
    - span-metrics
  processor:
    service_graphs:
      dimensions:
        - http.method
        - http.status_code
      max_items: 10000
    span_metrics:
      dimensions:
        - span.kind
        - http.route
        - db.system
      histogram_buckets: [0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048]
```

Generated metrics:

- `traces_spanmetrics_calls_total{service, span_name, span_kind, status_code}` — request rate
- `traces_spanmetrics_duration_seconds_bucket` — latency histogram
- `traces_service_graph_request_total{client, server}` — inter-service call rate
- `traces_service_graph_request_failed_total` — inter-service error rate

These metrics automatically include **exemplars** that link directly to the trace that contributed to that data point — enabling one-click navigation from a Prometheus alert or Grafana panel into a specific trace.


## Trace-to-Metrics Correlation

Prometheus exemplars carry a `trace_id` alongside a histogram sample. Grafana renders them as dots on a graph; clicking one opens the linked trace in Tempo.

```go
// Go — attach exemplar to a Prometheus histogram observation
histogram.With(prometheus.Labels{
    "method": r.Method,
    "status": strconv.Itoa(statusCode),
}).ObserveWithExemplar(duration.Seconds(), prometheus.Labels{
    "traceID": span.SpanContext().TraceID().String(),
})
```

Prometheus must be configured to accept exemplars:

```yaml
# prometheus.yml
storage:
  exemplars:
    max_exemplars: 100000
```

Grafana panel must use an exemplar-aware query and have the Tempo data source configured as the trace backend.


## Jaeger-to-Tempo Migration

When migrating an existing Jaeger deployment to Tempo:

1. Deploy Tempo alongside Jaeger (dual write via OTel Collector).
2. Add a second exporter in the Collector config pointing to Tempo while keeping the Jaeger exporter live.
3. Validate queries in Tempo using TraceQL against known trace IDs visible in Jaeger.
4. Switch Grafana dashboards to use the Tempo data source.
5. Remove the Jaeger exporter from the Collector and decommission the Jaeger backend once trace retention window has passed.

If using service mesh (Istio/Linkerd), reconfigure the mesh telemetry to route through an OTel sidecar (`sidecar.opentelemetry.io/inject=otel-sidecar`) rather than directly to the Jaeger backend.


---


# Loki Log Aggregation

## Label Design

Labels in Loki define log streams. Every unique combination of label names and values creates a separate stream, stored as its own set of compressed chunks. Getting label design wrong is the most common cause of Loki performance problems.

### Static Labels (always appropriate)

Static labels have a small, bounded set of values and reflect infrastructure topology. Use them freely:

- `cluster`, `region`, `env` (prod/staging/dev)
- `namespace`, `app`, `service`, `job`
- `host`, `node`, `pod` (when cardinality is bounded)
- `log_type` (access/app/audit)

### Cardinality Rules

- Keep each dynamic label to single digits or low tens of distinct values across your fleet.
- Target fewer than 100,000 active streams per tenant. Exceeding 1 million streams per day degrades index and chunk performance.
- A label that produces thousands of unique values (request IDs, trace IDs, order IDs, user IDs) must never be a label. Store it in the log line and filter with LogQL.
- Use `logcli --analyze-labels` or the Series API to audit running cardinality. A label like `requestId` appearing 24,000+ times is a signal to remove it.

### When to Add a Label

A new dynamic label is justified only when log volume from that stream is high enough to fill a full chunk (target: 1 MB compressed, ~5–10 MB uncompressed) before `max_chunk_age` expires. If the chunk flushes because of age rather than size, the label is generating idle, undersized streams — a net negative.

### Anti-patterns

```yaml
# BAD: unbounded cardinality
labels:
  trace_id: "abc123def456"   # millions of unique values
  user_id: "user-99182"       # unbounded

# GOOD: put high-cardinality values in the log line
# then query with: {app="api"} | json | trace_id="abc123def456"
labels:
  app: "api"
  env: "prod"
```


## Promtail Configuration

Promtail is the traditional log shipping agent. Grafana Alloy is the successor (see next section), but Promtail remains widely deployed.

### Basic Structure

```yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /var/log/positions.yaml   # tracks file read offsets across restarts

clients:
  - url: http://loki:3100/loki/api/v1/push
    tenant_id: my-team                 # omit for single-tenant

scrape_configs:
  - job_name: app-logs
    static_configs:
      - targets: [localhost]
        labels:
          app: my-app
          env: prod
          __path__: /var/log/myapp/*.log

    pipeline_stages:
      - json:
          expressions:
            level: level
            ts: timestamp
            msg: message
      - timestamp:
          source: ts
          format: RFC3339Nano
      - labels:
          level:
      - output:
          source: msg
```

### Pipeline Stages

Stages execute sequentially. Each stage reads from and writes to a shared extracted label map.

| Stage | Purpose |
| --- | --- |
| `docker` / `cri` | Parse container log wrappers |
| `json` | Extract fields from JSON log lines |
| `logfmt` | Extract key=value pairs |
| `regex` | Extract with named capture groups |
| `timestamp` | Override the log timestamp from an extracted field |
| `labels` | Promote extracted fields to stream labels |
| `structured_metadata` | Attach high-cardinality data without stream label impact |
| `output` | Replace log line content |
| `match` | Conditional stage execution based on label selectors |
| `metrics` | Emit Prometheus metrics from log patterns |
| `replace` | Regex substitution on the log line |
| `pack` | Serialize labels into JSON for downstream unpack |
| `template` | Manipulate extracted values with Go templates |

### Structured Metadata (avoiding cardinality)

Structured metadata attaches searchable data to log entries without creating new streams. Use it for high-cardinality identifiers:

```yaml
pipeline_stages:
  - json:
      expressions:
        trace_id: traceId
        span_id: spanId
  - structured_metadata:
      trace_id:
      span_id:
  # trace_id is NOT promoted to labels — no cardinality increase
  # but it IS queryable: {app="api"} | trace_id="abc123"
```

### Relabeling

Relabeling transforms labels before scraping, using Prometheus-compatible syntax:

```yaml
relabel_configs:
  - source_labels: [__meta_kubernetes_pod_name]
    target_label: pod
  - source_labels: [__meta_kubernetes_namespace]
    target_label: namespace
  - source_labels: [__meta_kubernetes_pod_label_app]
    target_label: app
  # drop pods without an app label
  - source_labels: [app]
    regex: ".+"
    action: keep
  # hash-based sharding across Promtail instances
  - source_labels: [__address__]
    modulus: 4
    target_label: __tmp_hash
    action: hashmod
  - source_labels: [__tmp_hash]
    regex: "0"
    action: keep
```

Labels prefixed with `__` are stripped post-relabeling. Use `__tmp_` for intermediate values.

### Kubernetes Autodiscovery

```yaml
scrape_configs:
  - job_name: kubernetes-pods
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_loki_io_scrape]
        regex: "true"
        action: keep
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod
      - source_labels: [__meta_kubernetes_pod_container_name]
        target_label: container
      - source_labels: [__meta_kubernetes_pod_label_app]
        target_label: app
      - replacement: /var/log/pods/*$1/*.log
        separator: /
        source_labels: [__meta_kubernetes_pod_uid, __meta_kubernetes_pod_container_name]
        target_label: __path__
```


## Storage and Retention

### Storage Backends

**TSDB + Object Storage** is the recommended setup for Loki 2.8+:

```yaml
schema_config:
  configs:
    - from: 2024-01-01
      store: tsdb
      object_store: s3
      schema: v13
      index:
        prefix: loki_index_
        period: 24h

storage_config:
  tsdb_shipper:
    active_index_directory: /data/tsdb-index
    cache_location: /data/tsdb-cache
  aws:
    s3: s3://us-east-1/my-loki-bucket
    s3forcepathstyle: false
```

Index period must be `24h`. TSDB stores index files directly in object storage; BoltDB-shipper is the predecessor (Loki 2.0–2.7).

### Chunk Tuning

```yaml
ingester:
  chunk_target_size: 1572864    # 1.5 MB — target compressed chunk size
  chunk_idle_period: 30m        # flush idle streams after this
  max_chunk_age: 2h             # force flush after this regardless of size
  chunk_encoding: snappy        # snappy (fast) or gzip (smaller)
```

Larger chunks improve compression and query performance but increase memory use in the ingester. The goal is chunks that fill by size before the idle/age timer fires.

### Retention Configuration

Retention is managed by the compactor. It must run as a singleton:

```yaml
compactor:
  working_directory: /data/compactor
  compaction_interval: 10m
  retention_enabled: true
  retention_delete_delay: 2h          # grace period before chunk deletion
  retention_delete_worker_count: 150  # parallelism for deletion

limits_config:
  retention_period: 744h              # global default: 30 days
```

#### Per-tenant retention

```yaml
# In per-tenant override configuration
overrides:
  team-a:
    retention_period: 2160h           # 90 days
    retention_stream:
      - selector: '{namespace="prod"}'
        priority: 2
        period: 2160h
      - selector: '{level="debug"}'
        priority: 1
        period: 168h                  # debug logs: 7 days
```

Priority hierarchy (highest wins): per-tenant stream selector > global stream selector > per-tenant period > global period. Default is 744h (30 days). Minimum retention is 24h. Changes do not apply retroactively to existing chunks.


## Log-Based Alerting

### Ruler Configuration

The ruler evaluates LogQL rules on a schedule. Run as a singleton or configure sharding for HA:

```yaml
ruler:
  storage:
    type: local
    local:
      directory: /etc/loki/rules
  rule_path: /tmp/loki-rules
  alertmanager_url: http://alertmanager:9093
  ring:
    kvstore:
      store: memberlist
  enable_api: true
  enable_sharding: true
```

### Alert Rule Syntax

Rules are Prometheus-compatible YAML, using LogQL expressions instead of PromQL:

```yaml
groups:
  - name: application-alerts
    interval: 1m
    rules:

      # Alert on sustained error rate
      - alert: HighErrorRate
        expr: |
          sum by (service) (
            rate({namespace="prod"} |= "error" [5m])
          )
          /
          sum by (service) (
            rate({namespace="prod"}[5m])
          )
          > 0.05
        for: 10m
        labels:
          severity: critical
          team: platform
        annotations:
          summary: "Error rate above 5% for {{ $labels.service }}"
          runbook: "https://wiki.example.com/runbook/high-error-rate"

      # Alert when a stream goes silent (no logs for 5 min)
      - alert: ServiceNotLogging
        expr: absent_over_time({app="payment-service", env="prod"}[5m])
        for: 0m
        labels:
          severity: warning

      # Alert on slow P99 latency from JSON logs
      - alert: SlowP99Latency
        expr: |
          quantile_over_time(0.99,
            {app="api"} | json | unwrap duration_ms [5m]
          ) by (endpoint)
          > 2000
        for: 5m
        labels:
          severity: warning
```

### Recording Rules

Precompute expensive queries and write them to a metrics backend:

```yaml
groups:
  - name: nginx-metrics
    interval: 1m
    rules:
      - record: nginx:http_requests:rate1m
        expr: sum by (status) (rate({job="nginx"} | pattern `<_> "<_>" <status> <_>` [1m]))
        labels:
          cluster: "us-east-1"
```

Results are written via remote-write to Prometheus, Mimir, or Thanos. This enables dashboards to use pre-aggregated metrics rather than scanning raw logs on every render.

### Rule Management with lokitool

```bash
# Validate rules before deployment
lokitool rules lint rules/

# Diff current rules against what is deployed
lokitool rules diff --address http://loki:3100 rules/

# Sync rules to Loki
lokitool rules sync --address http://loki:3100 rules/
```


## Operational Checklist

- Run `logcli --analyze-labels` monthly to audit stream cardinality.
- Never use timestamps, request IDs, user IDs, or trace IDs as stream labels.
- Set `chunk_target_size` to 1–1.5 MB and verify chunks fill by size not by age using Loki's `/metrics` (look for `loki_ingester_chunks_flushed_total` by reason).
- Use structured metadata for trace IDs and request IDs so they are queryable without cardinality impact.
- Configure `absent_over_time` alerts for every critical service to detect silent failures.
- Use recording rules to pre-aggregate high-traffic metric queries that drive dashboards.
- Set per-tenant stream retention for debug logs (7 days) vs. audit logs (1 year) to control storage costs.
- Validate alert rules with `lokitool rules lint` in CI before deploying to production.
