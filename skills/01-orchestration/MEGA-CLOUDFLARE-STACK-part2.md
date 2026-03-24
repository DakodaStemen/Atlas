---
name: "C3 CLI Reference (Part 2)"
description: "# C3 CLI Reference - Part 2"
---


## cURL

```bash
# List interconnects
curl "https://api.cloudflare.com/client/v4/accounts/${ACCOUNT_ID}/cni/interconnects" \
  -H "Authorization: Bearer ${CF_TOKEN}"

# Create interconnect
curl -X POST "https://api.cloudflare.com/client/v4/accounts/${ACCOUNT_ID}/cni/interconnects?validate_only=true" \
  -H "Authorization: Bearer ${CF_TOKEN}" -H "Content-Type: application/json" \
  -d '{"account": "id", "slot_id": "slot_abc", "type": "direct", "facility": "EWR1", "speed": "10G"}'

# LOA PDF
curl "https://api.cloudflare.com/client/v4/accounts/${ACCOUNT_ID}/cni/interconnects/${ICON_ID}/loa" \
  -H "Authorization: Bearer ${CF_TOKEN}" --output loa.pdf
```

## Not Available via API

### Missing Capabilities

- BGP session state query (use Dashboard or BGP logs)
- Bandwidth utilization metrics (use external monitoring)
- Traffic statistics per interconnect
- Historical uptime/downtime data
- Light level readings (contact account team)
- Maintenance window scheduling (notifications only)

## Resources

