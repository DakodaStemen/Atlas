---
name: container-local-development
description: Container local development patterns covering Docker Compose (multi-service orchestration, networking, volumes, profiles, health checks) and Skaffold/Tilt (Kubernetes local dev, hot reload, port forwarding, build optimization, dev workflow). Use when setting up local development environments with containers.
domain: infrastructure
tags: [docker-compose, skaffold, tilt, local-dev, containers, hot-reload, kubernetes-dev]
triggers: docker compose, skaffold, tilt, local development, container dev, hot reload, docker local
---


# Docker Compose Patterns

## File Structure

Use `compose.yaml` (the canonical modern name) or `docker-compose.yml`. The top-level keys are `services`, `networks`, `volumes`, `configs`, and `secrets`. Drop the `version` property — it is deprecated in the Compose Specification (v1.27+) and does nothing.

```yaml
# compose.yaml — base file, committed to version control
services:
  web:
    build: .
    ports:
      - "127.0.0.1:8000:8000"
    environment:
      - FLASK_ENV=${FLASK_ENV:-production}
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
        restart: true
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8000/up || exit 1"]
      interval: 60s
      timeout: 3s
      start_period: 5s
      retries: 3

  db:
    image: postgres:16
    environment:
      POSTGRES_USER: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
      interval: 10s
      timeout: 10s
      retries: 5
      start_period: 30s

volumes:
  postgres_data:
```


## Health Checks and depends_on Conditions

Compose supports three `depends_on` conditions:

| Condition | Meaning |
| --- | --- |
| `service_started` | Dependency container is running (default; does not guarantee readiness) |
| `service_healthy` | Dependency healthcheck is passing |
| `service_completed_successfully` | Dependency ran to completion (for init/migration jobs) |

Define health checks in `compose.yaml`, not the Dockerfile — this keeps the image portable to Kubernetes and other runtimes.

### PostgreSQL readiness pattern

```yaml
db:
  image: postgres:16
  healthcheck:
    test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
    interval: 10s
    timeout: 10s
    retries: 5
    start_period: 30s   # grace period before health checks begin

web:
  depends_on:
    db:
      condition: service_healthy
      restart: true     # if db restarts, web restarts too
```

#### Redis readiness

```yaml
redis:
  image: redis:7-alpine
  healthcheck:
    test: ["CMD", "redis-cli", "ping"]
    interval: 10s
    timeout: 5s
    retries: 5
```

#### One-shot migration job (runs before web, exits 0 when done)

```yaml
migrate:
  build: .
  command: ["python", "manage.py", "migrate"]
  depends_on:
    db:
      condition: service_healthy

web:
  depends_on:
    migrate:
      condition: service_completed_successfully
    db:
      condition: service_healthy
```

Note: healthchecks use double-dollar (`$$VAR`) to escape shell expansion inside the YAML — single `$VAR` would be interpolated by Compose before reaching the container shell.


## Named Volumes vs. Bind Mounts

**Named volumes** are managed by Docker. Docker owns the data on disk. Use for persistent data that the container owns (databases, uploaded files, compiled assets):

```yaml
volumes:
  postgres_data:       # Docker manages location on host
  redis_data:

services:
  db:
    volumes:
      - postgres_data:/var/lib/postgresql/data
```

**Bind mounts** point to a specific path on the host. Use for source code in development so edits are immediately reflected in the container:

```yaml
services:
  web:
    volumes:
      - .:/app                    # source code (dev only)
      - /app/node_modules         # anonymous volume shadows node_modules so host copy doesn't overwrite container's installed packages
```

### Rules of thumb

- Databases → named volume (never bind-mount DB data directories in dev, it causes permission nightmares)
- Source code in dev → bind mount to host directory
- Production → no bind mounts for code; code lives in the image
- Dependency directories (`node_modules`, Python venv) → anonymous volume or named volume to prevent host from overwriting them


## Profiles for Optional Services

Profiles let you define services that are off by default and only spin up on demand. This keeps `docker compose up` fast for the happy path.

```yaml
services:
  web:
    build: .
    # no profile — always starts

  db:
    image: postgres:16
    # no profile — always starts

  redis:
    image: redis:7-alpine
    # no profile — always starts

  mailhog:
    image: mailhog/mailhog
    profiles: [dev-tools]     # only starts with --profile dev-tools
    ports:
      - "127.0.0.1:8025:8025"

  pgadmin:
    image: dpage/pgadmin4
    profiles: [dev-tools]
    ports:
      - "127.0.0.1:5050:80"

  load-test:
    image: grafana/k6
    profiles: [testing]
    command: run /scripts/load.js
    volumes:
      - ./k6:/scripts
```

