---
name: security-scanner
description: Scans for secrets, audits dependencies, and ensures RLS policies are in place.
domain: security
triggers: security audit, check for vulnerabilities, scan for secrets, dependency audit, check security, find exposed secrets, gitleaks, npm audit
compatibility: git; optional Node.js for npm audit
---

# Security Scanner

## Trigger Phrases

- "security audit", "check for vulnerabilities", "scan for secrets"
- "dependency audit", "check security", "find exposed secrets"

## Prerequisites

- Git repository
- Node.js project

## Workflow

### 1. Secret Scanning

#### Install gitleaks

```bash
# macOS
brew install gitleaks

# Or via Docker
docker pull zricethezav/gitleaks
```

#### Run scan

```bash
gitleaks detect --source . --verbose
```

#### .gitleaks.toml (custom rules)

```toml
[allowlist]
description = "Allowlist for false positives"
paths = [
  '''\.env\.example$''',
  '''package-lock\.json$''',
]

[[rules]]
id = "custom-api-key"
description = "Custom API Key"
regex = '''(?i)my_api_key\s*[:=]\s*['\"][a-zA-Z0-9]{32,}['\"]'''
```

### 2. Pre-commit Secret Prevention

#### .husky/pre-commit

```bash
gitleaks protect --staged --verbose
```

### 3. Dependency Audit

#### NPM audit

```bash
npm audit
npm audit fix
npm audit --audit-level=high  # CI fails on high+ vulnerability
```

#### Better alternatives

```bash
# Socket.dev (catches supply chain attacks)
npx socket npm audit

# Snyk
npx snyk test
```

### 4. GitHub Actions Security Scan

#### .github/workflows/security.yml

```yaml
name: Security Scan

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  gitleaks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  dependency-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npm audit --audit-level=high

  codeql:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4
      - uses: github/codeql-action/init@v3
        with:
          languages: javascript
      - uses: github/codeql-action/autobuild@v3
      - uses: github/codeql-action/analyze@v3
```

### 5. Security Headers

#### next.config.js

```javascript
const securityHeaders = [
  {
    key: 'X-DNS-Prefetch-Control',
    value: 'on',
  },
  {
    key: 'Strict-Transport-Security',
    value: 'max-age=63072000; includeSubDomains; preload',
  },
  {
    key: 'X-Frame-Options',
    value: 'SAMEORIGIN',
  },
  {
    key: 'X-Content-Type-Options',
    value: 'nosniff',
  },
  {
    key: 'X-XSS-Protection',
    value: '1; mode=block',
  },
  {
    key: 'Referrer-Policy',
    value: 'origin-when-cross-origin',
  },
  {
    key: 'Content-Security-Policy',
    value: "default-src 'self'; script-src 'self' 'unsafe-eval' 'unsafe-inline'; style-src 'self' 'unsafe-inline';",
  },
]

module.exports = {
  async headers() {
    return [{ source: '/:path*', headers: securityHeaders }]
  },
}
```

### 6. Input Validation

```typescript
import { z } from 'zod'

// Always validate user input
const userInputSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8).max(100),
  name: z.string().max(100).regex(/^[a-zA-Z\s]+$/),
})

// In API route
export async function POST(req: Request) {
  const body = await req.json()
  const result = userInputSchema.safeParse(body)
  
  if (!result.success) {
    return Response.json({ error: result.error }, { status: 400 })
  }
  
  // result.data is now typed and validated
}
```

### 7. Database Security (Supabase RLS)

```sql
-- Enable RLS on tables
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;

-- Users can only read their own posts
CREATE POLICY "Users can read own posts"
  ON posts FOR SELECT
  USING (auth.uid() = user_id);

-- Users can only insert their own posts
CREATE POLICY "Users can create own posts"
  ON posts FOR INSERT
  WITH CHECK (auth.uid() = user_id);

-- Users can only update their own posts
CREATE POLICY "Users can update own posts"
  ON posts FOR UPDATE
  USING (auth.uid() = user_id);
```

### 8. Rate Limiting

```typescript
import { Ratelimit } from '@upstash/ratelimit'
import { Redis } from '@upstash/redis'

const ratelimit = new Ratelimit({
  redis: Redis.fromEnv(),
  limiter: Ratelimit.slidingWindow(10, '10 s'),
})

export async function POST(req: Request) {
  const ip = req.headers.get('x-forwarded-for') ?? '127.0.0.1'
  const { success, limit, reset, remaining } = await ratelimit.limit(ip)
  
  if (!success) {
    return Response.json(
      { error: 'Too many requests' },
      {
        status: 429,
        headers: {
          'X-RateLimit-Limit': limit.toString(),
          'X-RateLimit-Remaining': remaining.toString(),
          'X-RateLimit-Reset': reset.toString(),
        },
      }
    )
  }
  
  // Process request
}
```

## Security Checklist

- [ ] No secrets in git history
- [ ] Dependencies have no critical vulnerabilities
- [ ] Security headers configured
- [ ] User input validated with Zod
- [ ] RLS policies on all database tables
- [ ] Rate limiting on public APIs
- [ ] HTTPS enforced
- [ ] CSP configured

## Constraints

- Never log sensitive data
- Rotate compromised secrets immediately
- Review dependency updates before merging
- Use parameterized queries (Prisma handles this)
