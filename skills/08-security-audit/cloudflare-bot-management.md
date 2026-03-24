---
name: cloudflare-bot-management
description: Comprehensive Cloudflare Bot Management reference covering Workers API (BotManagement interface, JA3/JA4 signals, logpush fields), WAF rule configuration (score thresholds, JS detections, verified bot categories), gotchas (false positives, JSD limitations, plan restrictions), and defense patterns (e-commerce, API, SEO, AI scraper blocking, rate limiting, tarpitting).
domain: security
tags: [cloudflare, bot-management, waf, workers, ja3, ja4, javascript-detections, bot-score]
triggers: cloudflare bot management, bot score, verified bot, JA3, JA4, javascript detections, bot fight mode, super bot fight mode, WAF bot rules, AI scraper blocking
---

# Cloudflare Bot Management

## Workers API: BotManagement Interface

```typescript
interface BotManagement {
  score: number;              // 1-99 (Enterprise), 0 if not computed
  verifiedBot: boolean;       // Is verified bot
  staticResource: boolean;    // Serves static resource
  ja3Hash: string;            // JA3 fingerprint (Enterprise, HTTPS only)
  ja4: string;                // JA4 fingerprint (Enterprise, HTTPS only)
  jsDetection?: {
    passed: boolean;          // Passed JS detection (if enabled)
  };
  detectionIds: number[];     // Heuristic detection IDs
  corporateProxy?: boolean;   // From corporate proxy (Enterprise)
}

// DEPRECATED: Use botManagement.score instead
// request.cf.clientTrustScore (legacy, duplicate of botManagement.score)

// Access via request.cf
import type { IncomingRequestCfProperties } from '@cloudflare/workers-types';

export default {
  async fetch(request: Request): Promise<Response> {
    const cf = request.cf as IncomingRequestCfProperties | undefined;
    const botMgmt = cf?.botManagement;
    
    if (!botMgmt) return fetch(request);
    if (botMgmt.verifiedBot) return fetch(request); // Allow verified bots
    if (botMgmt.score === 1) return new Response('Blocked', { status: 403 });
    if (botMgmt.score < 30) return new Response('Challenge required', { status: 429 });
    
    return fetch(request);
  }
};
```

## WAF Fields Reference

```txt
# Score fields
cf.bot_management.score                    # 0-99 (0 = not computed)
cf.bot_management.verified_bot             # boolean
cf.bot_management.static_resource          # boolean
cf.bot_management.ja3_hash                 # string (Enterprise)
cf.bot_management.ja4                      # string (Enterprise)
cf.bot_management.detection_ids            # array
cf.bot_management.js_detection.passed      # boolean
cf.bot_management.corporate_proxy          # boolean (Enterprise)
cf.verified_bot_category                   # string

# Workers equivalent
request.cf.botManagement.score
request.cf.botManagement.verifiedBot
request.cf.botManagement.ja3Hash
request.cf.botManagement.ja4
request.cf.botManagement.jsDetection.passed
request.cf.verifiedBotCategory
```

## JA4 Signals (Enterprise)

```typescript
import type { IncomingRequestCfProperties } from '@cloudflare/workers-types';

interface JA4Signals {
  // Ratios (0.0-1.0)
  heuristic_ratio_1h?: number;  // Fraction flagged by heuristics
  browser_ratio_1h?: number;    // Fraction from real browsers  
  cache_ratio_1h?: number;      // Fraction hitting cache
  h2h3_ratio_1h?: number;       // Fraction using HTTP/2 or HTTP/3
  // Ranks (relative position in distribution)
  uas_rank_1h?: number;         // User-Agent diversity rank
  paths_rank_1h?: number;       // Path diversity rank
  reqs_rank_1h?: number;        // Request volume rank
  ips_rank_1h?: number;         // IP diversity rank
  // Quantiles (0.0-1.0, percentile in distribution)
  reqs_quantile_1h?: number;    // Request volume quantile
  ips_quantile_1h?: number;     // IP count quantile
}

export default {
  async fetch(request: Request): Promise<Response> {
    const cf = request.cf as IncomingRequestCfProperties | undefined;
    const ja4Signals = cf?.ja4Signals as JA4Signals | undefined;
    
    if (!ja4Signals) return fetch(request); // Not available for HTTP or Worker routing
    
    // Check for anomalous behavior
    // High heuristic_ratio or low browser_ratio = suspicious
    const heuristicRatio = ja4Signals.heuristic_ratio_1h ?? 0;
    const browserRatio = ja4Signals.browser_ratio_1h ?? 0;
    
    if (heuristicRatio > 0.5 || browserRatio < 0.3) {
      return new Response('Suspicious traffic', { status: 403 });
    }
    
    return fetch(request);
  }
};
```

