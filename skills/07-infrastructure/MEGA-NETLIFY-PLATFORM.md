---
name: Netlify CLI Commands Reference
description: # Netlify CLI Commands Reference
 
 Quick reference for common Netlify CLI commands used in deployments.
---

# Netlify CLI Commands Reference

Quick reference for common Netlify CLI commands used in deployments.

## Authentication

```bash
# Login via browser OAuth
npx netlify login

# Check authentication status and site link
npx netlify status

# Logout
npx netlify logout
```

## Site Management

```bash
# Link current directory to existing site
npx netlify link

# Link by Git remote URL
npx netlify link --git-remote-url <url>

# Create and link new site
npx netlify init

# Unlink from current site
npx netlify unlink

# Open site in Netlify dashboard
npx netlify open

# Open site admin panel
npx netlify open:admin

# Open site in browser
npx netlify open:site
```

## Deployment

```bash
# Deploy preview/draft (safe for testing)
npx netlify deploy

# Deploy to production
npx netlify deploy --prod

# Deploy with specific directory
npx netlify deploy --dir=dist

# Deploy with message
npx netlify deploy --message="Deploy message"

# List all deploys
npx netlify deploy:list
```

## Development

```bash
# Run local dev server with Netlify features
npx netlify dev

# Run local dev server on specific port
npx netlify dev --port 3000
```

## Site Information

```bash
# Get site info
npx netlify sites:list

# Get current site info
npx netlify api getSite --data '{"site_id": "YOUR_SITE_ID"}'
```

## Environment Variables

```bash
# List environment variables
npx netlify env:list

# Set environment variable
npx netlify env:set KEY value

# Get environment variable value
npx netlify env:get KEY

# Import env vars from file
npx netlify env:import .env
```

## Build

```bash
# Show build settings
npx netlify build --dry

# Run build locally
npx netlify build
```

## Functions (Serverless)

```bash
# List functions
npx netlify functions:list

# Invoke function locally
npx netlify functions:invoke FUNCTION_NAME

# Create new function
npx netlify functions:create FUNCTION_NAME
```

## Logs

```bash
# Stream function logs
npx netlify logs

# View logs for specific function
npx netlify logs:function FUNCTION_NAME
```

## Troubleshooting Commands

```bash
# Check CLI version
npx netlify --version

# Get help for any command
npx netlify help [command]

# Check status with verbose output
npx netlify status --verbose
```

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Authentication error
- `3` - Site not found
- `4` - Build failed

## Common Flags

- `--json` - Output as JSON
- `--silent` - Suppress output
- `--debug` - Show debug information
- `--force` - Skip confirmation prompts

## Resources

- Full CLI documentation: <https://docs.netlify.com/cli/get-started/>
- CLI GitHub repository: <https://github.com/netlify/cli>

## When to use

Use when the user asks about or needs: Netlify CLI Commands Reference.
﻿---
name: netlify-deploy
description: Deploy web projects to Netlify using the Netlify CLI (`npx netlify`). Use when the user asks to deploy, host, publish, or link a site/repo on Netlify, including preview and production deploys.
---

# Netlify Deployment Skill

Deploy web projects to Netlify using the Netlify CLI with intelligent detection of project configuration and deployment context.

## Overview

This skill automates Netlify deployments by:

- Verifying Netlify CLI authentication
- Detecting project configuration and framework
- Linking to existing sites or creating new ones
- Deploying to production or preview environments

## Prerequisites

- **Netlify CLI**: Installed via npx (no global install required)
- **Authentication**: Netlify account with active login session
- **Project**: Valid web project in current directory
- When sandboxing blocks the deployment network calls, rerun with `sandbox_permissions=require_escalated`.
- The deployment might take a few minutes. Use appropriate timeout values.

## Authentication Pattern

The skill uses the **pre-authenticated Netlify CLI** approach:

1. Check authentication status with `npx netlify status`
2. If not authenticated, guide user through `npx netlify login`
3. Fail gracefully if authentication cannot be established

Authentication uses either:

- **Browser-based OAuth** (primary): `netlify login` opens browser for authentication
- **API Key** (alternative): Set `NETLIFY_AUTH_TOKEN` environment variable

## Workflow

