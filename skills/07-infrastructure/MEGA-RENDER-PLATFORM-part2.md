---
name: "Render Blueprint Specification (Part 2)"
description: "# Render Blueprint Specification - Part 2"
---


## Native Language Runtimes

### Node.js (`runtime: node`)

**Supported Versions:** 14, 16, 18, 20, 21
**Default Version:** 20

#### Version Specification

Specify Node version in `package.json`:

```json
{
  "engines": {
    "node": "20.x"
  }
}
```

#### Package Managers

- **npm**: Default, uses `package-lock.json`
- **Yarn**: Auto-detected if `yarn.lock` exists
- **pnpm**: Auto-detected if `pnpm-lock.yaml` exists

#### Common Build Commands

```bash
npm ci                          # Recommended (faster, reproducible)
npm ci && npm run build         # Build step included
yarn install --frozen-lockfile  # Yarn equivalent
pnpm install --frozen-lockfile  # pnpm equivalent
```

#### Common Start Commands

```bash
npm start                       # Uses "start" script in package.json
node server.js                  # Direct file execution
node dist/main.js               # Built output
```

#### Popular Frameworks

- Express.js, Fastify, Koa (APIs)
- Next.js (full-stack React)
- Nest.js (enterprise TypeScript)
- Remix (full-stack React)
- Nuxt.js (full-stack Vue)

#### Example Configuration

```yaml
type: web
name: node-app
runtime: node
buildCommand: npm ci && npm run build
startCommand: npm start
```

---

### Python (`runtime: python`)

**Supported Versions:** 3.8, 3.9, 3.10, 3.11, 3.12
**Default Version:** 3.11

#### Version Specification (Python (`runtime: python`))

Option 1 - `runtime.txt`:

```text
python-3.11.5
```

Option 2 - `Pipfile`:

```toml
[requires]
python_version = "3.11"
```

#### Package Managers (Python (`runtime: python`))

- **pip**: Default, uses `requirements.txt`
- **Poetry**: Auto-detected if `pyproject.toml` exists
- **Pipenv**: Auto-detected if `Pipfile` exists

#### Common Build Commands (Python (`runtime: python`))

```bash
pip install -r requirements.txt
pip install -r requirements.txt && python manage.py collectstatic --no-input
poetry install --no-dev
pipenv install --deploy
```

#### Common Start Commands (Python (`runtime: python`))

```bash
gunicorn app:app                                    # Flask
gunicorn config.wsgi:application                    # Django
uvicorn main:app --host 0.0.0.0 --port $PORT       # FastAPI
celery -A tasks worker                              # Celery worker
```

#### Popular Frameworks (Python (`runtime: python`))

- Django (full-stack web framework)
- Flask (microframework)
- FastAPI (modern async API framework)
- Celery (task queue)

#### Example Configuration (Python (`runtime: python`))

```yaml
type: web
name: python-app
runtime: python
buildCommand: pip install -r requirements.txt
startCommand: gunicorn app:app --bind 0.0.0.0:$PORT
```

---

### Go (`runtime: go`)

**Supported Versions:** 1.20, 1.21, 1.22, 1.23
**Default Version:** Latest stable

#### Version Specification (Go (`runtime: go`))

Specify in `go.mod`:

```go
module myapp

go 1.22
```

**Build System:** Uses Go modules

#### Common Build Commands (Go (`runtime: go`))

```bash
go build -o bin/app .
go build -o bin/app cmd/server/main.go
go build -tags netgo -ldflags '-s -w' -o bin/app
```

#### Common Start Commands (Go (`runtime: go`))

```bash
./bin/app
./bin/server
```

#### Popular Frameworks (Go (`runtime: go`))

- net/http (standard library)
- Gin (fast web framework)
- Echo (high performance framework)
- Chi (lightweight router)
- Fiber (Express-inspired framework)
- Gorilla Mux (powerful router)

#### Example Configuration (Go (`runtime: go`))

```yaml
type: web
name: go-app
runtime: go
buildCommand: go build -o bin/app .
startCommand: ./bin/app
```

---

### Ruby (`runtime: ruby`)

**Supported Versions:** 3.0, 3.1, 3.2, 3.3
**Default Version:** 3.3

#### Version Specification (Ruby (`runtime: ruby`))

Option 1 - `.ruby-version`:

```text
3.3.0
```

Option 2 - `Gemfile`:

```ruby
ruby '3.3.0'
```

**Package Manager:** Bundler (uses `Gemfile` and `Gemfile.lock`)

#### Common Build Commands (Ruby (`runtime: ruby`))