## Common Patterns


## Bot Analytics

### Access Locations

- Dashboard: Security > Bots (old) or Security > Analytics > Bot analysis (new)
- GraphQL API for programmatic access
- Security Events & Security Analytics
- Logpush/Logpull

### Available Data

- **Enterprise BM**: Bot scores (1-99), bot score source, distribution
- **Pro/Business**: Bot groupings (automated, likely automated, likely human)
- Top attributes: IPs, paths, user agents, countries
- Detection sources: Heuristics, ML, AD, JSD
- Verified bot categories

### Time Ranges

- **Enterprise BM**: Up to 1 week at a time, 30 days history
- **Pro/Business**: Up to 72 hours at a time, 30 days history
- Real-time in most cases, adaptive sampling (1-10% depending on volume)

## Logpush Fields

```txt
BotScore              # 1-99 or 0 if not computed
BotScoreSrc           # Detection engine (ML, Heuristics, etc.)
BotTags               # Classification tags
BotDetectionIDs       # Heuristic detection IDs
```

### BotScoreSrc values

- `"Heuristics"` - Known fingerprint
- `"Machine Learning"` - ML model
- `"Anomaly Detection"` - Baseline anomaly
- `"JS Detection"` - JavaScript check
- `"Cloudflare Service"` - Zero Trust
- `"Not Computed"` - Score = 0

Access via Logpush (stream to cloud storage/SIEM), Logpull (API to fetch logs), or GraphQL API (query analytics data).

## Testing with Miniflare

Miniflare provides mock botManagement data for local development:

### Default values

- `score: 99` (human)
- `verifiedBot: false`
- `corporateProxy: false`
- `ja3Hash: "25b4882c2bcb50cd6b469ff28c596742"`
- `staticResource: false`
- `detectionIds: []`

#### Override in tests

```typescript
import { getPlatformProxy } from 'wrangler';

const { cf, dispose } = await getPlatformProxy();
// cf.botManagement is frozen mock object
expect(cf.botManagement.score).toBe(99);
```

For custom test data, mock request.cf in your test setup.

﻿---
name: Bot Management Configuration
description: # Bot Management Configuration
 
 ## Product Tiers
---

# Bot Management Configuration

## Product Tiers (Bot Management Configuration)

**Note:** Dashboard paths differ between old and new UI:

- **New:** Security > Settings > Filter "Bot traffic"
- **Old:** Security > Bots

Both UIs access same settings.

### Bot Score Groupings (Pro/Business)

Pro/Business users see bot score groupings instead of granular 1-99 scores:

| Score | Grouping | Meaning |
| ------- | ---------- | --------- |
| 0 | Not computed | Bot Management didn't run |
| 1 | Automated | Definite bot (heuristic match) |
| 2-29 | Likely automated | Probably bot (ML detection) |
| 30-99 | Likely human | Probably human |
| N/A | Verified bot | Allowlisted good bot |

Enterprise plans get granular 1-99 scores for custom thresholds.

### Bot Fight Mode (Free)

- Auto-blocks definite bots (score=1), excludes verified bots by default
- JavaScript Detections always enabled, no configuration options

### Super Bot Fight Mode (Pro/Business)

```txt
Dashboard: Security > Bots > Configure
- Definitely automated: Block/Challenge
- Likely automated: Challenge/Allow  
- Verified bots: Allow (recommended)
- Static resource protection: ON (may block mail clients)
- JavaScript Detections: Optional
```

### Bot Management for Enterprise

