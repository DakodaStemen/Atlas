---
name: "C3 CLI Reference (Part 1)"
description: # C3 CLI Reference
 
 ## Invocation
---


# C3 CLI Reference

## Invocation (C3 CLI Reference)

```bash
npm create cloudflare@latest [name] [-- flags]  # NPM requires --
yarn create cloudflare [name] [flags]
pnpm create cloudflare@latest [name] [-- flags]
```

## Core Flags

| Flag | Values | Description |
| ------ | -------- | ------------- |
| `--type` | `hello-world`, `web-app`, `demo`, `pre-existing`, `remote-template` | Application type |
| `--platform` | `workers` (default), `pages` | Target platform |
| `--framework` | `next`, `remix`, `astro`, `react-router`, `solid`, `svelte`, `qwik`, `vue`, `angular`, `hono` | Web framework (requires `--type=web-app`) |
| `--lang` | `ts`, `js`, `python` | Language (for `--type=hello-world`) |
| `--ts` / `--no-ts` | - | TypeScript for web apps |

## Deployment Flags

| Flag | Description |
| ------ | ------------- |
| `--deploy` / `--no-deploy` | Deploy immediately (prompts interactive, skips in CI) |
| `--git` / `--no-git` | Initialize git (default: yes) |
| `--open` | Open browser after deploy |

## Advanced Flags

| Flag | Description |
| ------ | ------------- |
| `--template=user/repo` | GitHub template or local path |
| `--existing-script=./src/worker.ts` | Existing script (requires `--type=pre-existing`) |
| `--category=ai\ | database\ | realtime` | Demo filter (requires `--type=demo`) |
| `--experimental` | Enable experimental features |
| `--wrangler-defaults` | Skip wrangler prompts |

## Environment Variables

```bash
CLOUDFLARE_API_TOKEN=xxx    # For deployment
CLOUDFLARE_ACCOUNT_ID=xxx   # Account ID
CF_TELEMETRY_DISABLED=1     # Disable telemetry
```

## Exit Codes

`0` success, `1` user abort, `2` error

## Examples

```bash
# TypeScript Worker
npm create cloudflare@latest my-api -- --type=hello-world --lang=ts --no-deploy

# Next.js on Pages
npm create cloudflare@latest my-app -- --type=web-app --framework=next --platform=pages --ts

# Astro blog
npm create cloudflare@latest my-blog -- --type=web-app --framework=astro --ts --deploy

# CI: non-interactive
npm create cloudflare@latest my-app -- --type=web-app --framework=next --ts --no-git --no-deploy

# GitHub template
npm create cloudflare@latest -- --template=cloudflare/templates/worker-openapi

# Convert existing project
npm create cloudflare@latest . -- --type=pre-existing --existing-script=./build/worker.js
```

## When to use

Use when the user asks about or needs: C3 CLI Reference.
﻿---
name: C3 Generated Configuration
description: # C3 Generated Configuration
 
 ## Output Structure
---

# C3 Generated Configuration

## Output Structure (C3 Generated Configuration)

```text
my-app/
├── src/index.ts          # Worker entry point
├── wrangler.jsonc        # Cloudflare config
├── package.json          # Scripts
├── tsconfig.json
└── .gitignore
```

## wrangler.jsonc

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/cloudflare/workers-sdk/main/packages/wrangler/config-schema.json",
  "name": "my-app",
  "main": "src/index.ts",
  "compatibility_date": "2026-01-27"
}
```

## Binding Placeholders

C3 generates **placeholder IDs** that must be replaced before deploy:

```jsonc
{
  "kv_namespaces": [{ "binding": "MY_KV", "id": "placeholder_kv_id" }],
  "d1_databases": [{ "binding": "DB", "database_id": "00000000-..." }]
}
```

### Replace with real IDs

```bash
npx wrangler kv namespace create MY_KV   # Returns real ID
npx wrangler d1 create my-database       # Returns real database_id
```

#### Deployment error if not replaced

```text
Error: Invalid KV namespace ID "placeholder_kv_id"
```

## Scripts

```json
{
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "cf-typegen": "wrangler types"
  }
}
```

## Type Generation

Run after adding bindings:

```bash
npm run cf-typegen
```

Generates `.wrangler/types/runtime.d.ts`:

```typescript
interface Env {
  MY_KV: KVNamespace;
  DB: D1Database;
}
```

## Post-Creation Checklist

1. Review `wrangler.jsonc` - check name, compatibility_date
2. Replace placeholder binding IDs with real resource IDs
3. Run `npm run cf-typegen`
4. Test: `npm run dev`
5. Deploy: `npm run deploy`
6. Add secrets: `npx wrangler secret put SECRET_NAME`

## When to use

Use when the user asks about or needs: C3 Generated Configuration.
﻿---
name: C3 Troubleshooting
description: # C3 Troubleshooting
 
 ## Deployment Issues
---

# C3 Troubleshooting

## Deployment Issues (C3 Troubleshooting)

### Placeholder IDs

**Error:** "Invalid namespace ID"  
**Fix:** Replace placeholders in wrangler.jsonc with real IDs:

```bash
npx wrangler kv namespace create MY_KV  # Get real ID
```

### Authentication

**Error:** "Not authenticated"  
**Fix:** `npx wrangler login` or set `CLOUDFLARE_API_TOKEN`

### Name Conflict

**Error:** "Worker already exists"  
**Fix:** Change `name` in wrangler.jsonc

## Platform Selection

| Need | Platform |
| ------ | ---------- |
| Git integration, branch previews | `--platform=pages` |
| Durable Objects, D1, Queues | Workers (default) |

Wrong platform? Recreate with correct `--platform` flag.

## TypeScript Issues

### "Cannot find name 'KVNamespace'"

```bash
npm run cf-typegen  # Regenerate types
# Restart TS server in editor
```

**Missing types after config change:** Re-run `npm run cf-typegen`

## Package Manager

### Multiple lockfiles causing issues

```bash
rm pnpm-lock.yaml  # If using npm
rm package-lock.json  # If using pnpm
```

## CI/CD

### CI hangs on prompts

```bash
npm create cloudflare@latest my-app -- \
  --type=hello-world --lang=ts --no-git --no-deploy