Start with optional tools:

```bash
docker compose --profile dev-tools up
docker compose --profile dev-tools --profile testing up
```


## YAML Anchors for DRY Configuration

When multiple services share configuration (same image, env vars, resource limits), use YAML anchors instead of repeating blocks:

```yaml
x-worker-defaults: &worker-defaults
  build: .
  restart: unless-stopped
  env_file: .env
  depends_on:
    db:
      condition: service_healthy
    redis:
      condition: service_healthy

services:
  worker-default:
    <<: *worker-defaults
    command: ["celery", "worker", "-Q", "default"]

  worker-high:
    <<: *worker-defaults
    command: ["celery", "worker", "-Q", "high-priority", "-c", "4"]

  beat:
    <<: *worker-defaults
    command: ["celery", "beat"]
```


## Common Patterns Quick Reference

### Recreate only one service after a code change

```bash
docker compose up --no-deps --build -d web
```

#### Validate compose file syntax

```bash
docker compose config
```

#### Preview startup order without starting

```bash
docker compose up --dry-run
```

#### Tail logs for specific services

```bash
docker compose logs -f web api
```

#### Run a one-off command in a service container

```bash
docker compose run --rm web python manage.py createsuperuser
```

#### Force-remove containers, networks, and volumes on teardown

```bash
docker compose down -v --remove-orphans
```


## Key Principles Summary

- Base `compose.yaml` is environment-agnostic and committed. Overlays (`override.yml`, `compose.production.yaml`) carry the differences.
- Never bind-mount source code into production. Code belongs in the image.
- Always pair `depends_on: condition: service_healthy` with an actual `healthcheck` block on the dependency, otherwise Compose has no signal to wait on.
- Bind ports to `127.0.0.1` in production; let a reverse proxy handle public traffic.
- Named volumes for database data; bind mounts for source code in dev only.
- Profiles keep `docker compose up` fast — optional services (admin UIs, mail catchers, load testers) belong behind a profile.
- Define health checks in Compose, not the Dockerfile, for portability.

---


# Skaffold and Tilt: Local Kubernetes Development

## Decision Matrix

| Criterion | Skaffold | Tilt |
| --- | --- | --- |
| Config language | YAML | Starlark (Python subset) |
| Live code sync (no rebuild) | Yes (file sync, limited) | Yes (live_update, first-class) |
| Web UI | No | Yes (localhost:10350) |
| Build systems | Docker, Jib, Bazel, Kaniko, Buildpacks, ko | Docker, custom scripts |
| Deploy methods | kubectl, Helm, Kustomize, Kpt | kubectl, Helm (ext), Docker Compose |
| Kustomize native support | Yes | Via kustomize() helper |
| GCP / Cloud Build integration | Yes | No |
| Resource dependency ordering | No | Yes (resource_deps) |
| Multi-service orchestration | Limited | Strong |
| CI mode | `skaffold run` / `skaffold build` | `tilt ci` |
| Resource consumption | Low | Higher with many services |
| Learning curve | Moderate (YAML-familiar) | Steeper (Starlark) |
| Programmatic config logic | No | Yes (full Python-like logic) |

### Choose Skaffold when

- The team is comfortable with YAML and wants a zero-magic pipeline
- You need Kustomize overlays, Bazel, Jib, or Kaniko build support
- GCP / Cloud Build is in the stack
- The project is a single service or a small number of independent services
- A predictable build → push → deploy loop matters more than sub-second updates

#### Choose Tilt when

- You have multiple interdependent microservices that need startup ordering
- Sub-second live updates (no rebuild) are critical to developer velocity
- You want a visual dashboard for logs, health, and service status
- Configuration logic is complex enough to benefit from a real programming language
- Teams mix DevOps and developers who want a GUI


## Tilt

### Tiltfile Basics

A `Tiltfile` lives in the repo root. It is Starlark — a deterministic, hermetic subset of Python. No classes, no imports from outside the Tilt stdlib, but conditionals, loops, functions, and dict/list operations all work.

```python
# Minimal single-service Tiltfile
docker_build('my-image', '.')
k8s_yaml('k8s/deployment.yaml')
k8s_resource('my-app', port_forwards='8080:8080')
```

### docker_build