- [API Docs](https://developers.cloudflare.com/api/resources/network_interconnects/)
- [TypeScript SDK](https://github.com/cloudflare/cloudflare-typescript)
- [Python SDK](https://github.com/cloudflare/cloudflare-python)

## When to use

Use when the user asks about or needs: CNI API Reference.
﻿---
name: CNI Gotchas & Troubleshooting
description: # CNI Gotchas & Troubleshooting
 
 ## Common Errors
---

# CNI Gotchas & Troubleshooting

## Common Errors (CNI Gotchas & Troubleshooting)

### "Status: Pending"

**Cause:** Cross-connect not installed, RX/TX fibers reversed, wrong fiber type, or low light levels

#### Solution

1. Verify cross-connect installed
2. Check fiber at patch panel
3. Swap RX/TX fibers
4. Check light with optical power meter (target > -20 dBm)
5. Contact account team

### "Status: Unhealthy"

**Cause:** Physical issue, low light (<-20 dBm), optic mismatch, or dirty connectors

#### Solution ("Status: Unhealthy")

1. Check physical connections
2. Clean fiber connectors
3. Verify optic types (10GBASE-LR/100GBASE-LR4)
4. Test with known-good optics
5. Check patch panel
6. Contact account team

### "BGP Session Down"

**Cause:** Wrong IP addressing, wrong ASN, password mismatch, or firewall blocking TCP/179

#### Solution ("BGP Session Down")

1. Verify IPs match CNI object
2. Confirm ASN correct
3. Check BGP password
4. Verify no firewall on TCP/179
5. Check BGP logs
6. Review BGP timers

### "Low Throughput"

**Cause:** MTU mismatch, fragmentation, single GRE tunnel (v1), or routing inefficiency

#### Solution ("Low Throughput")

1. Check MTU (1500↓/1476↑ for v1, 1500 both for v2)
2. Test various packet sizes
3. Add more GRE tunnels (v1)
4. Consider upgrading to v2
5. Review routing tables
6. Use LACP for bundling (v1)

## API Errors

### 400 Bad Request: "slot_id already occupied"

**Cause:** Another interconnect already uses this slot  
**Solution:** Use `occupied=false` filter when listing slots:

```typescript
await client.networkInterconnects.slots.list({
  account_id: id,
  occupied: false,
  facility: 'EWR1',
});
```

### 400 Bad Request: "invalid facility code"

**Cause:** Typo or unsupported facility  
**Solution:** Check [locations PDF](https://developers.cloudflare.com/network-interconnect/static/cni-locations-2026-01.pdf) for valid codes

### 403 Forbidden: "Enterprise plan required"

**Cause:** Account not enterprise-level  
**Solution:** Contact account team to upgrade

### 422 Unprocessable: "validate_only request failed"

**Cause:** Dry-run validation found issues (wrong slot, invalid config)  
**Solution:** Review error message details, fix config before real creation

### Rate Limiting

**Limit:** 1200 requests/5min per token  
**Solution:** Implement exponential backoff, cache slot listings

## Cloud-Specific Issues

### AWS Direct Connect: "VLAN not matching"

**Cause:** VLAN ID from AWS LOA doesn't match CNI config  

#### Solution (AWS Direct Connect: "VLAN not matching")

1. Get VLAN from AWS Console after ordering
2. Send exact VLAN to CF account team
3. Verify match in CNI object config

### AWS: "Connection stuck in Pending"

**Cause:** LOA not provided to CF or AWS connection not accepted  

#### Solution (AWS: "Connection stuck in Pending")

1. Verify AWS connection status is "Available"
2. Confirm LOA sent to CF account team
3. Wait for CF team acceptance (can take days)

### GCP: "BGP routes not propagating"

#### Cause:**BGP routes from GCP Cloud Router**ignored by design

**Solution:** Use [static routes](https://developers.cloudflare.com/magic-wan/configuration/manually/how-to/configure-routes/#configure-static-routes) in Magic WAN instead

### GCP: "Cannot query VLAN attachment status via API"

**Cause:** GCP Cloud Interconnect Dashboard-only (no API yet)  
**Solution:** Check status in CF Dashboard or GCP Console

## Partner Interconnect Issues

### Equinix: "Virtual circuit not appearing"

**Cause:** CF hasn't accepted Equinix connection request  

#### Solution (Equinix: "Virtual circuit not appearing")

1. Verify VC created in Equinix Fabric Portal
2. Contact CF account team to accept
3. Allow 2-3 business days

### Console Connect/Megaport: "API creation fails"

**Cause:** Partner interconnects require partner portal + CF approval  
**Solution:** Cannot fully automate. Order in partner portal, notify CF account team.

## Anti-Patterns

| Anti-Pattern | Why Bad | Solution |
| -------------- | --------- | ---------- |
| Single interconnect for production | No SLA, single point of failure | Use ≥2 with device diversity |
| No backup Internet | CNI fails = total outage | Always maintain alternate path |
| Polling status every second | Rate limits, wastes API calls | Poll every 30-60s max |
| Using v1 for Magic WAN v2 workloads | GRE overhead, complexity | Use v2 for simplified routing |
| Assuming BGP session = traffic flowing | BGP up ≠ routes installed | Verify routing tables + test traffic |
| Not enabling maintenance alerts | Surprise downtime during maintenance | Enable notifications immediately |
| Hardcoding VLAN in automation | VLAN assigned by CF (v1) | Get VLAN from CNI object response |
| Using Direct without colocation | Can't access cross-connect | Use Partner or Cloud interconnect |

## What's Not Queryable via API

### Cannot retrieve

- BGP session state (use Dashboard or BGP logs)
- Light levels (contact account team)
- Historical metrics (uptime, traffic)
- Bandwidth utilization per interconnect
- Maintenance window schedules (notifications only)
- Fiber path details
- Cross-connect installation status

#### Workarounds

- External monitoring for BGP state
- Log aggregation for historical data
- Notifications for maintenance windows

## Limits

| Resource/Limit | Value | Notes |
| ---------------- | ------- | ------- |
| Max optical distance | 10km | Physical limit |
| MTU (v1) | 1500↓ / 1476↑ | Asymmetric |
| MTU (v2) | 1500 both | Symmetric |
| GRE tunnel throughput | 1 Gbps | Per tunnel (v1) |
| Recovery time | Days | No formal SLA |
| Light level minimum | -20 dBm | Target threshold |
| API rate limit | 1200 req/5min | Per token |
| Health check delay | 6 hours | New maintenance alert subscriptions |

## When to use

Use when the user asks about or needs: CNI Gotchas & Troubleshooting.


---

<!-- merged from: analytics-engine-configuration.md -->

﻿---
name: Analytics Engine Configuration
description: # Analytics Engine Configuration
 
 ## Setup
---

# Analytics Engine Configuration

## Setup (Analytics Engine Configuration)

1. Add binding to `wrangler.jsonc`
2. Deploy Worker
3. Dataset created automatically on first write
4. Query via SQL API

## wrangler.jsonc

```jsonc
{
  "name": "my-worker",
  "analytics_engine_datasets": [
    { "binding": "ANALYTICS", "dataset": "my_events" }
  ]
}
```

Multiple datasets for separate concerns:

```jsonc
{
  "analytics_engine_datasets": [
    { "binding": "API_ANALYTICS", "dataset": "api_requests" },
    { "binding": "USER_EVENTS", "dataset": "user_activity" }
  ]
}
```

## TypeScript

```typescript
interface Env {
  ANALYTICS: AnalyticsEngineDataset;
}

export default {
  async fetch(request: Request, env: Env) {
    // No await - returns void, fire-and-forget
    env.ANALYTICS.writeDataPoint({
      blobs: [pathname, method, status],      // String dimensions (max 20)
      doubles: [latency, 1],                   // Numeric metrics (max 20)
      indexes: [apiKey]                        // High-cardinality filter (max 1)
    });
    return response;
  }
};
```

## Data Point Limits

| Field | Limit | SQL Access |
| ------- | ------- | ------------ |
| blobs | 20 strings, 16KB each | `blob1`...`blob20` |
| doubles | 20 numbers | `double1`...`double20` |
| indexes | 1 string, 16KB | `index1` |

## Write Behavior

| Scenario | Behavior |
| ---------- | ---------- |
| <1M writes/min | All accepted |
| >1M writes/min | Automatic sampling |
| Invalid data | Silent failure (check tail logs) |

**Mitigate sampling:** Pre-aggregate, use multiple datasets, write only critical metrics.

## Query Limits

| Resource | Limit |
| ---------- | ------- |
| Query timeout | 30 seconds |
| Data retention | 90 days (default) |
| Result size | ~10MB |

## Cost

**Free tier:** 10M writes/month, 1M reads/month

**Paid:** $0.05 per 1M writes, $1.00 per 1M reads

## Environment-Specific

```jsonc
{
  "analytics_engine_datasets": [
    { "binding": "ANALYTICS", "dataset": "prod_events" }
  ],
  "env": {
    "staging": {
      "analytics_engine_datasets": [
        { "binding": "ANALYTICS", "dataset": "staging_events" }
      ]
    }
  }
}
```

## Monitoring

```bash
npx wrangler tail  # Check for sampling/write errors
```

```sql
-- Check write activity
SELECT DATE_TRUNC('hour', timestamp) AS hour, COUNT(*) AS writes
FROM my_dataset
WHERE timestamp >= NOW() - INTERVAL '24' HOUR
GROUP BY hour
```


---

<!-- merged from: analytics-engine-patterns.md -->

﻿---
name: Analytics Engine Patterns
description: # Analytics Engine Patterns
 
 ## Use Cases
---

# Analytics Engine Patterns

## Use Cases (Analytics Engine Patterns)

| Use Case | Key Metrics | Index On |
| ---------- | ------------- | ---------- |
| API Metering | requests, bytes, compute_units | api_key |
| Feature Usage | feature, action, duration | user_id |
| Error Tracking | error_type, endpoint, count | customer_id |
| Performance | latency_ms, cache_status | endpoint |
| A/B Testing | variant, conversions | user_id |

## API Metering (Billing)

```typescript
env.ANALYTICS.writeDataPoint({
  blobs: [pathname, method, status, tier],
  doubles: [1, computeUnits, bytes, latencyMs],
  indexes: [apiKey]
});

// Query: Monthly usage by customer
// SELECT index1 AS api_key, SUM(double2) AS compute_units
// FROM usage WHERE timestamp >= DATE_TRUNC('month', NOW()) GROUP BY index1
```

## Error Tracking

```typescript
env.ANALYTICS.writeDataPoint({
  blobs: [endpoint, method, errorName, errorMessage.slice(0, 1000)],
  doubles: [1, timeToErrorMs],
  indexes: [customerId]
});
```

## Performance Monitoring

```typescript
env.ANALYTICS.writeDataPoint({
  blobs: [pathname, method, cacheStatus, status],
  doubles: [latencyMs, 1],
  indexes: [userId]
});

// Query: P95 latency by endpoint
// SELECT blob1, quantile(0.95)(double1) AS p95_ms FROM perf GROUP BY blob1
```

## Anti-Patterns

| ❌ Wrong | ✅ Correct |
| ---------- | ----------- |
| `await writeDataPoint()` | `writeDataPoint()` (fire-and-forget) |
| `indexes: [method]` (low cardinality) | `blobs: [method]`, `indexes: [userId]` |
| `blobs: [JSON.stringify(obj)]` | Store ID in blob, full object in D1/KV |
| Write every request at 10M/min | Pre-aggregate per second |
| Query from Worker | Query from external service/API |

## Best Practices

1. **Design schema upfront** - Document blob/double/index assignments
2. **Always include count metric** - `doubles: [latency, 1]` for AVG calculations
3. **Use enums for blobs** - Consistent values like `Status.SUCCESS`
4. **Handle sampling** - Use ratios (avg_latency = SUM(latency)/SUM(count))
5. **Test queries early** - Validate schema before heavy writes

## Schema Template

```typescript
/**
 * Dataset: my_metrics
 * 
 * Blobs:
 *   blob1: endpoint, blob2: method, blob3: status
 * 
 * Doubles:
 *   double1: latency_ms, double2: count (always 1)
 * 
 * Indexes:
 *   index1: customer_id (high cardinality)
 */
```


---

<!-- merged from: cache-reserve-configuration.md -->

﻿---
name: Cache Reserve Configuration
description: # Cache Reserve Configuration
 
 ## Dashboard Setup
---

# Cache Reserve Configuration

## Dashboard Setup (Cache Reserve Configuration)

### Minimum steps to enable

```bash
# Navigate to dashboard
https://dash.cloudflare.com/caching/cache-reserve

# Click "Enable Storage Sync" or "Purchase" button
```

#### Prerequisites

- Paid Cache Reserve plan or Smart Shield Advanced required
- Tiered Cache **required** for Cache Reserve to function optimally

## API Configuration

### REST API

```bash
# Enable
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/cache/cache_reserve" \
  -H "Authorization: Bearer $API_TOKEN" -H "Content-Type: application/json" \
  -d '{"value": "on"}'

# Check status
curl -X GET "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/cache/cache_reserve" \
  -H "Authorization: Bearer $API_TOKEN"
```

### TypeScript SDK

```bash
npm install cloudflare
```

```typescript
import Cloudflare from 'cloudflare';

const client = new Cloudflare({
  apiToken: process.env.CLOUDFLARE_API_TOKEN,
});

// Enable Cache Reserve
await client.cache.cacheReserve.edit({
  zone_id: 'abc123',
  value: 'on',
});

// Get Cache Reserve status
const status = await client.cache.cacheReserve.get({
  zone_id: 'abc123',
});
console.log(status.value); // 'on' or 'off'
```

### Python SDK

```bash
pip install cloudflare
```

```python
from cloudflare import Cloudflare

client = Cloudflare(api_token=os.environ.get("CLOUDFLARE_API_TOKEN"))

# Enable Cache Reserve
client.cache.cache_reserve.edit(
    zone_id="abc123",
    value="on"
)

# Get Cache Reserve status
status = client.cache.cache_reserve.get(zone_id="abc123")
print(status.value)  # 'on' or 'off'
```

### Terraform

```hcl
terraform {
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

resource "cloudflare_zone_cache_reserve" "example" {
  zone_id = var.zone_id
  enabled = true
}

# Tiered Cache is required for Cache Reserve
resource "cloudflare_tiered_cache" "example" {
  zone_id    = var.zone_id
  cache_type = "smart"
}
```

### Pulumi

```typescript
import * as cloudflare from "@pulumi/cloudflare";

// Enable Cache Reserve
const cacheReserve = new cloudflare.ZoneCacheReserve("example", {
  zoneId: zoneId,
  enabled: true,
});

// Enable Tiered Cache (required)
const tieredCache = new cloudflare.TieredCache("example", {
  zoneId: zoneId,
  cacheType: "smart",
});
```

### Required API Token Permissions

- `Zone Settings Read`
- `Zone Settings Write`
- `Zone Read`
- `Zone Write`

## Cache Rules Integration

Control Cache Reserve eligibility via Cache Rules:

```typescript
// Enable for static assets
{
  action: 'set_cache_settings',
  action_parameters: {
    cache_reserve: { eligible: true, minimum_file_ttl: 86400 },
    edge_ttl: { mode: 'override_origin', default: 86400 },
    cache: true
  },
  expression: '(http.request.uri.path matches "\\.(jpg|png|webp|pdf|zip)$")'
}

// Disable for APIs
{
  action: 'set_cache_settings',
  action_parameters: { cache_reserve: { eligible: false } },
  expression: '(http.request.uri.path matches "^/api/")'
}

// Create via API: PUT to zones/{zone_id}/rulesets/phases/http_request_cache_settings/entrypoint
```

## Wrangler Integration

Cache Reserve works automatically with Workers deployed via Wrangler. No special wrangler.jsonc configuration needed - enable Cache Reserve via Dashboard or API for the zone.

## See Also

- [README](./README.md) - Overview and core concepts
- [API Reference](./api.md) - Purging and monitoring APIs
- [Patterns](./patterns.md) - Best practices and optimization
- [Gotchas](./gotchas.md) - Common issues and troubleshooting


---

<!-- merged from: cache-reserve-patterns.md -->

﻿---
name: Cache Reserve Patterns
description: # Cache Reserve Patterns
 
 ## Best Practices
---

# Cache Reserve Patterns

## Best Practices (Cache Reserve Patterns)

### 1. Always Enable Tiered Cache

```typescript
// Cache Reserve is designed for use WITH Tiered Cache
const configuration = {
  tieredCache: 'enabled',    // Required for optimal performance
  cacheReserve: 'enabled',   // Works best with Tiered Cache
  
  hierarchy: [
    'Lower-Tier Cache (visitor)',
    'Upper-Tier Cache (origin region)',
    'Cache Reserve (persistent)',
    'Origin'
  ]
};
```

### 2. Set Appropriate Cache-Control Headers

```typescript
// Origin response headers for Cache Reserve eligibility
const originHeaders = {
  'Cache-Control': 'public, max-age=86400', // 24hr (minimum 10hr)
  'Content-Length': '1024000', // Required
  'Cache-Tag': 'images,product-123', // Optional: purging
  'ETag': '"abc123"', // Optional: revalidation
  // Avoid: 'Set-Cookie' and 'Vary: *' prevent caching
};
```

### 3. Use Cache Rules for Fine-Grained Control

```typescript
// Different TTLs for different content types
const cacheRules = [
  {
    description: 'Long-term cache for immutable assets',
    expression: '(http.request.uri.path matches "^/static/.*\\.[a-f0-9]{8}\\.")',
    action_parameters: {
      cache_reserve: { eligible: true },
      edge_ttl: { mode: 'override_origin', default: 2592000 }, // 30 days
      cache: true
    }
  },
  {
    description: 'Moderate cache for regular images',
    expression: '(http.request.uri.path matches "\\.(jpg|png|webp)$")',
    action_parameters: {
      cache_reserve: { eligible: true },
      edge_ttl: { mode: 'override_origin', default: 86400 }, // 24 hours
      cache: true
    }
  },
  {
    description: 'Exclude API from Cache Reserve',
    expression: '(http.request.uri.path matches "^/api/")',
    action_parameters: { cache_reserve: { eligible: false }, cache: false }
  }
];
```

### 4. Making Assets Cache Reserve Eligible from Workers

**Note**: This modifies response headers to meet eligibility criteria but does NOT directly control Cache Reserve storage (which is zone-level automatic).

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const response = await fetch(request);
    if (!response.ok) return response;
    
    const headers = new Headers(response.headers);
    headers.set('Cache-Control', 'public, max-age=36000'); // 10hr minimum
    headers.delete('Set-Cookie'); // Blocks caching
    
    // Ensure Content-Length present
    if (!headers.has('Content-Length')) {
      const blob = await response.blob();
      headers.set('Content-Length', blob.size.toString());
      return new Response(blob, { status: response.status, headers });
    }
    
    return new Response(response.body, { status: response.status, headers });
  }
};
```

### 5. Hostname Best Practices

Use Worker's hostname for efficient caching - avoid overriding hostname unnecessarily.

## Architecture Patterns

### Multi-Tier Caching + Immutable Assets

```typescript
// Optimal: L1 (visitor) → L2 (region) → L3 (Cache Reserve) → Origin
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const isImmutable = /\.[a-f0-9]{8,}\.(js|css|jpg|png|woff2)$/.test(url.pathname);
    const response = await fetch(request);
    
    if (isImmutable) {
      const headers = new Headers(response.headers);
      headers.set('Cache-Control', 'public, max-age=31536000, immutable');
      return new Response(response.body, { status: response.status, headers });
    }
    return response;
  }
};
```

## Cost Optimization

### Cost Calculator

```typescript
interface CacheReserveEstimate {
  avgAssetSizeGB: number;
  uniqueAssets: number;
  monthlyReads: number;
  monthlyWrites: number;
  originEgressCostPerGB: number; // e.g., AWS: $0.09/GB
}

function estimateMonthlyCost(input: CacheReserveEstimate) {
  // Cache Reserve pricing
  const storageCostPerGBMonth = 0.015;
  const classAPerMillion = 4.50; // writes
  const classBPerMillion = 0.36; // reads
  
  // Calculate Cache Reserve costs
  const totalStorageGB = input.avgAssetSizeGB * input.uniqueAssets;
  const storageCost = totalStorageGB * storageCostPerGBMonth;
  const writeCost = (input.monthlyWrites / 1_000_000) * classAPerMillion;
  const readCost = (input.monthlyReads / 1_000_000) * classBPerMillion;
  
  const cacheReserveCost = storageCost + writeCost + readCost;
  
  // Calculate origin egress cost (what you'd pay without Cache Reserve)
  const totalTrafficGB = (input.monthlyReads * input.avgAssetSizeGB);
  const originEgressCost = totalTrafficGB * input.originEgressCostPerGB;
  
  // Savings calculation
  const savings = originEgressCost - cacheReserveCost;
  const savingsPercent = ((savings / originEgressCost) * 100).toFixed(1);
  
  return {
    cacheReserveCost: `$${cacheReserveCost.toFixed(2)}`,
    originEgressCost: `$${originEgressCost.toFixed(2)}`,
    monthlySavings: `$${savings.toFixed(2)}`,
    savingsPercent: `${savingsPercent}%`,
    breakdown: {
      storage: `$${storageCost.toFixed(2)}`,
      writes: `$${writeCost.toFixed(2)}`,
      reads: `$${readCost.toFixed(2)}`,
    }
  };
}

// Example: Media library
const mediaLibrary = estimateMonthlyCost({
  avgAssetSizeGB: 0.005, // 5MB images
  uniqueAssets: 10_000,
  monthlyReads: 5_000_000,
  monthlyWrites: 50_000,
  originEgressCostPerGB: 0.09, // AWS S3
});

console.log(mediaLibrary);
// {
//   cacheReserveCost: "$9.98",
//   originEgressCost: "$25.00",
//   monthlySavings: "$15.02",
//   savingsPercent: "60.1%",
//   breakdown: { storage: "$0.75", writes: "$0.23", reads: "$9.00" }
// }
```

### Optimization Guidelines

- **Set appropriate TTLs**: 10hr minimum, 24hr+ optimal for stable content, 30d max cautiously
- **Cache high-value stable assets**: Images, media, fonts, archives, documentation
- **Exclude frequently changing**: APIs, user-specific content, real-time data
- **Compression note**: Cache Reserve fetches uncompressed from origin, serves compressed to visitors - factor in origin egress costs

## See Also

- [README](./README.md) - Overview and core concepts
- [Configuration](./configuration.md) - Setup and Cache Rules
- [API Reference](./api.md) - Purging and monitoring
- [Gotchas](./gotchas.md) - Common issues and troubleshooting


---

<!-- merged from: cron-triggers-configuration.md -->

﻿---
name: Cron Triggers Configuration
description: # Cron Triggers Configuration
 
 ## wrangler.jsonc
---

# Cron Triggers Configuration

## wrangler.jsonc (Cron Triggers Configuration)

```jsonc
{
  "$schema": "./node_modules/wrangler/config-schema.json",
  "name": "my-cron-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01", // Use current date for new projects
  
  "triggers": {
    "crons": [
      "*/5 * * * *",     // Every 5 minutes
      "0 */2 * * *",     // Every 2 hours
      "0 9 * * MON-FRI", // Weekdays at 9am UTC
      "0 2 1 * *"        // Monthly on 1st at 2am UTC
    ]
  }
}
```

## Green Compute (Beta)

Schedule crons during low-carbon periods for carbon-aware execution:

```jsonc
{
  "name": "eco-cron-worker",
  "triggers": {
    "crons": ["0 2 * * *"]
  },
  "placement": {
    "mode": "smart"  // Runs during low-carbon periods
  }
}
```

### Modes

- `"smart"` - Carbon-aware scheduling (may delay up to 24h for optimal window)
- Default (no placement config) - Standard scheduling (no delay)

#### How it works

- Cloudflare delays execution until grid carbon intensity is lower
- Maximum delay: 24 hours from scheduled time
- Ideal for batch jobs with flexible timing requirements

#### Use cases

- Nightly data processing and ETL pipelines
- Weekly/monthly report generation
- Database backups and maintenance
- Analytics aggregation
- ML model training

#### Not suitable for

- Time-sensitive operations (SLA requirements)
- User-facing features requiring immediate execution
- Real-time monitoring and alerting
- Compliance tasks with strict time windows

## Environment-Specific Schedules

```jsonc
{
  "name": "my-cron-worker",
  "triggers": {
    "crons": ["0 */6 * * *"]  // Prod: every 6 hours
  },
  "env": {
    "staging": {
      "triggers": {
        "crons": ["*/15 * * * *"]  // Staging: every 15min
      }
    },
    "dev": {
      "triggers": {
        "crons": ["*/5 * * * *"]  // Dev: every 5min
      }
    }
  }
}
```

## Schedule Format

**Structure:** `minute hour day-of-month month day-of-week`

**Special chars:** `*` (any), `,` (list), `-` (range), `/` (step), `L` (last), `W` (weekday), `#` (nth)

## Managing Triggers

**Remove all:** `"triggers": { "crons": [] }`  
**Preserve existing:** Omit `"triggers"` field entirely

## Deployment

```bash
# Deploy with config crons
npx wrangler deploy

# Deploy specific environment
npx wrangler deploy --env production

# View deployments
npx wrangler deployments list
```

### ⚠️ Changes take up to 15 minutes to propagate globally

## API Management

### Get triggers

```bash
curl "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{script_name}/schedules" \
  -H "Authorization: Bearer {api_token}"
```

#### Update triggers

```bash
curl -X PUT "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{script_name}/schedules" \
  -H "Authorization: Bearer {api_token}" \
  -H "Content-Type: application/json" \
  -d '{"crons": ["*/5 * * * *", "0 2 * * *"]}'
```

#### Delete all

```bash
curl -X PUT "https://api.cloudflare.com/client/v4/accounts/{account_id}/workers/scripts/{script_name}/schedules" \
  -H "Authorization: Bearer {api_token}" \
  -H "Content-Type: application/json" \
  -d '{"crons": []}'
```

## Combining Multiple Workers

For complex schedules, use multiple workers:

```jsonc
// worker-frequent.jsonc
{
  "name": "data-sync-frequent",
  "triggers": { "crons": ["*/5 * * * *"] }
}

// worker-daily.jsonc
{
  "name": "reports-daily",
  "triggers": { "crons": ["0 2 * * *"] },
  "placement": { "mode": "smart" }
}

// worker-weekly.jsonc
{
  "name": "cleanup-weekly",
  "triggers": { "crons": ["0 3 * * SUN"] }
}
```

### Benefits

- Separate CPU limits per worker
- Independent error isolation
- Different Green Compute policies
- Easier to maintain and debug

## Validation

### Test cron syntax

- [crontab.guru](https://crontab.guru/) - Interactive validator
- Wrangler validates on deploy but won't catch logic errors

#### Common mistakes

- `0 0 * * *` runs daily at midnight UTC, not your local timezone
- `*/60 * * * *` is invalid (use `0 * * * *` for hourly)
- `0 2 31 * *` only runs on months with 31 days

## See Also

- [README.md](./README.md) - Overview, quick start
- [api.md](./api.md) - Handler implementation
- [patterns.md](./patterns.md) - Multi-cron routing examples


---

<!-- merged from: cron-triggers-patterns.md -->

﻿---
name: Cron Triggers Patterns
description: # Cron Triggers Patterns
 
 ## API Data Sync
---

# Cron Triggers Patterns

## API Data Sync (Cron Triggers Patterns)

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const response = await fetch("https://api.example.com/data", {headers: { "Authorization": `Bearer ${env.API_KEY}` }});
    if (!response.ok) throw new Error(`API error: ${response.status}`);
    ctx.waitUntil(env.MY_KV.put("cached_data", JSON.stringify(await response.json()), {expirationTtl: 3600}));
  },
};
```

## Database Cleanup

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const result = await env.DB.prepare(`DELETE FROM sessions WHERE expires_at < datetime('now')`).run();
    console.log(`Deleted ${result.meta.changes} expired sessions`);
    ctx.waitUntil(env.DB.prepare("VACUUM").run());
  },
};
```

