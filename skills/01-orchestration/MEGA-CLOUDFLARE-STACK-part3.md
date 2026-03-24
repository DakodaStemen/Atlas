---
name: "C3 CLI Reference (Part 3)"
description: "# C3 CLI Reference - Part 3"
---


## Core SDK Patterns

### Basic Setup

```typescript
import RealtimeKitClient from '@cloudflare/realtimekit';

const meeting = new RealtimeKitClient({ authToken, video: true, audio: true });
meeting.self.on('roomJoined', () => console.log('Joined:', meeting.meta.meetingTitle));
meeting.participants.joined.on('participantJoined', (p) => console.log(`${p.name} joined`));
await meeting.join();
```

### Video Grid & Device Selection

```typescript
// Video grid
function VideoGrid({ meeting }) {
  const [participants, setParticipants] = useState([]);
  useEffect(() => {
    const update = () => setParticipants(meeting.participants.joined.toArray());
    meeting.participants.joined.on('participantJoined', update);
    meeting.participants.joined.on('participantLeft', update);
    update();
    return () => { meeting.participants.joined.off('participantJoined', update); meeting.participants.joined.off('participantLeft', update); };
  }, [meeting]);
  return <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))' }}>
    {participants.map(p => <VideoTile key={p.id} participant={p} />)}
  </div>;
}

function VideoTile({ participant }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    if (videoRef.current && participant.videoTrack) videoRef.current.srcObject = new MediaStream([participant.videoTrack]);
  }, [participant.videoTrack]);
  return <div><video ref={videoRef} autoPlay playsInline muted /><div>{participant.name}</div></div>;
}

// Device selection
const devices = await meeting.self.getAllDevices();
const switchCamera = (deviceId: string) => {
  const device = devices.find(d => d.deviceId === deviceId);
  if (device) await meeting.self.setDevice(device);
};
```

## React Hooks (Official)

```typescript
import { useRealtimeKitClient, useRealtimeKitSelector } from '@cloudflare/realtimekit-react-ui';

function MyComponent() {
  const [meeting, initMeeting] = useRealtimeKitClient();
  const audioEnabled = useRealtimeKitSelector(m => m.self.audioEnabled);
  const participantCount = useRealtimeKitSelector(m => m.participants.joined.size());
  
  useEffect(() => { initMeeting({ authToken: '<token>' }); }, []);
  
  return <div>
    <button onClick={() => meeting?.self.enableAudio()}>{audioEnabled ? 'Mute' : 'Unmute'}</button>
    <span>{participantCount} participants</span>
  </div>;
}
```

**Benefits:** Automatic re-renders, memoized selectors, type-safe

## Waitlist Handling

```typescript
// Monitor waitlist
meeting.participants.waitlisted.on('participantJoined', (participant) => {
  console.log(`${participant.name} is waiting`);
  // Show admin UI to approve/reject
});

// Approve from waitlist (backend only)
await fetch(
  `https://api.cloudflare.com/client/v4/accounts/${accountId}/realtime/kit/${appId}/meetings/${meetingId}/active-session/waitlist/approve`,
  {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${apiToken}` },
    body: JSON.stringify({ user_ids: [participant.userId] })
  }
);

// Client receives automatic transition when approved
meeting.self.on('roomJoined', () => console.log('Approved and joined'));
```

## Audio-Only Mode

```typescript
const meeting = new RealtimeKitClient({
  authToken: '<token>',
  video: false,  // Disable video
  audio: true,
  mediaConfiguration: {
    audio: {
      echoCancellation: true,
      noiseSuppression: true,
      autoGainControl: true
    }
  }
});

// Use audio grid component
import { RtkAudioGrid } from '@cloudflare/realtimekit-react-ui';
<RtkAudioGrid meeting={meeting} />
```

## Addon System

```typescript
// List available addons
meeting.plugins.all.forEach(plugin => {
  console.log(plugin.id, plugin.name, plugin.active);
});

// Activate collaborative app
await meeting.plugins.activate('whiteboard-addon-id');

// Listen for activations
meeting.plugins.on('pluginActivated', ({ plugin }) => {
  console.log(`${plugin.name} activated`);
});

// Deactivate
await meeting.plugins.deactivate();
```

## Backend Integration

### Token Generation (Workers)

```typescript
export interface Env { CLOUDFLARE_API_TOKEN: string; CLOUDFLARE_ACCOUNT_ID: string; REALTIMEKIT_APP_ID: string; }

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    
    if (url.pathname === '/api/join-meeting') {
      const { meetingId, userName, presetName } = await request.json();
      const response = await fetch(
        `https://api.cloudflare.com/client/v4/accounts/${env.CLOUDFLARE_ACCOUNT_ID}/realtime/kit/${env.REALTIMEKIT_APP_ID}/meetings/${meetingId}/participants`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${env.CLOUDFLARE_API_TOKEN}` },
          body: JSON.stringify({ name: userName, preset_name: presetName })
        }
      );
      const data = await response.json();
      return Response.json({ authToken: data.result.authToken });
    }
    
    return new Response('Not found', { status: 404 });
  }
};
```

## Best Practices

### Security

1. **Never expose API tokens client-side** - Generate participant tokens server-side only
2. **Don't reuse participant tokens** - Generate fresh token per session, use refresh endpoint if expired
3. **Use custom participant IDs** - Map to your user system for cross-session tracking

### Performance

1. **Event-driven updates** - Listen to events, don't poll. Use `toArray()` only when needed
2. **Media quality constraints** - Set appropriate resolution/bitrate limits based on network conditions
3. **Device management** - Enable `autoSwitchAudioDevice` for better UX, handle device list updates

### Architecture

1. **Separate Apps for environments** - staging vs production to prevent data mixing
2. **Preset strategy** - Create presets at App level, reuse across meetings
3. **Token management** - Backend generates tokens, frontend receives via authenticated endpoint

## In This Reference

- [README.md](README.md) - Overview, core concepts, quick start
- [configuration.md](configuration.md) - SDK config, presets, wrangler setup
- [api.md](api.md) - Client SDK APIs, REST endpoints
- [gotchas.md](gotchas.md) - Common issues, troubleshooting, limits


---

<!-- merged from: smart-placement-configuration.md -->

﻿---
name: Smart Placement Configuration
description: # Smart Placement Configuration
 
 ## wrangler.jsonc Setup
---

# Smart Placement Configuration

## wrangler.jsonc Setup (Smart Placement Configuration)

```jsonc
{
  "$schema": "./node_modules/wrangler/config-schema.json",
  "placement": {
    "mode": "smart"
  }
}
```

## Placement Mode Values

| Mode | Behavior |
| ------ | ---------- |
| `"smart"` | Enable Smart Placement - automatic optimization based on traffic analysis |
| `"off"` | Explicitly disable Smart Placement - always run at edge closest to user |
| Not specified | Default behavior - run at edge closest to user (same as `"off"`) |

**Note:** Smart Placement vs Explicit Placement are separate features. Smart Placement (`mode: "smart"`) uses automatic analysis. For manual placement control, see explicit placement options (`region`, `host`, `hostname` fields - not covered in this reference).

## Frontend + Backend Split Configuration

### Frontend Worker (No Smart Placement)

```jsonc
// frontend-worker/wrangler.jsonc
{
  "name": "frontend",
  "main": "frontend-worker.ts",
  // No "placement" - runs at edge
  "services": [
    {
      "binding": "BACKEND",
      "service": "backend-api"
    }
  ]
}
```

### Backend Worker (Smart Placement Enabled)

```jsonc
// backend-api/wrangler.jsonc
{
  "name": "backend-api",
  "main": "backend-worker.ts",
  "placement": {
    "mode": "smart"
  },
  "d1_databases": [
    {
      "binding": "DATABASE",
      "database_id": "xxx"
    }
  ]
}
```

## Requirements & Limitations

### Requirements

- **Wrangler version:** 2.20.0+
- **Analysis time:** Up to 15 minutes
- **Traffic requirements:** Consistent multi-location traffic
- **Workers plan:** All plans (Free, Paid, Enterprise)

### What Smart Placement Affects

#### CRITICAL LIMITATION - Smart Placement ONLY Affects `fetch` Handlers

Smart Placement is fundamentally limited to Workers with default `fetch` handlers. This is a key architectural constraint.

- ✅ **Affects:** `fetch` event handlers ONLY (the default export's fetch method)
- ❌ **Does NOT affect:**
  - RPC methods (Service Bindings with `WorkerEntrypoint` - see example below)
  - Named entrypoints (exports other than `default`)
  - Workers without `fetch` handlers
  - Queue consumers, scheduled handlers, or other event types

#### Example - Smart Placement ONLY affects `fetch`

```typescript
// ✅ Smart Placement affects this:
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // This runs close to backend when Smart Placement enabled
    const data = await env.DATABASE.prepare('SELECT * FROM users').all();
    return Response.json(data);
  }
}