```bash
bundle install --jobs=4 --retry=3
bundle install && bundle exec rails assets:precompile
```

#### Common Start Commands (Ruby (`runtime: ruby`))

```bash
bundle exec rails server -b 0.0.0.0 -p $PORT
bundle exec puma -C config/puma.rb
bundle exec rackup -o 0.0.0.0 -p $PORT
bundle exec sidekiq                                  # Worker
```

#### Popular Frameworks (Ruby (`runtime: ruby`))

- Ruby on Rails (full-stack framework)
- Sinatra (microframework)
- Sidekiq (background jobs)

#### Example Configuration (Ruby (`runtime: ruby`))

```yaml
type: web
name: rails-app
runtime: ruby
buildCommand: bundle install && bundle exec rails assets:precompile
startCommand: bundle exec puma -C config/puma.rb
```

---

### Rust (`runtime: rust`)

**Supported Versions:** Latest stable
**Default Version:** Latest stable

**Build System:** Cargo

#### Common Build Commands (Rust (`runtime: rust`))

```bash
cargo build --release
cargo build --release --locked
```

#### Common Start Commands (Rust (`runtime: rust`))

```bash
./target/release/myapp
```

#### Popular Frameworks (Rust (`runtime: rust`))

- Actix Web (powerful, performant)
- Rocket (web framework with focus on usability)
- Axum (modern, ergonomic framework)
- Warp (composable web framework)

#### Example Configuration (Rust (`runtime: rust`))

```yaml
type: web
name: rust-app
runtime: rust
buildCommand: cargo build --release
startCommand: ./target/release/myapp
```

---

### Elixir (`runtime: elixir`)

**Supported Versions:** Latest stable
**Default Version:** Latest stable

**Build System:** Mix

#### Common Build Commands (Elixir (`runtime: elixir`))

```bash
mix deps.get --only prod
mix deps.get && mix compile
mix do deps.get, compile, assets.deploy
```

#### Common Start Commands (Elixir (`runtime: elixir`))

```bash
mix phx.server
elixir --name myapp -S mix phx.server
```

#### Popular Frameworks (Elixir (`runtime: elixir`))

- Phoenix (full-stack web framework)
- Phoenix LiveView (real-time applications)

#### Example Configuration (Elixir (`runtime: elixir`))

```yaml
type: web
name: elixir-app
runtime: elixir
buildCommand: mix deps.get --only prod && mix compile
startCommand: mix phx.server
```

---

## Container Runtimes

### Docker (`runtime: docker`)

Build your application from a Dockerfile in your repository.

#### Additional Configuration

- `dockerfilePath`: Path to Dockerfile (default: `./Dockerfile`)
- `dockerContext`: Build context directory (default: `.`)

#### Example Configuration (Docker (`runtime: docker`))

```yaml
type: web
name: docker-app
runtime: docker
dockerfilePath: ./Dockerfile
dockerContext: .
```

#### Multi-stage Dockerfile Example

```dockerfile
# Build stage
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Production stage
FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY package*.json ./
RUN npm ci --only=production
EXPOSE 10000
CMD ["node", "dist/main.js"]
```

#### Best Practices

- Use multi-stage builds to reduce image size
- Copy `package.json` before source code (better caching)
- Use `.dockerignore` to exclude unnecessary files
- Expose port dynamically via `$PORT` environment variable
- Run as non-root user for security

---

### Pre-built Image (`runtime: image`)

Deploy pre-built Docker images from a container registry.

#### Additional Configuration (Pre-built Image (`runtime: image`))

- `image`: Full image URL with tag or digest
- `registryCredential`: Credentials for private registries

#### Example with Public Image

```yaml
type: web
name: prebuilt-app
runtime: image
image: ghcr.io/myorg/myapp:v1.2.3
```

#### Example with Private Registry

```yaml
type: web
name: private-app
runtime: image
image: myregistry.com/myapp:latest
registryCredential:
  username: my-username
  password:
    sync: false  # User provides in Dashboard
```

#### Use Cases

- Deploy images built in CI/CD pipeline
- Use images from container registries
- Deploy Docker Hub images
- Use private registry images

---

## Static Runtime (`runtime: static`)

Serve pre-built static files without a backend runtime. Files are served via CDN.

### Additional Configuration (Static Runtime (`runtime: static`))

- `staticPublishPath`: Directory containing built files (e.g., `./dist`, `./build`)

#### Common Build Commands by Framework

#### React (Create React App)

```bash
npm ci && npm run build
# Outputs to: ./build
```

