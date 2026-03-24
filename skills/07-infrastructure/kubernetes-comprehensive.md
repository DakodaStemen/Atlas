---
name: kubernetes-comprehensive
description: Kubernetes comprehensive covering packaging (Helm charts, Kustomize overlays, operators, CRDs), operations (networking, Gateway API, debugging, pod diagnostics), and cloud-managed services (Azure AKS, GCP GKE, Autopilot, Workload Identity). Use for any Kubernetes-related task.
domain: infrastructure
tags: [kubernetes, helm, kustomize, operators, networking, debugging, aks, gke, gateway-api, CRDs]
triggers: kubernetes, helm, kustomize, k8s operator, gateway api, k8s debugging, aks, gke, kubernetes networking
---



# Helm and Helmfile — Kubernetes Package Management

## Chart structure

A Helm chart is a directory with a fixed layout:

```text
mychart/
  Chart.yaml          # required metadata
  values.yaml         # default values
  values.schema.json  # optional JSON Schema for values validation
  templates/          # Go-template Kubernetes manifests
    _helpers.tpl      # named templates (no manifest output)
    deployment.yaml
    service.yaml
    NOTES.txt         # printed to stdout after install/upgrade
  charts/             # vendored subchart tarballs or directories
  crds/               # raw CRD YAMLs — installed before templates, never templated
```

### Chart.yaml required fields

```yaml
apiVersion: v2          # v2 for Helm 3; v1 still accepted but lacks dependency support
name: mychart
version: 1.3.0          # SemVer 2 — the chart version
appVersion: "2.7.1"     # the app version (informational, quote it — it can be a non-SemVer string)
type: application       # or "library" for template-only utility charts
description: Short description of what this chart deploys
```

`apiVersion: v2` is required when declaring dependencies directly in `Chart.yaml` (the Helm 3 way). Helm 2 used a separate `requirements.yaml`.


## Templates and named templates

### quote and integer gotchas

Always quote strings; never quote integers. Kubernetes parses the YAML value and `"80"` is a string, not a port number.

```yaml
# correct
name: {{ .Values.name | quote }}
port: {{ .Values.service.port }}       # integer — no quotes

# wrong — causes Kubernetes to reject the manifest
port: {{ .Values.service.port | quote }}
```

Environment variables must always be strings even when the value is numeric:

```yaml
env:
  - name: REPLICAS
    value: {{ .Values.replicaCount | quote }}
```

### required — fail fast on missing values

```yaml
image: {{ required "image.repository is required" .Values.image.repository | quote }}
```

`required` fails template rendering with a human-readable message when the value is empty or nil. Prefer it over silently producing a broken manifest.

### include vs template

Use `include` (not the built-in `template` action) whenever the result needs to feed a pipeline — `template` cannot be piped.

```yaml
# wrong — cannot pipe template
labels: {{ template "mychart.labels" . | indent 4 }}

# correct
labels:
{{ include "mychart.labels" . | indent 4 }}
```

### Named templates and _helpers.tpl

Files prefixed with `_` in `templates/` produce no Kubernetes manifests. `_helpers.tpl` is the conventional home for `define` blocks.

```yaml
{{/*
Common labels applied to every resource.
*/}}
{{- define "mychart.labels" -}}
app.kubernetes.io/name: {{ include "mychart.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}
```

Consume in a manifest:

```yaml
metadata:
  labels:
    {{- include "mychart.labels" . | nindent 4 }}
```

`nindent` (newline + indent) is cleaner than a leading newline plus `indent`.

### toYaml for arbitrary maps

```yaml
resources:
  {{- toYaml .Values.resources | nindent 2 }}
```

If `.Values.resources` is nil this produces an empty string. Guard if the key is truly optional.

### tpl — rendering values that contain templates

```yaml
# values.yaml
configSnippet: "host={{ .Values.db.host }}"

# template
data:
  config: {{ tpl .Values.configSnippet . | quote }}
```

Useful for letting users embed Helm expressions in values files.

### Checksum-based rolling restarts

Force pods to restart when a ConfigMap or Secret changes without bumping the Deployment version:

```yaml
spec:
  template:
    metadata:
      annotations:
        checksum/config: {{ include (print $.Template.BasePath "/configmap.yaml") . | sha256sum }}
```

