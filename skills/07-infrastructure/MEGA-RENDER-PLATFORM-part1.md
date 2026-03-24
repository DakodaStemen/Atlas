---
name: "Render Blueprint Specification (Part 1)"
description: # Render Blueprint Specification
 
 Complete reference for render.yaml Blueprint files. Blueprints define your infrastructure as code for reproducible deployments on Render.
---


# Render Blueprint Specification

Complete reference for render.yaml Blueprint files. Blueprints define your infrastructure as code for reproducible deployments on Render.

## Overview

A Blueprint is a YAML file (typically `render.yaml`) placed in your repository root that describes:

- Services (web, worker, cron, static, private)
- Databases (PostgreSQL, Redis)
- Environment variables and secrets
- Scaling and resource configuration
- Project organization

## Root-Level Structure

```yaml
# Top-level fields
services: []         # Array of service definitions
databases: []        # Array of PostgreSQL databases
envVarGroups: []     # Reusable environment variable groups (optional)
projects: []         # Project organization (optional)
ungrouped: []        # Resources outside projects (optional)
previews:            # Preview environment configuration (optional)
  generation: auto_preview | manual | none
```

## Service Types

### Web Services (`type: web`)

HTTP services, APIs, and web applications. Publicly accessible via HTTPS.

#### Required fields

- `name`: Unique service identifier
- `type`: Must be `web`
- `runtime`: Language/environment (see Runtimes section)
- `buildCommand`: Command to build the application
- `startCommand`: Command to start the server

#### Common optional fields

- `plan`: Instance type (default: `free`)
- `region`: Deployment region (default: `oregon`)
- `branch`: Git branch to deploy (default: `main`)
- `autoDeploy`: Auto-deploy on push (default: `true`)
- `envVars`: Environment variables array
- `healthCheckPath`: Health check endpoint (default: `/`)
- `numInstances`: Number of instances (manual scaling)
- `scaling`: Autoscaling configuration

#### Example

```yaml
services:
  - type: web
    name: api-server
    runtime: node
    plan: free
    buildCommand: npm ci
    startCommand: npm start
    branch: main
    autoDeploy: true
    envVars:
      - key: NODE_ENV
        value: production
      - key: PORT
        value: 10000
```

### Worker Services (`type: worker`)

Background job processors, queue consumers. Not publicly accessible.

#### Required fields (Worker Services (`type: worker`))

- `name`: Unique service identifier
- `type`: Must be `worker`
- `runtime`: Language/environment
- `buildCommand`: Command to build
- `startCommand`: Command to start worker process

#### Key differences from web services

- No public URL
- No health checks
- No port binding required

#### Example (Worker Services (`type: worker`))

```yaml
services:
  - type: worker
    name: job-processor
    runtime: python
    plan: free
    buildCommand: pip install -r requirements.txt
    startCommand: celery -A tasks worker --loglevel=info
    envVars:
      - key: REDIS_URL
        fromDatabase:
          name: redis
          property: connectionString
```

### Cron Jobs (`type: cron`)

Scheduled tasks that run on a cron schedule.

#### Required fields (Cron Jobs (`type: cron`))

- `name`: Unique service identifier
- `type`: Must be `cron`
- `runtime`: Language/environment
- `schedule`: Cron expression
- `buildCommand`: Command to build
- `startCommand`: Command to execute on schedule

**Schedule format:** Standard cron syntax (minute hour day month weekday)

#### Examples

- `0 0 * * *` - Daily at midnight UTC
- `*/15 * * * *` - Every 15 minutes
- `0 9 * * 1` - Every Monday at 9 AM UTC

#### Example (Cron Jobs (`type: cron`))

```yaml
services:
  - type: cron
    name: daily-backup
    runtime: node
    schedule: "0 2 * * *"
    buildCommand: npm ci
    startCommand: node scripts/backup.js
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: postgres
          property: connectionString
```

### Static Sites (`type: static` or `type: web` with `runtime: static`)

Serve static HTML/CSS/JS files via CDN.

#### Required fields (Static Sites (`type: static` or `type: web` with `runtime: static`))