```txt
Dashboard: Security > Bots > Configure > Auto-updates: ON (recommended)

# Template 1: Block definite bots
(cf.bot_management.score eq 1 and not cf.bot_management.verified_bot and not cf.bot_management.static_resource)
Action: Block

# Template 2: Challenge likely bots
(cf.bot_management.score ge 2 and cf.bot_management.score le 29 and not cf.bot_management.verified_bot and not cf.bot_management.static_resource)
Action: Managed Challenge
```

## JavaScript Detections Setup

### Enable via Dashboard

```txt
Security > Bots > Configure Bot Management > JS Detections: ON

Update CSP: script-src 'self' /cdn-cgi/challenge-platform/;
```

### Manual JS Injection (API)

```html
<script>
function jsdOnload() {
  window.cloudflare.jsd.executeOnce({ callback: function(result) { console.log('JSD:', result); } });
}
</script>
<script src="/cdn-cgi/challenge-platform/scripts/jsd/api.js?onload=jsdOnload" async></script>
```

**Use API for**: Selective deployment on specific pages  
**Don't combine**: Zone-wide toggle + manual injection

### WAF Rules for JSD

```txt
# NEVER use on first page visit (needs HTML page first)
(not cf.bot_management.js_detection.passed and http.request.uri.path eq "/api/user/create" and http.request.method eq "POST" and not cf.bot_management.verified_bot)
Action: Managed Challenge (always use Managed Challenge, not Block)
```

### Limitations

- First request won't have JSD data (needs HTML page first)
- Strips ETags from HTML responses
- Not supported with CSP via `<meta>` tags
- Websocket endpoints not supported
- Native mobile apps won't pass
- cf_clearance cookie: 15-minute lifespan, max 4096 bytes

## __cf_bm Cookie

Cloudflare sets `__cf_bm` cookie to smooth bot scores across user sessions:

- **Purpose:** Reduces false positives from score volatility
- **Scope:** Per-domain, HTTP-only
- **Lifespan:** Session duration
- **Privacy:** No PII—only session classification
- **Automatic:** No configuration required

Bot scores for repeat visitors consider session history via this cookie.

## Static Resource Protection

**File Extensions**: ico, jpg, png, jpeg, gif, css, js, tif, tiff, bmp, pict, webp, svg, svgz, class, jar, txt, csv, doc, docx, xls, xlsx, pdf, ps, pls, ppt, pptx, ttf, otf, woff, woff2, eot, eps, ejs, swf, torrent, midi, mid, m3u8, m4a, mp3, ogg, ts  
**Plus**: `/.well-known/` path (all files)

```txt
# Exclude static resources from bot rules
(cf.bot_management.score lt 30 and not cf.bot_management.static_resource)
```

**WARNING**: May block mail clients fetching static images

## JA3/JA4 Fingerprinting (Enterprise)

```txt
# Block specific attack fingerprint
(cf.bot_management.ja3_hash eq "8b8e3d5e3e8b3d5e")

# Allow mobile app by fingerprint
(cf.bot_management.ja4 eq "your_mobile_app_fingerprint")
```

Only available for HTTPS/TLS traffic. Missing for Worker-routed traffic or HTTP requests.

## Verified Bot Categories

```txt
# Allow search engines only
(cf.verified_bot_category eq "Search Engine Crawler")

# Block AI crawlers
(cf.verified_bot_category eq "AI Crawler")
Action: Block

# Or use dashboard: Security > Settings > Bot Management > Block AI Bots
```

| Category | String Value | Example |
| ---------- | -------------- | --------- |
| AI Crawler | `AI Crawler` | GPTBot, Claude-Web |
| AI Assistant | `AI Assistant` | Perplexity-User, DuckAssistBot |
| AI Search | `AI Search` | OAI-SearchBot |
| Accessibility | `Accessibility` | Accessible Web Bot |
| Academic Research | `Academic Research` | Library of Congress |
| Advertising & Marketing | `Advertising & Marketing` | Google Adsbot |
| Aggregator | `Aggregator` | Pinterest, Indeed |
| Archiver | `Archiver` | Internet Archive, CommonCrawl |
| Feed Fetcher | `Feed Fetcher` | RSS/Podcast updaters |
| Monitoring & Analytics | `Monitoring & Analytics` | Uptime monitors |
| Page Preview | `Page Preview` | Facebook/Slack link preview |
| SEO | `Search Engine Optimization` | Google Lighthouse |
| Security | `Security` | Vulnerability scanners |
| Social Media Marketing | `Social Media Marketing` | Brandwatch |
| Webhooks | `Webhooks` | Payment processors |
| Other | `Other` | Uncategorized bots |

