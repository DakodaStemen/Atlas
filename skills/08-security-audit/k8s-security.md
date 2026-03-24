---
name: k8s-security
description: Kubernetes security — Pod Security Standards, OPA Gatekeeper, Kyverno, Falco runtime defense, RBAC, network policies, and secrets management.
domain: devops
category: security
tags: [Kubernetes, OPA, Gatekeeper, Kyverno, Falco, RBAC, pod-security, network-policy, secrets, k8s-security]
triggers: Kubernetes security, OPA Gatekeeper, Kyverno policy, Falco runtime, pod security standard, k8s RBAC, k8s network policy, admission webhook
---

# Kubernetes Security

## When to Use

Apply this skill when hardening any of the following:

- **Multi-tenant clusters** where multiple teams or customers share a single control plane. Namespace isolation via RBAC, network policies, and admission control is mandatory — not optional.
- **Compliance-driven environments** (PCI-DSS, SOC 2, HIPAA, FedRAMP). Audit logging, least-privilege RBAC, and policy enforcement with a paper trail (OPA audit mode, Kyverno policy reports) are required.
- **Production workloads** moving from dev. Baseline Pod Security Standards are table stakes; restricted profile + Falco runtime monitoring + image signing close the bulk of attack surface.
- **Supply chain hardening**. Image signing (Cosign), registry scanning (Trivy), and admission-time verification prevent compromised images from reaching pods.
- **Post-incident hardening**. After a container escape or credential theft, implement default-deny NetworkPolicy, External Secrets Operator, and Falco syscall rules to detect recurrence.

---

## Pod Security Standards

Kubernetes built-in admission plugin (Pod Security Admission, GA in 1.25) enforces three profiles:

| Profile | Who uses it | What it blocks |
| --- | --- | --- |
| **privileged** | System/infra workloads (CNI, CSI) | Nothing — fully unrestricted |
| **baseline** | General application workloads | Host namespaces, privileged containers, hostPath volumes, dangerous capabilities |
| **restricted** | Security-sensitive / internet-facing | All baseline + requires non-root UID, drops all capabilities, enforces seccomp |

Each profile can run in three modes per namespace:

- `enforce` — rejects non-compliant pods at admission
- `audit` — allows but records violation in audit log
- `warn` — allows but returns warning to the client (kubectl shows it)

### Enforce restricted on a namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: payments
  labels:
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/enforce-version: v1.29
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
```

Running audit+warn in staging before flipping enforce in production avoids surprise pod rejections. The `-version` label pins to a specific Kubernetes release's definition of the profile so upgrades don't silently tighten rules.

#### A pod that satisfies restricted

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: hardened-app
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 10001
    seccompProfile:
      type: RuntimeDefault
  containers:
  - name: app
    image: ghcr.io/example/app:sha256-abc123
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop: ["ALL"]
```

---

## RBAC

### Design principles

1. **One ServiceAccount per workload** — never use `default`. Sharing a ServiceAccount across deployments means a compromise of one grants access to all.
2. **Namespace-scoped Role over ClusterRole** wherever possible. ClusterRoles grant cluster-wide access even when bound with a RoleBinding inside a namespace — understand the distinction.
3. **No `get`/`list`/`watch` on Secrets at cluster scope** unless the controller genuinely needs it (e.g., cert-manager, external-secrets). Audit these with `kubectl auth can-i --list --as=system:serviceaccount:NAMESPACE:NAME`.
4. **No `*` verbs or resources** in production. Wildcards are for bootstrapping, not runtime.

```yaml
# Scoped Role — only what the app needs
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: app-reader
  namespace: payments
rules:
- apiGroups: [""]
  resources: ["configmaps"]
  verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: app-reader-binding
  namespace: payments
subjects:
- kind: ServiceAccount
  name: payments-api
  namespace: payments
roleRef:
  kind: Role
  name: app-reader
  apiGroup: rbac.authorization.k8s.io
```

#### Audit commands

```bash
# Who can do what in a namespace
kubectl auth can-i --list -n payments --as=system:serviceaccount:payments:payments-api

# Find all ClusterRoleBindings with cluster-admin
kubectl get clusterrolebindings -o json | jq '.items[] | select(.roleRef.name=="cluster-admin") | .subjects'

# rbac-tool (open source) for visual graph
kubectl rbac-tool who-can get secrets
```

---

## Network Policies