- `name`: Unique service identifier
- `type`: `web`
- `runtime`: `static`
- `buildCommand`: Command to build static assets
- `staticPublishPath`: Path to built files (e.g., `./build`, `./dist`)

#### Optional configuration

- `routes`: Routing rules for SPAs
- `headers`: Custom HTTP headers
- `buildFilter`: Path filters for build triggers

#### Example (Static Sites (`type: static` or `type: web` with `runtime: static`))

```yaml
services:
  - type: web
    name: react-app
    runtime: static
    buildCommand: npm ci && npm run build
    staticPublishPath: ./dist
    routes:
      - type: rewrite
        source: /*
        destination: /index.html
    headers:
      - path: /*
        name: Cache-Control
        value: public, max-age=31536000, immutable
```

### Private Services (`type: pserv`)

Internal services accessible only within your Render account.

#### Required fields (Private Services (`type: pserv`))

- `name`: Unique service identifier
- `type`: Must be `pserv`
- `runtime`: Language/environment
- `buildCommand`: Command to build
- `startCommand`: Command to start

#### Use cases

- Internal APIs
- Database proxies
- Microservices not exposed to internet

#### Example (Private Services (`type: pserv`))

```yaml
services:
  - type: pserv
    name: internal-api
    runtime: go
    plan: free
    buildCommand: go build -o bin/app
    startCommand: ./bin/app
```

## Runtimes

### Native Runtimes

#### Node.js (`runtime: node`)

- Versions: 14, 16, 18, 20, 21
- Default version: 20
- Specify version in `package.json` engines field

#### Python (`runtime: python`)

- Versions: 3.8, 3.9, 3.10, 3.11, 3.12
- Default version: 3.11
- Specify version in `runtime.txt` or `Pipfile`

#### Go (`runtime: go`)

- Versions: 1.20, 1.21, 1.22, 1.23
- Uses go modules
- Version from `go.mod`

#### Ruby (`runtime: ruby`)

- Versions: 3.0, 3.1, 3.2, 3.3
- Uses Bundler
- Version from `.ruby-version` or `Gemfile`

#### Rust (`runtime: rust`)

- Latest stable version
- Uses Cargo

#### Elixir (`runtime: elixir`)

- Latest stable version
- Uses Mix

### Docker Runtime

#### Docker (`runtime: docker`)

Build from a Dockerfile in your repository.

#### Additional fields

- `dockerfilePath`: Path to Dockerfile (default: `./Dockerfile`)
- `dockerContext`: Build context directory (default: `.`)

#### Example (Docker Runtime)

```yaml
services:
  - type: web
    name: docker-app
    runtime: docker
    dockerfilePath: ./docker/Dockerfile
    dockerContext: .
    plan: free
```

#### Image (`runtime: image`)

Deploy pre-built Docker images from a registry.

#### Additional fields (2)

- `image`: Image URL (e.g., `registry.com/image:tag`)
- `registryCredential`: Credentials for private registries

#### Example (Docker Runtime) (2)

```yaml
services:
  - type: web
    name: prebuilt-app
    runtime: image
    image: myregistry.com/app:v1.2.3
    plan: free
```

## Service Plans

Available instance types:

| Plan | RAM | CPU | Price |
| ------ | ----- | ----- | ------- |
| `free` | 512 MB | 0.5 | Free (750 hrs/mo) |
| `starter` | 512 MB | 0.5 | $7/month |
| `standard` | 2 GB | 1 | $25/month |
| `pro` | 4 GB | 2 | $85/month |
| `pro_plus` | 8 GB | 4 | $175/month |

### Always default to `plan: free` unless user specifies otherwise

## Regions

Available deployment regions:

- `oregon` (US West) - Default
- `ohio` (US East)
- `virginia` (US East)
- `frankfurt` (EU)
- `singapore` (Asia)

### Example (Regions)

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    region: frankfurt
```

## Environment Variables

Three patterns for defining environment variables:

### 1. Hardcoded Values

For non-sensitive configuration:

```yaml
envVars:
  - key: NODE_ENV
    value: production
  - key: API_URL
    value: https://api.example.com
  - key: LOG_LEVEL
    value: info
```

### 2. Generated Secrets

Render generates a base64-encoded 256-bit random value:

```yaml
envVars:
  - key: SESSION_SECRET
    generateValue: true
  - key: ENCRYPTION_KEY
    generateValue: true