## Report Generation

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const startOfWeek = new Date(); startOfWeek.setDate(startOfWeek.getDate() - 7);
    const { results } = await env.DB.prepare(`SELECT date, revenue, orders FROM daily_stats WHERE date >= ? ORDER BY date`).bind(startOfWeek.toISOString()).all();
    const report = {period: "weekly", totalRevenue: results.reduce((sum, d) => sum + d.revenue, 0), totalOrders: results.reduce((sum, d) => sum + d.orders, 0), dailyBreakdown: results};
    const reportKey = `reports/weekly-${Date.now()}.json`;
    await env.REPORTS_BUCKET.put(reportKey, JSON.stringify(report));
    ctx.waitUntil(env.SEND_EMAIL.fetch("https://example.com/send", {method: "POST", body: JSON.stringify({to: "team@example.com", subject: "Weekly Report", reportUrl: `https://reports.example.com/${reportKey}`})}));
  },
};
```

## Health Checks

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const services = [{name: "API", url: "https://api.example.com/health"}, {name: "CDN", url: "https://cdn.example.com/health"}];
    const checks = await Promise.all(services.map(async (service) => {
      const start = Date.now();
      try {
        const response = await fetch(service.url, { signal: AbortSignal.timeout(5000) });
        return {name: service.name, status: response.ok ? "up" : "down", responseTime: Date.now() - start};
      } catch (error) {
        return {name: service.name, status: "down", responseTime: Date.now() - start, error: error.message};
      }
    }));
    ctx.waitUntil(env.STATUS_KV.put("health_status", JSON.stringify(checks)));
    const failures = checks.filter(c => c.status === "down");
    if (failures.length > 0) ctx.waitUntil(fetch(env.ALERT_WEBHOOK, {method: "POST", body: JSON.stringify({text: `${failures.length} service(s) down: ${failures.map(f => f.name).join(", ")}`})}));
  },
};
```

