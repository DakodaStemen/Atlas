---
name: iac-comprehensive
description: Infrastructure as Code patterns covering Pulumi (general-purpose languages, stacks, automation API, AWS), OpenTofu (Terraform fork, state encryption, provider mocking), and Crossplane (Kubernetes-native IaC, XRDs, Compositions). Use when choosing or implementing IaC tools beyond Terraform/CloudFormation.
domain: infrastructure
tags: [pulumi, opentofu, crossplane, iac, infrastructure-as-code, terraform-fork, kubernetes-native]
triggers: pulumi, opentofu, crossplane, IaC, infrastructure as code, tofu, terraform fork, XRD, composition
---

# Infrastructure as Code — Pulumi, OpenTofu, Crossplane

## 1. Pulumi

### When to Use

- Want to use general-purpose languages (TypeScript, Python, Go, C#, Java) instead of HCL.
- Need complex logic (loops, conditionals, abstractions) in infrastructure definitions.
- Building internal developer platforms with the Automation API.
- Teams already proficient in application code who want consistent tooling.

### Core Concepts

- **Projects**: Root unit of organization. Contains `Pulumi.yaml` manifest.
- **Stacks**: Environment instances (dev, staging, prod) of a project.
- **Resources**: Cloud infrastructure objects (same concept as Terraform resources).
- **Inputs/Outputs**: Type-safe wiring between resources (output of one → input of another).
- **ComponentResource**: Reusable abstractions grouping multiple resources.

### AWS Patterns

```typescript
import * as aws from "@pulumi/aws";
import * as eks from "@pulumi/eks";

const vpc = new aws.ec2.Vpc("main", { cidrBlock: "10.0.0.0/16", enableDnsHostnames: true });
const cluster = new eks.Cluster("app", { vpcId: vpc.id, instanceType: "t3.medium", desiredCapacity: 3 });
export const kubeconfig = cluster.kubeconfig;
```

### Automation API

Embed Pulumi operations in application code for self-service infrastructure:

```typescript
const stack = await LocalWorkspace.createOrSelectStack({ stackName: "dev", projectName: "app" });
await stack.up({ onOutput: console.log });
const outputs = await stack.outputs();
```

### State Management

- Default: Pulumi Cloud (managed state, secrets, history).
- Self-managed: S3, Azure Blob, GCS backends.
- Secrets: Encrypted by default. Use `pulumi config set --secret`.

## 2. OpenTofu

### When to Use

- Migrating from Terraform due to licensing (BSL → open-source).
- Need state encryption at rest (native feature, not in Terraform OSS).
- Want provider mocking for testing without real cloud calls.
- Need early variable evaluation in module sources.

### Migration from Terraform

```bash
# Drop-in replacement
tofu init        # Instead of terraform init
tofu plan        # Instead of terraform plan
tofu apply       # Instead of terraform apply
```

Existing `.tf` files and state work without modification. Provider registry compatible.

### State Encryption

```hcl
# backend config
terraform {
  encryption {
    method "aes_gcm" "main" {
      keys = passphrase { passphrase = var.state_passphrase }
    }
    state { method = method.aes_gcm.main }
    plan  { method = method.aes_gcm.main }
  }
}
```

### Provider Mocking

```hcl
# test file
mock_provider "aws" {
  mock_resource "aws_instance" {
    defaults = { id = "i-mock123", public_ip = "1.2.3.4" }
  }
}
```

### Testing

```bash
tofu test              # Run .tftest.hcl files
tofu test -filter=vpc  # Run specific test
```

## 3. Crossplane

### When to Use

- Want Kubernetes-native infrastructure management (kubectl for everything).
- Building a platform API where developers request infrastructure via custom resources.
- Multi-cloud orchestration using Kubernetes control plane.
- GitOps workflow for infrastructure (ArgoCD/Flux managing Crossplane resources).

### Core Concepts

- **Managed Resources**: Direct cloud API wrappers (like Terraform resources but as K8s CRDs).
- **Composite Resources (XR)**: Platform team abstractions combining multiple managed resources.
- **CompositeResourceDefinitions (XRD)**: Schema for platform APIs (like CRDs for XRs).
- **Compositions**: Implementation mapping XR fields to managed resources.
- **Claims (XRC)**: Developer-facing requests for composite resources (namespaced).

### Example Platform API

```yaml
apiVersion: database.example.com/v1
kind: PostgresInstance
metadata:
  name: my-db
spec:
  size: small
  version: "16"
```

The platform team defines the XRD and Composition. Developers just submit Claims.

## Decision Matrix

| Feature | Pulumi | OpenTofu | Crossplane |
|---------|--------|----------|------------|
| **Language** | TS/Python/Go/C#/Java | HCL | YAML (K8s) |
| **State** | Pulumi Cloud / S3 | S3 / encrypted | etcd (K8s) |
| **Learning curve** | Low (if you know the language) | Low (HCL) | Medium (K8s required) |
| **Drift detection** | `pulumi refresh` | `tofu plan` | Continuous reconciliation |
| **Best for** | Dev teams, complex logic | Terraform migration, security | K8s-native platforms |

## Checklist

- [ ] IaC tool chosen based on team skills and requirements
- [ ] State backend configured with encryption
- [ ] Secrets management integrated (never plaintext)
- [ ] CI/CD pipeline runs plan on PR, apply on merge
- [ ] Drift detection scheduled (daily or per-deploy)
- [ ] Modules/components created for reusable patterns