```

### 3. User-Provided Secrets

Prompt user for values during Blueprint creation:

```yaml
envVars:
  - key: STRIPE_SECRET_KEY
    sync: false
  - key: JWT_SECRET
    sync: false
  - key: API_KEY
    sync: false
```

#### The `sync: false` flag means "user will fill this in the Dashboard"

### 4. Database References

Link to database connection strings:

```yaml
envVars:
  - key: DATABASE_URL
    fromDatabase:
      name: postgres
      property: connectionString
  - key: REDIS_URL
    fromDatabase:
      name: redis
      property: connectionString
```

#### Available properties

- `connectionString`: Full connection URL
- `host`: Database host
- `port`: Database port
- `user`: Database username
- `password`: Database password
- `database`: Database name
- `hostport`: Combined `host:port`

### 5. Service References

Link to other services:

```yaml
envVars:
  - key: API_URL
    fromService:
      name: api-server
      type: web
      property: host
```

### 6. Environment Variable Groups

Reusable groups shared across services:

```yaml
envVarGroups:
  - name: shared-config
    envVars:
      - key: LOG_LEVEL
        value: info
      - key: ENVIRONMENT
        value: production

services:
  - type: web
    name: web-app
    runtime: node
    envVars:
      - fromGroup: shared-config
      - key: PORT
        value: 10000
```

## Databases

### PostgreSQL

```yaml
databases:
  - name: postgres
    databaseName: myapp_prod
    user: myapp_user
    plan: free
    postgresMajorVersion: "15"
    ipAllowList: []
```

#### Plans

- `free`: 1 GB storage, 97 MB RAM, 0.1 CPU
- `basic-256mb`, `basic-512mb`, `basic-1gb`, `basic-4gb`
- `pro-4gb`, `pro-8gb`, `pro-16gb`, etc.
- `accelerated-4gb`, `accelerated-8gb`, etc. (SSD-backed)

#### Key fields

- `name`: Identifier for references
- `databaseName`: Actual PostgreSQL database name
- `user`: Database username
- `postgresMajorVersion`: PostgreSQL version (11-16)
- `ipAllowList`: Array of CIDR blocks (empty = internal only)
- `diskSizeGB`: Storage size (paid plans only)

#### High Availability (paid plans)

```yaml
databases:
  - name: postgres
    databaseName: myapp_prod
    plan: pro-4gb
    highAvailabilityEnabled: true
```

#### Read Replicas (paid plans)

```yaml
databases:
  - name: postgres
    databaseName: myapp_prod
    plan: pro-4gb
    readReplicas:
      - name: read-replica-1
        region: ohio
      - name: read-replica-2
        region: frankfurt
```

### Redis (Key-Value Store)

```yaml
databases:
  - name: redis
    plan: free
    maxmemoryPolicy: allkeys-lru
    ipAllowList: []
```

**Plans:** Same as PostgreSQL

#### maxmemoryPolicy options

- `allkeys-lru`: Evict least recently used keys
- `volatile-lru`: Evict LRU keys with TTL
- `allkeys-random`: Evict random keys
- `volatile-random`: Evict random keys with TTL
- `volatile-ttl`: Evict keys with soonest TTL
- `noeviction`: Return errors when memory full

## Scaling

### Manual Scaling

Fixed number of instances:

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    plan: standard
    numInstances: 3
```

### Autoscaling

Dynamic scaling based on CPU/memory (Professional workspace required):

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    plan: standard
    scaling:
      minInstances: 1
      maxInstances: 5
      targetCPUPercent: 60
      targetMemoryPercent: 70
```

#### Notes

- Autoscaling disabled in preview environments
- Preview environments run `minInstances` count
- Requires Professional or higher workspace

## Health Checks

Configure health check endpoints:

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    healthCheckPath: /health
```

**Default:** `/` (root path)

**Recommended:** Add a dedicated `/health` endpoint that returns `200 OK`.

## Build Filters

Control when builds are triggered based on changed files:

```yaml
services:
  - type: web
    name: frontend
    runtime: static
    buildFilter:
      paths:
        - frontend/**
      ignoredPaths:
        - frontend/README.md
        - frontend/**/*.test.js
```