// ❌ Smart Placement DOES NOT affect these:
export class MyRPC extends WorkerEntrypoint {
  async myMethod() { 
    // This ALWAYS runs at edge, Smart Placement has NO EFFECT
    const data = await this.env.DATABASE.prepare('SELECT * FROM users').all();
    return data;
  }
}

export async function scheduled(event: ScheduledEvent, env: Env) {
  // NOT affected by Smart Placement
}
```

**Consequence:** If your backend logic uses RPC methods (`WorkerEntrypoint`), Smart Placement cannot optimize those calls. You must use fetch-based patterns for Smart Placement to work.

**Solution:** Convert RPC methods to fetch endpoints, or use a wrapper Worker with `fetch` handler that calls your backend RPC (though this adds latency).

### Baseline Traffic

Smart Placement automatically routes 1% of requests WITHOUT optimization as baseline for performance comparison.

### Validation Rules

#### Mutually exclusive fields

- `mode` cannot be used with explicit placement fields (`region`, `host`, `hostname`)
- Choose either Smart Placement OR explicit placement, not both

```jsonc
// ✅ Valid - Smart Placement
{ "placement": { "mode": "smart" } }

// ✅ Valid - Explicit Placement (different feature)
{ "placement": { "region": "us-east1" } }

// ❌ Invalid - Cannot combine
{ "placement": { "mode": "smart", "region": "us-east1" } }
```

## Dashboard Configuration

### Workers & Pages**→ Select Worker → **Settings** → **General** → **Placement: Smart** → Wait 15min → Check**Metrics

## TypeScript Types

```typescript
interface Env {
  BACKEND: Fetcher;
  DATABASE: D1Database;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const data = await env.DATABASE.prepare('SELECT * FROM table').all();
    return Response.json(data);
  }
} satisfies ExportedHandler<Env>;
```

## Cloudflare Pages/Assets Warning

**CRITICAL PERFORMANCE ISSUE:** Enabling Smart Placement with `assets.run_worker_first = true` in Pages projects **severely degrades asset serving performance**. This is one of the most common misconfigurations.

### Why this is bad

- Smart Placement routes ALL requests (including static assets) away from edge to remote locations
- Static assets (HTML, CSS, JS, images) should ALWAYS be served from edge closest to user
- Result: 2-5x slower asset loading times, poor user experience

**Problem:** Smart Placement routes asset requests away from edge, but static assets should always be served from edge closest to user.

#### Solutions (in order of preference)

1. **Recommended:** Split into separate Workers (frontend at edge + backend with Smart Placement)
2. Set `"mode": "off"` to explicitly disable Smart Placement for Pages/Assets Workers
3. Use `assets.run_worker_first = false` (serves assets first, bypasses Worker for static content)

```jsonc
// ❌ BAD - Degrades asset performance by 2-5x
{
  "name": "pages-app",
  "placement": { "mode": "smart" },
  "assets": { "run_worker_first": true }
}