## Best Practices

- **ML Auto-Updates**: Enable on Enterprise for latest models
- **Start with Managed Challenge**: Test before blocking
- **Always exclude verified bots**: Use `not cf.bot_management.verified_bot`
- **Exempt corporate proxies**: For B2B traffic via `cf.bot_management.corporate_proxy`
- **Use static resource exception**: Improves performance, reduces overhead

﻿---
name: Bot Management Gotchas
description: # Bot Management Gotchas
 
 ## Common Errors
---

# Bot Management Gotchas

## Common Errors (Bot Management Gotchas)

### "Bot Score = 0"

**Cause:** Bot Management didn't run (internal Cloudflare request, Worker routing to zone (Orange-to-Orange), or request handled before BM (Redirect Rules, etc.))  
**Solution:** Check request flow and ensure Bot Management runs in request lifecycle

### "JavaScript Detections Not Working"

**Cause:** `js_detection.passed` always false or undefined due to: CSP headers don't allow `/cdn-cgi/challenge-platform/`, using on first page visit (needs HTML page first), ad blockers or disabled JS, JSD not enabled in dashboard, or using Block action (must use Managed Challenge)  
**Solution:** Add CSP header `Content-Security-Policy: script-src 'self' /cdn-cgi/challenge-platform/;` and ensure JSD is enabled with Managed Challenge action

### "False Positives (Legitimate Users Blocked)"

**Cause:** Bot detection incorrectly flagging legitimate users  
**Solution:** Check Bot Analytics for affected IPs/paths, identify detection source (ML, Heuristics, etc.), create exception rule like `(cf.bot_management.score lt 30 and http.request.uri.path eq "/problematic-path")` with Action: Skip (Bot Management), or allowlist by IP/ASN/country

### "False Negatives (Bots Not Caught)"

**Cause:** Bots bypassing detection  
**Solution:** Lower score threshold (30 → 50), enable JavaScript Detections, add JA3/JA4 fingerprinting rules, or use rate limiting as fallback

### "Verified Bot Blocked"

**Cause:** Search engine bot blocked by WAF Managed Rules (not just Bot Management)  
**Solution:** Create WAF exception for specific rule ID and verify bot via reverse DNS

### "Yandex Bot Blocked During IP Update"

**Cause:** Yandex updates bot IPs; new IPs unrecognized for 48h during propagation  

#### Solution

1. Check Security Events for specific WAF rule ID blocking Yandex
2. Create WAF exception:

   ```txt
   (http.user_agent contains "YandexBot" and ip.src in {<yandex-ip-range>})
   Action: Skip (WAF Managed Ruleset)
   ```

3. Monitor Bot Analytics for 48h
4. Remove exception after propagation completes

Issue resolves automatically after 48h. Contact Cloudflare Support if persists.

### "JA3/JA4 Missing"

**Cause:** Non-HTTPS traffic, Worker routing traffic, Orange-to-Orange traffic via Worker, or Bot Management skipped  
**Solution:** JA3/JA4 only available for HTTPS/TLS traffic; check request routing

**JA3/JA4 Not User-Unique:** Same browser/library version = same fingerprint

- Don't use for user identification
- Use for client profiling only
- Fingerprints change with browser updates

## Bot Verification Methods

Cloudflare verifies bots via:

1. **Reverse DNS (IP validation):** Traditional method—bot IP resolves to expected domain
2. **Web Bot Auth:** Modern cryptographic verification—faster propagation

When `verifiedBot=true`, bot passed at least one method.

**Inactive verified bots:** IPs removed after 24h of no traffic.

## Detection Engine Behavior