```python
docker_build(
    ref='my-app',           # image name referenced in your k8s manifests
    context='.',            # build context directory
    dockerfile='Dockerfile.dev',
    build_args={'ENV': 'development'},
    # live_update bypasses a full image rebuild for incremental changes
    live_update=[
        sync('./src', '/app/src'),
        sync('./package.json', '/app/package.json'),
        run('npm install', trigger=['./package.json']),
        # restart_container() triggers a process restart inside the running pod
        # omit this if your app already watches for changes (nodemon, air)
    ],
    ignore=['./node_modules', './.git'],
)
```

### live_update

`live_update` is Tilt's core differentiator. Instead of rebuilding an image, Tilt copies changed files into the running container and optionally runs a command. This takes milliseconds.

The three primitives:

```python
live_update=[
    # sync(local_path, container_path) — copy files into the container
    sync('./app', '/app/app'),

    # run(cmd, trigger=[]) — run a shell command inside the container
    # trigger limits when it fires (only when those files change)
    run('pip install -r requirements.txt', trigger=['./requirements.txt']),

    # restart_container() — send SIGTERM then restart the entrypoint process
    # use when the app doesn't auto-reload on file changes
    restart_container(),
]
```

Language-specific patterns:

```python
# Node.js with nodemon (no restart_container needed — nodemon handles it)
docker_build('node-app', '.', live_update=[
    sync('./src', '/app/src'),
    sync('./package.json', '/app/package.json'),
    run('npm install', trigger=['./package.json']),
])

# Go with Air hot-reload tool
docker_build('go-api', '.', live_update=[
    sync('./cmd', '/app/cmd'),
    sync('./internal', '/app/internal'),
    sync('./go.mod', '/app/go.mod'),
    sync('./go.sum', '/app/go.sum'),
    run('go mod download', trigger=['./go.mod', './go.sum']),
])

# Python/Flask
docker_build('flask-app', '.', live_update=[
    sync('./app', '/app/app'),
    sync('./requirements.txt', '/app/requirements.txt'),
    run('pip install -r requirements.txt', trigger=['./requirements.txt']),
])
```

### k8s_yaml and k8s_resource

```python
# Load manifests — string, list, blob, or function output
k8s_yaml('k8s/deployment.yaml')
k8s_yaml(['k8s/deployment.yaml', 'k8s/service.yaml'])
k8s_yaml(kustomize('./k8s/overlays/local'))   # run kustomize build
k8s_yaml(helm('./charts/my-app',              # render a local Helm chart
              name='my-app',
              values=['charts/my-app/values.dev.yaml']))

# k8s_resource configures how Tilt handles a named k8s workload
k8s_resource(
    workload='my-app',
    port_forwards='8080:8080',      # local:container
    labels=['backend'],
    resource_deps=['postgres'],     # wait for postgres to be ready first
)
```

### Resource Dependencies

`resource_deps` controls startup ordering. Tilt will wait for the depended-on resource to reach a healthy state before starting the dependent.

```python
# Multi-service with ordered startup
docker_build('frontend', './frontend', live_update=[sync('./frontend/src', '/app/src')])
docker_build('api-gateway', './api-gateway', live_update=[sync('./api-gateway/src', '/app/src')])

k8s_yaml(['k8s/postgres.yaml', 'k8s/api-gateway.yaml', 'k8s/frontend.yaml'])

k8s_resource('postgres',     labels=['database'])
k8s_resource('api-gateway',  port_forwards='8080:8080', resource_deps=['postgres'], labels=['backend'])
k8s_resource('frontend',     port_forwards='3000:3000', resource_deps=['api-gateway'], labels=['frontend'])
```

### Helm Integration (Extension)

```python
load('ext://helm_resource', 'helm_resource', 'helm_repo')

# Add a remote chart repo
helm_repo('bitnami', 'https://charts.bitnami.com/bitnami')

# Deploy a Helm chart as a Tilt resource (gets full Tilt lifecycle management)
helm_resource(
    'postgres',
    'bitnami/postgresql',
    flags=['--set=auth.postgresPassword=devpassword'],
    labels=['database'],
)

docker_build('my-app', '.')
k8s_yaml('k8s/deployment.yaml')
k8s_resource('my-app', resource_deps=['postgres'], port_forwards='8080:8080')
```

### Environment Profiles with config

```python
config.define_string('env', args=True, usage='Target environment: local, dev')
cfg = config.parse()
env = cfg.get('env', 'local')

if env == 'local':
    k8s_yaml(kustomize('./k8s/overlays/local'))
    allow_k8s_contexts('docker-desktop')
elif env == 'dev':
    k8s_yaml(kustomize('./k8s/overlays/dev'))
    allow_k8s_contexts('dev-cluster')

docker_build('my-app', '.')
k8s_resource('my-app', port_forwards='8080:8080')
```

