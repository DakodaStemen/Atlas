---
name: cloudflare-ddos-protection
description: Comprehensive Cloudflare DDoS protection reference covering L7/L4 API endpoints (TypeScript SDK, ruleset overrides, alert configuration), dashboard configuration (sensitivity levels, action modes, rule categories), gotchas (override conflicts, origin flood, L3/4 scope), and protection patterns (API protection, WordPress, gaming, rate limiting integration, origin shielding).
domain: security
tags: [cloudflare, ddos, l7, l4, rate-limiting, waf, protection, ruleset-overrides]
triggers: cloudflare DDoS, DDoS protection, L7 DDoS, L4 DDoS, DDoS ruleset, sensitivity level, DDoS alert, origin flood, rate limiting
---

# Cloudflare DDoS Protection

## API Endpoints

### HTTP DDoS (L7)

```typescript
// Zone-level
PUT /zones/{zoneId}/rulesets/phases/ddos_l7/entrypoint
GET /zones/{zoneId}/rulesets/phases/ddos_l7/entrypoint

// Account-level (Enterprise Advanced)
PUT /accounts/{accountId}/rulesets/phases/ddos_l7/entrypoint
GET /accounts/{accountId}/rulesets/phases/ddos_l7/entrypoint
```

### Network DDoS (L3/4)

```typescript
// Account-level only
PUT /accounts/{accountId}/rulesets/phases/ddos_l4/entrypoint
GET /accounts/{accountId}/rulesets/phases/ddos_l4/entrypoint
```

## TypeScript SDK

**SDK Version**: Requires `cloudflare` >= 3.0.0 for ruleset phase methods.

```typescript
import Cloudflare from "cloudflare";

const client = new Cloudflare({ apiToken: process.env.CLOUDFLARE_API_TOKEN });

// STEP 1: Discover managed ruleset ID (required for overrides)
const allRulesets = await client.rulesets.list({ zone_id: zoneId });
const ddosRuleset = allRulesets.result.find(
  (r) => r.kind === "managed" && r.phase === "ddos_l7"
);
if (!ddosRuleset) throw new Error("DDoS managed ruleset not found");
const managedRulesetId = ddosRuleset.id;

// STEP 2: Get current HTTP DDoS configuration
const entrypointRuleset = await client.zones.rulesets.phases.entrypoint.get("ddos_l7", {
  zone_id: zoneId,
});

// STEP 3: Update HTTP DDoS ruleset with overrides
await client.zones.rulesets.phases.entrypoint.update("ddos_l7", {
  zone_id: zoneId,
  rules: [
    {
      action: "execute",
      expression: "true",
      action_parameters: {
        id: managedRulesetId, // From discovery step
        overrides: {
          sensitivity_level: "medium",
          action: "managed_challenge",
        },
      },
    },
  ],
});

// Network DDoS (account level, L3/4)
const l4Rulesets = await client.rulesets.list({ account_id: accountId });
const l4DdosRuleset = l4Rulesets.result.find(
  (r) => r.kind === "managed" && r.phase === "ddos_l4"
);
const l4Ruleset = await client.accounts.rulesets.phases.entrypoint.get("ddos_l4", {
  account_id: accountId,
});
```

## Alert Configuration

```typescript
interface DDoSAlertConfig {
  name: string;
  enabled: boolean;
  alert_type: "http_ddos_attack_alert" | "layer_3_4_ddos_attack_alert" 
    | "advanced_http_ddos_attack_alert" | "advanced_layer_3_4_ddos_attack_alert";
  filters?: {
    zones?: string[];
    hostnames?: string[];
    requests_per_second?: number;
    packets_per_second?: number;
    megabits_per_second?: number;
    ip_prefixes?: string[]; // CIDR
    ip_addresses?: string[];
    protocols?: string[];
  };
  mechanisms: {
    email?: Array<{ id: string }>;
    webhooks?: Array<{ id: string }>;
    pagerduty?: Array<{ id: string }>;
  };
}

// Create alert
await fetch(
  `https://api.cloudflare.com/client/v4/accounts/${accountId}/alerting/v3/policies`,
  {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(alertConfig),
  }
);
```

## Typed Override Examples

```typescript
// Override by category
interface CategoryOverride {
  action: "execute";
  expression: string;
  action_parameters: {
    id: string;
    overrides: {
      categories?: Array<{
        category: "http-flood" | "http-anomaly" | "udp-flood" | "syn-flood";
        sensitivity_level?: "default" | "medium" | "low" | "eoff";
        action?: "block" | "managed_challenge" | "challenge" | "log";
      }>;
    };
  };
}