### Preserving resources on uninstall

```yaml
metadata:
  annotations:
    "helm.sh/resource-policy": keep
```

Useful for PersistentVolumeClaims and Secrets that must outlive the release. Use sparingly — these resources become untracked orphans.


## Hooks

Hooks are ordinary Kubernetes resources (typically Jobs or Pods) annotated to run at specific lifecycle points instead of as normal release resources.

```yaml
metadata:
  annotations:
    "helm.sh/hook": pre-upgrade
    "helm.sh/hook-weight": "-5"          # lower numbers run first; can be negative
    "helm.sh/hook-delete-policy": before-hook-creation,hook-succeeded
```

### Hook types

| Annotation value | When it runs |
| --- | --- |
| `pre-install` | After template render, before any resources are created |
| `post-install` | After all resources are loaded |
| `pre-upgrade` | After template render, before resources are updated |
| `post-upgrade` | After all resources are upgraded |
| `pre-delete` | Before any resources are deleted |
| `post-delete` | After all resources are deleted |
| `pre-rollback` | After template render, before rollback |
| `post-rollback` | After rollback completes |
| `test` | On-demand via `helm test <release>` |

Helm waits for hook resources to be "Ready" before proceeding. For Jobs this means successful completion; the release fails if the Job fails.

### Hook deletion policies

- `before-hook-creation` (default) — delete the previous hook resource before creating a new one.
- `hook-succeeded` — delete after successful execution. Good for keeping clusters clean.
- `hook-failed` — delete on failure (useful when you only want to keep resources for debugging failed runs; combine with `hook-succeeded` to always delete).

Common pattern for a database migration Job:

```yaml
"helm.sh/hook": pre-upgrade,pre-install
"helm.sh/hook-weight": "0"
"helm.sh/hook-delete-policy": before-hook-creation,hook-succeeded
```


## Install, upgrade, rollback

```bash
# install or upgrade in one command (idempotent CI pattern)
helm upgrade --install myrelease ./mychart \
  --namespace mynamespace --create-namespace \
  -f values-prod.yaml \
  --set image.tag=v1.2.3 \
  --atomic \          # roll back automatically if the upgrade fails
  --timeout 5m \
  --wait              # wait for all pods/jobs to be ready before marking success

# check what would change before applying
helm diff upgrade myrelease ./mychart -f values-prod.yaml   # requires helm-diff plugin

# dry-run with rendered template output
helm install myrelease ./mychart --dry-run --debug

# rollback to the previous revision
helm rollback myrelease

# rollback to a specific revision
helm rollback myrelease 3

# view revision history
helm history myrelease
```

`--atomic` combines `--wait` with automatic rollback on failure, making it the right default for CI pipelines. Avoid using it in development where you want to inspect failing pods.

### Preventing secret regeneration on upgrade

Use `lookup` to check for an existing secret before generating a new random value:

```yaml
{{- $existing := lookup "v1" "Secret" .Release.Namespace "myrelease-creds" -}}
{{- if $existing }}
data:
  password: {{ $existing.data.password }}
{{- else }}
data:
  password: {{ randAlphaNum 24 | b64enc | quote }}
{{- end }}
```

Without this guard, every `helm upgrade` regenerates the secret and breaks connected applications.


## Helmfile — declarative multi-release management

Helmfile manages the full desired state of Helm releases across environments in a single version-controlled YAML file.

### helmfile.yaml structure

```yaml
repositories:
  - name: bitnami
    url: https://charts.bitnami.com/bitnami
  - name: myorg
    url: oci://registry.example.com/helm-charts
    oci: true

releases:
  - name: postgresql
    namespace: data
    chart: bitnami/postgresql
    version: "13.4.0"
    values:
      - values/postgresql.yaml
      - values/postgresql.{{ .Environment.Name }}.yaml   # env-specific
    set:
      - name: auth.postgresPassword
        value: {{ requiredEnv "POSTGRES_PASSWORD" }}
    needs:
      - data/namespace-setup          # wait for this release first

  - name: myapp
    namespace: apps
    chart: myorg/myapp
    version: "1.3.0"
    values:
      - values/myapp.yaml
    needs:
      - data/postgresql
```

