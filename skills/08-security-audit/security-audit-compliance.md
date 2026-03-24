---
name: security-audit-compliance
description: Comprehensive security audit and compliance reference covering OAuth2/OIDC, audit logging, certificates/mTLS, encryption, IAM/zero trust, OWASP Top 10 and ASVS, SAST/DAST, secrets management, SOC 2, threat modeling (STRIDE), pentest remediation, and GDPR/PII handling. Use when implementing or auditing any core security control.
domain: security
tags: [security, audit, compliance, oauth, oidc, tls, mtls, encryption, iam, owasp, sast, dast, secrets, soc2, stride, gdpr, pii, pentest]
triggers: OAuth2, OIDC, audit logging, mTLS, certificates, encryption, IAM, zero trust, OWASP, SAST, DAST, secrets management, SOC 2, STRIDE, threat modeling, GDPR, PII, pentest remediation
---

# Security Audit & Compliance

## API Auth: OAuth2 and OIDC

- **OAuth2**: Use authorization code with PKCE for public clients; avoid implicit flow; store tokens securely (httpOnly cookie, secure storage); use refresh tokens and short-lived access tokens.
- **OIDC**: Validate id_token (iss, aud, exp, nonce); use standard scopes (openid, profile, email) and custom scopes for API permissions.
- **Scopes**: Model permissions as scopes; request minimal scope; validate scope on each request.
- **Token storage**: Never store tokens in localStorage for web if XSS is a concern; prefer backend session or httpOnly cookie; mobile use secure storage (Keychain, Keystore).

## Audit Logging

- **What to log**: Authentication (success/failure), authorization failures, data access (sensitive reads/writes), config and admin changes, security events. Include actor, action, resource, timestamp, outcome. Avoid logging full PII or secrets.
- **Immutability**: Append-only store; no edit or delete by application; use WORM or append-only bucket; restrict write to audit service only.
- **Retention**: Retain per policy (1 year, 7 years depending on regulation); archive to cold storage after hot period; document legal hold process.

## Certificates and mTLS

- **Certificates**: Use valid CA (public or internal); set appropriate SANs; keep validity short (90 days) and rotate; automate issuance (ACME, Vault, cloud PKI); monitor expiry.
- **mTLS**: Require client cert for sensitive or internal services; validate client identity from cert (CN or SAN); map to authz.
- **Rotation**: Rotate without downtime (overlap period, reload on SIGHUP); distribute new certs before old expire.

## Encryption at Rest and in Transit

- **Transit**: TLS 1.2+ (prefer 1.3); disable weak ciphers; enforce HTTPS for all user and API traffic; use mTLS for service-to-service when required.
- **At rest**: Enable encryption for databases, volumes, and object storage; use platform encryption or application-level; protect keys with KMS or HSM.
- **Key management**: Use keys only for intended purpose (encryption vs signing); rotate per policy; separate keys per environment and sensitivity.
- **Never**: Embed keys in code or config; use custom crypto; recommend self-signed or long-lived certs for production without documenting risk.

## IAM and Zero Trust

- **Least privilege**: Grant minimum permissions needed; scope by resource and action; avoid wildcards and broad roles; review and trim regularly.
- **RBAC**: Model roles by function; assign roles not individual permissions; document role-to-permission mapping.
- **Zero trust**: Verify every request; do not trust network location; use identity and context (device, location); enforce at gateway and service; assume breach and segment.
- **Process**: Onboard/offboard access in sync with HR; review access periodically; use break-glass with logging.

## OWASP Top 10 and ASVS

### Top 10 Categories (map to design and code review)

A01 Broken Access Control, A02 Cryptographic Failures, A03 Injection, A04 Insecure Design, A05 Security Misconfiguration, A06 Vulnerable Components, A07 Auth Failures, A08 Integrity Failures, A09 Logging Failures, A10 SSRF.

### ASVS Verification Levels