### Behavior

- If `paths` specified: Build only when files in those paths change
- If `ignoredPaths` specified: Don't build when only ignored files change

## Projects and Environments

Organize services into projects with multiple environments:

```yaml
projects:
  - name: my-application
    environments:
      - name: production
        services:
          - type: web
            name: prod-api
            runtime: node
            plan: pro
            buildCommand: npm ci
            startCommand: npm start
        databases:
          - name: prod-postgres
            plan: pro-4gb
        networking:
          isolation: enabled
        permissions:
          protection: enabled

      - name: staging
        services:
          - type: web
            name: staging-api
            runtime: node
            plan: starter
            buildCommand: npm ci
            startCommand: npm start
        databases:
          - name: staging-postgres
            plan: free
```

### Environment features

- `networking.isolation`: Enable network isolation between environments
- `permissions.protection`: Require approval for environment changes

## Preview Environments

Configure automatic preview environments for pull requests:

```yaml
previews:
  generation: auto_preview  # auto_preview | manual | none
```

### Options

- `auto_preview`: Create preview environment for each PR automatically
- `manual`: User manually triggers preview creation
- `none`: Disable preview environments

## Complete Example

Full-featured Blueprint with multiple services and databases:

```yaml
services:
  # Web service
  - type: web
    name: web-app
    runtime: node
    plan: free
    region: oregon
    buildCommand: npm ci && npm run build
    startCommand: npm start
    branch: main
    autoDeploy: true
    healthCheckPath: /health
    envVars:
      - key: NODE_ENV
        value: production
      - key: DATABASE_URL
        fromDatabase:
          name: postgres
          property: connectionString
      - key: REDIS_URL
        fromDatabase:
          name: redis
          property: connectionString
      - key: JWT_SECRET
        sync: false

  # Background worker
  - type: worker
    name: queue-worker
    runtime: node
    plan: free
    buildCommand: npm ci
    startCommand: node worker.js
    envVars:
      - key: REDIS_URL
        fromDatabase:
          name: redis
          property: connectionString

  # Cron job
  - type: cron
    name: daily-cleanup
    runtime: node
    schedule: "0 3 * * *"
    buildCommand: npm ci
    startCommand: node scripts/cleanup.js
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: postgres
          property: connectionString

  # Static frontend
  - type: web
    name: frontend
    runtime: static
    buildCommand: npm ci && npm run build
    staticPublishPath: ./dist
    routes:
      - type: rewrite
        source: /*
        destination: /index.html

databases:
  - name: postgres
    databaseName: app_production
    user: app_user
    plan: free
    postgresMajorVersion: "15"
    ipAllowList: []

  - name: redis
    plan: free
    maxmemoryPolicy: allkeys-lru
    ipAllowList: []
```

## Validation

Validate your Blueprint before deploying (when CLI command is available):

```bash
render blueprint validate
```

### Common validation errors

- Missing required fields
- Invalid runtime values
- Incorrect environment variable references
- Invalid cron expressions
- Invalid YAML syntax

## Best Practices

1. **Always use `plan: free` by default** - Let users upgrade if needed
2. **Mark all secrets with `sync: false`** - Never hardcode sensitive values
3. **Use `fromDatabase` for database URLs** - Automatic internal connection strings
4. **Add health check endpoints** - Faster deployment detection
5. **Use non-interactive build commands** - Prevents build hangs
6. **Bind to `0.0.0.0:$PORT`** - Required for web services
7. **Use environment variable groups** - Share config across services
8. **Enable autoDeploy: true** - Deploy automatically on push
9. **Set appropriate regions** - Choose closest to your users
10. **Use build filters** - Optimize build triggers in monorepos

## Additional Resources

- Official Blueprint Specification: <https://render.com/docs/blueprint-spec>
- Render CLI Documentation: <https://render.com/docs/cli>
- Environment Variables Guide: <https://render.com/docs/environment-variables>

## When to use

Use when the user asks about or needs: Render Blueprint Specification.
﻿---
name: render-deploy
description: Deploy applications to Render by analyzing codebases, generating render.yaml Blueprints, and providing Dashboard deeplinks. Use when the user wants to deploy, host, publish, or set up their application on Render's cloud platform.
---