### Environments

```yaml
environments:
  dev:
    values:
      - envs/dev.yaml
    kubeContext: docker-desktop
  staging:
    values:
      - envs/staging.yaml
    secrets:
      - envs/staging.secrets.yaml    # decrypted via helm-secrets / SOPS
  production:
    values:
      - envs/production.yaml
    secrets:
      - envs/production.secrets.yaml
```

Select an environment with `--environment`:

```bash
helmfile --environment production sync
```

Inside release values files, reference the environment name:

```yaml
# values/myapp.yaml
logLevel: {{ if eq .Environment.Name "production" }}warn{{ else }}debug{{ end }}
```

### Values layering precedence

1. `bases:` block values
2. Root-level `values:` on the release
3. Environment `values:`
4. CLI `--set` overrides

### DAG-based ordering

`needs:` creates a dependency graph. Helmfile respects the order and runs independent releases concurrently.

```yaml
releases:
  - name: cert-manager
    namespace: cert-manager
    chart: jetstack/cert-manager

  - name: ingress-nginx
    namespace: ingress
    chart: ingress-nginx/ingress-nginx
    needs:
      - cert-manager/cert-manager    # format: namespace/release-name
```

### Core commands

```bash
helmfile sync                          # helm upgrade --install for all releases
helmfile apply                         # diff first, then sync if changes exist
helmfile diff                          # show what would change (requires helm-diff)
helmfile destroy                       # uninstall all releases in reverse dependency order
helmfile deps                          # helm dependency update for all charts
helmfile template                      # render all templates to stdout
helmfile --selector app=myapp sync     # target releases by label
helmfile --environment staging apply
```

### Modular helmfiles

Split large configurations using glob inclusion:

```yaml
helmfiles:
  - path: apps/*/helmfile.yaml
  - path: infra/helmfile.yaml
```

Prefix directories with numbers for explicit ordering when `--sequential-helmfiles` is required:

```text
00-namespaces/helmfile.yaml
01-cert-manager/helmfile.yaml
02-ingress/helmfile.yaml
10-apps/helmfile.yaml
```

### Templating in helmfile.yaml

Helmfile uses Go templates with extra functions. Files ending in `.gotmpl` are auto-templated:

```yaml
# requiredEnv — fail if env var is absent
password: {{ requiredEnv "DB_PASSWORD" }}

# readFile — inline a local file
caCert: |
  {{ readFile "certs/ca.pem" | indent 2 }}

# exec — run a shell command and use output
timestamp: {{ exec "date" (list "+%Y%m%d") }}
```


## Debugging commands

```bash
helm lint ./mychart                          # validate chart structure and templates
helm template myrelease ./mychart -f vals.yaml  # render without installing
helm install myrelease ./mychart --dry-run --debug  # render + show computed values
helm get manifest myrelease                  # show currently deployed manifests
helm get values myrelease                    # show values used in deployed release
helm history myrelease                       # list revisions
helm status myrelease                        # release state and NOTES output
```


---



# Kubernetes Networking: Gateway API

## Gateway API vs Ingress

The classic Ingress resource was designed for HTTP only, crammed all config into annotations, and conflated infrastructure concerns with application routing. Gateway API replaces it with a layered, role-aware model that reached GA (v1.0) in October 2023.

### Why Gateway API

- First-class support for HTTP, HTTPS, TLS passthrough, TCP, UDP, gRPC — no annotation hacks.
- Role separation baked into the resource hierarchy.
- Portable across controllers — same YAML works with Envoy Gateway, Cilium, NGINX Gateway Fabric, Istio.
- Extensible via policy attachment (BackendTLSPolicy, SecurityPolicy, etc.) rather than unstructured annotations.

#### Role separation

| Resource | Owner | Concern |
| --- | --- | --- |
| GatewayClass | Infrastructure provider | Defines the controller implementation |
| Gateway | Cluster operator | Creates listeners, manages certs, sets namespace scope |
| HTTPRoute / GRPCRoute | Application developer | Maps hostnames and paths to Services |

**Ingress migration path:** For ingress-nginx users, the recommended path is to replace `Ingress` objects with `HTTPRoute` objects and swap the ingress-nginx controller for either NGINX Gateway Fabric or Envoy Gateway. Most routing rules map directly; annotation-based rewrites and snippets require policy attachment or implementation-specific CRDs in the new model.