```

#### Auth in CI

```yaml
env:
  CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
  CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

## Framework-Specific

| Framework | Issue | Fix |
| ----------- | ------- | ----- |
| Next.js | create-next-app failed | `npm cache clean --force`, retry |
| Astro | Adapter missing | Install `@astrojs/cloudflare` |
| Remix | Module errors | Update `@remix-run/cloudflare*` |

## Compatibility Date

### "Feature X requires compatibility_date >= ..."

**Fix:** Update `compatibility_date` in wrangler.jsonc to today's date

## Node.js Version

### "Node.js version not supported"

**Fix:** Install Node.js 18+ (`nvm install 20`)

## Quick Reference

| Error | Cause | Fix |
| ------- | ------- | ----- |
| Invalid namespace ID | Placeholder binding | Create resource, update config |
| Not authenticated | No login | `npx wrangler login` |
| Cannot find KVNamespace | Missing types | `npm run cf-typegen` |
| Worker already exists | Name conflict | Change `name` |
| CI hangs | Missing flags | Add --type, --lang, --no-deploy |
| Template not found | Bad name | Check cloudflare/templates |

## When to use

Use when the user asks about or needs: C3 Troubleshooting.
﻿---
name: C3 Usage Patterns
description: # C3 Usage Patterns
 
 ## Quick Workflows
---

# C3 Usage Patterns

## Quick Workflows (C3 Usage Patterns)

```bash
# TypeScript API Worker
npm create cloudflare@latest my-api -- --type=hello-world --lang=ts --deploy

# Next.js on Pages
npm create cloudflare@latest my-app -- --type=web-app --framework=next --platform=pages --ts --deploy

# Astro static site  
npm create cloudflare@latest my-blog -- --type=web-app --framework=astro --platform=pages --ts
```

## CI/CD (GitHub Actions)

```yaml
- name: Deploy
  run: npm run deploy
  env:
    CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
    CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

### Non-interactive requires

```bash
--type=<value>       # Required
--no-git             # Recommended (CI already in git)
--no-deploy          # Deploy separately with secrets
--framework=<value>  # For web-app
--ts / --no-ts       # Required
```

## Monorepo

C3 detects workspace config (`package.json` workspaces or `pnpm-workspace.yaml`).

```bash
cd packages/
npm create cloudflare@latest my-worker -- --type=hello-world --lang=ts --no-deploy
```

## Custom Templates

```bash
# GitHub repo
npm create cloudflare@latest -- --template=username/repo
npm create cloudflare@latest -- --template=cloudflare/templates/worker-openapi

# Local path
npm create cloudflare@latest my-app -- --template=../my-template
```

### Template requires `c3.config.json`

```json
{
  "name": "my-template",
  "category": "hello-world",
  "copies": [{ "path": "src/" }, { "path": "wrangler.jsonc" }],
  "transforms": [{ "path": "package.json", "jsonc": { "name": "{{projectName}}" }}]
}
```

## Existing Projects

```bash
# Add Cloudflare to existing Worker
npm create cloudflare@latest . -- --type=pre-existing --existing-script=./dist/index.js

# Add to existing framework app
npm create cloudflare@latest . -- --type=web-app --framework=next --platform=pages --ts
```

## Post-Creation Checklist

1. Review `wrangler.jsonc` - set `compatibility_date`, verify `name`
2. Create bindings: `wrangler kv namespace create`, `wrangler d1 create`, `wrangler r2 bucket create`
3. Generate types: `npm run cf-typegen`
4. Test: `npm run dev`
5. Deploy: `npm run deploy`
6. Set secrets: `wrangler secret put SECRET_NAME`

## When to use

Use when the user asks about or needs: C3 Usage Patterns.
﻿---
name: CNI Configuration
description: # CNI Configuration
 
 See [README.md](README.md) for overview.
---

# CNI Configuration

See [README.md](README.md) for overview.

## Workflow (2-4 weeks)

1. **Submit request** (Week 1): Contact account team, provide type/location/use case
2. **Review config** (Week 1-2, v1 only): Approve IP/VLAN/spec doc
3. **Order connection** (Week 2-3):
   - **Direct**: Get LOA, order cross-connect from facility
   - **Partner**: Order virtual circuit in partner portal
   - **Cloud**: Order Direct Connect/Cloud Interconnect, send LOA+VLAN to CF
4. **Configure** (Week 3): Both sides configure per doc
5. **Test** (Week 3-4): Ping, verify BGP, check routes
6. **Health checks** (Week 4): Configure [Magic Transit](https://developers.cloudflare.com/magic-transit/how-to/configure-tunnel-endpoints/#add-tunnels) or [Magic WAN](https://developers.cloudflare.com/magic-wan/configuration/manually/how-to/configure-tunnel-endpoints/#add-tunnels) health checks
7. **Activate** (Week 4): Route traffic, verify flow
8. **Monitor**: Enable [maintenance notifications](https://developers.cloudflare.com/network-interconnect/monitoring-and-alerts/#enable-cloudflare-status-maintenance-notification)

## BGP Configuration

### v1 Requirements

- BGP ASN (provide during setup)
- /31 subnet for peering
- Optional: BGP password

**v2:** Simplified, less BGP config needed.

**BGP over CNI (Dec 2024):** Magic WAN/Transit can now peer BGP directly over CNI v2 (no GRE tunnel required).

#### Example v1 BGP

```text
Router ID: 192.0.2.1
Peer IP: 192.0.2.0
Remote ASN: 13335
Local ASN: 65000
Password: [optional]
VLAN: 100
```

## Cloud Interconnect Setup

### AWS Direct Connect (Beta)

**Requirements:** Magic WAN, AWS Dedicated Direct Connect 1/10 Gbps.

#### Process

1. Contact CF account team
2. Choose location
3. Order in AWS portal
4. AWS provides LOA + VLAN ID
5. Send to CF account team
6. Wait ~4 weeks

**Post-setup:** Add [static routes](https://developers.cloudflare.com/magic-wan/configuration/manually/how-to/configure-routes/#configure-static-routes) to Magic WAN. Enable [bidirectional health checks](https://developers.cloudflare.com/magic-wan/configuration/manually/how-to/configure-tunnel-endpoints/#legacy-bidirectional-health-checks).

### GCP Cloud Interconnect (Beta)

#### Setup via Dashboard

1. Interconnects → Create → Cloud Interconnect → Google
2. Provide name, MTU (match GCP VLAN attachment), speed (50M-50G granular options available for partner interconnects)
3. Enter VLAN attachment pairing key
4. Confirm order

**Routing to GCP:** Add [static routes](https://developers.cloudflare.com/magic-wan/configuration/manually/how-to/configure-routes/#configure-static-routes). BGP routes from GCP Cloud Router **ignored**.

**Routing to CF:** Configure [custom learned routes](https://cloud.google.com/network-connectivity/docs/router/how-to/configure-custom-learned-routes) in Cloud Router. Request prefixes from CF account team.

## Monitoring

### Dashboard Status

| Status | Meaning |
| -------- | --------- |
| **Healthy** | Link operational, traffic flowing, health checks passing |
| **Active** | Link up, sufficient light, Ethernet negotiated |
| **Unhealthy** | Link down, no/low light (<-20 dBm), can't negotiate |
| **Pending** | Cross-connect incomplete, device unresponsive, RX/TX swapped |
| **Down** | Physical link down, no connectivity |

#### Alerts

**CNI Connection Maintenance** (Magic Networking only):

```text
Dashboard → Notifications → Add
Product: Cloudflare Network Interconnect
Type: Connection Maintenance Alert
```

Warnings up to 2 weeks advance. 6hr delay for new additions.

**Cloudflare Status Maintenance** (entire PoP):

```text
Dashboard → Notifications → Add
Product: Cloudflare Status
Filter PoPs: gru,fra,lhr
```

#### Find PoP code

```sql
Dashboard → Magic Transit/WAN → Configuration → Interconnects
Select CNI → Note Data Center (e.g., "gru-b")
Use first 3 letters: "gru"
```

## Best Practices

### Critical config-specific practices

- /31 subnets required for BGP
- BGP passwords recommended
- BFD for fast failover (v1 only)
- Test ping connectivity before BGP
- Enable maintenance notifications immediately after activation
- Monitor status programmatically via API

For design patterns, HA architecture, and security best practices, see [patterns.md](./patterns.md).

## When to use

Use when the user asks about or needs: CNI Configuration.
﻿---
name: CNI Patterns
description: # CNI Patterns
 
 See [README.md](README.md) for overview.
---

# CNI Patterns

See [README.md](README.md) for overview.

## High Availability

**Critical:** Design for resilience from day one.

### Requirements

- Device-level diversity (separate hardware)
- Backup Internet connectivity (no SLA on CNI)
- Network-resilient locations preferred
- Regular failover testing

#### Architecture

```text
Your Network A ──10G CNI v2──> CF CCR Device 1
                                     │