// ✅ GOOD - Frontend at edge, backend optimized
// frontend-worker/wrangler.jsonc
{
  "name": "frontend",
  "assets": { "run_worker_first": true }
  // No placement - runs at edge
}

// backend-worker/wrangler.jsonc
{
  "name": "backend-api",
  "placement": { "mode": "smart" },
  "d1_databases": [{ "binding": "DB", "database_id": "xxx" }]
}
```

**Key takeaway:** Never enable Smart Placement on Workers that serve static assets with `run_worker_first = true`.

## Local Development

Smart Placement does NOT work in `wrangler dev` (local only). Test by deploying: `wrangler deploy --env staging`


---

<!-- merged from: smart-placement-patterns.md -->

﻿---
name: Smart Placement Patterns
description: # Smart Placement Patterns
 
 ## Backend Worker with Database Access
---

# Smart Placement Patterns

## Backend Worker with Database Access (Smart Placement Patterns)

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const user = await env.DATABASE.prepare('SELECT * FROM users WHERE id = ?').bind(userId).first();
    const orders = await env.DATABASE.prepare('SELECT * FROM orders WHERE user_id = ?').bind(userId).all();
    return Response.json({ user, orders });
  }
};
```

```jsonc
{ "placement": { "mode": "smart" }, "d1_databases": [{ "binding": "DATABASE", "database_id": "xxx" }] }
```

## Frontend + Backend Split (Service Bindings)

**Frontend:** Runs at edge for fast user response
**Backend:** Smart Placement runs close to database

```typescript
// Frontend Worker - routes requests to backend
interface Env {
  BACKEND: Fetcher;  // Service Binding to backend Worker
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (new URL(request.url).pathname.startsWith('/api/')) {
      return env.BACKEND.fetch(request);  // Forward to backend
    }
    return new Response('Frontend content');
  }
};

// Backend Worker - database operations
interface BackendEnv {
  DATABASE: D1Database;
}

export default {
  async fetch(request: Request, env: BackendEnv): Promise<Response> {
    const data = await env.DATABASE.prepare('SELECT * FROM table').all();
    return Response.json(data);
  }
};
```

**CRITICAL:** Use fetch-based Service Bindings (shown above). If using RPC with `WorkerEntrypoint`, Smart Placement will NOT optimize those method calls - only `fetch` handlers are affected.

**RPC vs Fetch - CRITICAL:** Smart Placement ONLY works with fetch-based bindings, NOT RPC.

```typescript
// ❌ RPC - Smart Placement has NO EFFECT on backend RPC methods
export class BackendRPC extends WorkerEntrypoint {
  async getData() {
    // ALWAYS runs at edge, Smart Placement ignored
    return await this.env.DATABASE.prepare('SELECT * FROM table').all();
  }
}

// ✅ Fetch - Smart Placement WORKS
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // Runs close to DATABASE when Smart Placement enabled
    const data = await env.DATABASE.prepare('SELECT * FROM table').all();
    return Response.json(data);
  }
};
```

## External API Integration

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const apiUrl = 'https://api.partner.com';
    const headers = { 'Authorization': `Bearer ${env.API_KEY}` };
    
    const [profile, transactions] = await Promise.all([
      fetch(`${apiUrl}/profile`, { headers }),
      fetch(`${apiUrl}/transactions`, { headers })
    ]);
    
    return Response.json({ 
      profile: await profile.json(), 
      transactions: await transactions.json()
    });
  }
};
```

## SSR / API Gateway Pattern

```typescript
// Frontend (edge) - auth/routing close to user
export default {
  async fetch(request: Request, env: Env) {
    if (!request.headers.get('Authorization')) {
      return new Response('Unauthorized', { status: 401 });
    }
    const data = await env.BACKEND.fetch(request);
    return new Response(renderPage(await data.json()), { 
      headers: { 'Content-Type': 'text/html' } 
    });
  }
};