## Batch Processing (Rate-Limited)

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const queueData = await env.QUEUE_KV.get("pending_items", "json");
    if (!queueData || queueData.length === 0) return;
    const batch = queueData.slice(0, 100);
    const results = await Promise.allSettled(batch.map(item => fetch("https://api.example.com/process", {method: "POST", headers: {"Authorization": `Bearer ${env.API_KEY}`, "Content-Type": "application/json"}, body: JSON.stringify(item)})));
    console.log(`Processed ${results.filter(r => r.status === "fulfilled").length}/${batch.length} items`);
    ctx.waitUntil(env.QUEUE_KV.put("pending_items", JSON.stringify(queueData.slice(100))));
  },
};
```

## Queue Integration

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const batch = await env.MY_QUEUE.receive({ batchSize: 100 });
    const results = await Promise.allSettled(batch.messages.map(async (msg) => {
      await processMessage(msg.body, env);
      await msg.ack();
    }));
    console.log(`Processed ${results.filter(r => r.status === "fulfilled").length}/${batch.messages.length}`);
  },
};
```

## Monitoring & Observability

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const startTime = Date.now();
    const meta = { cron: controller.cron, scheduledTime: controller.scheduledTime };
    console.log("[START]", meta);
    try {
      const result = await performTask(env);
      console.log("[SUCCESS]", { ...meta, duration: Date.now() - startTime, count: result.count });
      ctx.waitUntil(env.METRICS.put(`cron:${controller.scheduledTime}`, JSON.stringify({ ...meta, status: "success" }), { expirationTtl: 2592000 }));
    } catch (error) {
      console.error("[ERROR]", { ...meta, duration: Date.now() - startTime, error: error.message });
      ctx.waitUntil(fetch(env.ALERT_WEBHOOK, { method: "POST", body: JSON.stringify({ text: `Cron failed: ${controller.cron}`, error: error.message }) }));
      throw error;
    }
  },
};
```

**View logs:** `npx wrangler tail` or Dashboard → Workers & Pages → Worker → Logs

## Durable Objects Coordination

```typescript
export default {
  async scheduled(controller, env, ctx) {
    const stub = env.COORDINATOR.get(env.COORDINATOR.idFromName("cron-lock"));
    const acquired = await stub.tryAcquireLock(controller.scheduledTime);
    if (!acquired) {
      controller.noRetry();
      return;
    }
    try {
      await performTask(env);
    } finally {
      await stub.releaseLock();
    }
  },
};
```

## Python Handler

```python
from workers import WorkerEntrypoint