Your Network B ──10G CNI v2──> CF CCR Device 2
                                     │
                            CF Global Network (AS13335)
```

#### Capacity Planning

- Plan across all links
- Account for failover scenarios
- Your responsibility

## Pattern: Magic Transit + CNI v2

**Use Case:** DDoS protection, private connectivity, no GRE overhead.

```typescript
// 1. Create interconnect
const ic = await client.networkInterconnects.interconnects.create({
  account_id: id,
  type: 'direct',
  facility: 'EWR1',
  speed: '10G',
  name: 'magic-transit-primary',
});

// 2. Poll until active
const status = await pollUntilActive(id, ic.id);

// 3. Configure Magic Transit tunnel via Dashboard/API
```

**Benefits:** 1500 MTU both ways, simplified routing.

## Pattern: Multi-Cloud Hybrid

**Use Case:** AWS/GCP workloads with Cloudflare.

### AWS Direct Connect

```typescript
// 1. Order Direct Connect in AWS Console
// 2. Get LOA + VLAN from AWS
// 3. Send to CF account team (no API)
// 4. Configure static routes in Magic WAN

await configureStaticRoutes(id, {
  prefix: '10.0.0.0/8',
  nexthop: 'aws-direct-connect',
});
```

#### GCP Cloud Interconnect

```text
1. Get VLAN attachment pairing key from GCP Console
2. Create via Dashboard: Interconnects → Create → Cloud Interconnect → Google
   - Enter pairing key, name, MTU, speed
3. Configure static routes in Magic WAN (BGP routes from GCP ignored)
4. Configure custom learned routes in GCP Cloud Router
```

**Note:** Dashboard-only. No API/SDK support yet.

## Pattern: Multi-Location HA

**Use Case:** 99.99%+ uptime.

```typescript
// Primary (NY)
const primary = await client.networkInterconnects.interconnects.create({
  account_id: id,
  type: 'direct',
  facility: 'EWR1',
  speed: '10G',
  name: 'primary-ewr1',
});

// Secondary (NY, different hardware)
const secondary = await client.networkInterconnects.interconnects.create({
  account_id: id,
  type: 'direct',
  facility: 'EWR2',
  speed: '10G',
  name: 'secondary-ewr2',
});

// Tertiary (LA, different geography)
const tertiary = await client.networkInterconnects.interconnects.create({
  account_id: id,
  type: 'partner',
  facility: 'LAX1',
  speed: '10G',
  name: 'tertiary-lax1',
});