// Backend (Smart Placement) - DB operations close to data
export default {
  async fetch(request: Request, env: Env) {
    const data = await env.DATABASE.prepare('SELECT * FROM pages WHERE id = ?').bind(pageId).first();
    return Response.json(data);
  }
};
```

## Durable Objects with Smart Placement

**Key principle:** Smart Placement does NOT control WHERE Durable Objects run. DOs always run in their designated region (based on jurisdiction or smart location hints).

**What Smart Placement DOES affect:** The location of the coordinator Worker's `fetch` handler that makes calls to multiple DOs.

**Pattern:** Enable Smart Placement on coordinator Worker that aggregates data from multiple DOs:

```typescript
// Worker with Smart Placement - aggregates data from multiple DOs
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const userId = new URL(request.url).searchParams.get('user');
    
    // Get DO stubs
    const userDO = env.USER_DO.get(env.USER_DO.idFromName(userId));
    const analyticsID = env.ANALYTICS_DO.idFromName(`analytics-${userId}`);
    const analyticsDO = env.ANALYTICS_DO.get(analyticsID);
    
    // Fetch from multiple DOs
    const [userData, analyticsData] = await Promise.all([
      userDO.fetch(new Request('https://do/profile')),
      analyticsDO.fetch(new Request('https://do/stats'))
    ]);
    
    return Response.json({
      user: await userData.json(),
      analytics: await analyticsData.json()
    });
  }
};
```

```jsonc
// wrangler.jsonc
{
  "placement": { "mode": "smart" },
  "durable_objects": {
    "bindings": [
      { "name": "USER_DO", "class_name": "UserDO" },
      { "name": "ANALYTICS_DO", "class_name": "AnalyticsDO" }
    ]
  }
}
```

### When this helps

- Worker's `fetch` handler runs closer to DO regions, reducing network latency for multiple DO calls
- Most beneficial when DOs are geographically concentrated or in specific jurisdictions
- Helps when coordinator makes many sequential or parallel DO calls

#### When this DOESN'T help

- DOs are globally distributed (no single optimal Worker location)
- Worker only calls a single DO
- DO calls are infrequent or cached

## Best Practices

- Split full-stack apps: frontend at edge, backend with Smart Placement
- Use fetch-based Service Bindings (not RPC)
- Enable for backend logic: APIs, data aggregation, DB operations
- Don't enable for: static content, edge logic, RPC methods, Pages with `run_worker_first`
- Wait 15+ min for analysis, verify `placement_status = SUCCESS`


---

<!-- merged from: snippets-patterns.md -->

﻿---
name: Snippets Patterns
description: # Snippets Patterns
 
 ## Security Headers
---

# Snippets Patterns

## Security Headers (Snippets Patterns)

```javascript
export default {
  async fetch(request) {
    const response = await fetch(request);
    const newResponse = new Response(response.body, response);
    newResponse.headers.set("X-Frame-Options", "DENY");
    newResponse.headers.set("X-Content-Type-Options", "nosniff");
    newResponse.headers.delete("X-Powered-By");
    return newResponse;
  }
}
```

**Rule:** `true` (all requests)

## Geo-Based Routing

```javascript
export default {
  async fetch(request) {
    const country = request.cf.country;
    if (["GB", "DE", "FR"].includes(country)) {
      const url = new URL(request.url);
      url.hostname = url.hostname.replace(".com", ".eu");
      return Response.redirect(url.toString(), 302);
    }
    return fetch(request);
  }
}
```

## A/B Testing

```javascript
export default {
  async fetch(request) {
    const cookies = request.headers.get("Cookie") || "";
    let variant = cookies.match(/ab_test=([AB])/)?.[1] || (Math.random() < 0.5 ? "A" : "B");
    
    const req = new Request(request);
    req.headers.set("X-Variant", variant);
    const response = await fetch(req);
    
    if (!cookies.includes("ab_test=")) {
      const newResponse = new Response(response.body, response);
      newResponse.headers.append("Set-Cookie", `ab_test=${variant}; Path=/; Secure`);
      return newResponse;
    }
    return response;
  }
}
```

## Bot Detection

```javascript
export default {
  async fetch(request) {
    const botScore = request.cf.botManagement?.score;
    if (botScore && botScore < 30) return new Response("Denied", { status: 403 });
    return fetch(request);
  }
}
```

**Requires:** Bot Management plan

## API Auth Header Injection

```javascript
export default {
  async fetch(request) {
    if (new URL(request.url).pathname.startsWith("/api/")) {
      const req = new Request(request);
      req.headers.set("X-Internal-Auth", "secret_token");
      req.headers.delete("Authorization");
      return fetch(req);
    }
    return fetch(request);
  }
}
```

## CORS Headers

```javascript
export default {
  async fetch(request) {
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE",
          "Access-Control-Allow-Headers": "Content-Type, Authorization"
        }
      });
    }
    const response = await fetch(request);
    const newResponse = new Response(response.body, response);
    newResponse.headers.set("Access-Control-Allow-Origin", "*");
    return newResponse;
  }
}
```

## Maintenance Mode

```javascript
export default {
  async fetch(request) {
    if (request.headers.get("X-Bypass-Token") === "admin") return fetch(request);
    return new Response("<h1>Maintenance</h1>", {
      status: 503,
      headers: { "Content-Type": "text/html", "Retry-After": "3600" }
    });
  }
}
```

## Pattern Selection

| Pattern | Complexity | Use Case |
| --------- | ----------- | ---------- |
| Security Headers | Low | All sites |
| Geo-Routing | Low | Regional content |
| A/B Testing | Medium | Experiments |
| Bot Detection | Medium | Requires Bot Management |
| API Auth | Low | Backend protection |
| CORS | Low | API endpoints |
| Maintenance | Low | Deployments |


---

<!-- merged from: stream-configuration.md -->

﻿---
name: Stream Configuration
description: # Stream Configuration
 
 Setup, environment variables, and wrangler configuration.
---

# Stream Configuration

Setup, environment variables, and wrangler configuration.

## Installation

```bash
# Official Cloudflare SDK (Node.js, Workers, Pages)
npm install cloudflare

# React component library
npm install @cloudflare/stream-react

# TUS resumable uploads (large files)
npm install tus-js-client
```

## Environment Variables

```bash
# Required
CF_ACCOUNT_ID=your-account-id
CF_API_TOKEN=your-api-token

# For signed URLs (high volume)
STREAM_KEY_ID=your-key-id
STREAM_JWK=base64-encoded-jwk

# For webhooks
WEBHOOK_SECRET=your-webhook-secret

# Customer subdomain (from dashboard)
STREAM_CUSTOMER_CODE=your-customer-code
```

## Wrangler Configuration

```jsonc
{
  "name": "stream-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01", // Use current date for new projects
  "vars": {
    "CF_ACCOUNT_ID": "your-account-id"
  }
  // Store secrets: wrangler secret put CF_API_TOKEN
  // wrangler secret put STREAM_KEY_ID
  // wrangler secret put STREAM_JWK
  // wrangler secret put WEBHOOK_SECRET
}
```

## Signing Keys (High Volume)

Create once for self-signing tokens (thousands of daily users).

### Create key

```bash
curl -X POST \
  "https://api.cloudflare.com/client/v4/accounts/{account_id}/stream/keys" \
  -H "Authorization: Bearer <API_TOKEN>"