class Default(WorkerEntrypoint):
    async def scheduled(self, controller, env, ctx):
        data = await env.MY_KV.get("key")
        ctx.waitUntil(env.DB.execute("DELETE FROM logs WHERE created_at < datetime('now', '-7 days')"))
```

## Testing Patterns

### Local testing with /__scheduled

```bash
# Start dev server
npx wrangler dev

# Test specific cron
curl "http://localhost:8787/__scheduled?cron=*/5+*+*+*+*"

# Test with specific time
curl "http://localhost:8787/__scheduled?cron=0+2+*+*+*&scheduledTime=1704067200000"
```

#### Unit tests

```typescript
// test/scheduled.test.ts
import { describe, it, expect, vi } from "vitest";
import { env } from "cloudflare:test";
import worker from "../src/index";

describe("Scheduled Handler", () => {
  it("executes cron", async () => {
    const controller = { scheduledTime: Date.now(), cron: "*/5 * * * *", type: "scheduled" as const, noRetry: vi.fn() };
    const ctx = { waitUntil: vi.fn(), passThroughOnException: vi.fn() };
    await worker.scheduled(controller, env, ctx);
    expect(await env.MY_KV.get("last_run")).toBeDefined();
  });
  
  it("calls noRetry on duplicate", async () => {
    const controller = { scheduledTime: 1704067200000, cron: "0 2 * * *", type: "scheduled" as const, noRetry: vi.fn() };
    await env.EXECUTIONS.put("0 2 * * *-1704067200000", "1");
    await worker.scheduled(controller, env, { waitUntil: vi.fn(), passThroughOnException: vi.fn() });
    expect(controller.noRetry).toHaveBeenCalled();
  });
});
```

## See Also

- [README.md](./README.md) - Overview
- [api.md](./api.md) - Handler implementation
- [gotchas.md](./gotchas.md) - Troubleshooting


---

<!-- merged from: pipelines-configuration.md -->

﻿---
name: Pipelines Configuration
description: # Pipelines Configuration
 
 ## Worker Binding
---

# Pipelines Configuration

## Worker Binding (Pipelines Configuration)

```jsonc
// wrangler.jsonc
{
  "pipelines": [
    { "pipeline": "<STREAM_ID>", "binding": "STREAM" }
  ]
}
```

Get stream ID: `npx wrangler pipelines streams list`

## Schema (Structured Streams)

```json
{
  "fields": [
    { "name": "user_id", "type": "string", "required": true },
    { "name": "event_type", "type": "string", "required": true },
    { "name": "amount", "type": "float64", "required": false },
    { "name": "timestamp", "type": "timestamp", "required": true }
  ]
}
```

**Types:** `string`, `int32`, `int64`, `float32`, `float64`, `bool`, `timestamp`, `json`, `binary`, `list`, `struct`

## Stream Setup

```bash
# With schema
npx wrangler pipelines streams create my-stream --schema-file schema.json