// BGP local preferences:
// Primary: 200
// Secondary: 150
// Tertiary: 100
// Internet: Last resort
```

## Pattern: Partner Interconnect (Equinix)

**Use Case:** Quick deployment, no colocation.

### Setup

1. Order virtual circuit in Equinix Fabric Portal
2. Select Cloudflare as destination
3. Choose facility
4. Send details to CF account team
5. CF accepts in portal
6. Configure BGP

**No API automation** – partner portals managed separately.

## Failover & Security

### Failover Best Practices

- Use BGP local preferences for priority
- Configure BFD for fast detection (v1)
- Test regularly with traffic shift
- Document runbooks

#### Security

- BGP password authentication
- BGP route filtering
- Monitor unexpected routes
- Magic Firewall for DDoS/threats
- Minimum API token permissions
- Rotate credentials periodically

## Decision Matrix

| Requirement | Recommended |
| ------------- | ------------- |
| Collocated with CF | Direct |
| Not collocated | Partner |
| AWS/GCP workloads | Cloud |
| 1500 MTU both ways | v2 |
| VLAN tagging | v1 |
| Public peering | v1 |
| Simplest config | v2 |
| BFD fast failover | v1 |
| LACP bundling | v1 |

## Resources

- [Magic Transit Docs](https://developers.cloudflare.com/magic-transit/)
- [Magic WAN Docs](https://developers.cloudflare.com/magic-wan/)
- [Argo Smart Routing](https://developers.cloudflare.com/argo/)

## When to use

Use when the user asks about or needs: CNI Patterns.
﻿---
name: Wrangler Common Issues
description: # Wrangler Common Issues
 
 ## Common Errors
---

# Wrangler Common Issues

## Common Errors (Wrangler Common Issues)

### "Binding ID vs name mismatch"

**Cause:** Confusion between binding name (code) and resource ID
**Solution:** Bindings use `binding` (code name) and `id`/`database_id`/`bucket_name` (resource ID). Preview bindings need separate IDs: `preview_id`, `preview_database_id`

### "Environment not inheriting config"

**Cause:** Non-inheritable keys not redefined per environment
**Solution:** Non-inheritable keys (bindings, vars) must be redefined per environment. Inheritable keys (routes, compatibility_date) can be overridden

### "Local dev behavior differs from production"

**Cause:** Using local simulation instead of remote execution
**Solution:** Choose appropriate remote mode:

- `wrangler dev` (default): Local simulation, fast, limited accuracy
- `wrangler dev --remote`: Full remote execution, production-accurate, slower
- Use `remote: "minimal"` in tests for fast tests with real remote bindings

### "startWorker doesn't match production"

**Cause:** Using local mode when remote resources needed
**Solution:** Use `remote` option:

```typescript
const worker = await startWorker({ 
  config: "wrangler.jsonc",
  remote: true  // or "minimal" for faster tests
});
```

### "Unexpected runtime changes"

**Cause:** Missing compatibility_date
**Solution:** Always set `compatibility_date`:

```jsonc
{ "compatibility_date": "2025-01-01" }
```

### "Durable Object binding not working"

**Cause:** Missing script_name for external DOs
**Solution:** Always specify `script_name` for external Durable Objects:

```jsonc
{
  "durable_objects": {
    "bindings": [
      { "name": "MY_DO", "class_name": "MyDO", "script_name": "my-worker" }
    ]
  }
}
```

For local DOs in same Worker, `script_name` is optional.

### "Auto-provisioned resources not appearing"

**Cause:** IDs written back to config on first deploy, but config not reloaded
**Solution:** After first deploy with auto-provisioning, config file is updated with IDs. Commit the updated config. On subsequent deploys, existing resources are reused.

### "Secrets not available in local dev"

**Cause:** Secrets set with `wrangler secret put` only work in deployed Workers
**Solution:** For local dev, use `.dev.vars`

### "Node.js compatibility error"

**Cause:** Missing Node.js compatibility flag
**Solution:** Some bindings (Hyperdrive with `pg`) require:

```jsonc
{ "compatibility_flags": ["nodejs_compat_v2"] }
```

### "Workers Assets 404 errors"

**Cause:** Asset path mismatch or incorrect `html_handling`

#### Solution

- Check `assets.directory` points to correct build output
- Set `html_handling: "auto-trailing-slash"` for SPAs
- Use `not_found_handling: "single-page-application"` to serve index.html for 404s

```jsonc
{
  "assets": {
    "directory": "./dist",
    "html_handling": "auto-trailing-slash",
    "not_found_handling": "single-page-application"
  }
}
```

### "Placement not reducing latency"

**Cause:** Misunderstanding of Smart Placement
**Solution:** Smart Placement only helps when Worker accesses D1 or Durable Objects. It doesn't affect KV, R2, or external API latency.

```jsonc
{ "placement": { "mode": "smart" } }  // Only beneficial with D1/DOs
```

### "unstable_startWorker not found"

**Cause:** Using outdated API
**Solution:** Use stable `startWorker` instead:

```typescript
import { startWorker } from "wrangler";  // Not unstable_startWorker
```

### "outboundService not mocking fetch"

**Cause:** Mock function not returning Response
**Solution:** Always return Response, use `fetch(req)` for passthrough:

```typescript
const worker = await startWorker({
  outboundService: (req) => {
    if (shouldMock(req)) {
      return new Response("mocked");
    }
    return fetch(req);  // Required for non-mocked requests
  }
});
```

## Limits

| Resource/Limit | Value | Notes |
| ---------------- | ------- | ------- |
| Bindings per Worker | 64 | Total across all types |
| Environments | Unlimited | Named envs in config |
| Config file size | ~1MB | Keep reasonable |
| Workers Assets size | 25 MB | Per deployment |
| Workers Assets files | 20,000 | Max number of files |
| Script size (compressed) | 1 MB | Free, 10 MB paid |
| CPU time | 10-50ms | Free, 50-500ms paid |
| Subrequest limit | 50 | Free, 1000 paid |

## Troubleshooting

### Authentication Issues

```bash
wrangler logout
wrangler login
wrangler whoami
```

### Configuration Errors

```bash
wrangler check  # Validate config
```

Use wrangler.jsonc with `$schema` for validation.

### Binding Not Available

- Check binding exists in config
- For environments, ensure binding defined for that env
- Local dev: some bindings need `--remote`

### Deployment Failures

```bash
wrangler tail              # Check logs
wrangler deploy --dry-run  # Validate
wrangler whoami            # Check account limits
```

### Local Development Issues

```bash
rm -rf .wrangler/state     # Clear local state
wrangler dev --remote      # Use remote bindings
wrangler dev --persist-to ./local-state  # Custom persist location
wrangler dev --inspector-port 9229  # Enable debugging
```

### Testing Issues

```bash
# If tests hang, ensure dispose() is called
worker.dispose()  // Always cleanup