Kubernetes NetworkPolicy is enforced by the CNI plugin — **if your CNI doesn't support it, policies are silently ignored**. Calico, Cilium, and Weave Net support NetworkPolicy. Flannel does not without Canal.

### Default deny (apply to every namespace)

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-all
  namespace: payments
spec:
  podSelector: {}        # matches all pods in namespace
  policyTypes:
  - Ingress
  - Egress
```

This blocks everything. Then add explicit allow policies:

```yaml
# Allow ingress from the frontend namespace only
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-frontend
  namespace: payments
spec:
  podSelector:
    matchLabels:
      app: payments-api
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          kubernetes.io/metadata.name: frontend
    - podSelector:
        matchLabels:
          app: web
  ports:
  - protocol: TCP
    port: 8080
---
# Allow egress to DNS and a specific database
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-egress-dns-db
  namespace: payments
spec:
  podSelector:
    matchLabels:
      app: payments-api
  policyTypes:
  - Egress
  egress:
  - ports:
    - protocol: UDP
      port: 53       # DNS — required or nothing resolves
  - to:
    - podSelector:
        matchLabels:
          app: postgres
    ports:
    - protocol: TCP
      port: 5432
```

**Cilium** extends this with L7 policies (HTTP method/path, DNS FQDNs) via `CiliumNetworkPolicy` CRD, which is preferable for microservices with external egress.

---

## OPA Gatekeeper

Gatekeeper is a validating (and mutating) admission webhook backed by OPA. Policies are split into two Kubernetes resources:

- **ConstraintTemplate** — defines the Rego logic and the CRD it creates
- **Constraint** — an instance of that CRD, parameterized for a specific match scope

### ConstraintTemplate (define the rule)

```yaml
apiVersion: templates.gatekeeper.sh/v1
kind: ConstraintTemplate
metadata:
  name: k8srequiredlabels
spec:
  crd:
    spec:
      names:
        kind: K8sRequiredLabels
      validation:
        openAPIV3Schema:
          type: object
          properties:
            labels:
              type: array
              items:
                type: string
  targets:
  - target: admission.k8s.gatekeeper.sh
    rego: |
      package k8srequiredlabels

      violation[{"msg": msg, "details": {"missing_labels": missing}}] {
        provided := {label | input.review.object.metadata.labels[label]}
        required := {label | label := input.parameters.labels[_]}
        missing := required - provided
        count(missing) > 0
        msg := sprintf("Required labels missing: %v", [missing])
      }
```

### Constraint (instantiate the rule)

```yaml
apiVersion: constraints.gatekeeper.sh/v1beta1
kind: K8sRequiredLabels
metadata:
  name: require-app-label
spec:
  enforcementAction: deny      # or "warn" for soft enforcement / "dryrun" for audit
  match:
    kinds:
    - apiGroups: ["apps"]
      kinds: ["Deployment"]
    namespaceSelector:
      matchExpressions:
      - key: env
        operator: In
        values: ["production"]
  parameters:
    labels: ["app", "owner", "cost-center"]
```

### Enforcement modes

| `enforcementAction` | Effect |
| --- | --- |
| `deny` | Rejects the resource at admission |
| `warn` | Allows but returns a warning |
| `dryrun` | Allows, records violation in Constraint status for audit |

Use `dryrun` to assess blast radius before switching to `deny`. Check violations:

```bash
kubectl get k8srequiredlabels require-app-label -o json | jq '.status.violations'
```

### Exemptions

Exempt specific namespaces cluster-wide in the Gatekeeper config:

```yaml
apiVersion: config.gatekeeper.sh/v1alpha1
kind: Config
metadata:
  name: config
  namespace: gatekeeper-system
spec:
  match:
  - excludedNamespaces: ["kube-system", "gatekeeper-system", "cert-manager"]
    processes: ["*"]
```

Per-constraint exemptions use `match.excludedNamespaces` in the Constraint spec.

---

## Kyverno

Kyverno is a policy engine that operates as a Kubernetes admission webhook and uses native Kubernetes YAML (no Rego). Policy types:

- **validate** — reject or audit non-compliant resources
- **mutate** — patch resources at admission (add labels, set defaults)
- **generate** — create companion resources (e.g., NetworkPolicy when a Namespace is created)
- **verifyImages** — enforce image signing via Cosign/Notary

### ClusterPolicy — validate

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: disallow-privileged-containers
spec:
  validationFailureAction: Enforce   # or Audit
  background: true                   # also scan existing resources
  rules:
  - name: check-privileged
    match:
      any:
      - resources:
          kinds: ["Pod"]
    validate:
      message: "Privileged containers are not allowed."
      pattern:
        spec:
          containers:
          - =(securityContext):
              =(privileged): "false"
```