# Unstructured (no validation)
npx wrangler pipelines streams create my-stream

# List/get/delete
npx wrangler pipelines streams list
npx wrangler pipelines streams get <ID>
npx wrangler pipelines streams delete <ID>
```

## Sink Configuration

### R2 Data Catalog (Iceberg)

```bash
npx wrangler pipelines sinks create my-sink \
  --type r2-data-catalog \
  --bucket my-bucket --namespace default --table events \
  --catalog-token $TOKEN \
  --compression zstd --roll-interval 60
```

#### R2 Raw (Parquet)

```bash
npx wrangler pipelines sinks create my-sink \
  --type r2 --bucket my-bucket --format parquet \
  --path analytics/events \
  --partitioning "year=%Y/month=%m/day=%d" \
  --access-key-id $KEY --secret-access-key $SECRET
```

| Option | Values | Guidance |
| -------- | -------- | ---------- |
| `--compression` | `zstd`, `snappy`, `gzip` | `zstd` best ratio, `snappy` fastest |
| `--roll-interval` | Seconds | Low latency: 10-60, Query perf: 300 |
| `--roll-size` | MB | Larger = better compression |

## Pipeline Creation

```bash
npx wrangler pipelines create my-pipeline \
  --sql "INSERT INTO my_sink SELECT * FROM my_stream WHERE event_type = 'purchase'"