# If bindings don't work in tests
const worker = await startWorker({ 
  config: "wrangler.jsonc",
  remote: "minimal"  // Use remote bindings
});
```

## Resources

- Docs: <https://developers.cloudflare.com/workers/wrangler/>
- Config: <https://developers.cloudflare.com/workers/wrangler/configuration/>
- Commands: <https://developers.cloudflare.com/workers/wrangler/commands/>
- Examples: <https://github.com/cloudflare/workers-sdk/tree/main/templates>
- Discord: <https://discord.gg/cloudflaredev>

## See Also

- [README.md](./README.md) - Commands
- [configuration.md](./configuration.md) - Config
- [api.md](./api.md) - Programmatic API
- [patterns.md](./patterns.md) - Workflows

## When to use

Use when the user asks about or needs: Wrangler Common Issues.
﻿---
name: Wrangler Configuration
description: # Wrangler Configuration
 
 Configuration reference for wrangler.jsonc (recommended).
---

# Wrangler Configuration

Configuration reference for wrangler.jsonc (recommended).

## Config Format

**wrangler.jsonc recommended** (v3.91.0+) - provides schema validation.

```jsonc
{
  "$schema": "./node_modules/wrangler/config-schema.json",
  "name": "my-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",  // Use current date
  "vars": { "API_KEY": "dev-key" },
  "kv_namespaces": [{ "binding": "MY_KV", "id": "abc123" }]
}
```

## Field Inheritance

Inheritable: `name`, `main`, `compatibility_date`, `routes`, `triggers`
Non-inheritable (define per env): `vars`, bindings (KV, D1, R2, etc.)

## Environments

```jsonc
{
  "name": "my-worker",
  "vars": { "ENV": "dev" },
  "env": {
    "production": {
      "name": "my-worker-prod",
      "vars": { "ENV": "prod" },
      "route": { "pattern": "example.com/*", "zone_name": "example.com" }
    }
  }
}
```

Deploy: `wrangler deploy --env production`

## Routing

```jsonc
// Custom domain (recommended)
{ "routes": [{ "pattern": "api.example.com", "custom_domain": true }] }

// Zone-based
{ "routes": [{ "pattern": "api.example.com/*", "zone_name": "example.com" }] }

// workers.dev
{ "workers_dev": true }
```

## Bindings

```jsonc
// Variables
{ "vars": { "API_URL": "https://api.example.com" } }

// KV
{ "kv_namespaces": [{ "binding": "CACHE", "id": "abc123" }] }

// D1
{ "d1_databases": [{ "binding": "DB", "database_id": "abc-123" }] }

// R2
{ "r2_buckets": [{ "binding": "ASSETS", "bucket_name": "my-assets" }] }

// Durable Objects
{ "durable_objects": { 
  "bindings": [{ 
    "name": "COUNTER", 
    "class_name": "Counter",
    "script_name": "my-worker"  // Required for external DOs
  }] 
} }
{ "migrations": [{ "tag": "v1", "new_sqlite_classes": ["Counter"] }] }

// Service Bindings
{ "services": [{ "binding": "AUTH", "service": "auth-worker" }] }

// Queues
{ "queues": {
  "producers": [{ "binding": "TASKS", "queue": "task-queue" }],
  "consumers": [{ "queue": "task-queue", "max_batch_size": 10 }]
} }

// Vectorize
{ "vectorize": [{ "binding": "VECTORS", "index_name": "embeddings" }] }

// Hyperdrive (requires nodejs_compat_v2 for pg/postgres)
{ "hyperdrive": [{ "binding": "HYPERDRIVE", "id": "hyper-id" }] }
{ "compatibility_flags": ["nodejs_compat_v2"] }  // For pg/postgres

// Workers AI
{ "ai": { "binding": "AI" } }

// Workflows
{ "workflows": [{ "binding": "WORKFLOW", "name": "my-workflow", "class_name": "MyWorkflow" }] }

// Secrets Store (centralized secrets)
{ "secrets_store": [{ "binding": "SECRETS", "id": "store-id" }] }

// Constellation (AI inference)
{ "constellation": [{ "binding": "MODEL", "project_id": "proj-id" }] }
```

## Workers Assets (Static Files)

Recommended for serving static files (replaces old `site` config).

```jsonc
{
  "assets": {
    "directory": "./public",
    "binding": "ASSETS",
    "html_handling": "auto-trailing-slash",  // or "none", "force-trailing-slash"
    "not_found_handling": "single-page-application"  // or "404-page", "none"
  }
}
```

Access in Worker:

```typescript
export default {
  async fetch(request, env) {
    // Try serving static asset first
    const asset = await env.ASSETS.fetch(request);
    if (asset.status !== 404) return asset;
    
    // Custom logic for non-assets
    return new Response("API response");
  }
}
```

## Placement

Control where Workers run geographically.

```jsonc
{
  "placement": {
    "mode": "smart"  // or "off"
  }
}
```

- `"smart"`: Run Worker near data sources (D1, Durable Objects) to reduce latency
- `"off"`: Default distribution (run everywhere)

## Auto-Provisioning (Beta)

Omit resource IDs - Wrangler creates them and writes back to config on deploy.

```jsonc
{ "kv_namespaces": [{ "binding": "MY_KV" }] }  // No id - auto-provisioned
```

After deploy, ID is added to config automatically.

## Advanced

```jsonc
// Cron Triggers
{ "triggers": { "crons": ["0 0 * * *"] } }

// Observability (tracing)
{ "observability": { "enabled": true, "head_sampling_rate": 0.1 } }

// Runtime Limits
{ "limits": { "cpu_ms": 100 } }

// Browser Rendering
{ "browser": { "binding": "BROWSER" } }

// mTLS Certificates
{ "mtls_certificates": [{ "binding": "CERT", "certificate_id": "cert-uuid" }] }

// Logpush (stream logs to R2/S3)
{ "logpush": true }

// Tail Consumers (process logs with another Worker)
{ "tail_consumers": [{ "service": "log-worker" }] }

// Unsafe bindings (access to arbitrary bindings)
{ "unsafe": { "bindings": [{ "name": "MY_BINDING", "type": "plain_text", "text": "value" }] } }
```

## See Also

- [README.md](./README.md) - Overview and commands
- [api.md](./api.md) - Programmatic API
- [patterns.md](./patterns.md) - Workflows
- [gotchas.md](./gotchas.md) - Common issues

## When to use

Use when the user asks about or needs: Wrangler Configuration.
﻿---
name: Wrangler Development Patterns
description: # Wrangler Development Patterns
 
 Common workflows and best practices.
---

# Wrangler Development Patterns

Common workflows and best practices.

## New Worker Project

```bash
wrangler init my-worker && cd my-worker
wrangler dev              # Develop locally
wrangler deploy           # Deploy
```

## Local Development

```bash
wrangler dev              # Local mode (fast, simulated)
wrangler dev --remote     # Remote mode (production-accurate)
wrangler dev --env staging --port 8787
wrangler dev --inspector-port 9229  # Enable debugging
```

Debug: chrome://inspect → Configure → localhost:9229

## Secrets

```bash
# Production
echo "secret-value" | wrangler secret put SECRET_KEY