# Save `id` and `jwk` (base64) from response
```

#### Store in secrets

```bash
wrangler secret put STREAM_KEY_ID
wrangler secret put STREAM_JWK
```

## Webhooks

### Setup webhook URL

```bash
curl -X PUT \
  "https://api.cloudflare.com/client/v4/accounts/{account_id}/stream/webhook" \
  -H "Authorization: Bearer <API_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"notificationUrl": "https://your-worker.workers.dev/webhook"}'

# Save the returned `secret` for signature verification
```

#### Store secret

```bash
wrangler secret put WEBHOOK_SECRET
```

## Direct Upload / Live / Watermark Config

```typescript
// Direct upload
const uploadConfig = {
  maxDurationSeconds: 3600,
  expiry: new Date(Date.now() + 3600000).toISOString(),
  requireSignedURLs: true,
  allowedOrigins: ['https://yourdomain.com'],
  meta: { creator: 'user-123' }
};

// Live input
const liveConfig = {
  recording: { mode: 'automatic', timeoutSeconds: 30 },
  deleteRecordingAfterDays: 30
};

// Watermark
const watermark = {
  name: 'Logo', opacity: 0.7, padding: 20,
  position: 'lowerRight', scale: 0.15
};
```

## Access Rules & Player Config

```typescript
// Access rules: allow US/CA, block CN/RU, or IP allowlist
const geoRestrict = [
  { type: 'ip.geoip.country', action: 'allow', country: ['US', 'CA'] },
  { type: 'any', action: 'block' }
];

// Player params for iframe
const playerParams = new URLSearchParams({
  autoplay: 'true', muted: 'true', preload: 'auto', defaultTextTrack: 'en'
});
```

## In This Reference

- [README.md](./README.md) - Overview and quick start
- [api.md](./api.md) - On-demand video APIs
- [api-live.md](./api-live.md) - Live streaming APIs
- [patterns.md](./patterns.md) - Full-stack flows, best practices
- [gotchas.md](./gotchas.md) - Error codes, troubleshooting

## See Also

- [wrangler](../wrangler/) - Wrangler CLI and configuration
- [workers](../workers/) - Deploy Stream APIs in Workers


---

<!-- merged from: stream-patterns.md -->

﻿---
name: Stream Patterns
description: # Stream Patterns
 
 Common workflows, full-stack flows, and best practices.
---

# Stream Patterns

Common workflows, full-stack flows, and best practices.

## React Stream Player

`npm install @cloudflare/stream-react`

```tsx
import { Stream } from '@cloudflare/stream-react';

export function VideoPlayer({ videoId, token }: { videoId: string; token?: string }) {
  return <Stream controls src={token ? `${videoId}?token=${token}` : videoId} responsive />;
}
```

## Full-Stack Upload Flow

### Backend API (Workers/Pages)

```typescript
import Cloudflare from 'cloudflare';

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { videoName } = await request.json();
    const client = new Cloudflare({ apiToken: env.CF_API_TOKEN });
    const { uploadURL, uid } = await client.stream.directUpload.create({
      account_id: env.CF_ACCOUNT_ID,
      maxDurationSeconds: 3600,
      requireSignedURLs: true,
      meta: { name: videoName }
    });
    return Response.json({ uploadURL, uid });
  }
};
```

#### Frontend component

```tsx
import { useState } from 'react';

export function VideoUploader() {
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  
  async function handleUpload(file: File) {
    setUploading(true);
    const { uploadURL, uid } = await fetch('/api/upload-url', {
      method: 'POST',
      body: JSON.stringify({ videoName: file.name })
    }).then(r => r.json());
    
    const xhr = new XMLHttpRequest();
    xhr.upload.onprogress = (e) => setProgress((e.loaded / e.total) * 100);
    xhr.onload = () => { setUploading(false); window.location.href = `/videos/${uid}`; };
    xhr.open('POST', uploadURL);
    const formData = new FormData();
    formData.append('file', file);
    xhr.send(formData);
  }
  
  return (
    <div>
      <input type="file" accept="video/*" onChange={(e) => e.target.files?.[0] && handleUpload(e.target.files[0])} disabled={uploading} />
      {uploading && <progress value={progress} max={100} />}
    </div>
  );
}
```

## TUS Resumable Upload

For large files (>500MB). `npm install tus-js-client`

```typescript
import * as tus from 'tus-js-client';

async function uploadWithTUS(file: File, uploadURL: string, onProgress?: (pct: number) => void) {
  return new Promise<string>((resolve, reject) => {
    const upload = new tus.Upload(file, {
      endpoint: uploadURL,
      retryDelays: [0, 3000, 5000, 10000, 20000],
      chunkSize: 50 * 1024 * 1024,
      metadata: { filename: file.name, filetype: file.type },
      onError: reject,
      onProgress: (up, total) => onProgress?.((up / total) * 100),
      onSuccess: () => resolve(upload.url?.split('/').pop() || '')
    });
    upload.start();
  });
}
```

## Video State Polling

```typescript
async function waitForVideoReady(client: Cloudflare, accountId: string, videoId: string) {
  for (let i = 0; i < 60; i++) {
    const video = await client.stream.videos.get(videoId, { account_id: accountId });
    if (video.readyToStream || video.status.state === 'error') return video;
    await new Promise(resolve => setTimeout(resolve, 5000));
  }
  throw new Error('Video processing timeout');
}
```

## Webhook Handler

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const signature = request.headers.get('Webhook-Signature');
    const body = await request.text();
    if (!signature || !await verifyWebhook(signature, body, env.WEBHOOK_SECRET)) {
      return new Response('Unauthorized', { status: 401 });
    }
    const payload = JSON.parse(body);
    if (payload.readyToStream) console.log(`Video ${payload.uid} ready`);
    return new Response('OK');
  }
};

async function verifyWebhook(sig: string, body: string, secret: string): Promise<boolean> {
  const parts = Object.fromEntries(sig.split(',').map(p => p.split('=')));
  const timestamp = parseInt(parts.time || '0', 10);
  if (Math.abs(Date.now() / 1000 - timestamp) > 300) return false;
  
  const key = await crypto.subtle.importKey(
    'raw', new TextEncoder().encode(secret), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']
  );
  const computed = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(`${timestamp}.${body}`));
  const hex = Array.from(new Uint8Array(computed), b => b.toString(16).padStart(2, '0')).join('');
  return hex === parts.sig1;
}
```

