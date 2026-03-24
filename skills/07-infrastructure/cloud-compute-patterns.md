---
name: cloud-compute-patterns
description: Cloud compute patterns covering AWS Lambda (event sources, cold starts, limits), GCP Cloud Run (container serverless, concurrency, scaling), Azure Functions (triggers, bindings, Durable Functions), Dockerfile multi-stage builds, service mesh (Istio), Kubernetes pod deployment, and ArgoCD GitOps. Use when designing serverless, container, or deployment workflows.
domain: infrastructure
tags: [lambda, cloud-run, azure-functions, serverless, dockerfile, multi-stage, istio, service-mesh, kubernetes, argo-cd, gitops]
triggers: lambda, serverless, cloud run, azure functions, dockerfile, multi-stage build, istio, service mesh, kubernetes pod, argo cd, gitops
---

# Cloud Compute Patterns

## 1. AWS Lambda

### Function Design

- Keep handler thin. Initialize SDK clients and heavy dependencies outside handler for reuse across invocations.
- Use environment variables for configuration. Set memory and timeout appropriately (more memory = faster CPU = shorter cold start).
- Package size affects cold start: minimize dependencies. Use layers for shared dependencies.

### Event Sources

| Source | Invocation | Key Config |
|--------|-----------|------------|
| API Gateway | Sync | Timeout 29s max |
| SQS | Async | Batch size, partial failure |
| EventBridge | Async | Rule patterns |
| S3 | Async | Event type filter |
| DynamoDB Streams | Async | Batch size, starting position |

### Cold Start Mitigation

- Minimize package size (remove dev dependencies, use tree-shaking).
- Provisioned concurrency for critical paths. SnapStart for Java.
- More memory often reduces cold start duration.

### Limits

- Timeout: 15 minutes max. Memory: 128MB-10GB. Package: 50MB zipped, 250MB unzipped.
- Concurrent executions: 1000 default (request increase). Use reserved concurrency to protect downstream.

## 2. GCP Cloud Run

### When to Use

- Container-based serverless. Bring any Docker image. Scale to zero.
- Good for: APIs, web apps, background processing, scheduled jobs.

### Key Configuration

- `--concurrency`: Requests per container instance (default 80). Lower for CPU-intensive work.
- `--min-instances`: Prevent cold starts. Set 1+ for latency-sensitive services.
- `--max-instances`: Cost control. Prevents runaway scaling.
- `--cpu`: 1-8 vCPUs. More CPU for compute-intensive workloads.
- `--memory`: 128Mi-32Gi. Match to application requirements.

### Startup

- Container must listen on `$PORT` (default 8080). Startup probe: Cloud Run waits for successful health check before routing traffic.
- Use startup CPU boost for faster cold starts.

## 3. Azure Functions

### Triggers and Bindings

- **Triggers**: HTTP, Timer, Queue, Blob, Event Hub, Cosmos DB, Service Bus.
- **Input bindings**: Read data without boilerplate (Cosmos DB document, Blob content).
- **Output bindings**: Write data without SDK calls (Queue message, Table storage row).

### Durable Functions

- Orchestrator functions for stateful workflows. Fan-out/fan-in, human interaction, chaining.
- Entity functions for stateful actors.
- Timer triggers for scheduled work (cron expressions).

### Hosting Plans

| Plan | Scale | Cold Start | Cost |
|------|-------|------------|------|
| Consumption | Auto (0-200) | Yes | Pay per execution |
| Premium | Pre-warmed | Minimal | Per-second billing |
| Dedicated | Manual | None | App Service pricing |

## 4. Dockerfile Multi-Stage Builds

```dockerfile
# Build stage
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --production=false
COPY . .
RUN npm run build

# Production stage
FROM node:20-alpine
RUN addgroup -S app && adduser -S app -G app
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
USER app
EXPOSE 3000
CMD ["node", "dist/index.js"]
```

### Best Practices

- Use specific base image tags (not `latest`). Alpine for smaller images.
- Run as non-root user. Copy only needed files (use .dockerignore).
- Order layers by change frequency (dependencies before source code).
- Use `--no-cache` in CI. Scan images for vulnerabilities (Trivy, Snyk).

## 5. Service Mesh (Istio)

### When to Use

- Need mutual TLS between services without application changes.
- Require traffic management (canary, blue-green, fault injection).
- Need observability (distributed tracing, metrics) without SDK integration.
- Managing service-to-service authorization policies.

### Key Features

- **Traffic management**: VirtualService (routing rules), DestinationRule (load balancing, circuit breaking).
- **Security**: PeerAuthentication (mTLS mode), AuthorizationPolicy (RBAC for services).
- **Observability**: Automatic metrics, traces, and access logs via Envoy sidecar.

## 6. Kubernetes Pod Deployment

### Deployment Spec Essentials

- Set resource requests AND limits for CPU and memory.
- Configure liveness and readiness probes. Readiness gates traffic; liveness restarts pods.
- Use `PodDisruptionBudget` for availability during rollouts/maintenance.
- Set `terminationGracePeriodSeconds` to allow graceful shutdown.

### Rolling Update Strategy

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 25%
    maxUnavailable: 0
```

## 7. ArgoCD GitOps

### Core Pattern

- Git repository is the source of truth for desired state.
- ArgoCD continuously reconciles cluster state with git.
- Application CRD defines: source (git repo + path), destination (cluster + namespace), sync policy.

### Sync Policies

- **Automated**: Auto-sync on git push. Use with `selfHeal: true` to revert manual changes.
- **Manual**: Require explicit sync (for production, review before deploy).
- **Prune**: Delete resources removed from git. Enable carefully.

## Checklist

- [ ] Lambda: handler/init separated, package minimized, timeout/memory set
- [ ] Cloud Run: concurrency, min/max instances configured
- [ ] Azure Functions: trigger and binding types documented
- [ ] Dockerfile: multi-stage, non-root user, .dockerignore configured
- [ ] Istio: mTLS mode set, authorization policies defined
- [ ] K8s: resource requests/limits, probes, PDB configured
- [ ] ArgoCD: sync policy matches environment risk level