#### Vue

```bash
npm ci && npm run build
# Outputs to: ./dist
```

#### Next.js (Static Export)

```bash
npm ci && npm run build && npm run export
# Outputs to: ./out
```

#### Gatsby

```bash
npm ci && npm run build
# Outputs to: ./public
```

#### Vite

```bash
npm ci && npm run build
# Outputs to: ./dist
```

#### Example Configuration (Additional Configuration)

```yaml
type: web
name: react-app
runtime: static
buildCommand: npm ci && npm run build
staticPublishPath: ./build
```

---

## Runtime Comparison

| Runtime | Build Speed | Cold Start | Best For |
| --------- | ------------- | ------------ | ---------- |
| Node.js | Fast | Fast | APIs, full-stack apps |
| Python | Medium | Medium | Data apps, APIs, web |
| Go | Fast | Very Fast | High performance APIs |
| Ruby | Slow | Medium | Rails apps, traditional web |
| Rust | Very Slow | Very Fast | Performance-critical services |
| Elixir | Medium | Fast | Real-time, concurrent apps |
| Docker | Varies | Medium | Any language, custom setup |
| Static | Very Fast | N/A | SPAs, documentation, marketing |

---

## Choosing the Right Runtime

### Choose Node.js when

- Building JavaScript-based applications
- Need rich npm ecosystem
- Want fast iteration and deployment
- Building full-stack applications (Next.js, Remix)

#### Choose Python when

- Building data-heavy applications
- Need machine learning libraries
- Django or Flask expertise
- Data processing pipelines

#### Choose Go when

- Need high performance and low resource usage
- Building microservices
- Want simple deployment (single binary)
- Handling high concurrency

#### Choose Ruby when

- Building traditional web applications
- Ruby on Rails expertise
- Rapid development priority

#### Choose Rust when

- Maximum performance required
- Systems programming
- Resource-constrained environments

#### Choose Docker when

- Need custom system dependencies
- Multi-language application
- Existing Dockerfile
- Need full control over environment

#### Choose Static when

- Building SPAs or static sites
- No backend processing needed
- Want CDN caching and fast delivery
- Documentation or marketing sites

## When to use

Use when the user asks about or needs: Render Runtime Options.
﻿---
name: Render Service Types
description: # Render Service Types
 
 Detailed explanation of each service type available on Render. Choose the right service type based on your application's needs.
---

# Render Service Types

Detailed explanation of each service type available on Render. Choose the right service type based on your application's needs.

## Web Services (`type: web`)

### Purpose

Web services are HTTP servers that handle incoming requests from the internet. They're publicly accessible via HTTPS URLs.

### Use Cases

- **REST APIs**: JSON APIs for mobile apps or frontend applications
- **GraphQL servers**: GraphQL endpoints for client queries
- **Web applications**: Server-rendered websites (Django, Rails, Express)
- **Full-stack frameworks**: Next.js, Nuxt.js, Remix, SvelteKit
- **WebSocket servers**: Real-time communication servers
- **SSR applications**: Server-side rendered React, Vue, or Angular apps

### Key Characteristics

- **Public URL**: Automatically assigned `https://[service-name].onrender.com`
- **Port binding required**: Must bind to `0.0.0.0:$PORT`
- **Health checks**: Render pings your service to verify it's running
- **HTTPS**: Automatic SSL/TLS certificates
- **Load balancing**: Traffic distributed across multiple instances
- **Custom domains**: Support for your own domain names

### Required Configuration

```yaml
type: web
name: my-api
runtime: node
buildCommand: npm ci
startCommand: npm start
```

### Best Practices

1. **Bind to environment PORT**:

```javascript
const PORT = process.env.PORT || 3000;
app.listen(PORT, '0.0.0.0');
```

1. **Add health check endpoint**:

```javascript
app.get('/health', (req, res) => {
  res.status(200).json({ status: 'ok' });
});
```

1. **Use appropriate timeouts**: Web requests should complete within 30 seconds

2. **Implement graceful shutdown**: Handle SIGTERM signals properly

---

## Worker Services (`type: worker`)

### Purpose (Worker Services (`type: worker`))

Worker services run background tasks without handling HTTP requests. They're not publicly accessible.

### Use Cases (Worker Services (`type: worker`))

- **Queue processors**: Redis queue, BullMQ, Celery, Sidekiq
- **Background jobs**: Email sending, image processing, data exports
- **Event consumers**: Message queue consumers (Kafka, RabbitMQ, etc.)
- **Data pipeline workers**: ETL processes, data transformation
- **Scheduled background tasks**: Continuous processes (not cron)
- **WebSocket backend**: Dedicated WebSocket handler services