| Engine | Score | Timing | Plan | Notes |
| -------- | ------- | -------- | ------ | ------- |
| Heuristics | Always 1 | Immediate | All | Known fingerprints—overrides ML |
| ML | 1-99 | Immediate | All | Majority of detections |
| Anomaly Detection | Influences | After baseline | Enterprise | Optional, baseline analysis |
| JavaScript Detections | Pass/fail | After JS | Pro+ | Headless browser detection |
| Cloudflare Service | N/A | N/A | Enterprise | Zero Trust internal source |

**Priority:** Heuristics > ML—if heuristic matches, score=1 regardless of ML.

## Limits

| Limit | Value | Notes |
| ------- | ------- | ------- |
| Bot Score = 0 | Means not computed | Not score = 100 |
| First request JSD data | May not be available | JSD data appears on subsequent requests |
| Score accuracy | Not 100% guaranteed | False positives/negatives possible |
| JSD on first HTML page visit | Not supported | Requires subsequent page load |
| JSD requirements | JavaScript-enabled browser | Won't work with JS disabled or ad blockers |
| JSD ETag stripping | Strips ETags from HTML responses | May affect caching behavior |
| JSD CSP compatibility | Requires specific CSP | Not compatible with some CSP configurations |
| JSD meta CSP tags | Not supported | Must use HTTP headers |
| JSD WebSocket support | Not supported | WebSocket endpoints won't work with JSD |
| JSD mobile app support | Native apps won't pass | Only works in browsers |
| JA3/JA4 traffic type | HTTPS/TLS only | Not available for non-HTTPS traffic |
| JA3/JA4 Worker routing | Missing for Worker-routed traffic | Check request routing |
| JA3/JA4 uniqueness | Not unique per user | Shared by clients with same browser/library |
| JA3/JA4 stability | Can change with updates | Browser/library updates affect fingerprints |
| WAF custom rules (Free) | 5 | Varies by plan |
| WAF custom rules (Pro) | 20 | Varies by plan |
| WAF custom rules (Business) | 100 | Varies by plan |
| WAF custom rules (Enterprise) | 1,000+ | Varies by plan |
| Workers CPU time | Varies by plan | Applies to bot logic |
| Bot Analytics sampling | 1-10% adaptive | High-volume zones sampled more aggressively |
| Bot Analytics history | 30 days max | Historical data retention limit |
| CSP requirements for JSD | Must allow `/cdn-cgi/challenge-platform/` | Required for JSD to function |

### Plan Restrictions

| Feature | Free | Pro/Business | Enterprise |
| --------- | ------ | -------------- | ------------ |
| Granular scores (1-99) | No | No | Yes |
| JA3/JA4 | No | No | Yes |
| Anomaly Detection | No | No | Yes |
| Corporate Proxy detection | No | No | Yes |
| Verified bot categories | Limited | Limited | Full |
| Custom WAF rules | 5 | 20/100 | 1,000+ |

﻿---
name: Bot Management Patterns
description: # Bot Management Patterns
 
 ## E-commerce Protection
---

# Bot Management Patterns

## E-commerce Protection (Bot Management Patterns)

```txt
# High security for checkout
(cf.bot_management.score lt 50 and http.request.uri.path in {"/checkout" "/cart/add"} and not cf.bot_management.verified_bot and not cf.bot_management.corporate_proxy)
Action: Managed Challenge
```

## API Protection

```txt
# Protect API with JS detection + score
(http.request.uri.path matches "^/api/" and (cf.bot_management.score lt 30 or not cf.bot_management.js_detection.passed) and not cf.bot_management.verified_bot)
Action: Block
```

## SEO-Friendly Bot Handling

```txt
# Allow search engine crawlers
(cf.bot_management.score lt 30 and not cf.verified_bot_category in {"Search Engine Crawler"})
Action: Managed Challenge
```

## Block AI Scrapers

```txt
# Block training crawlers only (allow AI assistants/search)
(cf.verified_bot_category eq "AI Crawler")
Action: Block

# Block all AI-related bots (training + assistants + search)
(cf.verified_bot_category in {"AI Crawler" "AI Assistant" "AI Search"})
Action: Block

# Allow AI Search, block AI Crawler and AI Assistant
(cf.verified_bot_category in {"AI Crawler" "AI Assistant"})
Action: Block

# Or use dashboard: Security > Settings > Bot Management > Block AI Bots
```