## GatewayClass and Gateway

### GatewayClass

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: GatewayClass
metadata:
  name: envoy-gateway
spec:
  controllerName: gateway.envoyproxy.io/gatewayclass-controller
```

The `controllerName` matches what the controller pod advertises. Each implementation ships its own GatewayClass manifest; do not change `controllerName` manually.

### Gateway with TLS termination

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: prod-gateway
  namespace: infra
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  gatewayClassName: envoy-gateway
  listeners:
  - name: http
    port: 80
    protocol: HTTP
    allowedRoutes:
      namespaces:
        from: All
  - name: https
    hostname: "*.example.com"
    port: 443
    protocol: HTTPS
    tls:
      mode: Terminate
      certificateRefs:
      - name: wildcard-example-com-tls
        namespace: infra
    allowedRoutes:
      namespaces:
        from: Selector
        selector:
          matchLabels:
            gateway-access: "true"
```

#### Listener distinctiveness rules

- HTTP: protocol + port + hostname must be unique.
- HTTPS/TLS: protocol + port + hostname + Secret reference must be unique.
- TCP/UDP: protocol + port only — one Route per listener.

`allowedRoutes.namespaces` accepts `Same` (default), `All`, or `Selector`. The `Selector` option uses standard label selectors on namespace objects.


## Traffic Splitting

### Canary — header-based synthetic traffic

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: app-route
spec:
  hostnames:
  - app.example.com
  rules:
  - matches:
    - headers:
      - name: traffic
        value: test
    backendRefs:
    - name: app-v2
      port: 8080
  - backendRefs:
    - name: app-v1
      port: 8080
```

### Blue-green gradual shift — weight-based

```yaml
rules:
- backendRefs:
  - name: app-v1
    port: 8080
    weight: 90
  - name: app-v2
    port: 8080
    weight: 10
```

Weights are relative, not percentages. `weight: 0` removes a backend from rotation without deleting the entry. To complete a rollout set v1 weight to `0` and v2 weight to `1` (or any non-zero value).


## TLS Configuration

### cert-manager auto-provisioning

Annotate the Gateway; cert-manager watches for listeners with `tls.mode: Terminate` and creates a `Certificate` resource whose resulting Secret matches `tls.certificateRefs[].name`.

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: prod-gateway
  namespace: infra
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  gatewayClassName: envoy-gateway
  listeners:
  - name: https
    hostname: "api.example.com"
    port: 443
    protocol: HTTPS
    tls:
      mode: Terminate
      certificateRefs:
      - name: api-example-com-tls   # cert-manager creates this Secret
```

Requirements for cert-manager to pick up a listener: `hostname` must be set, `tls.mode` must be `Terminate`, `certificateRefs[].name` must be specified, and the Secret must live in the Gateway's namespace (cross-namespace cert references are not supported by cert-manager's Gateway integration).

### ReferenceGrant for cross-namespace Secrets

When a Gateway in `infra-ns` needs a Secret that lives in `certs-ns`:

```yaml
apiVersion: gateway.networking.k8s.io/v1beta1
kind: ReferenceGrant
metadata:
  name: allow-infra-to-certs
  namespace: certs-ns               # grant lives in the TARGET namespace
spec:
  from:
  - group: gateway.networking.k8s.io
    kind: Gateway
    namespace: infra-ns             # who is allowed to reference
  to:
  - group: ""
    kind: Secret                    # what they can reference
```

Without this grant, the Gateway controller rejects the `certificateRefs` entry silently (the Listener status shows `ResolvedRefs: False`).

### BackendTLSPolicy (upstream TLS)

```yaml
apiVersion: gateway.networking.k8s.io/v1alpha3
kind: BackendTLSPolicy
metadata:
  name: api-backend-tls
spec:
  targetRefs:
  - group: ""
    kind: Service
    name: api-svc
  validation:
    hostname: api-svc.apps.svc.cluster.local
    caCertificateRefs:
    - group: ""
      kind: ConfigMap
      name: backend-ca-cert
```


## Envoy Gateway