```

**⚠️ Pipelines are immutable** - cannot modify SQL. Must delete/recreate.

## Credentials

| Type | Permission | Get From |
| ------ | ------------ | ---------- |
| Catalog token | R2 Admin Read & Write | Dashboard → R2 → API tokens |
| R2 credentials | Object Read & Write | `wrangler r2 bucket create` output |
| HTTP ingest token | Workers Pipeline Send | Dashboard → Workers → API tokens |

## Complete Example

```bash
npx wrangler r2 bucket create my-bucket
npx wrangler r2 bucket catalog enable my-bucket
npx wrangler pipelines streams create my-stream --schema-file schema.json
npx wrangler pipelines sinks create my-sink --type r2-data-catalog --bucket my-bucket ...
npx wrangler pipelines create my-pipeline --sql "INSERT INTO my_sink SELECT * FROM my_stream"
npx wrangler deploy
```


---

<!-- merged from: pipelines-patterns.md -->

﻿---
name: Pipelines Patterns
description: # Pipelines Patterns
 
 ## Fire-and-Forget
---

# Pipelines Patterns

## Fire-and-Forget (Pipelines Patterns)

```typescript
export default {
  async fetch(request, env, ctx) {
    const event = { user_id: '...', event_type: 'page_view', timestamp: new Date().toISOString() };
    ctx.waitUntil(env.STREAM.send([event])); // Don't block response
    return new Response('OK');
  }
};
```

## Schema Validation with Zod

```typescript
import { z } from 'zod';

const EventSchema = z.object({
  user_id: z.string(),
  event_type: z.enum(['purchase', 'view']),
  amount: z.number().positive().optional()
});

const validated = EventSchema.parse(rawEvent); // Throws on invalid
await env.STREAM.send([validated]);
```

**Why:** Structured streams drop invalid events silently. Client validation gives immediate feedback.

## SQL Transform Patterns

```sql
-- Filter early (reduce storage)
INSERT INTO my_sink
SELECT user_id, event_type, amount
FROM my_stream
WHERE event_type = 'purchase' AND amount > 10

-- Select only needed fields
INSERT INTO my_sink
SELECT user_id, event_type, timestamp FROM my_stream

-- Enrich with CASE
INSERT INTO my_sink
SELECT user_id, amount,
  CASE WHEN amount > 1000 THEN 'vip' ELSE 'standard' END as tier
FROM my_stream
```

## Pipelines + Queues Fan-out

```typescript
await Promise.all([
  env.ANALYTICS_STREAM.send([event]),  // Long-term storage
  env.PROCESS_QUEUE.send(event)        // Immediate processing
]);
```

| Need | Use |
| ------ | ----- |
| Long-term storage, SQL queries | Pipelines |
| Immediate processing, retries | Queues |
| Both | Fan-out pattern |

## Performance Tuning

| Goal | Config |
| ------ | -------- |
| Low latency | `--roll-interval 10` |
| Query performance | `--roll-interval 300 --roll-size 100` |
| Cost optimal | `--compression zstd --roll-interval 300` |

## Schema Evolution

Pipelines are immutable. Use versioning:

```bash
# Create v2 stream/sink/pipeline
npx wrangler pipelines streams create events-v2 --schema-file v2.json

# Dual-write during transition
await Promise.all([env.EVENTS_V1.send([event]), env.EVENTS_V2.send([event])]);

# Query across versions with UNION ALL
```


---

<!-- merged from: realtimekit-configuration.md -->

﻿---
name: RealtimeKit Configuration
description: # RealtimeKit Configuration
 
 Configuration guide for RealtimeKit setup, client SDKs, and wrangler integration.
---

# RealtimeKit Configuration

Configuration guide for RealtimeKit setup, client SDKs, and wrangler integration.

## Installation

### React

```bash
npm install @cloudflare/realtimekit @cloudflare/realtimekit-react-ui
```

### Angular

```bash
npm install @cloudflare/realtimekit @cloudflare/realtimekit-angular-ui
```

### Web Components/HTML

```bash
npm install @cloudflare/realtimekit @cloudflare/realtimekit-ui
```

## Client SDK Configuration

### React UI Kit

```tsx
import { RtkMeeting } from '@cloudflare/realtimekit-react-ui';
<RtkMeeting authToken="<token>" onLeave={() => {}} />
```

### Angular UI Kit

```typescript
@Component({ template: `<rtk-meeting [authToken]="authToken" (rtkLeave)="onLeave($event)"></rtk-meeting>` })
export class AppComponent { authToken = '<token>'; onLeave() {} }
```

### Web Components

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/@cloudflare/realtimekit-ui/dist/realtimekit-ui/realtimekit-ui.esm.js"></script>
<rtk-meeting id="meeting"></rtk-meeting>
<script>
  document.getElementById('meeting').authToken = '<token>';
</script>
```

### Core SDK Configuration

```typescript
import RealtimeKitClient from '@cloudflare/realtimekit';

const meeting = new RealtimeKitClient({
  authToken: '<token>',
  video: true, audio: true, autoSwitchAudioDevice: true,
  mediaConfiguration: {
    video: { width: { ideal: 1280 }, height: { ideal: 720 }, frameRate: { ideal: 30 } },
    audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true },
    screenshare: { width: { max: 1920 }, height: { max: 1080 }, frameRate: { ideal: 15 } }
  }
});
await meeting.join();
```