// Override by rule ID
interface RuleOverride {
  action: "execute";
  expression: string;
  action_parameters: {
    id: string;
    overrides: {
      rules?: Array<{
        id: string;
        action?: "block" | "managed_challenge" | "challenge" | "log";
        sensitivity_level?: "default" | "medium" | "low" | "eoff";
      }>;
    };
  };
}

// Example: Override specific adaptive rule
const adaptiveOverride: RuleOverride = {
  action: "execute",
  expression: "true",
  action_parameters: {
    id: managedRulesetId,
    overrides: {
      rules: [
        { id: "...adaptive-origins-rule-id...", sensitivity_level: "low" },
      ],
    },
  },
};
```


﻿---
name: DDoS Configuration
description: # DDoS Configuration
 
 ## Dashboard Setup
---

# DDoS Configuration

## Dashboard Setup (DDoS Configuration)

1. Navigate to Security > DDoS
2. Select HTTP DDoS or Network-layer DDoS
3. Configure sensitivity & action per ruleset/category/rule
4. Apply overrides with optional expressions (Enterprise Advanced)
5. Enable Adaptive DDoS toggle (Enterprise/Enterprise Advanced, requires 7 days traffic history)

## Rule Structure

```typescript
interface DDoSOverride {
  description: string;
  rules: Array<{
    action: "execute";
    expression: string; // Custom expression (Enterprise Advanced) or "true" for all
    action_parameters: {
      id: string; // Managed ruleset ID (discover via api.md)
      overrides: {
        sensitivity_level?: "default" | "medium" | "low" | "eoff";
        action?: "block" | "managed_challenge" | "challenge" | "log"; // log = Enterprise Advanced only
        categories?: Array<{
          category: string; // e.g., "http-flood", "udp-flood"
          sensitivity_level?: string;
        }>;
        rules?: Array<{
          id: string;
          action?: string;
          sensitivity_level?: string;
        }>;
      };
    };
  }>;
}
```

## Expression Availability

| Plan | Custom Expressions | Example |
| ------ | ------------------- | --------- |
| Free/Pro/Business | ✗ | Use `"true"` only |
| Enterprise | ✗ | Use `"true"` only |
| Enterprise Advanced | ✓ | `ip.src in {...}`, `http.request.uri.path matches "..."` |

## Sensitivity Mapping

| UI | API | Threshold |
| ---- | ----- | ----------- |
| High | `default` | Most aggressive |
| Medium | `medium` | Balanced |
| Low | `low` | Less aggressive |
| Essentially Off | `eoff` | Minimal mitigation |

## Common Categories

- `http-flood`, `http-anomaly` (L7)
- `udp-flood`, `syn-flood`, `dns-flood` (L3/4)

## Override Precedence

Multiple override layers apply in this order (higher precedence wins):

```text
Zone-level > Account-level
Individual Rule > Category > Global sensitivity/action
```

**Example**: Zone rule for `/api/*` overrides account-level global settings.

## Adaptive DDoS Profiles

**Availability**: Enterprise, Enterprise Advanced  
**Learning period**: 7 days of traffic history required

| Profile Type | Description | Detects |
| -------------- | ------------- | --------- |
| **Origins** | Traffic patterns per origin server | Anomalous requests to specific origins |
| **User-Agents** | Traffic patterns per User-Agent | Malicious/anomalous user agent strings |
| **Locations** | Traffic patterns per geo-location | Attacks from specific countries/regions |
| **Protocols** | Traffic patterns per protocol (L3/4) | Protocol-specific flood attacks |

Configure by targeting specific adaptive rule IDs via API (see api.md#typed-override-examples).

## Alerting

Configure via Notifications:

- Alert types: `http_ddos_attack_alert`, `layer_3_4_ddos_attack_alert`, `advanced_*` variants
- Filters: zones, hostnames, RPS/PPS/Mbps thresholds, IPs, protocols
- Mechanisms: email, webhooks, PagerDuty


﻿---
name: DDoS Gotchas
description: # DDoS Gotchas
 
 ## Common Errors
---

# DDoS Gotchas

## Common Errors (DDoS Gotchas)

### "False positives blocking legitimate traffic"

**Cause**: Sensitivity too high, wrong action, or missing exceptions  

#### Solution

1. Lower sensitivity for specific rule/category
2. Use `log` action first to validate (Enterprise Advanced)
3. Add exception with custom expression (e.g., allowlist IPs)
4. Query flagged requests via GraphQL Analytics API to identify patterns

### "Attacks getting through"

**Cause**: Sensitivity too low or wrong action  
**Solution**: Increase to `default` sensitivity and use `block` action:

```typescript
const config = {
  rules: [{
    expression: "true",
    action: "execute",
    action_parameters: { id: managedRulesetId, overrides: { sensitivity_level: "default", action: "block" } },
  }],
};
```

### "Adaptive rules not working"

**Cause**: Insufficient traffic history (needs 7 days)  
**Solution**: Wait for baseline to establish, check dashboard for adaptive rule status

### "Zone override ignored"

**Cause**: Account overrides conflict with zone overrides  
**Solution**: Configure at zone level OR remove zone overrides to use account-level

### "Log action not available"

**Cause**: Not on Enterprise Advanced DDoS plan  
**Solution**: Use `managed_challenge` with low sensitivity for testing

### "Rule limit exceeded"

**Cause**: Too many override rules (Free/Pro/Business: 1, Enterprise Advanced: 10)  
**Solution**: Combine conditions in single expression using `and`/`or`

### "Cannot override rule"

**Cause**: Rule is read-only  
**Solution**: Check API response for read-only indicator, use different rule

### "Cannot disable DDoS protection"

**Cause**: DDoS managed rulesets cannot be fully disabled (always-on protection)  
**Solution**: Set `sensitivity_level: "eoff"` for minimal mitigation

### "Expression not allowed"

**Cause**: Custom expressions require Enterprise Advanced plan  
**Solution**: Use `expression: "true"` for all traffic, or upgrade plan

### "Managed ruleset not found"

**Cause**: Zone/account doesn't have DDoS managed ruleset, or incorrect phase  
**Solution**: Verify ruleset exists via `client.rulesets.list()`, check phase name (`ddos_l7` or `ddos_l4`)

## API Error Codes

| Error Code | Message | Cause | Solution |
| ------------ | --------- | ------- | ---------- |
| 10000 | Authentication error | Invalid/missing API token | Check token has DDoS permissions |
| 81000 | Ruleset validation failed | Invalid rule structure | Verify `action_parameters.id` is managed ruleset ID |
| 81020 | Expression not allowed | Custom expressions on wrong plan | Use `"true"` or upgrade to Enterprise Advanced |
| 81021 | Rule limit exceeded | Too many override rules | Reduce rules or upgrade (Enterprise Advanced: 10) |
| 81022 | Invalid sensitivity level | Wrong sensitivity value | Use: `default`, `medium`, `low`, `eoff` |
| 81023 | Invalid action | Wrong action for plan | Enterprise Advanced only: `log` action |

## Limits

| Resource/Limit | Free/Pro/Business | Enterprise | Enterprise Advanced |
| ---------------- | ------------------- | ------------ | --------------------- |
| Override rules per zone | 1 | 1 | 10 |
| Custom expressions | ✗ | ✗ | ✓ |
| Log action | ✗ | ✗ | ✓ |
| Adaptive DDoS | ✗ | ✓ | ✓ |
| Traffic history required | - | 7 days | 7 days |

## Tuning Strategy

1. Start with `log` action + `medium` sensitivity
2. Monitor for 24-48 hours
3. Identify false positives, add exceptions
4. Gradually increase to `default` sensitivity
5. Change action from `log` → `managed_challenge` → `block`
6. Document all adjustments

## Best Practices

- Test during low-traffic periods
- Use zone-level for per-site tuning
- Reference IP lists for easier management
- Set appropriate alert thresholds (avoid noise)
- Combine with WAF for layered defense
- Avoid over-tuning (keep config simple)


﻿---
name: DDoS Protection Patterns
description: # DDoS Protection Patterns
 
 ## Allowlist Trusted IPs
---

# DDoS Protection Patterns

## Allowlist Trusted IPs (DDoS Protection Patterns)

```typescript
const config = {
  description: "Allowlist trusted IPs",
  rules: [{
    expression: "ip.src in { 203.0.113.0/24 192.0.2.1 }",
    action: "execute",
    action_parameters: {
      id: managedRulesetId,
      overrides: { sensitivity_level: "eoff" },
    },
  }],
};

await client.accounts.rulesets.phases.entrypoint.update("ddos_l7", {
  account_id: accountId,
  ...config,
});
```

## Route-specific Sensitivity

```typescript
const config = {
  description: "Route-specific protection",
  rules: [
    {
      expression: "not http.request.uri.path matches \"^/api/\"",
      action: "execute",
      action_parameters: {
        id: managedRulesetId,
        overrides: { sensitivity_level: "default", action: "block" },
      },
    },
    {
      expression: "http.request.uri.path matches \"^/api/\"",
      action: "execute",
      action_parameters: {
        id: managedRulesetId,
        overrides: { sensitivity_level: "low", action: "managed_challenge" },
      },
    },
  ],
};
```

## Progressive Enhancement

```typescript
enum ProtectionLevel { MONITORING = "monitoring", LOW = "low", MEDIUM = "medium", HIGH = "high" }

const levelConfig = {
  [ProtectionLevel.MONITORING]: { action: "log", sensitivity: "eoff" },
  [ProtectionLevel.LOW]: { action: "managed_challenge", sensitivity: "low" },
  [ProtectionLevel.MEDIUM]: { action: "managed_challenge", sensitivity: "medium" },
  [ProtectionLevel.HIGH]: { action: "block", sensitivity: "default" },
} as const;

async function setProtectionLevel(zoneId: string, level: ProtectionLevel, rulesetId: string, client: Cloudflare) {
  const settings = levelConfig[level];
  return client.zones.rulesets.phases.entrypoint.update("ddos_l7", {
    zone_id: zoneId,
    rules: [{
      expression: "true",
      action: "execute",
      action_parameters: { id: rulesetId, overrides: { action: settings.action, sensitivity_level: settings.sensitivity } },
    }],
  });
}
```

## Dynamic Response to Attacks

```typescript
interface Env { CLOUDFLARE_API_TOKEN: string; ZONE_ID: string; KV: KVNamespace; }

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.url.includes("/attack-detected")) {
      const attackData = await request.json();
      await env.KV.put(`attack:${Date.now()}`, JSON.stringify(attackData), { expirationTtl: 86400 });
      const recentAttacks = await getRecentAttacks(env.KV);
      if (recentAttacks.length > 5) {
        await setProtectionLevel(env.ZONE_ID, ProtectionLevel.HIGH, managedRulesetId, client);
        return new Response("Protection increased");
      }
    }
    return new Response("OK");
  },
  async scheduled(event: ScheduledEvent, env: Env): Promise<void> {
    const recentAttacks = await getRecentAttacks(env.KV);
    if (recentAttacks.length === 0) await setProtectionLevel(env.ZONE_ID, ProtectionLevel.MEDIUM, managedRulesetId, client);
  },
};
```

## Multi-rule Tiered Protection (Enterprise Advanced)

```typescript
const config = {
  description: "Multi-tier DDoS protection",
  rules: [
    {
      expression: "not ip.src in $known_ips and not cf.bot_management.score gt 30",
      action: "execute",
      action_parameters: { id: managedRulesetId, overrides: { sensitivity_level: "default", action: "block" } },
    },
    {
      expression: "cf.bot_management.verified_bot",
      action: "execute",
      action_parameters: { id: managedRulesetId, overrides: { sensitivity_level: "medium", action: "managed_challenge" } },
    },
    {
      expression: "ip.src in $trusted_ips",
      action: "execute",
      action_parameters: { id: managedRulesetId, overrides: { sensitivity_level: "low" } },
    },
  ],
};
```

## Defense in Depth

Layered security stack: DDoS + WAF + Rate Limiting + Bot Management.

```typescript
// Layer 1: DDoS (volumetric attacks)
await client.zones.rulesets.phases.entrypoint.update("ddos_l7", {
  zone_id: zoneId,
  rules: [{ expression: "true", action: "execute", action_parameters: { id: ddosRulesetId, overrides: { sensitivity_level: "medium" } } }],
});

// Layer 2: WAF (exploit protection)
await client.zones.rulesets.phases.entrypoint.update("http_request_firewall_managed", {
  zone_id: zoneId,
  rules: [{ expression: "true", action: "execute", action_parameters: { id: wafRulesetId } }],
});

// Layer 3: Rate Limiting (abuse prevention)
await client.zones.rulesets.phases.entrypoint.update("http_ratelimit", {
  zone_id: zoneId,
  rules: [{ expression: "http.request.uri.path eq \"/api/login\"", action: "block", ratelimit: { characteristics: ["ip.src"], period: 60, requests_per_period: 5 } }],
});

// Layer 4: Bot Management (automation detection)
await client.zones.rulesets.phases.entrypoint.update("http_request_sbfm", {
  zone_id: zoneId,
  rules: [{ expression: "cf.bot_management.score lt 30", action: "managed_challenge" }],
});
```

## Cache Strategy for DDoS Mitigation

Exclude query strings from cache key to counter randomized query parameter attacks.

```typescript
const cacheRule = {
  expression: "http.request.uri.path matches \"^/api/\"",
  action: "set_cache_settings",
  action_parameters: {
    cache: true,
    cache_key: { ignore_query_strings_order: true, custom_key: { query_string: { exclude: { all: true } } } },
  },
};

await client.zones.rulesets.phases.entrypoint.update("http_request_cache_settings", { zone_id: zoneId, rules: [cacheRule] });
```

**Rationale**: Attackers randomize query strings (`?random=123456`) to bypass cache. Excluding query params ensures cache hits absorb attack traffic.