Run with: `tilt up -- --env dev`

### local_resource (Run Local Commands as Tilt Resources)

```python
# Unit tests run locally, re-triggered when source changes
local_resource(
    'unit-tests',
    cmd='go test ./...',
    deps=['./cmd', './internal'],
    labels=['tests'],
    auto_init=False,     # don't run on startup — trigger manually
)

# Integration tests run against the live cluster
local_resource(
    'integration-tests',
    cmd='kubectl exec deploy/my-app -- npm run test:integration',
    resource_deps=['my-app'],
    labels=['tests'],
    auto_init=False,
)
```

### kind and minikube Integration

#### kind with local registry

```python
# Tiltfile — point docker_build at your kind registry
docker_build('localhost:5000/my-app:latest', '.')
k8s_yaml('k8s/deployment.yaml')
k8s_resource('my-app', port_forwards='8080:8080')
```

Set up the kind cluster and local registry (shell, done once):

```bash
# Create registry
docker run -d --restart=always -p 5000:5000 --name kind-registry registry:2

# Create kind cluster
kind create cluster --name dev

# Connect registry to kind network
docker network connect kind kind-registry

# Annotate nodes (required for kind to trust the registry)
for node in $(kind get nodes --name dev); do
  kubectl annotate node "${node}" "kind.x-k8s.io/registry=localhost:5000"
done
```

#### minikube

```python
# Use minikube's built-in registry addon
# Run: minikube addons enable registry
docker_build('localhost:5000/my-app', '.', live_update=[...])
k8s_yaml('k8s/deployment.yaml')
```

Or use the `allow_k8s_contexts` guard to prevent accidents:

```python
allow_k8s_contexts(['minikube', 'docker-desktop', 'kind-dev'])
```

### Secrets via Extension

```python
load('ext://secret', 'secret_create_generic')

secret_create_generic(
    name='app-secrets',
    namespace='default',
    from_env_file='.env.local',
)
```

### CI Mode (Tilt)

`tilt ci` runs the full environment, waits for all resources to go healthy, then exits 0 (or non-zero on failure). It never opens the web UI.

```bash
tilt ci                        # run all resources, exit when stable or fail
tilt ci --timeout=10m          # hard timeout
tilt ci --file=./Tiltfile.ci   # use a separate CI-specific Tiltfile
```

A CI-specific Tiltfile can skip live_update and local_resource entries that don't make sense in CI:

```python
# Tiltfile.ci
docker_build('my-app', '.')
k8s_yaml('k8s/')
k8s_resource('my-app', port_forwards='8080:8080')
# no live_update, no local_resource tests
```

### Key CLI Commands (Tilt)

```bash
tilt up                   # start Tilt, open web UI at localhost:10350
tilt up -d                # start in background (daemon mode)
tilt down                 # tear down all resources
tilt trigger my-app       # force rebuild/redeploy of a named resource
tilt ci                   # CI mode: build, deploy, wait for healthy, exit
tilt logs                 # stream logs from all resources
tilt get uiresource       # list all Tilt resources and their status
tilt alpha tiltfile-result # dump the resolved Tiltfile state as JSON
```


## Pitfalls and Notes

- **imagePullPolicy: Always** breaks local development with both tools when images are not pushed to a registry. Use `IfNotPresent` or `Never` in dev manifests.
- **Skaffold tagPolicy sha256** ensures pods always pull the latest locally loaded image. Without it, Kubernetes may reuse a cached layer with the same tag.
- **Tilt resource_deps** checks liveness/readiness probes, so pods need proper probes defined for dependency ordering to be reliable.
- **Tilt high memory usage**: running 10+ services with live_update enabled consumes significant RAM. Use `auto_init=False` for rarely-needed services and start them manually.
- **Skaffold file sync** requires the container to already have a file watcher running at startup. If the container has no watcher, synced files change on disk but the process doesn't restart — add `restart_container()` (Tilt) or configure an entrypoint watcher.
- **kind registry**: kind nodes do not use the host Docker daemon. Images must be loaded with `kind load docker-image` or via a local registry; `push: false` alone is not enough.
- **Skaffold `skaffold diagnose`** prints the fully resolved configuration after profile merging — the first tool to reach for when a profile is not behaving as expected.
- **Tilt `allow_k8s_contexts`** is a safety guard that prevents `tilt up` from running against a production cluster by accident. Always set it.