### ClusterPolicy — mutate (set defaults)

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: add-default-labels
spec:
  rules:
  - name: add-managed-by
    match:
      any:
      - resources:
          kinds: ["Deployment"]
    mutate:
      patchStrategicMerge:
        metadata:
          labels:
            +(managed-by): kyverno    # "+" prefix = only add if not present
```

### ClusterPolicy — generate (auto-create NetworkPolicy per namespace)

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: default-deny-networkpolicy
spec:
  rules:
  - name: generate-default-deny
    match:
      any:
      - resources:
          kinds: ["Namespace"]
    generate:
      apiVersion: networking.k8s.io/v1
      kind: NetworkPolicy
      name: default-deny-all
      namespace: "{{request.object.metadata.name}}"
      synchronize: true    # keep in sync; delete policy = delete generated resource
      data:
        spec:
          podSelector: {}
          policyTypes: [Ingress, Egress]
```

### Policy Reports

Kyverno writes results to `PolicyReport` (namespace) and `ClusterPolicyReport` CRDs, queryable without Gatekeeper's per-constraint status:

```bash
kubectl get policyreport -A
kubectl get clusterpolicyreport -o yaml
```

---

## Falco

Falco monitors running workloads by hooking into the Linux kernel via an eBPF probe (default in Falco ≥ 0.36) or kernel module. It evaluates a stream of syscall events against a rules engine and fires alerts.

### How it works

1. eBPF probe (or kernel module) streams syscall events from the kernel.
2. Falco's rules engine matches events against conditions.
3. Matching events produce alerts sent to configured output channels.
4. Falco enriches events with Kubernetes pod/namespace/label metadata from the container runtime.

### Rules file structure

```yaml
# /etc/falco/falco_rules.local.yaml  (override default rules here, not in falco_rules.yaml)

- rule: Shell in Container
  desc: A shell was spawned inside a container
  condition: >
    spawned_process and
    container and
    not container.image.repository in (allowed_shell_images) and
    proc.name in (shell_binaries)
  output: >
    Shell spawned in container
    (user=%user.name container=%container.name image=%container.image.repository
     cmd=%proc.cmdline pod=%k8s.pod.name ns=%k8s.ns.name)
  priority: WARNING
  tags: [container, shell, mitre_execution]

- list: allowed_shell_images
  items: [debug-tools, datadog-agent]

- list: shell_binaries
  items: [bash, sh, zsh, dash, fish]
```

Override rules in `falco_rules.local.yaml`, never in `falco_rules.yaml` (overwritten on upgrade).

### Kubernetes deployment

Deploy Falco as a DaemonSet via Helm:

```bash
helm repo add falcosecurity https://falcosecurity.github.io/charts
helm install falco falcosecurity/falco \
  --namespace falco --create-namespace \
  --set driver.kind=ebpf \
  --set falcosidekick.enabled=true \
  --set falcosidekick.config.slack.webhookurl="https://hooks.slack.com/..."
```

### falcosidekick alert channels

falcosidekick fans out Falco events to 50+ destinations: Slack, PagerDuty, Elasticsearch, Loki, Prometheus, Datadog, AWS SNS, and more. Configure via `falcosidekick.config.*` helm values or a dedicated ConfigMap.

### Tuning false positives

1. Move noisy rules to `NOTICE` or `INFO` priority rather than deleting them.
2. Use `- macro: my_safe_process` appended with `or proc.name = my-tool` rather than patching the base macro directly.
3. Use `append: true` to extend an existing list or macro:

```yaml
- list: allowed_shell_images
  items: [my-debug-image]
  append: true
```

1. Tag stable rules with `MITRE ATT&CK` tags; filter dashboards by tag to cut noise from low-signal categories.
2. Review `falco --dry-run` output before deploying rule changes in production.

---

## Secrets Management

**Never mount secrets as environment variables.** They appear in `kubectl describe`, process listings, and crash dumps. Prefer volume mounts. Better: never store secrets in etcd at all.

### Hierarchy (weakest to strongest)