# Local: use .dev.vars (gitignored)
# SECRET_KEY=local-dev-key
```

## Adding KV

```bash
wrangler kv namespace create MY_KV
wrangler kv namespace create MY_KV --preview
# Add to wrangler.jsonc: { "binding": "MY_KV", "id": "abc123" }
wrangler deploy
```

## Adding D1

```bash
wrangler d1 create my-db
wrangler d1 migrations create my-db "initial_schema"
# Edit migration file in migrations/, then:
wrangler d1 migrations apply my-db --local
wrangler deploy
wrangler d1 migrations apply my-db --remote

# Time Travel (restore to point in time)
wrangler d1 time-travel restore my-db --timestamp 2025-01-01T12:00:00Z
```

## Multi-Environment

```bash
wrangler deploy --env staging
wrangler deploy --env production
```

```jsonc
{ "env": { "staging": { "vars": { "ENV": "staging" } } } }
```

## Testing

### Integration Tests with Node.js Test Runner

```typescript
import { startWorker } from "wrangler";
import { describe, it, before, after } from "node:test";
import assert from "node:assert";

describe("API", () => {
  let worker;
  
  before(async () => {
    worker = await startWorker({ 
      config: "wrangler.jsonc",
      remote: "minimal"  // Fast tests with real bindings
    });
  });
  
  after(async () => await worker.dispose());
  
  it("creates user", async () => {
    const response = await worker.fetch("http://example.com/api/users", {
      method: "POST",
      body: JSON.stringify({ name: "Alice" })
    });
    assert.strictEqual(response.status, 201);
  });
});
```

### Testing with Vitest

Install: `npm install -D vitest @cloudflare/vitest-pool-workers`

#### vitest.config.ts

```typescript
import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";
export default defineWorkersConfig({
  test: { poolOptions: { workers: { wrangler: { configPath: "./wrangler.jsonc" } } } }
});
```

#### tests/api.test.ts

```typescript
import { env, SELF } from "cloudflare:test";
import { describe, it, expect } from "vitest";

it("fetches users", async () => {
  const response = await SELF.fetch("https://example.com/api/users");
  expect(response.status).toBe(200);
});

it("uses bindings", async () => {
  await env.MY_KV.put("key", "value");
  expect(await env.MY_KV.get("key")).toBe("value");
});
```

### Multi-Worker Development (Service Bindings)

```typescript
const authWorker = await startWorker({ config: "./auth/wrangler.jsonc" });
const apiWorker = await startWorker({
  config: "./api/wrangler.jsonc",
  bindings: { AUTH: authWorker }  // Service binding
});

// Test API calling AUTH
const response = await apiWorker.fetch("http://example.com/api/protected");
await authWorker.dispose();
await apiWorker.dispose();
```

### Mock External APIs

```typescript
const worker = await startWorker({ 
  config: "wrangler.jsonc",
  outboundService: (req) => {
    const url = new URL(req.url);
    if (url.hostname === "api.external.com") {
      return new Response(JSON.stringify({ mocked: true }), {
        headers: { "content-type": "application/json" }
      });
    }
    return fetch(req);  // Pass through other requests
  }
});

// Test Worker that calls external API
const response = await worker.fetch("http://example.com/proxy");
// Worker internally fetches api.external.com - gets mocked response
```

## Monitoring & Versions

```bash
wrangler tail                 # Real-time logs
wrangler tail --status error  # Filter errors
wrangler versions list
wrangler rollback [id]
```

## TypeScript

```bash
wrangler types  # Generate types from config
```

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return Response.json({ value: await env.MY_KV.get("key") });
  }
} satisfies ExportedHandler<Env>;
```

## Workers Assets

```jsonc
{ "assets": { "directory": "./dist", "binding": "ASSETS" } }
```

```typescript
export default {
  async fetch(request, env) {
    // API routes first
    if (new URL(request.url).pathname.startsWith("/api/")) {
      return Response.json({ data: "from API" });
    }
    return env.ASSETS.fetch(request);  // Static assets
  }
}
```

## See Also

- [README.md](./README.md) - Commands
- [configuration.md](./configuration.md) - Config
- [api.md](./api.md) - Programmatic API
- [gotchas.md](./gotchas.md) - Issues

## When to use

Use when the user asks about or needs: Wrangler Development Patterns.
﻿---
name: Zaraz Configuration
description: # Zaraz Configuration
 
 ## Dashboard Setup
---

# Zaraz Configuration

## Dashboard Setup (Zaraz Configuration)

1. Domain → Zaraz → Start setup
2. Add tool (e.g., Google Analytics 4)
3. Enter credentials (GA4: `G-XXXXXXXXXX`)
4. Configure triggers
5. Save and Publish

## Triggers

| Type | When | Use Case |
| ------ | ------ | ---------- |
| Pageview | Page load | Track page views |
| Click | Element clicked | Button tracking |
| Form Submission | Form submitted | Lead capture |
| History Change | URL changes (SPA) | React/Vue routing |
| Variable Match | Custom condition | Conditional firing |

### History Change (SPA)

```text
Type: History Change
Event: pageview
```

Fires on `pushState`, `replaceState`, hash changes. **No manual tracking needed.**

### Click Trigger

```text
Type: Click
CSS Selector: .buy-button
Event: purchase_intent
Properties:
  button_text: {{system.clickElement.text}}
```

## Tool Configuration

### GA4

```text
Measurement ID: G-XXXXXXXXXX
Events: page_view, purchase, user_engagement
```

#### Facebook Pixel

```text
Pixel ID: 1234567890123456
Events: PageView, Purchase, AddToCart
```

#### Google Ads

