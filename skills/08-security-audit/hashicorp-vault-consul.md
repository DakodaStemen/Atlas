---
name: hashicorp-vault-consul
description: Patterns, configuration, and operational practices for HashiCorp Vault (secrets management, auth methods, secret engines, Vault Agent) and Consul (service discovery, health checks, KV store, service mesh with Connect/Envoy). Covers Kubernetes and AppRole integration, dynamic credentials, PKI, lease management, Consul intentions, and production hardening.
domain: devops
category: secrets
tags: [Vault, Consul, secrets-management, service-discovery, dynamic-secrets, AppRole, Kubernetes, PKI, service-mesh, Envoy]
triggers: [vault, consul, secrets management, dynamic secrets, AppRole, Kubernetes auth, service discovery, service mesh, Consul Connect, PKI certificates, secret rotation, Vault Agent, lease renewal]
---

# HashiCorp Vault and Consul Patterns

## Vault — Core Concepts

Vault is a secrets management platform that acts as a single source of truth for sensitive data: API keys, database credentials, certificates, and encryption keys. It enforces access via auth methods, stores data in secret engines, and issues time-limited leases so credentials expire automatically without manual cleanup.

Three terms to keep straight:

- **Token** — the primary unit of authentication; every Vault request requires one.
- **Policy** — HCL or JSON document mapping paths to capabilities (`read`, `create`, `update`, `delete`, `list`, `sudo`).
- **Lease** — TTL attached to a secret or token; Vault revokes the resource when the lease expires.

---

## Auth Methods

### AppRole

AppRole is designed for machine-to-machine authentication (CI/CD, non-Kubernetes services). It issues a `role_id` (static, embeddable in config) and a `secret_id` (ephemeral, injected at runtime). The split reduces risk: compromising the config file alone gives nothing.

```bash
vault auth enable approle

vault write auth/approle/role/myapp \
  secret_id_ttl=10m \
  token_ttl=1h \
  token_max_ttl=4h \
  secret_id_num_uses=1 \
  policies="myapp-policy"

# Retrieve role_id (safe to bake into config)
vault read auth/approle/role/myapp/role-id

# Generate a secret_id (inject at deploy time, not in config)
vault write -f auth/approle/role/myapp/secret-id

# Login
vault write auth/approle/login \
  role_id=<role_id> \
  secret_id=<secret_id>
```

Set `secret_id_num_uses=1` in production so each secret_id is single-use. Use response wrapping (`-wrap-ttl=60s`) when handing the secret_id to an orchestrator so it travels encrypted.

### Kubernetes Auth

The Kubernetes auth method lets pods authenticate using their projected service account token. Vault calls the Kubernetes `TokenReview` API to validate the JWT, then issues a Vault token scoped by the bound role.

```bash
vault auth enable kubernetes

vault write auth/kubernetes/config \
  kubernetes_host=https://<K8S_API>:443 \
  kubernetes_ca_cert=@/var/run/secrets/kubernetes.io/serviceaccount/ca.crt
  # Omit token_reviewer_jwt when Vault runs in-cluster; it reads the local SA token automatically (Vault 1.9.3+)

vault write auth/kubernetes/role/myapp \
  bound_service_account_names=myapp-sa \
  bound_service_account_namespaces=production \
  policies=myapp-policy \
  ttl=1h
```

The Vault pod's service account needs `system:auth-delegator` to call the TokenReview API:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: vault-token-reviewer
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: system:auth-delegator
subjects:
  - kind: ServiceAccount
    name: vault
    namespace: vault
```

For Kubernetes 1.21+, `disable_iss_validation=true` is the default for new mounts (Vault 1.9.0+). Do not re-enable it — issuer URLs vary by cluster configuration.

### AWS Auth

Vault validates either an IAM identity (`iam` sub-method) or EC2 instance identity document (`ec2` sub-method). The IAM method is preferred for ECS, Lambda, and EC2; the instance identity document method is useful for bare EC2 where you can bind on AMI, account, or region.

```bash
vault auth enable aws