### Key Characteristics (Worker Services (`type: worker`))

- **No public URL**: Not accessible from internet
- **No port binding**: Doesn't need to listen on a port
- **No health checks**: Render monitors process health differently
- **Long-running**: Can run indefinitely
- **Private communication**: Access via internal networking
- **Restart on crash**: Automatically restarted if process dies

### Required Configuration (Worker Services (`type: worker`))

```yaml
type: worker
name: queue-processor
runtime: python
buildCommand: pip install -r requirements.txt
startCommand: celery -A tasks worker --loglevel=info
```

### Best Practices (Worker Services (`type: worker`))

1. **Connect to message queue**:

```python
import redis
r = redis.from_url(os.environ['REDIS_URL'])
```

1. **Implement retry logic**: Handle failures gracefully

2. **Monitor queue depth**: Track pending jobs

3. **Log processing status**: Make debugging easier

4. **Graceful shutdown**: Finish current jobs before exiting

### Common Patterns

#### Node.js with BullMQ

```yaml
type: worker
name: job-processor
runtime: node
buildCommand: npm ci
startCommand: node worker.js
envVars:
  - key: REDIS_URL
    fromDatabase:
      name: redis
      property: connectionString
```

#### Python with Celery

```yaml
type: worker
name: celery-worker
runtime: python
buildCommand: pip install -r requirements.txt
startCommand: celery -A app.celery worker
envVars:
  - key: REDIS_URL
    fromDatabase:
      name: redis
      property: connectionString
```

---

## Cron Jobs (`type: cron`)

### Purpose (Cron Jobs (`type: cron`))

Cron jobs run scheduled tasks on a repeating schedule. They execute, complete, and shut down.

### Use Cases (Cron Jobs (`type: cron`))

- **Database backups**: Regular automated backups
- **Report generation**: Daily/weekly reports
- **Data cleanup**: Delete old records periodically
- **Cache warming**: Pre-populate caches
- **Email digests**: Send scheduled email summaries
- **Data synchronization**: Sync between systems
- **Batch processing**: Process accumulated data

### Key Characteristics (Cron Jobs (`type: cron`))

- **Scheduled execution**: Runs on cron schedule
- **Automatic shutdown**: Shuts down after completing
- **No persistent port**: Doesn't maintain listening port
- **No health checks**: Task either completes or fails
- **UTC timezone**: All schedules in UTC
- **Maximum runtime**: Jobs timeout after configured limit

### Required Configuration (Cron Jobs (`type: cron`))

```yaml
type: cron
name: daily-backup
runtime: node
schedule: "0 2 * * *"  # Daily at 2 AM UTC
buildCommand: npm ci
startCommand: node scripts/backup.js
```

### Schedule Format

Standard cron syntax: `minute hour day month weekday`

#### Common schedules

| Schedule | Description |
| ---------- | ------------- |
| `*/5 * * * *` | Every 5 minutes |
| `0 * * * *` | Every hour |
| `0 0 * * *` | Daily at midnight UTC |
| `0 9 * * 1-5` | Weekdays at 9 AM UTC |
| `0 0 1 * *` | First day of each month |
| `0 9 * * 1` | Every Monday at 9 AM UTC |

### Best Practices (Cron Jobs (`type: cron`))

1. **Handle failures gracefully**: Jobs should be idempotent

2. **Log completion status**: Track success/failure

3. **Set appropriate timeouts**: Match expected job duration

4. **Use UTC times**: All schedules are UTC-based

5. **Test thoroughly**: Test with different data scenarios

### Example Use Cases

#### Daily Database Backup

```yaml
type: cron
name: db-backup
runtime: python
schedule: "0 1 * * *"  # 1 AM UTC daily
buildCommand: pip install -r requirements.txt
startCommand: python scripts/backup.py
envVars:
  - key: DATABASE_URL
    fromDatabase:
      name: postgres
      property: connectionString
  - key: S3_BUCKET
    value: my-backups
```

#### Hourly Cache Refresh

```yaml
type: cron
name: cache-refresh
runtime: node
schedule: "0 * * * *"  # Top of every hour
buildCommand: npm ci
startCommand: node scripts/refresh-cache.js
```

---

## Static Sites (`type: web` + `runtime: static`)

### Purpose (Static Sites (`type: web` + `runtime: static`))

Serve static HTML, CSS, and JavaScript files via CDN. No backend runtime.