Envoy Gateway (conformant with Gateway API v1.4.0) uses the standard Gateway API resources plus its own CRDs (`SecurityPolicy`, `BackendTrafficPolicy`, `EnvoyProxy`) for extensions.

### Installation

```bash
helm install eg oci://docker.io/envoyproxy/gateway-helm \
  --version v1.7.1 \
  -n envoy-gateway-system \
  --create-namespace

kubectl apply -f https://github.com/envoyproxy/gateway/releases/download/v1.7.1/quickstart.yaml -n default
```

### EnvoyProxy customization

Attach an `EnvoyProxy` to a GatewayClass to tune the data-plane deployment:

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: EnvoyProxy
metadata:
  name: custom-proxy
  namespace: envoy-gateway-system
spec:
  provider:
    type: Kubernetes
    kubernetes:
      envoyDeployment:
        replicas: 3
        container:
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi

## Cilium Networking

Cilium is both a CNI and a Gateway API implementation. It uses eBPF to intercept and forward traffic in the kernel via TPROXY, with Envoy handling L7 proxy duties transparently.

### Prerequisites

```bash
cilium install --version 1.19.1 \
  --set kubeProxyReplacement=true \
  --set gatewayAPI.enabled=true \
  --set l7Proxy=true
```

Or via Helm values:

```yaml
kubeProxyReplacement: true
l7Proxy: true
gatewayAPI:
  enabled: true
  hostNetwork:
    enabled: true   # use node host network instead of LoadBalancer Service (1.16+)
```

GatewayClass controller name: `cilium`

### Gateway and HTTPRoute with Cilium

```yaml
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: cilium-gateway
  namespace: infra
spec:
  gatewayClassName: cilium
  listeners:
  - name: web
    port: 80
    protocol: HTTP
    allowedRoutes:
      namespaces:
        from: All

## DNS and External-DNS

External-DNS watches Gateway and Route resources and creates DNS records in the configured provider (Route53, Cloudflare, etc.).

### Supported source types

```text
--source=gateway-httproute
--source=gateway-grpcroute
--source=gateway-tlsroute
--source=gateway-tcproute
--source=gateway-udproute
```

HTTPRoute and GRPCRoute hostnames are read from the spec. TCPRoute and UDPRoute have no hostname field — use the annotation:

```text
external-dns.alpha.kubernetes.io/hostname: tcp.example.com
```

### Deployment snippet

```yaml
args:
- --source=service
- --source=gateway-httproute
- --source=gateway-grpcroute
- --domain-filter=example.com
- --provider=cloudflare          # or aws, google, azure, etc.
- --cloudflare-proxied           # enable Cloudflare orange-cloud
- --registry=txt
- --txt-owner-id=my-cluster
```

### RBAC

```yaml
rules:
- apiGroups: ["gateway.networking.k8s.io"]
  resources:
  - gateways
  - httproutes
  - grpcroutes
  - tlsroutes
  - tcproutes
  - udproutes
  verbs: ["get", "watch", "list"]
- apiGroups: [""]
  resources: ["namespaces"]
  verbs: ["get", "watch", "list"]