- **Level 1 (Basic)**: Common vulnerabilities, automated/black-box testing. Bare minimum for all apps.
- **Level 2 (Standard)**: Business applications handling sensitive data. Requires source code access.
- **Level 3 (Advanced)**: Critical systems (banking, healthcare, infrastructure). Deep architectural verification.

### Key ASVS-to-Top-10 Mappings

- A01 (Broken Access Control) -> V4: Focus on least privilege and IDOR prevention
- A03 (Injection) -> V5: Validation, sanitization, and parameterized queries
- A07 (Auth) -> V2 & V3: MFA, password complexity, session flags

### Design and Review Checklist

- [ ] Top 10 mapped to design and review checklist
- [ ] Design covers access control, auth, and input validation
- [ ] Code review includes injection, XSS, auth, and dependency checks
- [ ] SAST/DAST scans in place; findings triaged

## SAST and DAST

- **SAST**: Run on every commit or PR; focus on injection, hardcoded secrets, unsafe APIs; tune rules to reduce noise; fail build on high/critical per policy.
- **DAST**: Run against running app (staging or prod-like); find runtime issues (config, auth, XSS, CSRF); use auth and scope to avoid damaging data.
- **Timing**: SAST in CI; DAST after deploy to test env or on schedule; both before release; add SCA for known CVEs.
- **Triage**: True positive, false positive, accept risk; assign owner and severity; track to closure; document acceptance.
- **Tools**: OWASP ZAP / Burp Suite for DAST Level 1. Semgrep / Snyk for SAST Level 2/3.

## Secrets Management

- **Store**: Dedicated secrets manager (HashiCorp Vault, AWS Secrets Manager); avoid secrets in code, config repo, or image; use short-lived tokens or dynamic secrets.
- **Rotation**: Rotate on schedule or trigger; support overlapping validity; update consumers in sync; test in non-prod first.
- **Injection**: Prefer runtime injection (sidecar, init container, operator) over env for sensitive values.
- **Access**: Principle of least privilege; audit access; use namespaces or path-based policy.

## SOC 2 Controls

- **Control mapping**: Map technical and process controls to SOC 2 criteria (CC6.1 access, CC7.1 monitoring); document how each control satisfies the criterion.
- **Evidence**: Collect config screenshots, logs, policy docs, runbooks; retain per policy; organize by control and date.
- **Audit**: Prepare evidence package; respond to requests in timeframe; document exceptions and remediation.
- **Ongoing**: Assign control owners; review periodically; update evidence when process or system changes.

## Threat Modeling (STRIDE)

- **STRIDE per element**: Spoofing (auth/identity), Tampering (integrity/signing), Repudiation (audit logs), Information disclosure (encryption/access control), DoS (rate limit/resilience), Elevation (least privilege/validation).
- **Data flows**: Draw trust boundaries and data flow (DFD); label data sensitivity; identify entry points and assets.
- **Mitigations**: Document mitigation per threat (control, acceptance, or transfer); prioritize by risk; track in backlog.
- **Process**: Run at design phase and after major changes; use consistent template; involve security and dev; keep model in repo.

## Pentest Remediation

- **Prioritization**: Fix critical and high first (RCE, auth bypass, data exposure); consider exploitability and impact; address dependency order (fix auth before access control).
- **Remediation**: Assign owner per finding; set due date; implement fix and test; document fix; request retest for critical/high.
- **Risk acceptance**: Document rationale, owner, and review date; get stakeholder sign-off; track in register; re-review periodically.

## GDPR and PII Handling

- **PII handling**: Identify PII in systems; minimize collection and retention; encrypt at rest and in transit; restrict access; document purpose and lawful basis.
- **Consent**: Record what, when, version; support withdrawal; do not bundle consent with terms.
- **Right to delete**: Fulfill erasure requests within SLA; delete or anonymize from all stores (DB, cache, search, backups); document scope and exceptions.
- **Data retention**: Define retention per data category; automate purge or archive by policy; document legal or regulatory basis.
- **Portability**: Provide data in machine-readable format; document export process.
- **Anonymization**: Remove or hash identifiers irreversibly; document what is anonymized; consider re-identification risk.
