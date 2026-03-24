---
name: cloudflare-infrastructure
description: Cloudflare infrastructure patterns covering Terraform/HCL configuration for Workers, D1, KV, R2, Queues, Pages, Access, WAF, and rulesets. Also covers Cloudflare Tunnel setup (networking, API, gotchas, patterns), DNS/CDN configuration, and deployment workflows. Use when provisioning or managing Cloudflare infrastructure.
domain: infrastructure
tags: [cloudflare, terraform, tunnel, dns, cdn, workers, d1, kv, r2, access, waf, deployment]
triggers: cloudflare terraform, cloudflare tunnel, cloudflared, cloudflare deploy, cloudflare dns, cloudflare cdn, cloudflare access, cloudflare WAF, cloudflare rulesets
---

# Cloudflare Infrastructure

## 1. Terraform Configuration

### Zone and DNS

```hcl
resource "cloudflare_zone" "example" {
  account = { id = var.account_id }; name = "example.com"; type = "full"
}
resource "cloudflare_zone_settings_override" "example" {
  zone_id = cloudflare_zone.example.id
  settings { ssl = "strict"; always_use_https = "on"; min_tls_version = "1.2"; tls_1_3 = "on" }
}
resource "cloudflare_dns_record" "www" {
  zone_id = cloudflare_zone.example.id; name = "www"; content = "192.0.2.1"; type = "A"; proxied = true
}
```

### Workers

```hcl
resource "cloudflare_workers_script" "api" {
  account_id = var.account_id; name = "api-worker"
  content = file("worker.js"); module = true; compatibility_date = "2025-01-01"
  kv_namespace_binding { name = "KV"; namespace_id = cloudflare_workers_kv_namespace.cache.id }
  d1_database_binding { name = "DB"; database_id = cloudflare_d1_database.main.id }
  r2_bucket_binding { name = "R2"; bucket_name = cloudflare_r2_bucket.uploads.name }
}
resource "cloudflare_workers_route" "api" {
  zone_id = cloudflare_zone.example.id; pattern = "api.example.com/*"
  script = cloudflare_workers_script.api.name
}
```

### Storage Resources

```hcl
resource "cloudflare_workers_kv_namespace" "cache" {
  account_id = var.account_id; title = "prod-cache"
}
resource "cloudflare_d1_database" "main" {
  account_id = var.account_id; name = "prod-db"
}
resource "cloudflare_r2_bucket" "uploads" {
  account_id = var.account_id; name = "prod-uploads"
  location = "ENAM"
}
```

### Access and Security

```hcl
resource "cloudflare_zero_trust_access_application" "internal" {
  zone_id = cloudflare_zone.example.id
  name = "Internal App"; domain = "internal.example.com"
  session_duration = "24h"; type = "self_hosted"
}
resource "cloudflare_zero_trust_access_policy" "employees" {
  zone_id = cloudflare_zone.example.id
  application_id = cloudflare_zero_trust_access_application.internal.id
  name = "Allow employees"; decision = "allow"; precedence = 1
  include { email_domain = ["company.com"] }
}
```

### Troubleshooting Terraform

- State drift: Run `terraform plan` regularly. Use `terraform import` for manually created resources.
- API rate limits: Add `-parallelism=5` for large configs. Use `depends_on` to serialize dependent resources.
- Provider version: Pin the cloudflare provider version. Check changelog before upgrading.

## 2. Cloudflare Tunnel

### Connectivity

| Port | Protocol | Purpose | Required |
|------|----------|---------|----------|
| 7844 | TCP/UDP | Primary tunnel (QUIC) | Yes |
| 443 | TCP | Fallback (HTTP/2) | Yes |

Firewall: Allow outbound tcp/udp 7844 and tcp 443 to `*.argotunnel.com`.

### Tunnel Setup

```bash
# Create tunnel
cloudflared tunnel create my-tunnel

# Configure (config.yml)
tunnel: <TUNNEL_ID>
credentials-file: /root/.cloudflared/<TUNNEL_ID>.json
ingress:
  - hostname: app.example.com
    service: http://localhost:8080
  - hostname: ssh.example.com
    service: ssh://localhost:22
  - service: http_status:404  # Catch-all (required)
```

### DNS Route

```bash
cloudflared tunnel route dns my-tunnel app.example.com
# Creates CNAME: app.example.com → <TUNNEL_ID>.cfargotunnel.com
```

### Common Gotchas

- **Catch-all required**: Last ingress rule must be a catch-all (`service: http_status:404`).
- **Credential file**: Must match tunnel ID exactly. Regenerate with `cloudflared tunnel token`.
- **WebSocket**: Add `noTLSVerify: true` for origins using self-signed certs.
- **Health checks**: Enable `originRequest.noHappyEyeballs: true` for IPv4-only origins.
- **Replicas**: Multiple `cloudflared` instances can run for HA. Use `--edge-ip-version auto`.

### Tunnel API

```bash
# List tunnels
curl -X GET "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/tunnels" \
  -H "Authorization: Bearer $API_TOKEN"

# Create tunnel
curl -X POST "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/tunnels" \
  -H "Authorization: Bearer $API_TOKEN" \
  -d '{"name": "my-tunnel", "tunnel_secret": "<base64-secret>"}'
```

### Tunnel Patterns

- **Multi-service**: Single tunnel, multiple hostnames via ingress rules.
- **Blue-green**: Two tunnels, swap DNS CNAME for zero-downtime deployment.
- **Private network**: Route entire CIDR through tunnel for VPN-like access.
- **SSH/RDP**: Proxy SSH/RDP through tunnel with Access policies.

## 3. Deployment Workflows

### Wrangler Deploy

```bash
npx wrangler deploy                    # Deploy worker
npx wrangler pages deploy ./dist       # Deploy Pages site
npx wrangler d1 migrations apply DB    # Run D1 migrations
```

### CI/CD Integration

```yaml
# GitHub Actions
- uses: cloudflare/wrangler-action@v3
  with:
    apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
    command: deploy
```

### DNS and CDN

- Proxied records (orange cloud): Get CDN, DDoS protection, WAF, caching.
- DNS-only records (gray cloud): Direct DNS resolution, no Cloudflare proxy.
- Cache rules: Configure per-path TTLs via Page Rules or Cache Rules.
- Purge: `curl -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE/purge_cache" -d '{"purge_everything": true}'`

## Checklist

- [ ] Terraform state stored remotely (Cloudflare R2 or S3)
- [ ] Provider version pinned in required_providers
- [ ] Tunnel credential files secured (not in git)
- [ ] Catch-all ingress rule configured
- [ ] Access policies defined for all exposed services
- [ ] DNS records set to proxied where CDN/WAF needed
- [ ] Wrangler deploy in CI/CD pipeline