vault write auth/aws/config/client \
  access_key=<ACCESS_KEY> \
  secret_key=<SECRET_KEY>

vault write auth/aws/role/myapp \
  auth_type=iam \
  bound_iam_principal_arn=arn:aws:iam::123456789:role/myapp-role \
  policies=myapp-policy \
  ttl=1h
```

### Other Auth Methods

- **OIDC / JWT** — delegate authentication to an identity provider (GitHub Actions, Google, Okta). Useful for CI/CD pipelines that already have OIDC tokens.
- **TLS certificates** — mTLS-based authentication; useful when client certificates are already managed.
- **Token** — direct token creation; only appropriate for human operators or initial bootstrap. Do not use for application authentication in production.
- **LDAP / Active Directory** — human users authenticate with existing directory credentials.

---

## Secret Engines

### KV v2

Versioned key-value store. Enables soft-delete and version history. Always use v2 for new mounts.

```bash
vault secrets enable -path=secret kv-v2

# Write
vault kv put secret/myapp/config db_password=s3cr3t api_key=abc123

# Read
vault kv get -field=db_password secret/myapp/config

# Read specific version
vault kv get -version=3 secret/myapp/config

# Roll back
vault kv rollback -version=2 secret/myapp/config
```

Naming convention: `/<app>/<environment>/<component>` (e.g., `secret/payments/production/database`). Consistent paths make policy authoring predictable and audit logs readable.

### Database Dynamic Secrets

The database engine generates unique, short-lived credentials on demand. The application never handles a long-lived password; each lease produces a new database user that is dropped when the lease expires.

```bash
vault secrets enable database

# Configure the connection (Vault stores and rotates the root credential)
vault write database/config/postgres \
  plugin_name=postgresql-database-plugin \
  connection_url="postgresql://{{username}}:{{password}}@postgres:5432/mydb?sslmode=require" \
  allowed_roles=myapp-role \
  username=vault_root \
  password=root_password

# Define a role with a creation statement
vault write database/roles/myapp-role \
  db_name=postgres \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT SELECT ON ALL TABLES IN SCHEMA public TO \"{{name}}\";" \
  default_ttl=1h \
  max_ttl=24h

# Generate credentials
vault read database/creds/myapp-role

# Rotate root credential so Vault owns it exclusively
vault write -force database/rotate-root/postgres
```

Supported plugins: PostgreSQL, MySQL/MariaDB, MSSQL, MongoDB, Oracle, Cassandra, Elasticsearch, Redis. The same pattern applies across all of them — configure the plugin, define a role with a creation statement, read credentials.

### PKI Secrets Engine

Vault acts as an intermediate CA to issue X.509 certificates on demand. Short-lived certificates (hours to days) replace the traditional long-lived cert + manual renewal cycle.

```bash
vault secrets enable pki
vault secrets tune -max-lease-ttl=87600h pki  # 10 years for root

# Generate or import root CA
vault write pki/root/generate/internal \
  common_name=example.com \
  ttl=87600h

# Create intermediate CA
vault secrets enable -path=pki_int pki
vault write pki_int/intermediate/generate/internal \
  common_name="example.com Intermediate CA"
# Sign the CSR with the root, then import the signed cert

# Create an issuance role
vault write pki_int/roles/example-dot-com \
  allowed_domains=example.com \
  allow_subdomains=true \
  max_ttl=72h

# Issue a certificate
vault write pki_int/issue/example-dot-com \
  common_name=api.example.com \
  ttl=24h
```

### AWS Secrets Engine

Issues temporary IAM credentials (access key, secret key, session token) scoped to a policy. Credentials expire automatically; no IAM key rotation scripts needed.

```bash
vault secrets enable aws

vault write aws/config/root \
  access_key=<ADMIN_ACCESS_KEY> \
  secret_key=<ADMIN_SECRET_KEY> \
  region=us-east-1