## Self-Sign JWT (High Volume Tokens)

For >1k tokens/day. Prerequisites: Create signing key (see configuration.md).

```typescript
async function selfSignToken(keyId: string, jwkBase64: string, videoId: string, expiresIn = 3600) {
  const key = await crypto.subtle.importKey(
    'jwk', JSON.parse(atob(jwkBase64)), { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['sign']
  );
  const now = Math.floor(Date.now() / 1000);
  const header = btoa(JSON.stringify({ alg: 'RS256', kid: keyId })).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
  const payload = btoa(JSON.stringify({ sub: videoId, kid: keyId, exp: now + expiresIn, nbf: now }))
    .replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
  const message = `${header}.${payload}`;
  const sig = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key, new TextEncoder().encode(message));
  const b64Sig = btoa(String.fromCharCode(...new Uint8Array(sig))).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
  return `${message}.${b64Sig}`;
}

// With access rules (geo-restriction)
const payloadWithRules = {
  sub: videoId, kid: keyId, exp: now + 3600, nbf: now,
  accessRules: [{ type: 'ip.geoip.country', action: 'allow', country: ['US'] }]
};
```

## Best Practices

- **Use Direct Creator Uploads** - Avoid proxying through servers
- **Enable requireSignedURLs** - Control private content access
- **Self-sign tokens at scale** - Use signing keys for >1k/day
- **Set allowedOrigins** - Prevent hotlinking
- **Use webhooks over polling** - Efficient status updates
- **Set maxDurationSeconds** - Prevent abuse
- **Enable live recordings** - Auto VOD after stream

## In This Reference

- [README.md](./README.md) - Overview and quick start
- [configuration.md](./configuration.md) - Setup and config
- [api.md](./api.md) - On-demand video APIs
- [api-live.md](./api-live.md) - Live streaming APIs
- [gotchas.md](./gotchas.md) - Error codes, troubleshooting

## See Also

- [workers](../workers/) - Deploy Stream APIs in Workers
- [pages](../pages/) - Integrate Stream with Pages


---

<!-- merged from: turn-configuration.md -->

﻿---
name: TURN Configuration
description: # TURN Configuration
 
 Setup and configuration for Cloudflare TURN service in Workers and applications.
---

# TURN Configuration

Setup and configuration for Cloudflare TURN service in Workers and applications.

## Environment Variables

```bash
# .env
CLOUDFLARE_ACCOUNT_ID=your_account_id
CLOUDFLARE_API_TOKEN=your_api_token
TURN_KEY_ID=your_turn_key_id
TURN_KEY_SECRET=your_turn_key_secret
```

Validate with zod:

```typescript
import { z } from 'zod';

const envSchema = z.object({
  CLOUDFLARE_ACCOUNT_ID: z.string().min(1),
  CLOUDFLARE_API_TOKEN: z.string().min(1),
  TURN_KEY_ID: z.string().min(1),
  TURN_KEY_SECRET: z.string().min(1)
});

export const config = envSchema.parse(process.env);
```

## wrangler.jsonc

```jsonc
{
  "name": "turn-credentials-api",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",
  "vars": {
    "TURN_KEY_ID": "your-turn-key-id"  // Non-sensitive, can be in vars
  },
  "env": {
    "production": {
      "kv_namespaces": [
        {
          "binding": "CREDENTIALS_CACHE",
          "id": "your-kv-namespace-id"
        }
      ]
    }
  }
}
```

### Store secrets separately

```bash
wrangler secret put TURN_KEY_SECRET
```

## Cloudflare Worker Integration

### Worker Binding Types

```typescript
interface Env {
  TURN_KEY_ID: string;
  TURN_KEY_SECRET: string;
  CREDENTIALS_CACHE?: KVNamespace;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // See patterns.md for implementation
  }
}
```