# Deploy to Render

Render supports **Git-backed** services and **prebuilt Docker image** services.

This skill covers **Git-backed** flows:

1. **Blueprint Method** - Generate render.yaml for Infrastructure-as-Code deployments
2. **Direct Creation** - Create services instantly via MCP tools

Blueprints can also run a **prebuilt Docker image** by using `runtime: image`, but the `render.yaml` still must live in a Git repo.

If there is no Git remote, stop and ask the user to either:

- Create/push a Git remote (can be minimal if only the Blueprint is needed), or
- Use the Render Dashboard/API to deploy a prebuilt Docker image (MCP cannot create image-backed services).

## Prerequisites

- When sandboxing blocks the deployment network calls, rerun with `sandbox_permissions=require_escalated`.
- The deployment might take a few minutes. Use appropriate timeout values.

## When to Use This Skill

Activate this skill when users want to:

- Deploy an application to Render
- Create a render.yaml Blueprint file
- Set up Render deployment for their project
- Host or publish their application on Render's cloud platform
- Create databases, cron jobs, or other Render resources

## Happy Path (New Users)

Use this short prompt sequence before deep analysis to reduce friction:

1. Ask whether they want to deploy from a Git repo or a prebuilt Docker image.
2. Ask whether Render should provision everything the app needs (based on what seems likely from the user's description) or only the app while they bring their own infra. If dependencies are unclear, ask a short follow-up to confirm whether they need a database, workers, cron, or other services.

Then proceed with the appropriate method below.

## Choose Your Source Path

**Git Repo Path:** Required for both Blueprint and Direct Creation. The repo must be pushed to GitHub, GitLab, or Bitbucket.

**Prebuilt Docker Image Path:** Supported by Render via image-backed services. This is **not** supported by MCP; use the Dashboard/API. Ask for:

- Image URL (registry + tag)
- Registry auth (if private)
- Service type (web/worker) and port

If the user chooses a Docker image, guide them to the Render Dashboard image deploy flow or ask them to add a Git remote (so you can use a Blueprint with `runtime: image`).

## Choose Your Deployment Method (Git Repo)

Both methods require a Git repository pushed to GitHub, GitLab, or Bitbucket. (If using `runtime: image`, the repo can be minimal and only contain `render.yaml`.)

| Method | Best For | Pros |
| -------- | ---------- | ------ |
| **Blueprint** | Multi-service apps, IaC workflows | Version controlled, reproducible, supports complex setups |
| **Direct Creation** | Single services, quick deployments | Instant creation, no render.yaml file needed |

### Method Selection Heuristic

Use this decision rule by default unless the user requests a specific method. Analyze the codebase first; only ask if deployment intent is unclear (e.g., DB, workers, cron).

#### Use Direct Creation (MCP) when ALL are true

- Single service (one web app or one static site)
- No separate worker/cron services
- No attached databases or Key Value
- Simple env vars only (no shared env groups)
If this path fits and MCP isn't configured yet, stop and guide MCP setup before proceeding.

#### Use Blueprint when ANY are true

- Multiple services (web + worker, API + frontend, etc.)
- Databases, Redis/Key Value, or other datastores are required
- Cron jobs, background workers, or private services
- You want reproducible IaC or a render.yaml committed to the repo
- Monorepo or multi-env setup that needs consistent configuration

If unsure, ask a quick clarifying question, but default to Blueprint for safety. For a single service, strongly prefer Direct Creation via MCP and guide MCP setup if needed.

## Prerequisites Check

When starting a deployment, verify these requirements in order:

### 1. Confirm Source Path (Git vs Docker)

If using Git-based methods (Blueprint or Direct Creation), the repo must be pushed to GitHub/GitLab/Bitbucket. Blueprints that reference a prebuilt image still require a Git repo with `render.yaml`.

```bash
git remote -v
```

- If no remote exists, stop and ask the user to create/push a remote **or** switch to Docker image deploy.

#### 2. Check MCP Tools Availability (Preferred for Single-Service)

MCP tools provide the best experience. Check if available by attempting:

```text
list_services()
```

If MCP tools are available, you can skip CLI installation for most operations.

#### 3. Check Render CLI Installation (for Blueprint validation)

```bash
render --version
```

If not installed, offer to install:

- macOS: `brew install render`
- Linux/macOS: `curl -fsSL https://raw.githubusercontent.com/render-oss/cli/main/bin/install.sh | sh`

#### 4. MCP Setup (if MCP isn't configured)

If `list_services()` fails because MCP isn't configured, ask whether they want to set up MCP (preferred) or continue with the CLI fallback. If they choose MCP, ask which AI tool they're using, then provide the matching instructions below. Always use their API key.

### Cursor

Walk the user through these steps:

1) Get a Render API key:

```yaml
https://dashboard.render.com/u/*/settings#api-keys
```

1) Add this to `~/.cursor/mcp.json` (replace `<YOUR_API_KEY>`):

```json
{
  "mcpServers": {
    "render": {
      "url": "https://mcp.render.com/mcp",
      "headers": {
        "Authorization": "Bearer <YOUR_API_KEY>"
      }
    }
  }
}
```

1) Restart Cursor, then retry `list_services()`.

### Claude Code

Walk the user through these steps:

1) Get a Render API key:

```yaml
https://dashboard.render.com/u/*/settings#api-keys
```

1) Add the MCP server with Claude Code (replace `<YOUR_API_KEY>`):

```bash
claude mcp add --transport http render https://mcp.render.com/mcp --header "Authorization: Bearer <YOUR_API_KEY>"
```

1) Restart Claude Code, then retry `list_services()`.

### Codex

Walk the user through these steps:

1) Get a Render API key:

```yaml
https://dashboard.render.com/u/*/settings#api-keys
```

1) Set it in their shell:

```bash
export RENDER_API_KEY="<YOUR_API_KEY>"
```

1) Add the MCP server with the Codex CLI:

```bash
codex mcp add render --url https://mcp.render.com/mcp --bearer-token-env-var RENDER_API_KEY
```

1) Restart Codex, then retry `list_services()`.

### Other Tools

If the user is on another AI app, direct them to the Render MCP docs for that tool's setup steps and install method.

### Workspace Selection

After MCP is configured, have the user set the active Render workspace with a prompt like:

```text
Set my Render workspace to [WORKSPACE_NAME]
```

#### 5. Check Authentication (CLI fallback only)

If MCP isn't available, use the CLI instead and verify you can access your account:

```bash
# Check if user is logged in (use -o json for non-interactive mode)
render whoami -o json
```

If `render whoami` fails or returns empty data, the CLI is not authenticated. The CLI won't always prompt automatically, so explicitly prompt the user to authenticate:

If neither is configured, ask user which method they prefer:

- **API Key (CLI)**: `export RENDER_API_KEY="rnd_xxxxx"` (Get from <https://dashboard.render.com/u/*/settings#api-keys>)
- **Login**: `render login` (Opens browser for OAuth)

#### 6. Check Workspace Context

Verify the active workspace:

```text
get_selected_workspace()
```

Or via CLI:

```bash
render workspace current -o json
```

To list available workspaces:

```text
list_workspaces()
```

If user needs to switch workspaces, they must do so via Dashboard or CLI (`render workspace set`).

Once prerequisites are met, proceed with deployment workflow.

---

## Method 1: Blueprint Deployment (Recommended for Complex Apps)

## Blueprint Workflow

### Step 1: Analyze Codebase

Analyze the codebase to determine framework/runtime, build and start commands, required env vars, datastores, and port binding. Use the detailed checklists in [references/codebase-analysis.md](references/codebase-analysis.md).

### Step 2: Generate render.yaml

Create a `render.yaml` Blueprint file following the Blueprint specification.

Complete specification: [references/blueprint-spec.md](references/blueprint-spec.md)

#### Key Points

- Always use `plan: free` unless user specifies otherwise
- Include ALL environment variables the app needs
- Mark secrets with `sync: false` (user fills these in Dashboard)
- Use appropriate service type: `web`, `worker`, `cron`, `static`, or `pserv`
- Use appropriate runtime: [references/runtimes.md](references/runtimes.md)

#### Basic Structure

```yaml
services:
  - type: web
    name: my-app
    runtime: node
    plan: free
    buildCommand: npm ci
    startCommand: npm start
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: postgres
          property: connectionString
      - key: JWT_SECRET
        sync: false  # User fills in Dashboard

databases:
  - name: postgres
    databaseName: myapp_db
    plan: free
```

#### Service Types

- `web`: HTTP services, APIs, web applications (publicly accessible)
- `worker`: Background job processors (not publicly accessible)
- `cron`: Scheduled tasks that run on a cron schedule
- `static`: Static sites (HTML/CSS/JS served via CDN)
- `pserv`: Private services (internal only, within same account)

Service type details: [references/service-types.md](references/service-types.md)
Runtime options: [references/runtimes.md](references/runtimes.md)
Template examples: [assets/](assets/)

### Step 2.5: Immediate Next Steps (Always Provide)

After creating `render.yaml`, always give the user a short, explicit checklist and run validation immediately when the CLI is available:

1. **Authenticate (CLI)**: run `render whoami -o json` (if not logged in, run `render login` or set `RENDER_API_KEY`)
2. **Validate (recommended)**: run `render blueprints validate`
   - If the CLI isn't installed, offer to install it and provide the command.
3. **Commit + push**: `git add render.yaml && git commit -m "Add Render deployment configuration" && git push origin main`
4. **Open Dashboard**: Use the Blueprint deeplink and complete Git OAuth if prompted
5. **Fill secrets**: Set env vars marked `sync: false`
6. **Deploy**: Click "Apply" and monitor the deploy

### Step 3: Validate Configuration

Validate the render.yaml file to catch errors before deployment. If the CLI is installed, run the commands directly; only prompt the user if the CLI is missing:

```bash
render whoami -o json  # Ensure CLI is authenticated (won't always prompt)
render blueprints validate
```

Fix any validation errors before proceeding. Common issues:

- Missing required fields (`name`, `type`, `runtime`)
- Invalid runtime values
- Incorrect YAML syntax
- Invalid environment variable references

Configuration guide: [references/configuration-guide.md](references/configuration-guide.md)

### Step 4: Commit and Push

**IMPORTANT:** You must merge the `render.yaml` file into your repository before deploying.

Ensure the `render.yaml` file is committed and pushed to your Git remote:

```bash
git add render.yaml
git commit -m "Add Render deployment configuration"
git push origin main
```

If there is no Git remote yet, stop here and guide the user to create a GitHub/GitLab/Bitbucket repo, add it as `origin`, and push before continuing.

**Why this matters:** The Dashboard deeplink will read the render.yaml from your repository. If the file isn't merged and pushed, Render won't find the configuration and deployment will fail.

Verify the file is in your remote repository before proceeding to the next step.

### Step 5: Generate Deeplink

Get the Git repository URL:

```bash
git remote get-url origin
```

This will return a URL from your Git provider. **If the URL is SSH format, convert it to HTTPS:**

| SSH Format | HTTPS Format |
| ------------ | -------------- |
| `git@github.com:user/repo.git` | `https://github.com/user/repo` |
| `git@gitlab.com:user/repo.git` | `https://gitlab.com/user/repo` |
| `git@bitbucket.org:user/repo.git` | `https://bitbucket.org/user/repo` |

**Conversion pattern:** Replace `git@<host>:` with `https://<host>/` and remove `.git` suffix.

Format the Dashboard deeplink using the HTTPS repository URL:

```yaml
https://dashboard.render.com/blueprint/new?repo=<REPOSITORY_URL>
```

Example:

```yaml
https://dashboard.render.com/blueprint/new?repo=https://github.com/username/repo-name
```

### Step 6: Guide User

**CRITICAL:** Ensure the user has merged and pushed the render.yaml file to their repository before clicking the deeplink. If the file isn't in the repository, Render cannot read the Blueprint configuration and deployment will fail.

Provide the deeplink to the user with these instructions:

1. **Verify render.yaml is merged** - Confirm the file exists in your repository on GitHub/GitLab/Bitbucket
2. Click the deeplink to open Render Dashboard
3. Complete Git provider OAuth if prompted
4. Name the Blueprint (or use default from render.yaml)
5. Fill in secret environment variables (marked with `sync: false`)
6. Review services and databases configuration
7. Click "Apply" to deploy

The deployment will begin automatically. Users can monitor progress in the Render Dashboard.

### Step 7: Verify Deployment

After the user deploys via Dashboard, verify everything is working.

#### Check deployment status via MCP

```text
list_deploys(serviceId: "<service-id>", limit: 1)
```

Look for `status: "live"` to confirm successful deployment.

#### Check for runtime errors (wait 2-3 minutes after deploy)

```text
list_logs(resource: ["<service-id>"], level: ["error"], limit: 20)
```

#### Check service health metrics

```text
get_metrics(
  resourceId: "<service-id>",
  metricTypes: ["http_request_count", "cpu_usage", "memory_usage"]
)
```

If errors are found, proceed to the **Post-deploy verification and basic triage** section below.

---

## Method 2: Direct Service Creation (Quick Single-Service Deployments)

For simple deployments without Infrastructure-as-Code, create services directly via MCP tools.

## When to Use Direct Creation

- Single web service or static site
- Quick prototypes or demos
- When you don't need a render.yaml file in your repo
- Adding databases or cron jobs to existing projects

## Prerequisites for Direct Creation

**Repository must be pushed to a Git provider.** Render clones your repository to build and deploy services.

```bash
git remote -v  # Verify remote exists
git push origin main  # Ensure code is pushed
```

Supported providers: GitHub, GitLab, Bitbucket

If no remote exists, stop and ask the user to create/push a remote or switch to Docker image deploy.

**Note:** MCP does not support creating image-backed services. Use the Dashboard/API for prebuilt Docker image deploys.

## Direct Creation Workflow

Use the concise steps below, and refer to [references/direct-creation.md](references/direct-creation.md) for full MCP command examples and follow-on configuration.

### Step 1: Analyze Codebase (Direct Creation Workflow)

Use [references/codebase-analysis.md](references/codebase-analysis.md) to determine runtime, build/start commands, env vars, and datastores.

### Step 2: Create Resources via MCP

Create the service (web or static) and any required databases or key-value stores. See [references/direct-creation.md](references/direct-creation.md).

If MCP returns an error about missing Git credentials or repo access, stop and guide the user to connect their Git provider in the Render Dashboard, then retry.

### Step 3: Configure Environment Variables

Add required env vars via MCP after creation. See [references/direct-creation.md](references/direct-creation.md).

Remind the user that secrets can be set in the Dashboard if they prefer not to pass them via MCP.

### Step 4: Verify Deployment

Check deploy status, logs, and metrics. See [references/direct-creation.md](references/direct-creation.md).

---

For service discovery, configuration details, quick commands, and common issues, see [references/deployment-details.md](references/deployment-details.md).

---

## Post-deploy verification and basic triage (All Methods)

Keep this short and repeatable. If any check fails, fix it before redeploying.

1. Confirm the latest deploy is `live` and serving traffic
2. Hit the health endpoint (or root) and verify a 200 response
3. Scan recent error logs for a clear failure signature
4. Verify required env vars and port binding (`0.0.0.0:$PORT`)

Detailed checklist and commands: [references/post-deploy-checks.md](references/post-deploy-checks.md)

If the service fails to start or health checks time out, use the basic triage guide:
[references/troubleshooting-basics.md](references/troubleshooting-basics.md)

## Troubleshooting

### Escalated Network Access

If deployment fails due to network issues (timeouts, DNS errors, connection resets), rerun the deploy with escalated permissions (use `sandbox_permissions=require_escalated`). The deploy requires escalated network access when sandbox networking blocks outbound requests.

Example guidance to the user:

```text
The deploy needs escalated network access to deploy to Render. I can rerun the command with escalated permissions—want me to proceed?
```

Optional: If you need deeper diagnostics (metrics/DB checks/error catalog), suggest installing the
`render-debug` skill. It is not required for the core deploy flow.
﻿---
name: Render Runtime Options
description: # Render Runtime Options
 
 Complete guide to available runtimes on Render, including versions, configuration, and best practices for each language.
---

# Render Runtime Options

Complete guide to available runtimes on Render, including versions, configuration, and best practices for each language.
