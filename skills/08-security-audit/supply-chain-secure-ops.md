---
name: supply-chain-secure-ops
description: Supply chain security, secure sandbox execution, skill/agent trust verification, secret scanning, and environment audit. Use when hardening the software or agent supply chain, running untrusted code, scanning for secrets, or auditing environment security posture.
domain: security
tags: [supply-chain, slsa, sbom, sandbox, secrets, environment-audit, agent-security, skill-trust]
triggers: supply chain security, SLSA, SBOM, sandbox execution, secret scan, gitleaks, environment audit, skill trust, agent supply chain, destructive ops
---

# Supply Chain & Secure Operations

## Software Supply Chain Security

### SLSA Framework Levels

- **Level 1**: Build process documented and automated; provenance metadata generated.
- **Level 2**: Build service used; provenance signed; source and build integrity verifiable.
- **Level 3**: Hardened build platform; hermetic, reproducible builds; non-falsifiable provenance.

### SBOM (Software Bill of Materials)

- Generate SBOM in CycloneDX or SPDX format during build.
- Include transitive dependencies with exact versions.
- Attach SBOM to release artifacts; scan SBOM against vulnerability databases.
- Update SBOM on every dependency change.

### Artifact Signing

- Sign all release artifacts (binaries, containers, packages) with Sigstore/cosign or GPG.
- Verify signatures before deployment; reject unsigned artifacts.
- Key management: use short-lived signing certificates (Fulcio) or managed KMS keys.

### Dependency Management

- Pin dependency versions; use lockfiles.
- Audit dependencies for known CVEs regularly (npm audit, cargo audit, pip-audit).
- Monitor for supply chain attacks: typosquatting, compromised maintainers, malicious updates.
- Use Socket.dev or similar for behavioral analysis of dependency changes.

---

## Agent & Skill Supply Chain Security

### Scan Before Use

- Never execute an agent-provided skill or code without scanning it first.
- Check for known malicious patterns: data exfiltration URLs, encoded payloads, filesystem traversal.
- Verify skill source: signed by trusted maintainer, from known registry, version-pinned.

### Sandbox Execution

- Run untrusted code in isolated environments: containers, VMs, Firecracker, or gVisor.
- Network isolation: block outbound by default; allowlist specific endpoints.
- Filesystem isolation: read-only root, ephemeral writable layer, no access to host secrets.
- Resource limits: CPU, memory, disk, and time bounds to prevent resource exhaustion.
- Capability dropping: run with minimum kernel capabilities; no privileged mode.

### Human-in-the-Loop (HITL)

- Require explicit user approval for destructive operations: file deletion, database drops, git force-push.
- Log all approved destructive actions with actor, timestamp, and scope.
- Implement dry-run mode for destructive operations.

### Prompt Injection Defense

- Treat agent-generated code as untrusted input.
- Validate agent outputs against expected schemas before execution.
- Monitor for instruction override patterns in agent responses.

---

## Secret Scanning

### Pre-Commit Scanning

```bash
# Install gitleaks
brew install gitleaks  # macOS
# Or: docker pull zricethezav/gitleaks

# Scan repository
gitleaks detect --source . --verbose

# Pre-commit hook (.husky/pre-commit)
gitleaks protect --staged --verbose
```

### Custom Rules (.gitleaks.toml)

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

### High-Entropy String Detection

- Scan for base64-encoded strings > 40 characters in source files.
- Flag .pem, .key, .p12 files added to repository.
- Check for hardcoded tokens matching known patterns (AWS, GitHub, Stripe, etc.).

### CI Integration

```yaml
# .github/workflows/security.yml
jobs:
  gitleaks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: gitleaks/gitleaks-action@v2
        env: { GITHUB_TOKEN: "${{ secrets.GITHUB_TOKEN }}" }
```

---

## Environment Audit & Remediation

### Audit Checklist

- [ ] All services running with least-privilege service accounts
- [ ] No default credentials on any system or database
- [ ] SSH key-based auth only; password auth disabled
- [ ] Firewall rules reviewed; no unnecessary open ports
- [ ] TLS certificates valid and auto-renewing
- [ ] Secrets stored in vault/KMS, not in environment variables or config files
- [ ] Logging enabled for auth events, admin actions, and data access
- [ ] Backup encryption verified; restore tested
- [ ] Dependency versions pinned; known CVEs patched
- [ ] Network segmentation between environments (dev/staging/prod)

### Remediation Priority

1. **Immediate**: Exposed secrets, default credentials, missing auth
2. **Urgent**: Unpatched critical CVEs, open admin ports, missing encryption
3. **Standard**: Logging gaps, backup verification, permission cleanup
4. **Maintenance**: Documentation updates, policy alignment, access reviews