```text
Conversion ID: AW-XXXXXXXXX
Conversion Label: YYYYYYYYYY
```

## Consent Management

1. Settings → Consent → Create purposes (analytics, marketing)
2. Map tools to purposes
3. Set behavior: "Do not load until consent granted"

### Programmatic consent

```javascript
zaraz.consent.setAll({ analytics: true, marketing: true });
```

## Privacy Features

| Feature | Default |
| --------- | --------- |
| IP Anonymization | Enabled |
| Cookie Control | Via consent purposes |
| GDPR/CCPA | Consent modal |

## Testing

1. **Preview Mode** - test without publishing
2. **Debug Mode** - `zaraz.debug = true`
3. **Network tab** - filter "zaraz"

## Limits

| Resource | Limit |
| ---------- | ------- |
| Event properties | 100KB |
| Consent purposes | 20 |

## When to use

Use when the user asks about or needs: Zaraz Configuration.
﻿---
name: Zaraz Patterns
description: # Zaraz Patterns
 
 ## SPA Tracking
---

# Zaraz Patterns

## SPA Tracking (Zaraz Patterns)

**History Change Trigger (Recommended):** Configure in dashboard - no code needed, Zaraz auto-detects route changes.

### Manual tracking (React/Vue/Next.js)

```javascript
// On route change
zaraz.track('pageview', { page_path: pathname, page_title: document.title });
```

## User Identification

```javascript
// Login
zaraz.set({ userId: user.id, email: user.email, plan: user.plan });
zaraz.track('login', { method: 'oauth' });

// Logout - set to null (cannot clear)
zaraz.set('userId', null);
```

## E-commerce Funnel

| Event | Method |
| ------- | -------- |
| View | `zaraz.ecommerce('Product Viewed', { product_id, name, price })` |
| Add to cart | `zaraz.ecommerce('Product Added', { product_id, quantity })` |
| Checkout | `zaraz.ecommerce('Checkout Started', { cart_id, products: [...] })` |
| Purchase | `zaraz.ecommerce('Order Completed', { order_id, total, products })` |

## A/B Testing

```javascript
zaraz.set('experiment_checkout', variant);
zaraz.track('experiment_viewed', { experiment_id: 'checkout', variant });
// On conversion
zaraz.track('experiment_conversion', { experiment_id, variant, value });
```

## Worker Integration

**Context Enricher** - Modify context before tools execute:

```typescript
export default {
  async fetch(request, env) {
    const body = await request.json();
    body.system.userRegion = request.cf?.region;
    return Response.json(body);
  }
};
```

Configure: Zaraz > Settings > Context Enrichers

**Worker Variables** - Compute dynamic values server-side, use as `{{worker.variable_name}}`.

## GTM Migration

| GTM | Zaraz |
| ----- | ------- |
| `dataLayer.push({event: 'purchase'})` | `zaraz.ecommerce('Order Completed', {...})` |
| `{{Page URL}}` | `{{system.page.url}}` |
| `{{Page Title}}` | `{{system.page.title}}` |
| Page View trigger | Pageview trigger |
| Click trigger | Click (selector: `*`) |

## Best Practices

1. Use dashboard triggers over inline code
2. Enable History Change for SPAs (no manual code)
3. Debug with `zaraz.debug = true`
4. Implement consent early (GDPR/CCPA)
5. Use Context Enrichers for sensitive/server data

## When to use

Use when the user asks about or needs: Zaraz Patterns.
﻿---
name: Zaraz Reference Implementation Summary
description: # Zaraz Reference Implementation Summary
 
 ## Files Created
---

# Zaraz Reference Implementation Summary

## Files Created (Zaraz Reference Implementation Summary)

| File | Lines | Purpose |
| ------ | ------- | --------- |
| README.md | 111 | Navigation, decision tree, quick start |
| api.md | 287 | Web API reference, Zaraz Context |
| configuration.md | 307 | Dashboard setup, triggers, tools, consent |
| patterns.md | 430 | SPA, e-commerce, Worker integration |
| gotchas.md | 317 | Troubleshooting, limits, tool-specific issues |
| **Total** | **1,452** | **vs 366 original** |

## Key Improvements Applied

### Structure

- ✅ Created 5-file progressive disclosure system
- ✅ Added navigation table in README
- ✅ Added decision tree for routing
- ✅ Added "Reading Order by Task" guide
- ✅ Cross-referenced files throughout

### New Content Added

- ✅ Zaraz Context (system/client properties)
- ✅ History Change trigger for SPA tracking
- ✅ Context Enrichers pattern
- ✅ Worker Variables pattern
- ✅ Consent management deep dive
- ✅ Tool-specific quirks (GA4, Facebook, Google Ads)
- ✅ GTM migration guide
- ✅ Comprehensive troubleshooting
- ✅ "When NOT to use Zaraz" section
- ✅ TypeScript type definitions

### Preserved Content

- ✅ All original API methods
- ✅ E-commerce tracking examples
- ✅ Consent management
- ✅ Workers integration (expanded)
- ✅ Common patterns (expanded)
- ✅ Debugging tools
- ✅ Reference links

## Progressive Disclosure Impact

### Before (Monolithic)

All tasks loaded 366 lines regardless of need.

### After (Progressive)

- **Track event task**: README (111) + api.md (287) = 398 lines
- **Debug issue**: gotchas.md (317) = 317 lines (13% reduction)
- **Configure tool**: configuration.md (307) = 307 lines (16% reduction)
- **SPA tracking**: README + patterns.md (SPA section) ~180 lines (51% reduction)

**Net effect:** Task-specific loading reduces unnecessary content by 13-51% depending on use case.

## File Summary

### README.md (111 lines)

- Overview and core concepts
- Quick start guide
- When to use Zaraz vs Workers
- Navigation table
- Reading order by task
- Decision tree

### api.md (287 lines)

- zaraz.track()
- zaraz.set()
- zaraz.ecommerce()
- Zaraz Context (system/client properties)
- zaraz.consent API
- zaraz.debug
- Cookie methods
- TypeScript definitions

### configuration.md (307 lines)

- Dashboard setup flow
- Trigger types (including History Change)
- Tool configuration (GA4, Facebook, Google Ads)
- Actions and action rules
- Selective loading
- Consent management setup
- Privacy features
- Testing workflow