### Use Cases (Static Sites (`type: web` + `runtime: static`))

- **Single Page Applications (SPAs)**: React, Vue, Angular apps
- **Static site generators**: Gatsby, Next.js (static export), Hugo
- **Documentation sites**: MkDocs, Docusaurus, VitePress
- **Landing pages**: Marketing sites
- **Portfolio sites**: Personal websites
- **JAMstack sites**: Static sites with API integration

### Key Characteristics (Static Sites (`type: web` + `runtime: static`))

- **CDN delivery**: Global edge caching
- **No backend runtime**: Only serves built files
- **Build output only**: Serves contents of build directory
- **Routing support**: Rewrite rules for SPA routing
- **Custom headers**: Cache control, security headers
- **Fast deployment**: Quick to build and deploy

### Required Configuration (Static Sites (`type: web` + `runtime: static`))

```yaml
type: web
name: frontend
runtime: static
buildCommand: npm ci && npm run build
staticPublishPath: ./dist  # or ./build, ./out, ./public
```

### Routing for SPAs

Single Page Applications need rewrite rules to handle client-side routing:

```yaml
type: web
name: react-app
runtime: static
buildCommand: npm ci && npm run build
staticPublishPath: ./build
routes:
  - type: rewrite
    source: /*
    destination: /index.html
```

### Custom Headers

Add cache control and security headers:

```yaml
type: web
name: static-site
runtime: static
buildCommand: npm ci && npm run build
staticPublishPath: ./dist
headers:
  # Cache static assets
  - path: /static/*
    name: Cache-Control
    value: public, max-age=31536000, immutable

  # Security headers
  - path: /*
    name: X-Frame-Options
    value: DENY
  - path: /*
    name: X-Content-Type-Options
    value: nosniff
```

### Build Filters

For monorepos, only build when frontend files change:

```yaml
type: web
name: frontend
runtime: static
buildCommand: npm ci && npm run build
staticPublishPath: ./dist
buildFilter:
  paths:
    - frontend/**
  ignoredPaths:
    - frontend/**/*.test.js
    - frontend/README.md
```

### Best Practices (Static Sites (`type: web` + `runtime: static`))

1. **Optimize build output**: Minify, compress, tree-shake

2. **Use proper cache headers**: Long cache for hashed assets

3. **Add security headers**: Protect against common attacks

4. **Configure SPA routing**: Add rewrite rules for client routing

5. **Handle 404s**: Create custom 404.html page

---

## Private Services (`type: pserv`)

### Purpose (Private Services (`type: pserv`))

Internal services accessible only within your Render account. Not exposed to the internet.

### Use Cases (Private Services (`type: pserv`))

- **Internal APIs**: Services accessed only by other services
- **Database proxies**: Connection pools, read replicas
- **Microservices**: Service mesh architectures
- **Admin tools**: Internal dashboards
- **Cache layers**: Internal caching services
- **Message brokers**: Internal message queues

### Key Characteristics (Private Services (`type: pserv`))

- **No public URL**: Only accessible via internal DNS
- **Internal networking**: Fast, low-latency connections
- **Port binding required**: Must bind to `0.0.0.0:$PORT`
- **Private DNS**: `[service-name].render-internal.com`
- **Same-account only**: Only accessible from same account
- **No internet access**: Traffic stays within Render network

### Required Configuration (Private Services (`type: pserv`))

```yaml
type: pserv
name: internal-api
runtime: node
buildCommand: npm ci
startCommand: npm start
```

### Accessing Private Services

From other services in the same account:

```javascript
// Use .render-internal.com domain
const API_URL = 'http://internal-api.render-internal.com:10000';
```

Or use service references:

```yaml
services:
  - type: web
    name: frontend
    runtime: node
    envVars:
      - key: INTERNAL_API_URL
        fromService:
          name: internal-api
          type: pserv
          property: hostport
```

### Best Practices (Private Services (`type: pserv`))

1. **Use internal DNS**: Always use `.render-internal.com` domains

2. **No authentication needed**: Already isolated to account

3. **Fast communication**: Low latency between services

4. **Simplify architecture**: No need for external load balancers

---

## Comparison Table

| Feature | Web | Worker | Cron | Static | Private |
| --------- | ----- | -------- | ------ | -------- | --------- |
| Public URL | ✅ Yes | ❌ No | ❌ No | ✅ Yes | ❌ No |
| Port Binding | ✅ Required | ❌ Not needed | ❌ Not needed | ❌ N/A | ✅ Required |
| Health Checks | ✅ Yes | ❌ No | ❌ No | ❌ N/A | ✅ Yes |
| Runtime | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No | ✅ Yes |
| Persistent | ✅ Yes | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |
| Scaling | ✅ Yes | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |
| Use Case | HTTP servers | Background jobs | Scheduled tasks | Static files | Internal services |