vault write aws/roles/s3-read \
  credential_type=iam_user \
  policy_document='{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["s3:GetObject"],"Resource":"arn:aws:s3:::my-bucket/*"}]}'

vault read aws/creds/s3-read
```

Use `credential_type=assumed_role` with `role_arns` when you need STS assumed-role tokens instead of IAM users.

### Transit Secrets Engine (Encryption-as-a-Service)

Transit does not store data. It encrypts/decrypts data in transit, keeping the key inside Vault. Applications send plaintext to Vault and receive ciphertext back — they never handle a raw encryption key.

```bash
vault secrets enable transit
vault write -f transit/keys/payments  # Create a named key

# Encrypt (plaintext must be base64-encoded)
vault write transit/encrypt/payments plaintext=$(echo -n "4111111111111111" | base64)

# Decrypt
vault write transit/decrypt/payments ciphertext=vault:v1:...
```

Key rotation: `vault write -f transit/keys/payments/rotate`. Old ciphertext still decrypts; new encryptions use the latest key version.

---

## Vault Agent

Vault Agent is a client daemon that handles authentication, token renewal, and secret rendering so applications do not need Vault SDK integration. It runs as a sidecar or daemonset.

Key capabilities:

- **Auto-auth** — authenticates to Vault using any auth method (Kubernetes, AppRole, AWS), renews tokens automatically.
- **Template rendering** — writes secrets to files using Go templates; re-renders on lease expiry or rotation.
- **Caching** — proxies requests to Vault; caches tokens and leases to reduce API load.
- **Exec** — wraps a process, setting secrets as environment variables and re-launching on rotation (useful for 12-factor apps).

Minimal Kubernetes Agent config (`vault-agent-config.hcl`):

```hcl
vault {
  address = "https://vault.vault.svc:8200"
}

auto_auth {
  method "kubernetes" {
    mount_path = "auth/kubernetes"
    config = {
      role = "myapp"
    }
  }

  sink "file" {
    config = {
      path = "/home/vault/.vault-token"
    }
  }
}

template {
  source      = "/vault/templates/app.env.tpl"
  destination = "/vault/secrets/app.env"
  perms       = 0640
}
```

Template file (`app.env.tpl`):

```json
{{- with secret "database/creds/myapp-role" -}}
DB_USERNAME={{ .Data.username }}
DB_PASSWORD={{ .Data.password }}
{{- end }}
```

The Vault Agent Injector (Kubernetes mutating admission webhook) automates this by injecting Agent as an init container and sidecar based on pod annotations:

```yaml
annotations:
  vault.hashicorp.com/agent-inject: "true"
  vault.hashicorp.com/role: "myapp"
  vault.hashicorp.com/agent-inject-secret-config: "database/creds/myapp-role"
  vault.hashicorp.com/agent-inject-template-config: |
    {{- with secret "database/creds/myapp-role" -}}
    export DB_USER="{{ .Data.username }}"
    export DB_PASS="{{ .Data.password }}"
    {{- end }}
```

Alternative: **Vault Secrets Operator** syncs Vault secrets into native Kubernetes Secret objects via a CRD (`VaultStaticSecret`, `VaultDynamicSecret`). The operator handles renewal and updates the Secret in place.

---

## Lease Management

Every dynamic secret and token has a lease. Leases can be renewed before expiry or revoked early.

```bash
# List active leases (Enterprise / recent OSS)
vault list sys/leases/lookup/database/creds/myapp-role

# Renew a specific lease
vault lease renew database/creds/myapp-role/<lease_id>

# Revoke a specific lease
vault lease revoke database/creds/myapp-role/<lease_id>

# Revoke all leases for a path (incident response)
vault lease revoke -prefix database/creds/myapp-role
```

Monitor `vault.expire.num_leases` (Prometheus/StatsD) — a spike indicates something is generating credentials but not renewing or revoking them.

Keep `default_ttl` short (1h for DB creds) and `max_ttl` as a hard ceiling (24h). Applications that need longer-lived credentials should renew the lease rather than requesting a longer TTL.

---

## Policies

Policies follow least-privilege. Write them to the narrowest path needed.

```hcl
# myapp-policy.hcl
path "secret/data/myapp/*" {
  capabilities = ["read", "list"]
}