### Basic Worker Example

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.url.endsWith('/turn-credentials')) {
      // Validate client auth
      const authHeader = request.headers.get('Authorization');
      if (!authHeader) {
        return new Response('Unauthorized', { status: 401 });
      }

      const response = await fetch(
        `https://rtc.live.cloudflare.com/v1/turn/keys/${env.TURN_KEY_ID}/credentials/generate`,
        {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${env.TURN_KEY_SECRET}`,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ ttl: 3600 })
        }
      );

      if (!response.ok) {
        return new Response('Failed to generate credentials', { status: 500 });
      }

      const data = await response.json();

      // Filter port 53 for browser clients
      const filteredUrls = data.iceServers.urls.filter(
        (url: string) => !url.includes(':53')
      );

      return Response.json({
        iceServers: [
          { urls: 'stun:stun.cloudflare.com:3478' },
          {
            urls: filteredUrls,
            username: data.iceServers.username,
            credential: data.iceServers.credential
          }
        ]
      });
    }

    return new Response('Not found', { status: 404 });
  }
};
```

## IP Allowlisting (Enterprise/Firewall)

For strict firewalls, allowlist these IPs for `turn.cloudflare.com`:

| Type | Address | Protocol |
| ------ | --------- | ---------- |
| IPv4 | 141.101.90.1/32 | All |
| IPv4 | 162.159.207.1/32 | All |
| IPv6 | 2a06:98c1:3200::1/128 | All |
| IPv6 | 2606:4700:48::1/128 | All |

**IMPORTANT**: These IPs may change with 14-day notice. Monitor DNS:

```bash
# Check A and AAAA records
dig turn.cloudflare.com A
dig turn.cloudflare.com AAAA
```

Set up automated monitoring to detect IP changes and update allowlists within 14 days.

## IPv6 Support

- **Client-to-TURN**: Both IPv4 and IPv6 supported
- **Relay addresses**: IPv4 only (no RFC 6156 support)
- **TCP relaying**: Not supported (RFC 6062)

Clients can connect via IPv6, but relayed traffic uses IPv4 addresses.

## TLS Configuration

### Supported TLS Versions

- TLS 1.1
- TLS 1.2
- TLS 1.3

### Recommended Ciphers (TLS 1.3)

- AEAD-AES128-GCM-SHA256
- AEAD-AES256-GCM-SHA384
- AEAD-CHACHA20-POLY1305-SHA256

### Recommended Ciphers (TLS 1.2)

- ECDHE-ECDSA-AES128-GCM-SHA256
- ECDHE-RSA-AES128-GCM-SHA256
- ECDHE-RSA-AES128-SHA (also TLS 1.1)
- AES128-GCM-SHA256

## See Also

- [api.md](./api.md) - TURN key creation, credential generation API
- [patterns.md](./patterns.md) - Full Worker implementation patterns
- [gotchas.md](./gotchas.md) - Security best practices, troubleshooting


---

<!-- merged from: turn-implementation-patterns.md -->

﻿---
name: TURN Implementation Patterns
description: # TURN Implementation Patterns
 
 Production-ready patterns for implementing Cloudflare TURN in WebRTC applications.
---

# TURN Implementation Patterns

Production-ready patterns for implementing Cloudflare TURN in WebRTC applications.

## Prerequisites

Before implementing these patterns, ensure you have:

- TURN key created: see [api.md#create-turn-key](./api.md#create-turn-key)
- Worker configured: see [configuration.md#cloudflare-worker-integration](./configuration.md#cloudflare-worker-integration)

## Basic TURN Configuration (Browser)

```typescript
interface RTCIceServer {
  urls: string | string[];
  username?: string;
  credential?: string;
  credentialType?: "password" | "oauth";
}

async function getTURNConfig(): Promise<RTCIceServer[]> {
  const response = await fetch('/api/turn-credentials');
  const data = await response.json();
  
  return [
    {
      urls: 'stun:stun.cloudflare.com:3478'
    },
    {
      urls: [
        'turn:turn.cloudflare.com:3478?transport=udp',
        'turn:turn.cloudflare.com:3478?transport=tcp',
        'turns:turn.cloudflare.com:5349?transport=tcp',
        'turns:turn.cloudflare.com:443?transport=tcp'
      ],
      username: data.username,
      credential: data.credential,
      credentialType: 'password'
    }
  ];
}

// Use in RTCPeerConnection
const iceServers = await getTURNConfig();
const peerConnection = new RTCPeerConnection({ iceServers });
```

## Port Selection Strategy

Recommended order for browser clients:

1. **3478/udp** (primary, lowest latency)
2. **3478/tcp** (fallback for UDP-blocked networks)
3. **5349/tls** (corporate firewalls, most reliable)
4. **443/tls** (alternate TLS port, firewall-friendly)

**Avoid port 53**—blocked by Chrome and Firefox.

```typescript
function filterICEServersForBrowser(urls: string[]): string[] {
  return urls
    .filter(url => !url.includes(':53'))  // Remove port 53
    .sort((a, b) => {
      // Prioritize UDP over TCP over TLS
      if (a.includes('transport=udp')) return -1;
      if (b.includes('transport=udp')) return 1;
      if (a.includes('transport=tcp') && !a.startsWith('turns:')) return -1;
      if (b.includes('transport=tcp') && !b.startsWith('turns:')) return 1;
      return 0;
    });
}
```

## Credential Refresh (Mid-Session)

When credentials expire during long calls:

```typescript
async function refreshTURNCredentials(pc: RTCPeerConnection): Promise<void> {
  const newCreds = await fetch('/turn-credentials').then(r => r.json());
  const config = pc.getConfiguration();
  config.iceServers = newCreds.iceServers;
  pc.setConfiguration(config);
  // Note: setConfiguration() does NOT trigger ICE restart
  // Combine with restartIce() if connection fails
}

// Auto-refresh before expiry
setInterval(async () => {
  await refreshTURNCredentials(peerConnection);
}, 3000000);  // 50 minutes if TTL is 1 hour
```

## ICE Restart Pattern

After network change, TURN server maintenance, or credential expiry:

```typescript
pc.addEventListener('iceconnectionstatechange', async () => {
  if (pc.iceConnectionState === 'failed') {
    console.warn('ICE connection failed, restarting...');
    
    // Refresh credentials
    await refreshTURNCredentials(pc);
    
    // Trigger ICE restart
    pc.restartIce();
    const offer = await pc.createOffer({ iceRestart: true });
    await pc.setLocalDescription(offer);
    
    // Send offer to peer via signaling channel...
  }
});
```

## Credentials Caching Pattern

```typescript
class TURNCredentialsManager {
  private creds: { username: string; credential: string; urls: string[]; expiresAt: number; } | null = null;

  async getCredentials(keyId: string, keySecret: string): Promise<RTCIceServer[]> {
    const now = Date.now();
    
    if (this.creds && this.creds.expiresAt > now) {
      return this.buildIceServers(this.creds);
    }

    const ttl = 3600;
    if (ttl > 172800) throw new Error('TTL max 48hrs');

    const res = await fetch(
      `https://rtc.live.cloudflare.com/v1/turn/keys/${keyId}/credentials/generate`,
      {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${keySecret}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ ttl })
      }
    );

    const data = await res.json();
    const filteredUrls = data.iceServers.urls.filter((url: string) => !url.includes(':53'));

    this.creds = {
      username: data.iceServers.username,
      credential: data.iceServers.credential,
      urls: filteredUrls,
      expiresAt: now + (ttl * 1000) - 60000
    };

    return this.buildIceServers(this.creds);
  }

  private buildIceServers(c: { username: string; credential: string; urls: string[] }): RTCIceServer[] {
    return [
      { urls: 'stun:stun.cloudflare.com:3478' },
      { urls: c.urls, username: c.username, credential: c.credential, credentialType: 'password' as const }
    ];
  }
}
```

## Common Use Cases

```typescript
// Video conferencing: TURN as fallback
const config = { iceServers: await getTURNConfig(), iceTransportPolicy: 'all' };