```

External-DNS reads the `status.addresses` of the Gateway to determine what IP/hostname to publish in DNS. The Gateway must have an assigned address (LoadBalancer IP or hostname) before External-DNS can act.


## References

- [Kubernetes Gateway API — Official Docs](https://gateway-api.sigs.k8s.io/)
- [Gateway API Concepts & Overview](https://gateway-api.sigs.k8s.io/concepts/api-overview/)
- [HTTPRoute Guide](https://gateway-api.sigs.k8s.io/guides/http-routing/)
- [Traffic Splitting Guide](https://gateway-api.sigs.k8s.io/guides/traffic-splitting/)
- [TLS Guide](https://gateway-api.sigs.k8s.io/guides/tls/)
- [GRPCRoute Guide](https://gateway-api.sigs.k8s.io/guides/grpc-routing/)
- [Implementations List](https://gateway-api.sigs.k8s.io/implementations/)
- [Envoy Gateway Docs](https://gateway.envoyproxy.io/docs/)
- [Envoy Gateway Rate Limiting](https://gateway.envoyproxy.io/docs/tasks/traffic/global-rate-limit/)
- [Cilium Gateway API](https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/gateway-api/)
- [cert-manager Gateway Integration](https://cert-manager.io/docs/usage/gateway/)
- [ingress-nginx Annotations](https://kubernetes.github.io/ingress-nginx/user-guide/nginx-configuration/annotations/)
- [External-DNS Gateway API](https://kubernetes-sigs.github.io/external-dns/)


---



# Azure Kubernetes Service, Container Apps, and Service Bus

## AKS Cluster Setup

### System vs User Node Pools

AKS requires at least one system node pool. System node pools run kube-system pods (CoreDNS, metrics-server, konnectivity) and must use a VM SKU with at least 2 vCPUs and 4 GB memory—4 vCPU or more is recommended. They need a minimum of three nodes for reliability.

Application workloads belong in user node pools. This isolation ensures that resource pressure or misbehaving workloads cannot starve the control plane components. A practical minimum is two nodes per user node pool.

Separate workloads into distinct node pools when they have different resource profiles—GPU workloads, memory-optimized batch jobs, or latency-sensitive APIs each benefit from their own pool with appropriate VM SKUs. Avoid proliferating pools unnecessarily; multiple VM SKUs can coexist in a single pool as long as they share the same scheduling requirements.

Enable the cluster autoscaler per node pool. User node pools can scale to zero, which is not possible for system node pools (minimum one node is required there). Combine autoscaler with `HorizontalPodAutoscaler` for pod-level scaling and `VerticalPodAutoscaler` (preview) for right-sizing requests and limits.

Node autoprovisioning (NAP) goes further—it dynamically selects the most cost-efficient VM SKU based on pending pod requirements, removing the need to manually pre-define every node pool profile. AKS Automatic enables NAP by default.

### Network Plugin Choice

For most production clusters use Azure CNI. It assigns real VNet IPs to pods, which is required for Windows node pools, Kubernetes network policies, and direct pod-to-pod communication with other Azure resources. Azure CNI with static block allocation gives pods predictable IP ranges for easier firewall rule authoring and capacity planning.

Kubenet works for smaller clusters where IP address conservation is the top concern, but it lacks support for Windows pools and some network policy implementations.

### Cluster Autoscaler and Availability Zones

Spread node pools across availability zones by setting `--zones 1 2 3` at pool creation time. The cluster autoscaler is zone-aware and will attempt balanced distribution. For critical workloads, deploy multiple clusters in separate regions and route with Azure Front Door or Traffic Manager.

Use the Standard pricing tier for any production cluster—it provides the API server uptime SLA (99.9% for single-region, 99.95% with availability zones). The Free tier has no SLA and no uptime guarantee.

Use a NAT gateway for clusters with high concurrent outbound connections to avoid SNAT port exhaustion on the standard load balancer.


## KEDA on AKS

KEDA (Kubernetes Event-Driven Autoscaling) ships as a cluster extension for AKS. It introduces the `ScaledObject` and `ScaledJob` CRDs and a metrics adapter. KEDA scales Deployments (and Jobs) based on external event sources—over 50 scalers cover Azure Service Bus, Event Hubs, Storage Queues, Prometheus, HTTP, and more.

A `ScaledObject` for Azure Service Bus queue on AKS:

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: order-processor-scaler
  namespace: my-app
spec:
  scaleTargetRef:
    name: order-processor
  minReplicaCount: 0
  maxReplicaCount: 20
  triggers:
    - type: azure-servicebus
      authenticationRef:
        name: keda-sb-auth
      metadata:
        queueName: orders
        namespace: my-servicebus-namespace
        messageCount: "30"     # target messages per replica
```

Authentication via workload identity uses a `TriggerAuthentication` object referencing a service account rather than a connection string:

```yaml
apiVersion: keda.sh/v1alpha1
kind: TriggerAuthentication
metadata:
  name: keda-sb-auth
spec:
  podIdentity:
    provider: azure-workload
```

This avoids storing Service Bus connection strings in Kubernetes secrets. Grant the KEDA operator's service account the `Azure Service Bus Data Owner` or `Azure Service Bus Data Receiver` role on the namespace.

Scale-to-zero with `minReplicaCount: 0` works but introduces cold-start latency. For low-latency requirements keep at least one replica warm.