path "database/creds/myapp-role" {
  capabilities = ["read"]
}

path "pki_int/issue/example-dot-com" {
  capabilities = ["create", "update"]
}

# Allow token self-renewal
path "auth/token/renew-self" {
  capabilities = ["update"]
}
```

```bash
vault policy write myapp-policy myapp-policy.hcl
```

Use entity groups to assign policies across multiple auth mounts without repeating them per-role. This scales cleanly when the same application authenticates via both AppRole (CI) and Kubernetes (runtime).

---

## Production Hardening

### Seal types

- *Shamir* (default) — master key split into N shares, M required to unseal. Secure but requires human intervention after every restart.
- *Auto-unseal* — delegates unseal to AWS KMS, Azure Key Vault, GCP Cloud KMS, or another Vault cluster (Transit seal). Enables automated restarts and simplifies HA operations. Shifts trust to the KMS provider.

```hcl
seal "awskms" {
  region     = "us-east-1"
  kms_key_id = "alias/vault-unseal"
}
```

**Audit logging** — always enable at least two audit devices so Vault continues to respond if one device is unavailable. Vault blocks all requests if it cannot write to any enabled audit device.

```bash
vault audit enable file file_path=/var/log/vault/audit.log
vault audit enable syslog tag="vault" facility="AUTH"
```

Audit logs hash sensitive values with HMAC-SHA256. The raw hash can be checked with `vault audit hash`.

**Root token** — revoke the root token after initial setup. Re-generate it only for emergency operations, then revoke again immediately.

**TLS** — never disable TLS in production (`tls_disable = 0`). Provide a certificate signed by your internal CA or use Vault's PKI engine to bootstrap its own TLS cert.

**Service account isolation** — run the Vault process under a dedicated unprivileged OS user. Disable core dumps (`LimitCORE=0` in systemd). Do not run other processes on the same host.

**HA storage** — use Integrated Raft storage (built-in, no external dependency) or Consul storage backend. Raft is the default recommendation for new deployments. Three or five nodes for quorum.

#### Snapshots

```bash
vault operator raft snapshot save /backup/vault-$(date +%Y%m%d-%H%M).snap
```

Schedule hourly snapshots; test restoration quarterly.

#### Telemetry

```hcl
telemetry {
  prometheus_retention_time = "30s"
  disable_hostname          = true
}
```

Key metrics: `vault.core.handle_request`, `vault.expire.num_leases`, `vault.barrier.get`, `vault.barrier.put`, `vault.token.create`.

---

## Consul — Core Concepts

Consul provides service discovery, health checking, distributed key-value storage, and a service mesh (Connect). It uses a Raft consensus cluster of server nodes; client agents run on each host and forward requests to servers.

Run 3 servers for small deployments, 5 for larger ones. Even numbers of servers provide no quorum benefit and increase split-brain risk.

---

## Service Registration

**Config file** (preferred for static services):

```json
{
  "service": {
    "id": "web-api-1",
    "name": "web-api",
    "tags": ["production", "v2"],
    "port": 8080,
    "check": {
      "http": "http://localhost:8080/health",
      "interval": "10s",
      "timeout": "2s",
      "deregister_critical_service_after": "60s"
    }
  }
}
```

Place the file in the Consul data directory and reload: `consul reload`. Consul watches the directory for changes.

**HTTP API** (for dynamic or ephemeral services):

```bash
curl -s -X PUT http://localhost:8500/v1/agent/service/register \
  -H "Content-Type: application/json" \
  -d '{
    "ID": "web-api-2",
    "Name": "web-api",
    "Port": 8081,
    "Tags": ["production"],
    "Check": {
      "HTTP": "http://localhost:8081/health",
      "Interval": "10s",
      "Timeout": "2s"
    }
  }'