| Approach | Risk | Notes |
| --- | --- | --- |
| `env.valueFrom.secretKeyRef` | High — visible in process env | Avoid in production |
| `volumeMount` from Secret | Medium — at-rest in etcd (encrypt etcd!) | Acceptable if etcd encrypted |
| Sealed Secrets (Bitnami) | Low — encrypted in Git, decrypted in cluster | Good for GitOps; key rotation is manual |
| Vault Agent Injector | Low — secrets never touch etcd | Requires Vault; sidecar injects at runtime |
| External Secrets Operator | Low — syncs from AWS SM, GCP SM, Azure KV | Best for cloud-native; no Vault required |

### External Secrets Operator (ESO)

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: db-credentials
  namespace: payments
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secretsmanager
    kind: ClusterSecretStore
  target:
    name: db-credentials    # creates this Kubernetes Secret
    creationPolicy: Owner
  data:
  - secretKey: password
    remoteRef:
      key: prod/payments/db
      property: password
```

### Vault Agent Injector

Annotate the pod; Vault Agent sidecar injects the secret as a file:

```yaml
annotations:
  vault.hashicorp.com/agent-inject: "true"
  vault.hashicorp.com/role: "payments-api"
  vault.hashicorp.com/agent-inject-secret-db-creds: "secret/data/payments/db"
  vault.hashicorp.com/agent-inject-template-db-creds: |
    {{- with secret "secret/data/payments/db" -}}
    {{ .Data.data.password }}
    {{- end -}}
```

Secret appears at `/vault/secrets/db-creds` inside the pod, never in env or etcd.

---

## Image Security

### Image signing with Cosign

Sign in CI after build:

```bash
cosign sign --key cosign.key ghcr.io/example/app:v1.2.3
```

Verify at admission with Kyverno:

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: verify-image-signature
spec:
  validationFailureAction: Enforce
  rules:
  - name: check-cosign-signature
    match:
      any:
      - resources:
          kinds: ["Pod"]
    verifyImages:
    - imageReferences: ["ghcr.io/example/*"]
      attestors:
      - count: 1
        entries:
        - keys:
            publicKeys: |-
              -----BEGIN PUBLIC KEY-----
              MFkw...
              -----END PUBLIC KEY-----
```

### Trivy in CI (GitHub Actions)

```yaml
- name: Scan image
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: ghcr.io/example/app:${{ github.sha }}
    format: sarif
    output: trivy-results.sarif
    severity: CRITICAL,HIGH
    exit-code: '1'    # fail the build on CRITICAL/HIGH
```

### Image pull policy

Always set `imagePullPolicy: Always` for mutable tags (`latest`, branch names). For immutable SHA-pinned references, `IfNotPresent` is acceptable and reduces registry load.

Never use `:latest` in production manifests — pin to a digest:

```yaml
image: ghcr.io/example/app@sha256:abc123def456...
```

---

## Audit Logging

kube-apiserver audit logs are your forensic record of every API call. Without them, post-incident investigation is guesswork.

### Audit policy (minimal production-grade)

```yaml
# /etc/kubernetes/audit-policy.yaml
apiVersion: audit.k8s.io/v1
kind: Policy
rules:
# Log exec/attach at RequestResponse level
- level: RequestResponse
  resources:
  - group: ""
    resources: ["pods/exec", "pods/attach", "pods/portforward"]

# Log secret access at Metadata level (don't log values)
- level: Metadata
  resources:
  - group: ""
    resources: ["secrets", "configmaps"]

# Skip read-only requests on non-sensitive resources
- level: None
  verbs: ["get", "list", "watch"]
  resources:
  - group: ""
    resources: ["events", "nodes", "pods"]

# Default: log metadata for everything else
- level: Metadata
```

Pass to kube-apiserver:

```text
--audit-policy-file=/etc/kubernetes/audit-policy.yaml
--audit-log-path=/var/log/kubernetes/audit.log
--audit-log-maxage=30
--audit-log-maxbackup=10
--audit-log-maxsize=100
```

### Forwarding to SIEM

Use Fluentd/Fluent Bit DaemonSet to tail the audit log file and ship to Elasticsearch, Splunk, or Datadog. Alternatively use `--audit-webhook-config-file` to push directly to a webhook endpoint (Falco, Sysdig, Datadog Agent).

---

## Critical Rules / Gotchas

**NetworkPolicy silently no-ops if CNI doesn't support it.** Verify with `kubectl describe node | grep -i cni` and test with a curl from a pod that should be blocked. Flannel users need Canal or must switch to Calico/Cilium.