## Dapr Integration

### On Azure Container Apps

Dapr is a first-class citizen. Enable it per app in the container app configuration:

```json
"dapr": {
  "enabled": true,
  "appId": "order-processor",
  "appProtocol": "http",
  "appPort": 5001
}
```

Dapr components (pub/sub, state store, bindings) are defined at the Container Apps Environment level and automatically available to all enabled apps. No Helm, no cert-manager, no sidecar container manifest—the runtime injects the sidecar.

### On AKS

Install Dapr via Helm or the AKS extension:

```bash
helm repo add dapr https://dapr.github.io/helm-charts/
helm install dapr dapr/dapr --namespace dapr-system --create-namespace
```

Enable sidecar injection per namespace with the annotation:

```text
dapr.io/enabled: "true"
dapr.io/app-id: "order-processor"
dapr.io/app-port: "5001"
```

Component YAML manifests are deployed per namespace. AKS gives full control over Dapr configuration—custom middleware chains, scopes per component, multiple control plane replicas for HA.

The application API is identical on both platforms. Dapr abstractions (`/v1.0/publish`, `/v1.0/invoke`, state store operations) work without code changes when moving between ACA and AKS.

### Dapr + KEDA on ACA

For Dapr pub/sub apps on ACA, combine a KEDA scaling rule with Dapr to scale the subscriber based on Service Bus message count:

```bicep
scale: {
  minReplicas: 0
  maxReplicas: 10
  rules: [
    {
      name: 'sb-topic-scaling'
      custom: {
        type: 'azure-servicebus'
        identity: 'system'
        metadata: {
          topicName: 'orders'
          subscriptionName: 'order-processor-sub'
          messageCount: '30'   // one replica per 30 pending messages
        }
      }
    }
  ]
}
```

Set `identity: 'system'` (or `'user'` with a resource ID) to authenticate via managed identity rather than a connection string. The publisher should set `minReplicas: 1` to keep it running when there is no inbound HTTP traffic.


## Security and Observability

### AKS Network Controls

Enable Azure network policies or Calico to restrict pod-to-pod traffic. Default behavior allows unrestricted communication between pods in a cluster.

For API server access: private clusters prevent traffic from leaving the VNet. For public clusters, configure `--api-server-authorized-ip-ranges` to limit access to known CIDRs (build agents, operations hosts, node pool egress IPs).

Route all egress through Azure Firewall or an HTTP proxy to enforce outbound security policy and prevent data exfiltration.

Disable local accounts (`--disable-local-accounts`) and enforce all cluster access through Microsoft Entra ID RBAC.

### Pod Security

Set resource `requests` and `limits` on every container. Pods without limits can consume unbounded node resources and trigger OOM kills on neighbors. Enforce this at cluster scope with Azure Policy.

Use Pod Security Standards (`restricted` or `baseline` profile) via the built-in admission controller. Avoid running containers as root; avoid `privileged: true`; avoid `hostNetwork` and `hostPID` unless absolutely required.

### Service Bus Security

Prefer Microsoft Entra ID RBAC over Shared Access Signatures for application-level access. Assign specific roles (`Azure Service Bus Data Sender`, `Azure Service Bus Data Receiver`, `Azure Service Bus Data Owner`) at the queue or topic level rather than namespace level to minimize blast radius.

For applications running in AKS, use workload identity to authenticate; no connection strings need to be stored anywhere. For applications outside Azure, use a managed identity if possible, otherwise a service principal with a short-lived certificate credential rather than a client secret.

Enable IP firewall rules and Private Endpoints on Premium namespaces to prevent public internet access.

### Monitoring

Enable Container Insights (Azure Monitor add-on) on AKS for node and pod metrics, logs, and performance trends. For large clusters (hundreds of nodes), enable High Scale mode to reduce agent resource overhead.

Key metrics to alert on: node CPU/memory saturation, pod restart count, HPA at max replicas (signals that autoscaler cannot keep up), and Service Bus DLQ depth.

For Service Bus, monitor `ActiveMessages`, `DeadLetteredMessages`, `ScheduledMessages`, and `IncomingMessages` per entity in Azure Monitor. DLQ depth growing without being drained indicates a consumer defect.