### 1. Verify Netlify CLI Authentication

Check if the user is logged into Netlify:

```bash
npx netlify status
```

#### Expected output patterns

- ✅ Authenticated: Shows logged-in user email and site link status
- ❌ Not authenticated: "Not logged into any site" or authentication error

**If not authenticated**, guide the user:

```bash
npx netlify login
```

This opens a browser window for OAuth authentication. Wait for user to complete login, then verify with `netlify status` again.

#### Alternative: API Key authentication

If browser authentication isn't available, users can set:

```bash
export NETLIFY_AUTH_TOKEN=your_token_here
```

Tokens can be generated at: <https://app.netlify.com/user/applications#personal-access-tokens>

### 2. Detect Site Link Status

From `netlify status` output, determine:

- **Linked**: Site already connected to Netlify (shows site name/URL)
- **Not linked**: Need to link or create site

### 3. Link to Existing Site or Create New

**If already linked** → Skip to step 4

**If not linked**, attempt to link by Git remote:

```bash
# Check if project is Git-based
git remote show origin

# If Git-based, extract remote URL
# Format: https://github.com/username/repo or git@github.com:username/repo.git

# Try to link by Git remote
npx netlify link --git-remote-url <REMOTE_URL>
```

**If link fails** (site doesn't exist on Netlify):

```bash
# Create new site interactively
npx netlify init
```

This guides user through:

1. Choosing team/account
2. Setting site name
3. Configuring build settings
4. Creating netlify.toml if needed

### 4. Verify Dependencies

Before deploying, ensure project dependencies are installed:

```bash
# For npm projects
npm install

# For other package managers, detect and use appropriate command
# yarn install, pnpm install, etc.
```

### 5. Deploy to Netlify

Choose deployment type based on context:

**Preview/Draft Deploy** (default for existing sites):

```bash
npx netlify deploy
```

This creates a deploy preview with a unique URL for testing.

**Production Deploy** (for new sites or explicit production deployments):

```bash
npx netlify deploy --prod
```

This deploys to the live production URL.

#### Deployment process

1. CLI detects build settings (from netlify.toml or prompts user)
2. Builds the project locally
3. Uploads built assets to Netlify
4. Returns deployment URL

### 6. Report Results

After deployment, report to user:

- **Deploy URL**: Unique URL for this deployment
- **Site URL**: Production URL (if production deploy)
- **Deploy logs**: Link to Netlify dashboard for logs
- **Next steps**: Suggest `netlify open` to view site or dashboard

## Handling netlify.toml

If a `netlify.toml` file exists, the CLI uses it automatically. If not, the CLI will prompt for:

- **Build command**: e.g., `npm run build`, `next build`
- **Publish directory**: e.g., `dist`, `build`, `.next`

Common framework defaults:

- **Next.js**: build command `npm run build`, publish `.next`
- **React (Vite)**: build command `npm run build`, publish `dist`
- **Static HTML**: no build command, publish current directory

The skill should detect framework from `package.json` if possible and suggest appropriate settings.

## Example Full Workflow

```bash
# 1. Check authentication
npx netlify status

# If not authenticated:
npx netlify login

# 2. Link site (if needed)
# Try Git-based linking first
git remote show origin
npx netlify link --git-remote-url https://github.com/user/repo

# If no site exists, create new one:
npx netlify init

# 3. Install dependencies
npm install

# 4. Deploy (preview for testing)
npx netlify deploy

# 5. Deploy to production (when ready)
npx netlify deploy --prod
```

## Error Handling

Common issues and solutions:

### "Not logged in"

→ Run `npx netlify login`

#### "No site linked"

→ Run `npx netlify link` or `npx netlify init`

#### "Build failed"

→ Check build command and publish directory in netlify.toml or CLI prompts
→ Verify dependencies are installed
→ Review build logs for specific errors

#### "Publish directory not found"

→ Verify build command ran successfully
→ Check publish directory path is correct

## Troubleshooting

### Escalated Network Access

If deployment fails due to network issues (timeouts, DNS errors, connection resets), rerun the deploy with escalated permissions (use `sandbox_permissions=require_escalated`). The deploy requires escalated network access when sandbox networking blocks outbound requests.

Example guidance to the user:

```text
The deploy needs escalated network access to deploy to Netlify. I can rerun the command with escalated permissions—want me to proceed?
```

## Environment Variables

For secrets and configuration:

1. Never commit secrets to Git
2. Set in Netlify dashboard: Site Settings → Environment Variables
3. Access in builds via `process.env.VARIABLE_NAME`

## Tips

- Use `netlify deploy` (no `--prod`) first to test before production
- Run `netlify open` to view site in Netlify dashboard
- Run `netlify logs` to view function logs (if using Netlify Functions)
- Use `netlify dev` for local development with Netlify Functions

## Reference

- Netlify CLI Docs: <https://docs.netlify.com/cli/get-started/>
- netlify.toml Reference: <https://docs.netlify.com/configure-builds/file-based-configuration/>

## Bundled References (Load As Needed)

- [CLI commands](references/cli-commands.md)
- [Deployment patterns](references/deployment-patterns.md)
- [netlify.toml guide](references/netlify-toml.md)

## When to use

Use when the user asks about or needs: netlify-deploy.
﻿---
name: Netlify Deployment Patterns
description: # Netlify Deployment Patterns
 
 Common deployment scenarios and best practices for the Netlify skill.
---

# Netlify Deployment Patterns

Common deployment scenarios and best practices for the Netlify skill.

## Deployment Decision Tree

```text
Is user authenticated?
├─ No → Run `netlify login`
└─ Yes → Is site linked?
    ├─ No → Is it a Git repo?
    │   ├─ Yes → Try `netlify link --git-remote-url`
    │   │   ├─ Success → Continue to deploy
    │   │   └─ Fail → Run `netlify init`
    │   └─ No → Run `netlify init`
    └─ Yes → Is this first deploy or existing site?
        ├─ First deploy/new site → `netlify deploy --prod`
        └─ Existing site → `netlify deploy` (preview)
```

## Scenario 1: First-Time Deployment (New Project)

**Context**: User has a project that has never been deployed to Netlify.

### Steps

1. Check authentication: `npx netlify status`
2. If not authenticated: `npx netlify login`
3. Initialize new site: `npx netlify init`
   - This guides user through setup
   - Creates netlify.toml if needed
4. Install dependencies: `npm install`
5. Deploy to production: `npx netlify deploy --prod`

#### Example

```bash
npx netlify status
# Not linked to a site

npx netlify login
# Opens browser for authentication

npx netlify init
# Walks through site creation

npm install
npx netlify deploy --prod
```

## Scenario 2: Linking Existing Git Repository to Existing Site

**Context**: User has a site already on Netlify and wants to link their local repo.

### Steps (Scenario 2: Linking Existing Git Repository to Existing Site)

1. Check authentication: `npx netlify status`
2. Get Git remote: `git remote show origin`
3. Extract URL (e.g., `https://github.com/user/repo.git`)
4. Link by remote: `npx netlify link --git-remote-url <URL>`
5. If found, linked. If not, run `netlify init`

#### Example (2)

```bash
git remote show origin
# * remote origin
#   Fetch URL: https://github.com/user/my-app.git

npx netlify link --git-remote-url https://github.com/user/my-app.git
# Site linked successfully
```

## Scenario 3: Preview Deployment (Testing Changes)

**Context**: User wants to test changes before pushing to production.

### Steps (Scenario 3: Preview Deployment (Testing Changes))

1. Ensure site is linked: `npx netlify status`
2. Make code changes
3. Deploy preview: `npx netlify deploy`
4. Review preview URL
5. If approved, deploy to prod: `npx netlify deploy --prod`

#### Example (3)

```bash
# Make changes to code

npx netlify deploy
# Draft deploy URL: https://507f1f77bcf86cd799439011-my-app.netlify.app

# Test the preview, then:
npx netlify deploy --prod
```

## Scenario 4: Framework-Specific Deployments

### Next.js

```bash
# Next.js typically uses .next as output
npx netlify deploy --prod

# netlify.toml should have:
# [build]
#   command = "npm run build"
#   publish = ".next"
```

### React (Vite)

```bash
# Vite outputs to dist by default
npm run build
npx netlify deploy --dir=dist --prod

# netlify.toml:
# [build]
#   command = "npm run build"
#   publish = "dist"
```

### Static HTML

```bash
# No build step needed
npx netlify deploy --dir=. --prod
```

## Scenario 5: Monorepo Deployment

**Context**: Project is in a subdirectory of a monorepo.

### Steps (Scenario 5: Monorepo Deployment)

1. Navigate to project subdirectory: `cd packages/frontend`
2. Or set base in netlify.toml:

   ```toml
   [build]
     base = "packages/frontend"
     command = "npm run build"
     publish = "dist"
   ```

3. Deploy normally: `npx netlify deploy --prod`

## Scenario 6: Environment Variables

**Context**: Project needs secrets or environment-specific config.

### Steps (Scenario 6: Environment Variables)

1. Never commit secrets to Git
2. Set in Netlify dashboard or CLI:

   ```bash
   npx netlify env:set API_KEY "secret_value"
   npx netlify env:set NODE_ENV "production"
   ```

3. Access in code: `process.env.API_KEY`
4. Deploy: `npx netlify deploy --prod`

## Scenario 7: Custom Domain Setup

**Context**: User wants to use a custom domain.

### Steps (Scenario 7: Custom Domain Setup)

1. Deploy site first: `npx netlify deploy --prod`
2. Add domain via dashboard or CLI:

   ```bash
   npx netlify open:admin
   # Navigate to Domain settings
   ```

3. Update DNS records as instructed by Netlify
4. Wait for DNS propagation (can take up to 48 hours)

## Best Practices

### 1. Always Preview First

```bash
# Deploy preview
npx netlify deploy

# Test thoroughly
# Then deploy to production
npx netlify deploy --prod
```

### 2. Use netlify.toml for Consistency

Create a `netlify.toml` file in your repo root:

```toml
[build]
  command = "npm run build"
  publish = "dist"

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

This ensures consistent builds across all deployments.

### 3. Framework Detection

Let Netlify auto-detect when possible. Only specify build settings if:

- Netlify can't detect your framework
- You need custom build commands
- Your project has a non-standard structure

### 4. Dependency Installation

Always ensure dependencies are installed before deploying:

```bash
npm install  # or yarn install, pnpm install
npx netlify deploy
```

### 5. Build Locally First

Test builds locally before deploying:

```bash
npm run build
# Check that build output exists

npx netlify deploy --dir=dist
```

### 6. Use Deploy Messages

Add context to deployments:

```bash
npx netlify deploy --prod --message="Fix login bug"
```

## Error Recovery Patterns

### "Publish directory not found"

**Cause**: Build command didn't create expected output directory.

#### Fix

1. Run build locally: `npm run build`
2. Check output directory name
3. Update netlify.toml or CLI prompts with correct path

### "Command failed with exit code 1"

**Cause**: Build command failed.

#### Fix ("Command failed with exit code 1")

1. Check build logs for specific error
2. Run build locally to reproduce: `npm run build`
3. Fix the build error
4. Deploy again

### "Not logged in"

**Cause**: Authentication token expired or missing.

#### Fix ("Not logged in")

```bash
npx netlify logout
npx netlify login
```

### "No site linked"

**Cause**: Project not connected to a Netlify site.

#### Fix ("No site linked")

```bash
# Try linking to existing site
npx netlify link

# Or create new site
npx netlify init
```

## Performance Tips

1. **Enable processing** in netlify.toml for auto-optimization:

   ```toml
   [build.processing.css]
     bundle = true
     minify = true
   ```

2. **Use caching headers** for static assets:

   ```toml
   [[headers]]
     for = "/assets/*"
     [headers.values]
       Cache-Control = "public, max-age=31536000, immutable"
   ```

3. **Optimize images** before deploying or use Netlify Image CDN

4. **Use Netlify Functions** for serverless backend (avoid external API calls when possible)

## Resources

- Netlify CLI Documentation: <https://docs.netlify.com/cli/get-started/>
- Framework Integration Guides: <https://docs.netlify.com/frameworks/>
- Build Configuration: <https://docs.netlify.com/configure-builds/>

## When to use

Use when the user asks about or needs: Netlify Deployment Patterns.
﻿---
name: netlify.toml Configuration Reference
description: # netlify.toml Configuration Reference
 
 Configuration file for Netlify builds and deployments.
---

# netlify.toml Configuration Reference

Configuration file for Netlify builds and deployments.

## Basic Structure

```toml
[build]
  command = "npm run build"
  publish = "dist"
```

## Build Settings

### Common Configuration

```toml
[build]
  # Command to build your site
  command = "npm run build"

  # Directory to publish (relative to repo root)
  publish = "dist"

  # Functions directory
  functions = "netlify/functions"

  # Base directory (if not repo root)
  base = "packages/frontend"

  # Ignore builds for specific conditions
  ignore = "git diff --quiet HEAD^ HEAD package.json"
```

## Environment Variables

```toml
[build.environment]
  NODE_VERSION = "18"
  NPM_FLAGS = "--prefix=/dev/null"

[context.production.environment]
  NODE_ENV = "production"
```

## Framework Detection

Netlify auto-detects frameworks, but you can override:

### Next.js

```toml
[build]
  command = "npm run build"
  publish = ".next"
```

### React (Vite)

```toml
[build]
  command = "npm run build"
  publish = "dist"
```

### Vue

```toml
[build]
  command = "npm run build"
  publish = "dist"
```

### Astro

```toml
[build]
  command = "npm run build"
  publish = "dist"
```

### SvelteKit

```toml
[build]
  command = "npm run build"
  publish = "build"
```

## Redirects and Rewrites

```toml
[[redirects]]
  from = "/old-path"
  to = "/new-path"
  status = 301

[[redirects]]
  from = "/api/*"
  to = "https://api.example.com/:splat"
  status = 200

# SPA fallback (for client-side routing)
[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

## Headers

```toml
[[headers]]
  for = "/*"
  [headers.values]
    X-Frame-Options = "DENY"
    X-XSS-Protection = "1; mode=block"
    Content-Security-Policy = "default-src 'self'"

[[headers]]
  for = "/assets/*"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
```

## Context-Specific Configuration

Deploy different settings per context:

```toml
# Production
[context.production]
  command = "npm run build:prod"
  [context.production.environment]
    NODE_ENV = "production"

# Deploy previews
[context.deploy-preview]
  command = "npm run build:preview"

# Branch deploys
[context.branch-deploy]
  command = "npm run build:staging"

# Specific branch
[context.staging]
  command = "npm run build:staging"
```

## Functions Configuration

```toml
[functions]
  directory = "netlify/functions"
  node_bundler = "esbuild"

[[functions]]
  path = "/api/*"
  function = "api"
```

## Build Plugins

```toml
[[plugins]]
  package = "@netlify/plugin-lighthouse"

  [plugins.inputs]
    output_path = "reports/lighthouse.html"

[[plugins]]
  package = "netlify-plugin-submit-sitemap"

  [plugins.inputs]
    baseUrl = "https://example.com"
    sitemapPath = "/sitemap.xml"
```

## Edge Functions

```toml
[[edge_functions]]
  function = "geolocation"
  path = "/api/location"
```

## Processing

```toml
[build.processing]
  skip_processing = false

[build.processing.css]
  bundle = true
  minify = true

[build.processing.js]
  bundle = true
  minify = true

[build.processing.html]
  pretty_urls = true

[build.processing.images]
  compress = true
```

## Common Patterns

### Single Page Application (SPA)

```toml
[build]
  command = "npm run build"
  publish = "dist"

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

### Monorepo with Base Directory

```toml
[build]
  base = "packages/web"
  command = "npm run build"
  publish = "dist"
```

### Multiple Redirects with Country-Based Routing

```toml
[[redirects]]
  from = "/"
  to = "/uk"
  status = 302
  conditions = {Country = ["GB"]}

[[redirects]]
  from = "/"
  to = "/us"
  status = 302
  conditions = {Country = ["US"]}
```

## Validation

Validate your netlify.toml:

```bash
npx netlify build --dry
```

## Resources

- Full configuration reference: <https://docs.netlify.com/configure-builds/file-based-configuration/>
- Framework-specific guides: <https://docs.netlify.com/frameworks/>

## When to use

Use when the user asks about or needs: netlify.toml Configuration Reference.