### patterns.md (430 lines)

- SPA tracking (React, Vue, Next.js)
- User identification flows
- Complete e-commerce funnel
- A/B testing
- Worker integration (Context Enrichers, Worker Variables, HTML injection)
- Multi-tool coordination
- GTM migration
- Best practices

### gotchas.md (317 lines)

- Events not firing (5-step debug process)
- Consent issues
- SPA tracking pitfalls
- Performance issues
- Tool-specific quirks
- Data layer issues
- Limits table
- When NOT to use Zaraz
- Debug checklist

## Quality Metrics

- ✅ All files use consistent markdown formatting
- ✅ Code examples include language tags
- ✅ Tables for structured data (limits, parameters, comparisons)
- ✅ Problem → Cause → Solution format in gotchas
- ✅ Cross-references between files
- ✅ No "see documentation" placeholders
- ✅ Real, actionable examples throughout
- ✅ Verified API syntax for Workers

## Original Backup

Original SKILL.md preserved as `_SKILL_old.md` for reference.

## When to use

Use when the user asks about or needs: Zaraz Reference Implementation Summary.
﻿---
name: CNI API Reference
description: # CNI API Reference
 
 See [README.md](README.md) for overview.
---

# CNI API Reference

See [README.md](README.md) for overview.

## Base

```yaml
https://api.cloudflare.com/client/v4
Auth: Authorization: Bearer <token>
```

## SDK Namespaces

### Primary (recommended)

```typescript
client.networkInterconnects.interconnects.*
client.networkInterconnects.cnis.*
client.networkInterconnects.slots.*
```

#### Alternate (deprecated)

```typescript
client.magicTransit.cfInterconnects.*
```

Use `networkInterconnects` namespace for all new code.

## Interconnects

```http
GET    /accounts/{account_id}/cni/interconnects              # Query: page, per_page
POST   /accounts/{account_id}/cni/interconnects              # Query: validate_only=true (optional)
GET    /accounts/{account_id}/cni/interconnects/{icon}
GET    /accounts/{account_id}/cni/interconnects/{icon}/status
GET    /accounts/{account_id}/cni/interconnects/{icon}/loa   # Returns PDF
DELETE /accounts/{account_id}/cni/interconnects/{icon}
```

**Create Body:** `account`, `slot_id`, `type`, `facility`, `speed`, `name`, `description`  
**Status Values:** `active` | `healthy` | `unhealthy` | `pending` | `down`

### Response Example

```json
{"result": [{"id": "icon_abc", "name": "prod", "type": "direct", "facility": "EWR1", "speed": "10G", "status": "active"}]}
```

## CNI Objects (BGP config)

```http
GET    /accounts/{account_id}/cni/cnis
POST   /accounts/{account_id}/cni/cnis
GET    /accounts/{account_id}/cni/cnis/{cni}
PUT    /accounts/{account_id}/cni/cnis/{cni}
DELETE /accounts/{account_id}/cni/cnis/{cni}
```

Body: `account`, `cust_ip`, `cf_ip`, `bgp_asn`, `bgp_password`, `vlan`

## Slots

```http
GET /accounts/{account_id}/cni/slots
GET /accounts/{account_id}/cni/slots/{slot}
```

Query: `facility`, `occupied`, `speed`

## Health Checks

Configure via Magic Transit/WAN tunnel endpoints (CNI v2).

```typescript
await client.magicTransit.tunnels.update(accountId, tunnelId, {
  health_check: { enabled: true, target: '192.0.2.1', rate: 'high', type: 'request' },
});
```

Rates: `high` | `medium` | `low`. Types: `request` | `reply`. See [Magic Transit docs](https://developers.cloudflare.com/magic-transit/how-to/configure-tunnel-endpoints/#add-tunnels).

## Settings

```http
GET /accounts/{account_id}/cni/settings
PUT /accounts/{account_id}/cni/settings
```

Body: `default_asn`

## TypeScript SDK

```typescript
import Cloudflare from 'cloudflare';

const client = new Cloudflare({ apiToken: process.env.CF_TOKEN });

// List
await client.networkInterconnects.interconnects.list({ account_id: id });

// Create with validation
await client.networkInterconnects.interconnects.create({
  account_id: id,
  account: id,
  slot_id: 'slot_abc',
  type: 'direct',
  facility: 'EWR1',
  speed: '10G',
  name: 'prod-interconnect',
}, {
  query: { validate_only: true }, // Dry-run validation
});

// Create without validation
await client.networkInterconnects.interconnects.create({
  account_id: id,
  account: id,
  slot_id: 'slot_abc',
  type: 'direct',
  facility: 'EWR1',
  speed: '10G',
  name: 'prod-interconnect',
});

// Status
await client.networkInterconnects.interconnects.get(accountId, iconId);

// LOA (use fetch)
const res = await fetch(`https://api.cloudflare.com/client/v4/accounts/${id}/cni/interconnects/${iconId}/loa`, {
  headers: { Authorization: `Bearer ${token}` },
});
await fs.writeFile('loa.pdf', Buffer.from(await res.arrayBuffer()));

// CNI object
await client.networkInterconnects.cnis.create({
  account_id: id,
  account: id,
  cust_ip: '192.0.2.1/31',
  cf_ip: '192.0.2.0/31',
  bgp_asn: 65000,
  vlan: 100,
});

// Slots (filter by facility and speed)
await client.networkInterconnects.slots.list({
  account_id: id,
  occupied: false,
  facility: 'EWR1',
  speed: '10G',
});
```

## Python SDK

```python
from cloudflare import Cloudflare

client = Cloudflare(api_token=os.environ["CF_TOKEN"])

# List, create, status (same pattern as TypeScript)
client.network_interconnects.interconnects.list(account_id=id)
client.network_interconnects.interconnects.create(account_id=id, account=id, slot_id="slot_abc", type="direct", facility="EWR1", speed="10G")
client.network_interconnects.interconnects.get(account_id=id, icon=icon_id)

# CNI objects and slots
client.network_interconnects.cnis.create(account_id=id, cust_ip="192.0.2.1/31", cf_ip="192.0.2.0/31", bgp_asn=65000)
client.network_interconnects.slots.list(account_id=id, occupied=False)
```