```

---

## Health Checks

Consul supports four check types:

- **HTTP** — GET to an endpoint; 2xx is passing, 429 is warning, anything else is critical.
- **TCP** — TCP handshake; success is passing.
- **Script** — arbitrary executable; exit 0 passing, exit 1 warning, exit 2+ critical. Requires `enable_script_checks = true` in the agent config (disabled by default in production; prefer HTTP/TCP where possible).
- **TTL** — service pushes status updates to Consul; if no update within the TTL the check goes critical. Use for processes that cannot be polled externally.

Always set `deregister_critical_service_after` on health checks. Without it, dead services accumulate in the catalog.

---

## Service Discovery

**DNS interface** (port 8600 by default):

```bash
# A record for any healthy instance
dig @127.0.0.1 -p 8600 web-api.service.consul

# Filter by tag
dig @127.0.0.1 -p 8600 production.web-api.service.consul

# SRV record includes port
dig @127.0.0.1 -p 8600 web-api.service.consul SRV

# Cross-datacenter
dig @127.0.0.1 -p 8600 web-api.service.dc2.consul
```

Most production setups configure `dnsmasq` or `systemd-resolved` to forward `.consul` queries to port 8600, so applications can use `web-api.service.consul` as a hostname without custom DNS configuration.

### HTTP API

```bash
# Healthy instances only
curl "http://localhost:8500/v1/health/service/web-api?passing"

# Filter by tag
curl "http://localhost:8500/v1/health/service/web-api?passing&tag=production"

# Watch for changes (long-poll with index)
curl "http://localhost:8500/v1/health/service/web-api?passing&index=<last_index>&wait=30s"
```

The blocking query pattern (supplying `index` + `wait`) is the correct way to watch for service changes without polling at a fixed interval. Consul returns immediately when the result changes or the wait timeout expires.

---

## Consul KV Store

Flat key-value store, useful for feature flags, configuration, and distributed coordination. Not a replacement for Vault — store no secrets here.

```bash
consul kv put config/myapp/log_level info
consul kv get config/myapp/log_level
consul kv get -recurse config/myapp/

# Watch a key and run a handler on change
consul watch -type=key -key=config/myapp/log_level /usr/local/bin/reload-config.sh

# Delete
consul kv delete config/myapp/log_level
consul kv delete -recurse config/myapp/
```

KV is backed by Raft, so all writes are linearizable. Do not use it as a high-throughput data store — it is optimized for correctness, not throughput.

---

## Service Mesh — Consul Connect

Connect provides mutual TLS between services via sidecar proxies (Envoy by default). Services communicate over encrypted, authenticated channels without any application code changes.

### Enable Connect in the agent config

```json
{
  "connect": {
    "enabled": true
  }
}
```

#### Register a service with a sidecar

```json
{
  "service": {
    "name": "payments",
    "port": 8080,
    "connect": {
      "sidecar_service": {
        "proxy": {
          "upstreams": [
            {
              "destination_name": "postgres",
              "local_bind_port": 5432
            }
          ]
        }
      }
    }
  }
}
```

The application connects to `localhost:5432` and the sidecar handles mTLS to the postgres service's sidecar transparently.

Start the sidecar proxy:

```bash
consul connect proxy -sidecar-for payments
```

In Kubernetes, the Consul Helm chart injects Envoy sidecars automatically via mutating webhook.

---

## Intentions — Service-to-Service Authorization

Intentions define which services are allowed to communicate. The default policy can be `allow` (permissive, for gradual migration) or `deny` (zero-trust). Use `deny` as default in new deployments.

```bash
# Deny all by default (set in agent config: default_policy = "deny")

# Allow payments to talk to postgres
consul intention create -allow payments postgres

# Deny a specific path
consul intention create -deny frontend vault-agent

# List all intentions
consul intention list