## Backend Setup

### Create App & Credentials

**Dashboard**: <https://dash.cloudflare.com/?to=/:account/realtime/kit>

#### API

```bash
curl -X POST 'https://api.cloudflare.com/client/v4/accounts/<account_id>/realtime/kit/apps' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer <api_token>' \
  -d '{"name": "My RealtimeKit App"}'
```

**Required Permissions**: API token with **Realtime / Realtime Admin** permissions

### Create Presets

```bash
curl -X POST 'https://api.cloudflare.com/client/v4/accounts/<account_id>/realtime/kit/<app_id>/presets' \
  -H 'Authorization: Bearer <api_token>' \
  -d '{
    "name": "host",
    "permissions": {
      "canShareAudio": true,
      "canShareVideo": true,
      "canRecord": true,
      "canLivestream": true,
      "canStartStopRecording": true
    }
  }'
```

## Wrangler Configuration

### Basic Configuration

```jsonc
// wrangler.jsonc
{
  "name": "realtimekit-app",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",  // Use current date
  "vars": {
    "CLOUDFLARE_ACCOUNT_ID": "abc123",
    "REALTIMEKIT_APP_ID": "xyz789"
  }
  // Secrets: wrangler secret put CLOUDFLARE_API_TOKEN
}
```

### With Database & Storage

```jsonc
{
  "d1_databases": [{ "binding": "DB", "database_name": "meetings", "database_id": "d1-id" }],
  "r2_buckets": [{ "binding": "RECORDINGS", "bucket_name": "recordings" }],
  "kv_namespaces": [{ "binding": "SESSIONS", "id": "kv-id" }]
}
```

### Multi-Environment

```bash
# Deploy to environments
wrangler deploy --env staging
wrangler deploy --env production
```

## TURN Service Configuration

RealtimeKit can use Cloudflare's TURN service for connectivity through restrictive networks:

```jsonc
// wrangler.jsonc
{
  "vars": {
    "TURN_SERVICE_ID": "your_turn_service_id"
  }
  // Set secret: wrangler secret put TURN_SERVICE_TOKEN
}
```

TURN automatically configured when enabled in account - no client-side changes needed.

## Theming & Design Tokens

```typescript
import type { UIConfig } from '@cloudflare/realtimekit';

const uiConfig: UIConfig = {
  designTokens: {
    colors: {
      brand: { 500: '#0066ff', 600: '#0052cc' },
      background: { 1000: '#1A1A1A', 900: '#2D2D2D' },
      text: { 1000: '#FFFFFF', 900: '#E0E0E0' }
    },
    borderRadius: 'extra-rounded',  // 'rounded' | 'extra-rounded' | 'sharp'
    theme: 'dark'  // 'light' | 'dark'
  },
  logo: { url: 'https://example.com/logo.png', altText: 'Company' }
};

// Apply to React
<RtkMeeting authToken={token} config={uiConfig} onLeave={() => {}} />

// Or use CSS variables
// :root { --rtk-color-brand-500: #0066ff; --rtk-border-radius: 12px; }
```

## Internationalization (i18n)

### Custom Language Strings

```typescript
import { useLanguage } from '@cloudflare/realtimekit-ui';

const customLanguage = {
  'join': 'Entrar',
  'leave': 'Salir',
  'mute': 'Silenciar',
  'unmute': 'Activar audio',
  'turn_on_camera': 'Encender cámara',
  'turn_off_camera': 'Apagar cámara',
  'share_screen': 'Compartir pantalla',
  'stop_sharing': 'Dejar de compartir'
};

const t = useLanguage(customLanguage);

// React usage
<RtkMeeting authToken={token} t={t} onLeave={() => {}} />
```

### Supported Locales

Default locales available: `en`, `es`, `fr`, `de`, `pt`, `ja`, `zh`

```typescript
import { setLocale } from '@cloudflare/realtimekit-ui';
setLocale('es');  // Switch to Spanish
```

## See Also

- [API](./api.md) - Meeting APIs, REST endpoints
- [Patterns](./patterns.md) - Backend integration examples
- [README](./README.md) - Overview and quick start


---

<!-- merged from: realtimekit-patterns.md -->

﻿---
name: RealtimeKit Patterns
description: # RealtimeKit Patterns
 
 ## UI Kit (Minimal Code)
---

# RealtimeKit Patterns

## UI Kit (Minimal Code) (RealtimeKit Patterns)

```tsx
// React
import { RtkMeeting } from '@cloudflare/realtimekit-react-ui';
<RtkMeeting authToken="<token>" onLeave={() => console.log('Left')} />

// Angular
@Component({ template: `<rtk-meeting [authToken]="authToken" (rtkLeave)="onLeave($event)"></rtk-meeting>` })
export class AppComponent { authToken = '<token>'; onLeave(event: unknown) {} }

// HTML/Web Components
<script type="module" src="https://cdn.jsdelivr.net/npm/@cloudflare/realtimekit-ui/dist/realtimekit-ui/realtimekit-ui.esm.js"></script>
<rtk-meeting id="meeting"></rtk-meeting>
<script>document.getElementById('meeting').authToken = '<token>';</script>
```

## UI Components

RealtimeKit provides 133+ pre-built Stencil.js Web Components with framework wrappers:

### Layout Components

- `<RtkMeeting>` - Full meeting UI (all-in-one)
- `<RtkHeader>`, `<RtkStage>`, `<RtkControlbar>` - Layout sections
- `<RtkSidebar>` - Chat/participants sidebar
- `<RtkGrid>` - Adaptive video grid

### Control Components  

- `<RtkMicToggle>`, `<RtkCameraToggle>` - Media controls
- `<RtkScreenShareToggle>` - Screen sharing
- `<RtkLeaveButton>` - Leave meeting
- `<RtkSettingsModal>` - Device settings

### Grid Variants

- `<RtkSpotlightGrid>` - Active speaker focus
- `<RtkAudioGrid>` - Audio-only mode
- `<RtkPaginatedGrid>` - Paginated layout

**See full catalog**: <https://docs.realtime.cloudflare.com/ui-kit>