// IoT/predictable connectivity: force TURN
const config = { iceServers: await getTURNConfig(), iceTransportPolicy: 'relay' };

// Screen sharing: reduce overhead
const pc = new RTCPeerConnection({ iceServers: await getTURNConfig(), bundlePolicy: 'max-bundle' });
```

## Integration with Cloudflare Calls SFU

```typescript
// TURN is automatically used when needed
// Cloudflare Calls handles TURN + SFU coordination
const session = await callsClient.createSession({
  appId: 'your-app-id',
  sessionId: 'meeting-123'
});
```

## Debugging ICE Connectivity

```typescript
pc.addEventListener('icecandidate', (event) => {
  if (event.candidate) {
    console.log('ICE candidate:', event.candidate.type, event.candidate.protocol);
  }
});

pc.addEventListener('iceconnectionstatechange', () => {
  console.log('ICE state:', pc.iceConnectionState);
});

// Check selected candidate pair
const stats = await pc.getStats();
stats.forEach(report => {
  if (report.type === 'candidate-pair' && report.selected) {
    console.log('Selected:', report);
  }
});
```

## See Also

- [api.md](./api.md) - Credential generation API, types
- [configuration.md](./configuration.md) - Worker setup, environment variables
- [gotchas.md](./gotchas.md) - Common mistakes, troubleshooting


---

<!-- merged from: web-analytics-patterns.md -->

﻿---
name: Web Analytics Patterns
description: # Web Analytics Patterns
 
 ## Core Web Vitals Debugging
---

# Web Analytics Patterns

## Core Web Vitals Debugging (Web Analytics Patterns)

Dashboard → Core Web Vitals → Click metric → Debug View shows top 5 problematic elements.

### LCP Fixes

```html
<!-- Priority hints -->
<img src="hero.jpg" loading="eager" fetchpriority="high" />
<link rel="preload" as="image" href="/hero.jpg" fetchpriority="high" />
```

### CLS Fixes

```css
/* Reserve space */
.ad-container { min-height: 250px; }
img { width: 400px; height: 300px; } /* Explicit dimensions */
```

### INP Fixes

```typescript
// Debounce expensive operations
const handleInput = debounce(search, 300);

// Yield to main thread
await task(); await new Promise(r => setTimeout(r, 0)); await task2();

// Move to Web Worker for heavy computation
```

| Metric | Good | Poor |
| -------- | ------ | ------ |
| LCP | ≤2.5s | >4s |
| INP | ≤200ms | >500ms |
| CLS | ≤0.1 | >0.25 |

## GDPR Consent

```typescript
// Load beacon only after consent
const consent = localStorage.getItem('analytics-consent');
if (consent === 'accepted') {
  const script = document.createElement('script');
  script.src = 'https://static.cloudflareinsights.com/beacon.min.js';
  script.setAttribute('data-cf-beacon', '{"token": "TOKEN", "spa": true}');
  document.body.appendChild(script);
}
```

Alternative: Dashboard → "Enable, excluding visitor data in the EU"

## SPA Navigation

```html
<!-- REQUIRED for React/Vue/etc routing -->
<script data-cf-beacon='{"token": "TOKEN", "spa": true}' ...></script>
```

Without `spa: true`: only initial pageload tracked.

## Staging/Production Separation

```typescript
// Use env-specific tokens
const token = process.env.NEXT_PUBLIC_CF_ANALYTICS_TOKEN;
// .env.production: production token
// .env.staging: staging token (or empty to disable)
```

## Bot Filtering

Dashboard → Filters → "Exclude Bot Traffic"

Filters: Search crawlers, monitoring services, known bots.  
Not filtered: Headless browsers (Playwright/Puppeteer).

## Ad-Blocker Impact

~25-40% of users may block `cloudflareinsights.com`. No official workaround.
Dashboard shows minimum baseline; use server logs for complete picture.

## Limitations

- No UTM parameter tracking
- No webhooks/alerts/API
- No custom beacon domains
- Max 10 non-proxied sites


---

<!-- merged from: binding-configuration-reference.md -->

﻿---
name: Binding Configuration Reference
description: # Binding Configuration Reference
 
 ## Storage Bindings
---

# Binding Configuration Reference

## Storage Bindings (Binding Configuration Reference)

```jsonc
{
  "kv_namespaces": [{ "binding": "MY_KV", "id": "..." }],
  "r2_buckets": [{ "binding": "MY_BUCKET", "bucket_name": "my-bucket" }],
  "d1_databases": [{ "binding": "DB", "database_name": "my-db", "database_id": "..." }],
  "durable_objects": { "bindings": [{ "name": "MY_DO", "class_name": "MyDO" }] },
  "vectorize": [{ "binding": "VECTORIZE", "index_name": "my-index" }],
  "queues": { "producers": [{ "binding": "MY_QUEUE", "queue": "my-queue" }] }
}
```

### Create commands

```bash
npx wrangler kv namespace create MY_KV
npx wrangler r2 bucket create my-bucket
npx wrangler d1 create my-db
npx wrangler vectorize create my-index --dimensions=768 --metric=cosine
npx wrangler queues create my-queue

# List existing resources
npx wrangler kv namespace list
npx wrangler r2 bucket list
npx wrangler d1 list
npx wrangler vectorize list
npx wrangler queues list
```