## Choosing the Right Service Type

### Use Web Service when

- Your app handles HTTP requests
- Users need to access it via URL
- You need load balancing and scaling

#### Use Worker Service when

- Processing background jobs
- Consuming from message queues
- Running long-lived processes without HTTP

#### Use Cron Job when

- Running scheduled tasks
- Processing doesn't need to be always-on
- Tasks run periodically (hourly, daily, weekly)

#### Use Static Site when

- Serving pre-built HTML/CSS/JS
- No backend processing needed
- Want CDN caching and fast delivery

#### Use Private Service when

- Service only accessed by other services
- Want internal-only communication
- Building microservice architectures

## When to use

Use when the user asks about or needs: Render Service Types.
﻿---
name: Render Configuration Guide
description: # Render Configuration Guide
 
 Common configuration patterns, best practices, and troubleshooting for Render deployments.
---

# Render Configuration Guide

Common configuration patterns, best practices, and troubleshooting for Render deployments.

## Environment Variables

### Required vs Optional Variables

**Always declare ALL environment variables in render.yaml**, even if values are provided by user later.

#### Three categories

1. **Configuration values** (hardcoded):

```yaml
envVars:
  - key: NODE_ENV
    value: production
  - key: LOG_LEVEL
    value: info
  - key: API_URL
    value: https://api.example.com
```

1. **Secrets** (user provides):

```yaml
envVars:
  - key: JWT_SECRET
    sync: false
  - key: STRIPE_SECRET_KEY
    sync: false
  - key: API_KEY
    sync: false
```

1. **Auto-generated** (Render provides):

```yaml
envVars:
  - key: SESSION_SECRET
    generateValue: true
  - key: ENCRYPTION_KEY
    generateValue: true
```

### Database Connection Patterns

#### PostgreSQL

```yaml
envVars:
  - key: DATABASE_URL
    fromDatabase:
      name: postgres
      property: connectionString
```

#### Redis

```yaml
envVars:
  - key: REDIS_URL
    fromDatabase:
      name: redis
      property: connectionString
```

#### Multiple databases

```yaml
envVars:
  - key: PRIMARY_DB_URL
    fromDatabase:
      name: postgres-primary
      property: connectionString
  - key: ANALYTICS_DB_URL
    fromDatabase:
      name: postgres-analytics
      property: connectionString
  - key: CACHE_URL
    fromDatabase:
      name: redis
      property: connectionString
```

### Cross-Service References

Reference other services in your account:

```yaml
services:
  - type: web
    name: frontend
    runtime: node
    envVars:
      - key: API_URL
        fromService:
          name: backend-api
          type: web
          property: host  # or hostport, port

  - type: web
    name: backend-api
    runtime: node
```

#### Available properties

- `host`: Service hostname
- `port`: Service port
- `hostport`: Combined `host:port`

### Environment Variable Groups

Share common configuration across services:

```yaml
envVarGroups:
  - name: common-config
    envVars:
      - key: NODE_ENV
        value: production
      - key: LOG_LEVEL
        value: info
      - key: TZ
        value: UTC

services:
  - type: web
    name: web-app
    runtime: node
    envVars:
      - fromGroup: common-config
      - key: PORT
        value: 10000

  - type: worker
    name: worker
    runtime: node
    envVars:
      - fromGroup: common-config
```

---

## Port Binding

### The Port Binding Requirement

**CRITICAL:** Web services must bind to `0.0.0.0:$PORT`

#### Why this matters

- Render sets `PORT` environment variable (default: 10000)
- Services must bind to `0.0.0.0` (not `localhost` or `127.0.0.1`)
- Health checks fail if port binding is incorrect
- Deployment will fail or service won't receive traffic

### Code Examples by Language

#### Node.js / Express

```javascript
const express = require('express');
const app = express();

const PORT = process.env.PORT || 3000;

app.listen(PORT, '0.0.0.0', () => {
  console.log(`Server running on port ${PORT}`);
});
```

#### Python / Flask

```python
import os
from flask import Flask

app = Flask(__name__)

if __name__ == '__main__':
    port = int(os.environ.get('PORT', 5000))
    app.run(host='0.0.0.0', port=port)
```

#### Python / Django

In `settings.py`:

```python
# Django runs on port specified by environment
ALLOWED_HOSTS = ['*']
```

Start command in render.yaml:

```yaml
startCommand: gunicorn config.wsgi:application --bind 0.0.0.0:$PORT
```

#### Python / FastAPI

```python
import os
import uvicorn
from fastapi import FastAPI

app = FastAPI()

if __name__ == "__main__":
    port = int(os.environ.get("PORT", 8000))
    uvicorn.run(app, host="0.0.0.0", port=port)
```

Start command:

```yaml
startCommand: uvicorn main:app --host 0.0.0.0 --port $PORT
```

#### Go

```go
package main

import (
    "fmt"
    "net/http"
    "os"
)

func main() {
    port := os.Getenv("PORT")
    if port == "" {
        port = "3000"
    }

    http.HandleFunc("/", handler)
    fmt.Printf("Server starting on port %s\n", port)
    http.ListenAndServe(":"+port, nil)
}
```

#### Ruby / Rails

In `config/puma.rb`:

```ruby
port ENV.fetch("PORT") { 3000 }
bind "tcp://0.0.0.0:#{ENV.fetch('PORT', 3000)}"
```

#### Rust / Actix

```rust
use actix_web::{App, HttpServer};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    HttpServer::new(|| App::new())
        .bind(&addr)?
        .run()
        .await
}
```

---

## Build Commands

### Non-Interactive Flags

**Always use non-interactive flags** to prevent builds from hanging waiting for input.

#### npm (Node.js)

```yaml
buildCommand: npm ci
# NOT: npm install
```

#### pip (Python)

```yaml
buildCommand: pip install -r requirements.txt
# Already non-interactive
```

#### apt (System packages)

```yaml
buildCommand: apt-get update && apt-get install -y libpq-dev
# Use -y flag to auto-confirm
```

#### bundler (Ruby)

```yaml
buildCommand: bundle install --jobs=4 --retry=3
```

### Build with Additional Steps

#### Node.js with build step

```yaml
buildCommand: npm ci && npm run build
```

#### Python Django with static files

```yaml
buildCommand: pip install -r requirements.txt && python manage.py collectstatic --no-input
```

#### Ruby Rails with assets

```yaml
buildCommand: bundle install && bundle exec rails assets:precompile
```

### Build Timeouts

**Free tier:** 15 minutes
**Paid tiers:** Configurable

#### If builds timeout

1. Optimize dependencies (remove unused packages)
2. Use build caching
3. Consider pre-building in CI/CD
4. Upgrade to paid tier for longer timeouts

---

## Database Connections

### Internal vs External URLs

#### Use internal URLs for better performance

When using `fromDatabase`, Render automatically provides internal `.render-internal.com` URLs:

```yaml
envVars:
  - key: DATABASE_URL
    fromDatabase:
      name: postgres
      property: connectionString
```

This provides: `postgresql://user:pass@postgres.render-internal.com:5432/db`

#### Benefits

- Lower latency (same data center)
- No external bandwidth charges
- Automatic internal DNS

### Connection Pooling

#### Node.js / PostgreSQL

```javascript
const { Pool } = require('pg');

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
  ssl: process.env.NODE_ENV === 'production' ? { rejectUnauthorized: false } : false,
  max: 20, // Maximum pool size
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
});
```

#### Python / PostgreSQL

```python
import psycopg2.pool

pool = psycopg2.pool.SimpleConnectionPool(
    minconn=1,
    maxconn=20,
    dsn=os.environ['DATABASE_URL']
)
```

#### Django Settings

```python
DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.postgresql',
        'URL': os.environ['DATABASE_URL'],
        'CONN_MAX_AGE': 600,  # Connection pooling
    }
}
```

### Database Migrations

#### Run migrations during build

#### Django

```yaml
buildCommand: pip install -r requirements.txt && python manage.py migrate
```

#### Rails

```yaml
buildCommand: bundle install && bundle exec rails db:migrate
```

#### Node.js / Prisma

```yaml
buildCommand: npm ci && npx prisma migrate deploy
```

---

## Free Tier Limitations

### What's Included

#### Free tier provides

- 1 web service
- 1 PostgreSQL database (1 GB storage, 97 MB RAM)
- 750 hours/month compute
- 512 MB RAM per service
- 0.5 CPU per service
- 100 GB bandwidth/month

### Resource Limits

#### Memory (512 MB)

- Monitor memory usage in logs
- Optimize for memory-constrained environments
- Use lightweight dependencies

#### CPU (0.5 cores)

- Suitable for low-traffic applications
- Consider upgrading for higher traffic