**OPA Gatekeeper webhook timeout causes availability risk.** If Gatekeeper pods are down and the webhook `failurePolicy` is `Fail`, all admission requests fail — the cluster stops accepting resource changes. Set `failurePolicy: Ignore` or deploy Gatekeeper with ≥2 replicas and PodDisruptionBudget. Use `dryrun` mode initially to avoid this during rollout.

**Kyverno webhook timeout under load.** Kyverno's mutating webhook runs before the validating webhook. Complex mutation rules (JMESPath lookups against the API) add admission latency. Profile policies with `kyverno test` locally. Set `webhookTimeoutSeconds` in the MutatingWebhookConfiguration if needed (default 10s).

**Kyverno `generate` + `synchronize: true` deletes resources.** If you delete the policy, generated resources (NetworkPolicies, RoleBindings) are also deleted. This can break namespaces silently. Audit generated resources with `kubectl get networkpolicy -A` before removing policies.

**Pod Security Standards `enforce` + existing workloads.** Switching a namespace label to `enforce: restricted` on a namespace with running pods does NOT evict them — it only blocks new/updated pods. Use `audit` and `warn` first, fix violations, then switch to `enforce`.

**Falco eBPF probe not loading on hardened kernels.** Some CIS-hardened nodes disable `bpf` syscall. Check `dmesg | grep -i bpf`. Fall back to kernel module or use Falco's userspace driver (`--driver none` + ptrace) as a last resort.

**Sealed Secrets key rotation.** If the sealing key is lost or the controller namespace is deleted, all SealedSecrets are permanently unreadable. Export and back up the sealing key: `kubectl get secret -n kube-system -l sealedsecrets.bitnami.com/sealed-secrets-key -o yaml`.

**RBAC `list secrets` is equivalent to `get secrets`.** Listing secrets returns their values in the API response. `list` and `watch` on Secrets must be treated with the same caution as `get`.

---

## Key Commands / APIs

```bash
# Check effective permissions for a serviceaccount
kubectl auth can-i --list -n NAMESPACE --as=system:serviceaccount:NAMESPACE:SA_NAME

# Verify Pod Security Admission is enforcing
kubectl label namespace my-ns pod-security.kubernetes.io/enforce=restricted --dry-run=server

# List all Gatekeeper constraint violations
kubectl get constraints -o json | jq '[.items[] | {name:.metadata.name, violations:.status.violations}]'

# Check Kyverno policy reports
kubectl get policyreport -A -o wide
kubectl get clusterpolicyreport -o yaml

# Test a Kyverno policy locally
kyverno test . --detailed-results

# Falco — test rules without starting the daemon
falco --dry-run -r /etc/falco/falco_rules.local.yaml

# Trivy — scan a running cluster for misconfigurations
trivy k8s --report summary cluster

# Detect image running as root
kubectl get pods -A -o json | jq '.items[] | select(.spec.containers[].securityContext.runAsUser == 0 or .spec.containers[].securityContext == null) | .metadata.name'

# Audit who has cluster-admin
kubectl get clusterrolebindings -o json | \
  jq -r '.items[] | select(.roleRef.name=="cluster-admin") | "\(.metadata.name): \(.subjects // [] | map(.name) | join(", "))"'
```

---

## References

- [Kubernetes Security Concepts](https://kubernetes.io/docs/concepts/security/)
- [Pod Security Standards](https://kubernetes.io/docs/concepts/security/pod-security-standards/)
- [Pod Security Admission](https://kubernetes.io/docs/concepts/security/pod-security-admission/)
- [RBAC Good Practices](https://kubernetes.io/docs/concepts/security/rbac-good-practices/)
- [OPA Gatekeeper Docs](https://open-policy-agent.github.io/gatekeeper/website/docs/)
- [OPA Gatekeeper ConstraintTemplates](https://open-policy-agent.github.io/gatekeeper/website/docs/constrainttemplates/)
- [Kyverno Policies](https://kyverno.io/docs/writing-policies/)
- [Falco Documentation](https://falco.org/docs/)
- [External Secrets Operator](https://external-secrets.io/latest/)
- [Cosign / Sigstore](https://docs.sigstore.dev/cosign/overview/)
- [Trivy](https://aquasecurity.github.io/trivy/)
- [Sealed Secrets](https://github.com/bitnami-labs/sealed-secrets)
- [Kyverno vs OPA Gatekeeper Comparison — Nirmata (2025)](https://nirmata.com/2025/02/07/kubernetes-policy-comparison-kyverno-vs-opa-gatekeeper/)