# Check whether a connection would be allowed
consul intention check payments postgres
```

Intentions work at the service identity level — they survive IP changes, scaling events, and redeployments. This is the zero-trust advantage over IP-based firewall rules.

Kubernetes: intentions can be defined as `ServiceIntentions` CRDs when using the Consul Helm chart.

---

## ACLs

ACLs protect the Consul API. Always enable them in production.

```bash
# Bootstrap (generates the initial management token)
consul acl bootstrap

# Create a policy for a specific service
consul acl policy create \
  -name "payments-policy" \
  -rules 'service "payments" { policy = "write" } service_prefix "" { policy = "read" } node_prefix "" { policy = "read" }'

# Create a token bound to the policy
consul acl token create \
  -description "payments service token" \
  -policy-name "payments-policy"
```

Each service, Vault agent, and operator gets a dedicated token. Rotate tokens on a schedule. Use the token with the API via the `X-Consul-Token` header or `CONSUL_HTTP_TOKEN` environment variable.

---

## Multi-Datacenter Federation

Consul federates datacenters over WAN gossip. Services in one datacenter can discover services in another.

```bash
# Join a remote datacenter's server
consul join -wan <remote-server-ip>

# Query a remote DC
curl "http://localhost:8500/v1/health/service/web-api?dc=us-west-2&passing"

# DNS cross-DC
dig @127.0.0.1 -p 8600 web-api.service.us-west-2.consul
```

Prepared queries add automatic failover: if the local datacenter has no healthy instances, the query fans out to remote datacenters transparently.

---

## Vault + Consul Integration

Consul can serve as Vault's HA storage backend, though Integrated Raft is now the default recommendation. When using Consul as storage:

```hcl
# vault.hcl
storage "consul" {
  address = "127.0.0.1:8500"
  path    = "vault/"
  token   = "<consul-acl-token>"
}

listener "tcp" {
  address     = "0.0.0.0:8200"
  tls_cert_file = "/etc/vault/tls/vault.crt"
  tls_key_file  = "/etc/vault/tls/vault.key"
}
```

Give Vault a Consul ACL token with `write` on the `vault/` prefix and `read` on the session endpoint. Only the active Vault node holds the Consul session lock; standby nodes redirect requests to the active node.

---

## Operational Runbook Fragments

### Rotate a database root credential

```bash
vault write -force database/rotate-root/postgres
```

#### Revoke all credentials after a breach

```bash
vault lease revoke -prefix database/creds/myapp-role
```

#### Check Consul cluster health

```bash
consul operator raft list-peers
consul members
```

#### Snapshot Consul state

```bash
consul snapshot save consul-$(date +%Y%m%d-%H%M).snap
consul snapshot inspect consul-$(date +%Y%m%d-%H%M).snap
```

#### Check Vault seal status

```bash
vault status
vault operator key-status
```

**Re-key Vault** (rotate the master key shares without changing data encryption key):

```bash
vault operator rekey -init -shares=5 -threshold=3
```

---

## Common Pitfalls

- **Not rotating the root credential** after configuring the database engine. Vault and the DBA both know it until `rotate-root` is called.
- **Long TTLs on dynamic secrets** defeating the purpose. Keep `default_ttl` at 1h and let applications renew, not extend via `max_ttl` hacks.
- **AppRole secret_id in source control**. The `role_id` is public; the `secret_id` is the credential. Treat it like a password. Use single-use secret_ids and response wrapping when possible.
- **Consul health checks without `deregister_critical_service_after`**. Dead service instances accumulate in the catalog and return in DNS queries.
- **Consul KV for secrets**. Consul KV has no encryption at rest or audit logging. All secrets belong in Vault.
- **Open Consul intentions (`default_policy = allow`)** in Connect. The migration path is fine; the final state should be `deny` with explicit allow intentions.
- **Disabling audit devices** to improve performance. The performance hit is minimal; losing audit logs in a breach investigation is catastrophic.
- **Running without `disable_iss_validation=true`** on Kubernetes 1.21+ clusters. Vault will reject valid service account tokens because the issuer URL doesn't match.