#### Spin Down (Free services)

- Services spin down after 15 minutes of inactivity
- First request after spin down takes ~30 seconds (cold start)
- Upgrade to paid tier for always-on services

### When to Upgrade

#### Upgrade to paid plan when

- Need more than 1 web service
- Need always-on services (no spin down)
- Traffic exceeds free tier limits
- Need more memory/CPU
- Need faster build times
- Need preview environments

---

## Health Checks

### Adding Health Check Endpoints

#### Node.js / Express (Adding Health Check Endpoints)

```javascript
app.get('/health', (req, res) => {
  res.status(200).json({
    status: 'ok',
    timestamp: new Date().toISOString()
  });
});
```

#### Python / Flask (Adding Health Check Endpoints)

```python
@app.route('/health')
def health():
    return {'status': 'ok'}, 200
```

#### Python / FastAPI (Adding Health Check Endpoints)

```python
@app.get("/health")
async def health():
    return {"status": "ok"}
```

#### Go (Adding Health Check Endpoints)

```go
http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
    w.WriteHeader(http.StatusOK)
    w.Write([]byte(`{"status":"ok"}`))
})
```

### Configure in render.yaml

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    healthCheckPath: /health
```

#### Benefits (Configure in render.yaml)

- Faster deployment detection
- Better monitoring
- Automatic restart on health check failures

---

## Common Deployment Issues

### Issue 1: Missing Environment Variables

**Symptom:** Service crashes with "undefined variable" errors

**Solution:** Add all required env vars to render.yaml:

```yaml
envVars:
  - key: DATABASE_URL
    fromDatabase:
      name: postgres
      property: connectionString
  - key: JWT_SECRET
    sync: false  # User fills in Dashboard
```

### Issue 2: Port Binding Errors

**Symptom:** `EADDRINUSE` or health check timeout errors

**Solution:** Ensure app binds to `0.0.0.0:$PORT`:

```javascript
const PORT = process.env.PORT || 3000;
app.listen(PORT, '0.0.0.0');
```

### Issue 3: Build Hangs

**Symptom:** Build times out after 15 minutes

**Solution:** Use non-interactive build commands:

```yaml
buildCommand: npm ci  # NOT npm install
```

### Issue 4: Database Connection Fails

**Symptom:** `ECONNREFUSED` on port 5432

#### Solutions

1. Use `fromDatabase` for automatic internal URLs
2. Enable SSL for external connections
3. Check `ipAllowList` settings

### Issue 5: Static Site 404s

**Symptom:** Client-side routes return 404

**Solution:** Add SPA rewrite rules:

```yaml
routes:
  - type: rewrite
    source: /*
    destination: /index.html
```

### Issue 6: Out of Memory (OOM)

**Symptom:** Service crashes with `JavaScript heap out of memory`

#### Solutions (Issue 6: Out of Memory (OOM))

1. Optimize application memory usage
2. Reduce dependency size
3. Upgrade to higher plan with more RAM

---

## Best Practices Checklist

### Environment Variables (Best Practices Checklist)

- [ ] All env vars declared in render.yaml
- [ ] Secrets marked with `sync: false`
- [ ] Database URLs use `fromDatabase` references

#### Port Binding (Environment Variables)

- [ ] App binds to `process.env.PORT`
- [ ] Bind to `0.0.0.0` (not `localhost`)

#### Build Commands (Environment Variables)

- [ ] Use non-interactive flags (`npm ci`, `-y`, etc.)
- [ ] Build completes under 15 minutes (free tier)

#### Start Commands

- [ ] Command starts HTTP server correctly
- [ ] Server binds to correct port

#### Health Checks (Environment Variables)

- [ ] `/health` endpoint implemented
- [ ] Returns 200 status code

#### Database

- [ ] Connection pooling configured
- [ ] Using internal URLs (`.render-internal.com`)
- [ ] SSL enabled if needed

#### Plans

- [ ] Using `plan: free` by default
- [ ] Documented upgrade path for users

#### Git Repository

- [ ] render.yaml committed to repository
- [ ] Pushed to git remote (GitHub/GitLab/Bitbucket)
- [ ] Branch specified in render.yaml (if not main)

---

## Additional Resources

- Blueprint Specification: [blueprint-spec.md](blueprint-spec.md)
- Service Types: [service-types.md](service-types.md)
- Runtimes: [runtimes.md](runtimes.md)
- Official Render Docs: <https://render.com/docs>

## When to use

Use when the user asks about or needs: Render Configuration Guide.