## Rate Limiting by Bot Score

```txt
# Stricter limits for suspicious traffic
(cf.bot_management.score lt 50)
Rate: 10 requests per 10 seconds

(cf.bot_management.score ge 50)
Rate: 100 requests per 10 seconds
```

## Mobile App Allowlisting

```txt
# Identify mobile app by JA3/JA4
(cf.bot_management.ja4 in {"fingerprint1" "fingerprint2"})
Action: Skip (all remaining rules)
```

## Datacenter Detection

```typescript
import type { IncomingRequestCfProperties } from '@cloudflare/workers-types';

// Low score + not corporate proxy = likely datacenter bot
export default {
  async fetch(request: Request): Promise<Response> {
    const cf = request.cf as IncomingRequestCfProperties | undefined;
    const botMgmt = cf?.botManagement;
    
    if (botMgmt?.score && botMgmt.score < 30 && 
        !botMgmt.corporateProxy && !botMgmt.verifiedBot) {
      return new Response('Datacenter traffic blocked', { status: 403 });
    }
    
    return fetch(request);
  }
};
```

## Conditional Delay (Tarpit)

```typescript
import type { IncomingRequestCfProperties } from '@cloudflare/workers-types';

// Add delay proportional to bot suspicion
export default {
  async fetch(request: Request): Promise<Response> {
    const cf = request.cf as IncomingRequestCfProperties | undefined;
    const botMgmt = cf?.botManagement;
    
    if (botMgmt?.score && botMgmt.score < 50 && !botMgmt.verifiedBot) {
      // Delay: 0-2 seconds for scores 50-0
      const delayMs = Math.max(0, (50 - botMgmt.score) * 40);
      await new Promise(r => setTimeout(r, delayMs));
    }
    
    return fetch(request);
  }
};
```

## Layered Defense

```txt
1. Bot Management (score-based)
2. JavaScript Detections (for JS-capable clients)
3. Rate Limiting (fallback protection)
4. WAF Managed Rules (OWASP, etc.)
```

## Progressive Enhancement

```txt
Public content: High threshold (score < 10)
Authenticated: Medium threshold (score < 30)
Sensitive: Low threshold (score < 50) + JSD
```

## Zero Trust for Bots

```txt
1. Default deny (all scores < 30)
2. Allowlist verified bots
3. Allowlist mobile apps (JA3/JA4)
4. Allowlist corporate proxies
5. Allowlist static resources
```

## Workers: Score + JS Detection

```typescript
import type { IncomingRequestCfProperties } from '@cloudflare/workers-types';

export default {
  async fetch(request: Request): Promise<Response> {
    const cf = request.cf as IncomingRequestCfProperties | undefined;
    const botMgmt = cf?.botManagement;
    const url = new URL(request.url);
    
    if (botMgmt?.staticResource) return fetch(request); // Skip static
    
    // API endpoints: require JS detection + good score
    if (url.pathname.startsWith('/api/')) {
      const jsDetectionPassed = botMgmt?.jsDetection?.passed ?? false;
      const score = botMgmt?.score ?? 100;
      
      if (!jsDetectionPassed || score < 30) {
        return new Response('Unauthorized', { status: 401 });
      }
    }
    
    return fetch(request);
  }
};
```

## Rate Limiting by JWT Claim + Bot Score

```txt
# Enterprise: Combine bot score with JWT validation
Rate limiting > Custom rules
- Field: lookup_json_string(http.request.jwt.claims["{config_id}"][0], "sub")
- Matches: user ID claim
- Additional condition: cf.bot_management.score lt 50
```

## WAF Integration Points

- **WAF Custom Rules**: Primary enforcement mechanism
- **Rate Limiting Rules**: Bot score as dimension, stricter limits for low scores
- **Transform Rules**: Pass score to origin via custom header
- **Workers**: Programmatic bot logic, custom scoring algorithms
- **Page Rules / Configuration Rules**: Zone-level overrides, path-specific settings


