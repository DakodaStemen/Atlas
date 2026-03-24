---
name: aws-services-comprehensive
description: AWS services comprehensive covering container orchestration (EKS, ECS/Fargate), Infrastructure as Code (CloudFormation, CDK), and messaging (SQS, SNS). Use when building or managing AWS infrastructure, containers, or event-driven architectures on AWS.
domain: infrastructure
tags: [aws, eks, ecs, fargate, cloudformation, cdk, sqs, sns, messaging, containers, iac]
triggers: aws eks, aws ecs, fargate, cloudformation, aws cdk, sqs, sns, aws messaging, aws container, aws iac
---



# AWS EKS Cluster Management Patterns

## Cluster Creation

### Quick start with defaults

```bash
# One-command cluster — 2x m5.large managed nodes, default region
eksctl create cluster --name prod-cluster --region us-east-1 --version 1.30

# Specify zones explicitly (avoids us-east-1 AZ availability issues)
eksctl create cluster --name prod-cluster --region us-east-1 \
  --zones us-east-1a,us-east-1b,us-east-1d
```

### ClusterConfig YAML (recommended for production)

```yaml
apiVersion: eksctl.io/v1alpha5
kind: ClusterConfig

metadata:
  name: prod-cluster
  region: us-east-1
  version: "1.30"
  tags:
    env: production
    team: platform

# Enable all control plane log types
cloudWatch:
  clusterLogging:
    enableTypes:
      - api
      - audit
      - authenticator
      - controllerManager
      - scheduler

# KMS envelope encryption for Secrets
secretsEncryption:
  keyARN: arn:aws:kms:us-east-1:123456789012:key/mrk-xxxxxxxx

vpc:
  subnets:
    private:
      us-east-1a: { id: subnet-aaa111 }
      us-east-1b: { id: subnet-bbb222 }
      us-east-1d: { id: subnet-ccc333 }

# IRSA-enabled service accounts created at cluster creation
iam:
  withOIDC: true
  serviceAccounts:
    - metadata:
        name: aws-load-balancer-controller
        namespace: kube-system
      wellKnownPolicies:
        awsLoadBalancerController: true
    - metadata:
        name: ebs-csi-controller-sa
        namespace: kube-system
      wellKnownPolicies:
        ebsCSIController: true
    - metadata:
        name: cluster-autoscaler
        namespace: kube-system
      wellKnownPolicies:
        autoScaler: true

managedNodeGroups:
  - name: system-ng
    instanceTypes: [m5.large, m5a.large]
    minSize: 2
    maxSize: 4
    desiredCapacity: 2
    privateNetworking: true
    amiFamily: AmazonLinux2023
    updateConfig:
      maxUnavailablePercentage: 33
    labels:
      role: system
    taints:
      - key: CriticalAddonsOnly
        value: "true"
        effect: NoSchedule
    iam:
      withAddonPolicies:
        imageBuilder: false
        autoScaler: false   # Karpenter handles this
        externalDNS: true
        certManager: true
        ebs: true
        efs: true
        albIngress: false   # Using AWS-LBC via IRSA
        cloudWatch: true

addons:
  - name: vpc-cni
    version: latest
    attachPolicyARNs:
      - arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy
  - name: coredns
    version: latest
  - name: kube-proxy
    version: latest
  - name: aws-ebs-csi-driver
    version: latest
    wellKnownPolicies:
      ebsCSIController: true
```

```bash
eksctl create cluster -f cluster.yaml
```

### Version strategy

- Track N-1 from latest EKS release in production; test on N.
- EKS supports each minor version for ~14 months after release.
- Upgrade one minor version at a time — skipping is not supported for control plane.
- Check the [EKS Kubernetes versions page](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html) for end-of-support dates before planning upgrades.

## Managed Node Groups

### Instance type and family selection

```yaml
managedNodeGroups:
  - name: general-ng
    # Specify multiple types — EC2 picks from available capacity
    instanceTypes: [m5.2xlarge, m5a.2xlarge, m6i.2xlarge, m6a.2xlarge]
    minSize: 1
    maxSize: 20
    desiredCapacity: 3
    privateNetworking: true
    amiFamily: AmazonLinux2023   # AL2023 preferred over AL2; bottlerocket also supported
    volumeSize: 50
    volumeType: gp3
    updateConfig:
      maxUnavailablePercentage: 25   # Rolling update — keeps 75% capacity online
    labels:
      node.kubernetes.io/workload-type: general
    iam:
      withAddonPolicies:
        cloudWatch: true
```

For GPU workloads:

```yaml
  - name: gpu-ng
    instanceTypes: [g4dn.xlarge, g4dn.2xlarge]
    amiFamily: AmazonLinux2023
    minSize: 0
    maxSize: 10
    desiredCapacity: 0
    labels:
      nvidia.com/gpu: "true"
    taints:
      - key: nvidia.com/gpu
        value: "true"
        effect: NoSchedule
```

### Launch template integration

Attach a custom launch template when you need custom userdata, specific AMI, or additional security group rules:

```bash
eksctl create nodegroup \
  --cluster prod-cluster \
  --name custom-ng \
  --launch-template-id lt-0123456789abcdef0 \
  --launch-template-version 1
```

### Node group upgrade

```bash
# Upgrade a managed node group to latest AMI for current k8s version
eksctl upgrade nodegroup \
  --cluster prod-cluster \
  --name general-ng \
  --kubernetes-version 1.30
```

Managed node groups respect `updateConfig.maxUnavailablePercentage` during rolling updates — set to 33% for most workloads, lower for stateful or sensitive services.

## Karpenter

Karpenter replaces Cluster Autoscaler with node-level, just-in-time provisioning. Run the Karpenter controller on Fargate or a dedicated managed node group — never on nodes it manages (circular dependency risk).

### EC2NodeClass

```yaml
apiVersion: karpenter.k8s.aws/v1
kind: EC2NodeClass
metadata:
  name: default
spec:
  # Pin AMI alias for reproducible nodes; use specific version in production
  amiSelectorTerms:
    - alias: al2023@latest   # replace with pinned version e.g. al2023@v20240807

  # Discover subnets and security groups by cluster tag
  subnetSelectorTerms:
    - tags:
        karpenter.sh/discovery: "prod-cluster"

  securityGroupSelectorTerms:
    - tags:
        karpenter.sh/discovery: "prod-cluster"

  # IAM role for nodes (not instance profile — Karpenter creates the profile)
  role: "KarpenterNodeRole-prod-cluster"

  # Enforce IMDSv2 — hop limit 1 blocks containers from reaching IMDS
  metadataOptions:
    httpEndpoint: enabled
    httpTokens: required
    httpPutResponseHopLimit: 1
    httpProtocolIPv6: disabled

  blockDeviceMappings:
    - deviceName: /dev/xvda
      ebs:
        volumeSize: 80Gi
        volumeType: gp3
        encrypted: true
        deleteOnTermination: true

  detailedMonitoring: true
  associatePublicIPAddress: false

  tags:
    cluster: prod-cluster
    managed-by: karpenter
```

### NodePool

```yaml
apiVersion: karpenter.sh/v1
kind: NodePool
metadata:
  name: default
spec:
  template:
    spec:
      nodeClassRef:
        group: karpenter.k8s.aws
        kind: EC2NodeClass
        name: default

      # Node expires after 30 days — forces AMI drift refresh
      expireAfter: 720h

      requirements:
        - key: kubernetes.io/arch
          operator: In
          values: [amd64, arm64]
        - key: kubernetes.io/os
          operator: In
          values: [linux]
        # Try spot first; fall through to on-demand
        - key: karpenter.sh/capacity-type
          operator: In
          values: [spot, on-demand]
        - key: karpenter.k8s.aws/instance-category
          operator: In
          values: [c, m, r]
        - key: karpenter.k8s.aws/instance-generation
          operator: Gt
          values: ["4"]
        - key: karpenter.k8s.aws/instance-size
          operator: NotIn
          values: [nano, micro, small, medium]

  disruption:
    # Remove empty or underutilized nodes; consolidate after 1 minute idle
    consolidationPolicy: WhenEmptyOrUnderutilized
    consolidateAfter: 1m
    # Freeze disruption during business hours (scale-down only — new nodes still provision)
    budgets:
      - nodes: "10%"
      - schedule: "0 9 * * mon-fri"
        duration: 8h
        nodes: "0"

  limits:
    cpu: "500"
    memory: 2000Gi
```

### Spot interruption handling

Enable the SQS-based interruption queue so Karpenter taints, drains, and terminates nodes before EC2 reclaims them (typically 2-minute warning):

```bash
# During Karpenter install — pass queue name
helm upgrade --install karpenter oci://public.ecr.aws/karpenter/karpenter \
  --namespace kube-system \
  --set "settings.clusterName=prod-cluster" \
  --set "settings.interruptionQueue=prod-cluster-karpenter" \
  --set controller.resources.requests.cpu=1 \
  --set controller.resources.requests.memory=1Gi
```

Do not run both Karpenter interruption handling and the Node Termination Handler — they conflict.

### Drift detection

When `expireAfter` is set or AMI is updated, Karpenter automatically replaces nodes via controlled drift. Annotate pods that must not be disrupted:

```yaml
metadata:
  annotations:
    karpenter.sh/do-not-disrupt: "true"
```

### GPU NodePool

```yaml
apiVersion: karpenter.sh/v1
kind: NodePool
metadata:
  name: gpu
spec:
  template:
    spec:
      nodeClassRef:
        group: karpenter.k8s.aws
        kind: EC2NodeClass
        name: default
      taints:
        - key: nvidia.com/gpu
          value: "true"
          effect: NoSchedule
      requirements:
        - key: node.kubernetes.io/instance-type
          operator: In
          values: [g4dn.xlarge, g4dn.2xlarge, g5.xlarge, g5.2xlarge]
        - key: karpenter.sh/capacity-type
          operator: In
          values: [spot, on-demand]
  limits:
    cpu: "100"
```

## IRSA — IAM Roles for Service Accounts

IRSA lets pods assume IAM roles without node-level permissions. The pod receives a projected service account token; the AWS SDK exchanges it via `sts:AssumeRoleWithWebIdentity`.

### Enable OIDC provider

```bash
eksctl utils associate-iam-oidc-provider \
  --cluster prod-cluster \
  --approve
```

### Create IAM role + service account via eksctl

```bash
eksctl create iamserviceaccount \
  --cluster prod-cluster \
  --namespace my-app \
  --name my-app-sa \
  --attach-policy-arn arn:aws:iam::123456789012:policy/MyAppPolicy \
  --approve
```

### IAM trust policy (scoped to exact namespace + SA)

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::123456789012:oidc-provider/oidc.eks.us-east-1.amazonaws.com/id/EXAMPLID"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "oidc.eks.us-east-1.amazonaws.com/id/EXAMPLID:aud": "sts.amazonaws.com",
          "oidc.eks.us-east-1.amazonaws.com/id/EXAMPLID:sub": "system:serviceaccount:my-app:my-app-sa"
        }
      }
    }
  ]
}
```

Always scope the `sub` condition to the exact namespace and service account name — a wildcard allows any SA in the cluster to assume the role.

### Service account annotation

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: my-app-sa
  namespace: my-app
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/my-app-role
# Disable default token mount when IRSA is the only auth mechanism
automountServiceAccountToken: false
```

Pod spec:

```yaml
spec:
  serviceAccountName: my-app-sa
  securityContext:
    fsGroup: 65534   # required for token file ownership
  containers:
    - name: app
      image: myapp:latest
      # AWS SDK automatically picks up AWS_ROLE_ARN and AWS_WEB_IDENTITY_TOKEN_FILE
```

### IRSA vs EKS Pod Identity

| | IRSA | Pod Identity |
| --- | --- | --- |
| OIDC provider required | Yes (per cluster) | No |
| Cross-account access | Direct via AssumeRoleWithWebIdentity | Role chaining needed |
| ABAC session tags | No | Yes |
| SDK version requirement | Older SDKs supported | Requires recent SDK versions |
| Private cluster IRSA | Needs STS VPC endpoint | N/A |

For new workloads on EKS 1.24+, Pod Identity is simpler to manage (no OIDC setup per cluster). IRSA remains the standard for cross-account scenarios and older SDK compatibility.

## AWS Load Balancer Controller

The AWS Load Balancer Controller (AWS-LBC) provisions ALBs for Kubernetes Ingress and NLBs for Services of type LoadBalancer.

### Install via Helm (after IRSA SA is created)

```bash
helm repo add eks https://aws.github.io/eks-charts
helm repo update

helm install aws-load-balancer-controller eks/aws-load-balancer-controller \
  --namespace kube-system \
  --set clusterName=prod-cluster \
  --set serviceAccount.create=false \
  --set serviceAccount.name=aws-load-balancer-controller \
  --set replicaCount=2
```

### ALB Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-app-ingress
  namespace: my-app
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing   # or internal
    alb.ingress.kubernetes.io/target-type: ip           # ip preferred over instance
    alb.ingress.kubernetes.io/listen-ports: '[{"HTTPS":443}]'
    alb.ingress.kubernetes.io/certificate-arn: arn:aws:acm:us-east-1:123456789012:certificate/xxxx
    alb.ingress.kubernetes.io/ssl-policy: ELBSecurityPolicy-TLS13-1-2-2021-06
    alb.ingress.kubernetes.io/wafv2-acl-arn: arn:aws:wafv2:us-east-1:123456789012:regional/webacl/my-acl/xxxx
    alb.ingress.kubernetes.io/group.name: prod-shared   # IngressGroup — shared ALB
    alb.ingress.kubernetes.io/healthcheck-path: /healthz
    alb.ingress.kubernetes.io/success-codes: "200"
spec:
  rules:
    - host: api.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: my-app-svc
                port:
                  number: 80
```

Use IP target type to route directly to pods — avoids double NAT through kube-proxy and reduces latency.

### NLB for TCP/UDP services

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-tcp-service
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: external
    service.beta.kubernetes.io/aws-load-balancer-nlb-target-type: ip
    service.beta.kubernetes.io/aws-load-balancer-scheme: internet-facing
    service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled: "true"
spec:
  type: LoadBalancer
  selector:
    app: my-tcp-app
  ports:
    - port: 443
      targetPort: 8443
      protocol: TCP
```

### TargetGroupBinding — attach existing TGB to pods

```yaml
apiVersion: elbv2.k8s.aws/v1beta1
kind: TargetGroupBinding
metadata:
  name: my-tgb
spec:
  serviceRef:
    name: my-app-svc
    port: 80
  targetGroupARN: arn:aws:elasticloadbalancing:us-east-1:123456789012:targetgroup/my-tg/xxxx
  targetType: ip
```

### Zero-downtime deployment checklist

- Set readiness probes so pods aren't registered until healthy.
- Add `preStop: exec: command: [sleep, "10"]` to allow ELB target deregistration before pod termination.
- Use Pod Disruption Budgets to protect against simultaneous eviction.

## EKS Add-ons

Managed add-ons are reconciled by EKS. Do not modify them with `kubectl` — changes will be overwritten.

### Core add-ons and their purposes

| Add-on | Purpose | Notes |
| --- | --- | --- |
| `vpc-cni` | Pod networking (AWS VNI) | Attach IAM policy `AmazonEKS_CNI_Policy` via IRSA |
| `coredns` | Cluster DNS | Scale replicas for large clusters |
| `kube-proxy` | iptables/IPVS rules | Keep in sync with k8s version |
| `aws-ebs-csi-driver` | EBS persistent volumes | Requires IRSA with EBS permissions |
| `aws-efs-csi-driver` | EFS persistent volumes | Requires IRSA + EFS mount target |
| `adot` | AWS Distro for OpenTelemetry | Metrics/traces pipeline |
| `amazon-cloudwatch-observability` | Container Insights + Fluent Bit | Replaces legacy cloudwatch-agent DaemonSet |

### Managing add-on versions

```bash
# List available versions for a given add-on
aws eks describe-addon-versions \
  --addon-name vpc-cni \
  --kubernetes-version 1.30 \
  --query 'addons[].addonVersions[].addonVersion'

# Update add-on (one minor version at a time)
aws eks update-addon \
  --cluster-name prod-cluster \
  --addon-name vpc-cni \
  --addon-version v1.18.1-eksbuild.1 \
  --resolve-conflicts OVERWRITE

# Check update status
aws eks describe-addon \
  --cluster-name prod-cluster \
  --addon-name vpc-cni \
  --query 'addon.status'
```

Add-ons do not auto-upgrade during control plane upgrades. After each control plane upgrade, verify each add-on is compatible with the new k8s version and update manually.

### VPC-CNI prefix delegation (more IPs per node)

```bash
kubectl set env daemonset aws-node \
  -n kube-system \
  ENABLE_PREFIX_DELEGATION=true \
  WARM_PREFIX_TARGET=1
```

Increases available pod IPs significantly — a `m5.large` goes from ~29 pods to ~110.

## VPC and Networking

### Subnet tagging requirements

EKS uses subnet tags to discover where to place load balancers and nodes. Missing tags = silent failures.

```bash
# Public subnets — internet-facing ALBs/NLBs
aws ec2 create-tags --resources subnet-aaa111 --tags \
  Key=kubernetes.io/role/elb,Value=1 \
  Key=kubernetes.io/cluster/prod-cluster,Value=shared

# Private subnets — internal LBs and worker nodes
aws ec2 create-tags --resources subnet-bbb222 --tags \
  Key=kubernetes.io/role/internal-elb,Value=1 \
  Key=kubernetes.io/cluster/prod-cluster,Value=shared

# Karpenter subnet discovery
aws ec2 create-tags --resources subnet-bbb222 --tags \
  Key=karpenter.sh/discovery,Value=prod-cluster
```

### Security groups for pods (SGP)

SGP lets you attach VPC security groups directly to individual pods instead of nodes. Requires trunk network interfaces — not all instance types support it.

```bash
# Enable on vpc-cni
kubectl set env daemonset aws-node \
  -n kube-system \
  ENABLE_POD_ENI=true

# Annotate pod/deployment
kubectl annotate pod my-pod \
  vpc.amazonaws.com/pod-eni='[{"eniId":"eni-xxxx","ifAddress":"xx:xx:xx:xx:xx:xx","privateIp":"10.0.0.x","vlanId":1,"subnetCidr":"10.0.0.0/24"}]'
```

Then reference a SecurityGroup in a SecurityGroupPolicy:

```yaml
apiVersion: vpcresources.k8s.aws/v1beta1
kind: SecurityGroupPolicy
metadata:
  name: my-sg-policy
  namespace: my-app
spec:
  podSelector:
    matchLabels:
      app: my-app
  securityGroups:
    groupIds:
      - sg-0123456789abcdef0
```

### Endpoint access modes

| Mode | Internal traffic | External access |
| --- | --- | --- |
| Public only (default) | Via internet | Yes |
| Public + Private | Via cross-account ENIs in VPC | Yes |
| Private only | Via cross-account ENIs in VPC | No (needs PrivateLink/bastion) |

Public + Private is the recommended default for production — internal kubelet/node traffic stays within the VPC, reducing costs and latency.

### NAT Gateway placement

One NAT Gateway per AZ avoids cross-AZ data transfer charges and eliminates single-AZ dependency. Three AZs = three NAT Gateways.

### Cluster subnet sizing

Reserve at least `/28` subnets for EKS cluster ENIs (control plane cross-account interfaces). These subnets are separate from node subnets. Undersizing causes upgrade failures when EKS needs to place additional ENIs.

## EKS Security

### Control plane logging

Enable all five log types at cluster creation — audit logs are essential for incident response:

```bash
aws eks update-cluster-config \
  --name prod-cluster \
  --logging '{"clusterLogging":[{"types":["api","audit","authenticator","controllerManager","scheduler"],"enabled":true}]}'
```

### IMDSv2 enforcement

Block instance metadata access from pods by setting hop limit to 1 on all nodes:

```bash
# Node launch template or eksctl nodeGroup.instanceMetadataOptions
aws ec2 modify-instance-metadata-options \
  --instance-id i-xxxx \
  --http-tokens required \
  --http-put-response-hop-limit 1
```

In eksctl ClusterConfig under each node group:

```yaml
    instanceMetadataOptions:
      httpTokens: required
      httpPutResponseHopLimit: 1
```

### aws-auth ConfigMap → Access Entries migration

The `aws-auth` ConfigMap is the legacy mechanism for mapping IAM principals to Kubernetes RBAC. It is a single unannotated ConfigMap — misconfiguration can lock you out of the cluster. The modern replacement is the Cluster Access Manager API (Access Entries).

**Migration path** (irreversible once set to API-only):

```bash
# Step 1: add API support alongside ConfigMap
aws eks update-cluster-config \
  --name prod-cluster \
  --access-config authenticationMode=API_AND_CONFIG_MAP

# Step 2: create access entries for all existing mapRoles/mapUsers entries
aws eks create-access-entry \
  --cluster-name prod-cluster \
  --principal-arn arn:aws:iam::123456789012:role/DevTeamRole \
  --type STANDARD

aws eks associate-access-policy \
  --cluster-name prod-cluster \
  --principal-arn arn:aws:iam::123456789012:role/DevTeamRole \
  --policy-arn arn:aws:eks::aws:cluster-access-policy/AmazonEKSViewPolicy \
  --access-scope '{"type":"namespace","namespaces":["my-app"]}'

# Step 3: after validating, move to API-only (disables ConfigMap — cannot reverse)
aws eks update-cluster-config \
  --name prod-cluster \
  --access-config authenticationMode=API
```

Node roles must also be added as access entries (type `EC2_LINUX`) before disabling ConfigMap.

### Envelope encryption for Kubernetes Secrets

```yaml
# In ClusterConfig
secretsEncryption:
  keyARN: arn:aws:kms:us-east-1:123456789012:key/mrk-xxxxxxxx
```

Or via CLI:

```bash
aws eks associate-encryption-config \
  --cluster-name prod-cluster \
  --encryption-config '[{"resources":["secrets"],"provider":{"keyArn":"arn:aws:kms:..."}}]'
```

### Pod Security Standards

```bash
# Apply Restricted standard to a namespace
kubectl label namespace my-app \
  pod-security.kubernetes.io/enforce=restricted \
  pod-security.kubernetes.io/audit=restricted \
  pod-security.kubernetes.io/warn=restricted
```

### Disable automountServiceAccountToken where unnecessary

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: no-token-sa
  namespace: my-app
automountServiceAccountToken: false
```

### CIS benchmark validation

```bash
# hardeneks — AWS-provided Python CLI to check EKS against CIS benchmark
pip install hardeneks
hardeneks --cluster-name prod-cluster --region us-east-1
```

## Cluster Upgrade Strategy

### In-place upgrade (standard path)

```bash
# 1. Check for deprecated API usage
kubent   # or: pluto detect-all-in-cluster

# 2. Verify subnet IP availability (need ≥ 5 free IPs per subnet)
CLUSTER=prod-cluster
aws ec2 describe-subnets \
  --subnet-ids $(aws eks describe-cluster --name ${CLUSTER} \
    --query 'cluster.resourcesVpcConfig.subnetIds' --output text) \
  --query 'Subnets[*].[SubnetId,AvailableIpAddressCount]' \
  --output table

# 3. Upgrade control plane (one minor version at a time)
aws eks update-cluster-version \
  --name prod-cluster \
  --kubernetes-version 1.31

# Wait for control plane upgrade to complete
aws eks wait cluster-active --name prod-cluster

# 4. Update add-ons to versions compatible with new k8s version
aws eks update-addon --cluster-name prod-cluster --addon-name vpc-cni --addon-version v1.19.0-eksbuild.1
aws eks update-addon --cluster-name prod-cluster --addon-name coredns --addon-version v1.11.1-eksbuild.4
aws eks update-addon --cluster-name prod-cluster --addon-name kube-proxy --addon-version v1.31.0-eksbuild.2
aws eks update-addon --cluster-name prod-cluster --addon-name aws-ebs-csi-driver --addon-version v1.35.0-eksbuild.1

# 5. Upgrade managed node groups
eksctl upgrade nodegroup \
  --cluster prod-cluster \
  --name general-ng \
  --kubernetes-version 1.31

# 6. If using Karpenter — drift handles node replacement automatically
# Nodes with expireAfter set will cycle; or force drift:
kubectl annotate ec2nodeclass default \
  karpenter.k8s.aws/ami-selector-terms-hash-$(date +%s)=force-drift
```

### Blue-green cluster migration

Used when jumping multiple versions or testing a major configuration change. Costs roughly 2x during cutover.

1. Create new cluster at target version with new ClusterConfig.
2. Deploy workloads to new cluster (GitOps / re-apply manifests).
3. Use Route 53 weighted routing or ALB to shift traffic incrementally.
4. Restore stateful data from Velero backup or re-provision persistent volumes.
5. Monitor for 24-48 hours, then decommission old cluster.

Limitations: API endpoint changes break hardcoded kubeconfig references; stateful workloads (databases, PVCs) require careful migration planning.

### Rollback

EKS clusters cannot be downgraded after a control plane upgrade. Recovery requires creating a new cluster at the prior version and restoring workloads from backup (Velero for PVs, GitOps for manifests).

Plan upgrades in staging first. Never skip minor versions.

## Observability

### Container Insights (managed add-on)

```bash
aws eks create-addon \
  --cluster-name prod-cluster \
  --addon-name amazon-cloudwatch-observability \
  --service-account-role-arn arn:aws:iam::123456789012:role/CWAgentRole
```

This single add-on replaces the legacy `cloudwatch-agent` DaemonSet and the standalone Fluent Bit DaemonSet. It ships metrics to CloudWatch Container Insights and logs to CloudWatch Logs.

### Fluent Bit for structured log forwarding

If using the managed add-on, Fluent Bit is bundled. For custom configuration:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluent-bit-config
  namespace: amazon-cloudwatch
data:
  fluent-bit.conf: |
    [SERVICE]
        Flush         5
        Log_Level     info
    [INPUT]
        Name              tail
        Tag               kube.*
        Path              /var/log/containers/*.log
        Parser            docker
        DB                /var/log/flb_kube.db
    [FILTER]
        Name              kubernetes
        Match             kube.*
        Merge_Log         On
        Keep_Log          Off
    [OUTPUT]
        Name              cloudwatch_logs
        Match             kube.*
        region            us-east-1
        log_group_name    /aws/containerinsights/prod-cluster/application
        log_stream_prefix ${HOSTNAME}-
        auto_create_group true
```

### AWS Distro for OpenTelemetry (ADOT)

```bash
# Install via managed add-on
aws eks create-addon \
  --cluster-name prod-cluster \
  --addon-name adot \
  --service-account-role-arn arn:aws:iam::123456789012:role/ADOTRole
```

Configure an `OpenTelemetryCollector` CR to pipeline traces to X-Ray and metrics to CloudWatch EMF or Amazon Managed Prometheus.

### Prometheus + Grafana (self-managed)

```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install kube-prometheus-stack prometheus-community/kube-prometheus-stack \
  --namespace monitoring --create-namespace \
  --set prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.storageClassName=gp3 \
  --set prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.resources.requests.storage=50Gi
```

### Useful CloudWatch Insights query — detect aws-auth mutations

```sql
fields @timestamp, @message
| filter objectRef.resource="configmaps"
    and objectRef.name="aws-auth"
    and verb in ["update","patch"]
| sort @timestamp desc
```

## Cost Optimization

### Karpenter spot consolidation

Use `WhenEmptyOrUnderutilized` consolidation policy and allow both spot and on-demand in the same NodePool. Karpenter will prefer spot when available and bin-pack to remove underutilized nodes automatically.

Annotate long-running batch jobs or stateful pods with `karpenter.sh/do-not-disrupt: "true"` to prevent premature eviction.

### Graviton instances (20% cheaper per vCPU)

```yaml
requirements:
  - key: kubernetes.io/arch
    operator: In
    values: [arm64, amd64]   # allow both; Karpenter picks cheapest available
```

Ensure container images are multi-arch (`linux/amd64,linux/arm64`) before enabling arm64.

### Right-sizing workloads

- Use [Goldilocks](https://github.com/FairwindsOps/goldilocks) to get VPA recommendations in a dashboard.
- Use [KRR (Kubernetes Resource Recommender)](https://github.com/robusta-dev/krr) for per-workload CPU/memory request recommendations.
- Use [Kubecost](https://www.kubecost.com/) for per-namespace/team cost attribution.

### Savings Plans

- **Compute Savings Plans**: up to 66% discount, flexible across instance family, region, and OS. Best fit for Karpenter clusters with diverse instance types.
- **EC2 Instance Savings Plans**: up to 72%, locked to region + instance family. Use for stable baseline on-demand capacity in managed node groups.

Commit only to what you can measure. Use AWS Compute Optimizer to get machine-learning-based instance size recommendations before purchasing.

### Cluster right-sizing

```bash
# Check node utilization to identify over-provisioned nodes
kubectl top nodes

# Get Compute Optimizer recommendations for EKS node groups
aws compute-optimizer get-ec2-instance-recommendations \
  --filters Name=Finding,Values=Overprovisioned \
  --query 'instanceRecommendations[].{Instance:instanceArn,CurrentType:currentInstanceType,Recommended:recommendationOptions[0].instanceType}'
```

## Critical Rules and Gotchas

**aws-auth ConfigMap is a single point of failure.** A bad edit locks everyone out including the cluster creator (unless they have full IAM permissions at the EKS API level). Always edit with `eksctl create iamidentitymapping` or `kubectl edit` with a backup. Migrate to Access Entries at your next maintenance window.

**Subnet IP exhaustion silently breaks scaling.** EKS, Karpenter, and managed node groups all fail to place nodes when subnets run out of IP space. Use VPC-CNI prefix delegation and properly sized subnets (/21 or larger for worker subnets). Monitor `AvailableIPAddressCount` via CloudWatch or VPC metrics.

**IMDSv2 hop limit.** Setting `httpPutResponseHopLimit: 2` allows containers on the node to reach IMDS and inherit the node's instance profile — which typically has broad EC2/ECR permissions. Always set hop limit to 1 for multi-tenant clusters.

**Add-ons do not auto-upgrade.** After every control plane upgrade, manually verify and update each add-on. Incompatible add-on versions cause subtle networking and DNS failures.

**One minor version at a time.** EKS control plane and managed node group upgrades each allow only one minor version jump. Plan multi-version upgrades as multiple steps with testing between each.

**Karpenter controller must not run on Karpenter-managed nodes.** If Karpenter's controller pod is evicted during scale-down, it cannot provision replacement nodes — deadlock. Use a Fargate profile or a dedicated managed node group with a taint + toleration.

**Karpenter v0.x Provisioner API is removed in v1.x.** If upgrading Karpenter from 0.x to 1.x, all `Provisioner` and `AWSNodeTemplate` objects must be migrated to `NodePool` and `EC2NodeClass`.

**CoreDNS lameduck.** Configure a lameduck duration in CoreDNS to avoid DNS failures during rapid node churn — pods may query a CoreDNS pod that is terminating:

```yaml
# CoreDNS Corefile
.:53 {
    health {
        lameduck 15s
    }
    ready
    ...
}
```

**Private cluster requirements.** Private-only API endpoint clusters need VPC endpoints for: `sts` (IRSA), `ssm` (Karpenter launch), `ec2`, `ecr.api`, `ecr.dkr`, `s3` (for ECR layer pulls), and `logs` (CloudWatch). Missing endpoints cause silent bootstrap failures.

## References

- [AWS EKS Best Practices Guide](https://aws.github.io/aws-eks-best-practices/)
- [Karpenter Documentation](https://karpenter.sh/docs/)
- [eksctl Documentation](https://eksctl.io/)
- [AWS Load Balancer Controller](https://kubernetes-sigs.github.io/aws-load-balancer-controller/)
- [EKS Add-ons Version Compatibility](https://docs.aws.amazon.com/eks/latest/userguide/managing-add-ons.html)
- [EKS Cluster Access Manager (Access Entries)](https://docs.aws.amazon.com/eks/latest/userguide/access-entries.html)
- [VPC CNI Prefix Delegation](https://docs.aws.amazon.com/eks/latest/userguide/cni-increase-ip-addresses.html)
- [EKS Pod Identity](https://docs.aws.amazon.com/eks/latest/userguide/pod-identities.html)
- [hardeneks CIS benchmark tool](https://github.com/aws-samples/hardeneks)


---



# AWS CloudFormation

## Template Structure

A CloudFormation template is a JSON or YAML document with up to eight top-level sections. YAML is strongly preferred for hand-authored templates because it supports comments and is less error-prone to edit. JSON is better when templates are generated programmatically.

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Description: "What this stack does"

Parameters: {}     # Inputs supplied at deploy time
Mappings: {}       # Static lookup tables keyed by region, env, etc.
Conditions: {}     # Boolean expressions that gate resource creation
Transform: []      # SAM or custom macro transforms applied before provisioning
Resources: {}      # Required — the actual AWS resources
Outputs: {}        # Values exported for cross-stack reference or display
```

`Resources` is the only required section. Every other section is optional.


## Mappings and Conditions

### Mappings — static lookup tables

```yaml
Mappings:
  InstanceSizeByEnv:
    prod:
      web: t3.large
      db: r6g.xlarge
    dev:
      web: t3.micro
      db: t3.small

Resources:
  WebServer:
    Type: AWS::EC2::Instance
    Properties:
      InstanceType: !FindInMap [InstanceSizeByEnv, !Ref Environment, web]
```

### Conditions — gate resource creation

```yaml
Conditions:
  IsProd: !Equals [!Ref Environment, prod]
  CreateReplica: !And
    - !Condition IsProd
    - !Equals [!Ref EnableReplica, "true"]

Resources:
  ReadReplica:
    Type: AWS::RDS::DBInstance
    Condition: CreateReplica
    Properties: ...
```


## StackSets: Multi-Account and Multi-Region Deployments

StackSets deploy a single template to multiple AWS accounts and regions in one operation.

### Permission models

**Service-managed (recommended for AWS Organizations users):** No manual IAM role setup. CloudFormation uses Organizations-level trust. Supports auto-deployment to new accounts joining an OU.

**Self-managed:** You create `AWSCloudFormationStackSetAdministrationRole` in the admin account and `AWSCloudFormationStackSetExecutionRole` in each target account manually.

### Deployment options

```yaml
# Conservative rollout — safest for production
MaxConcurrentCount: 1
FailureToleranceCount: 0
# Start with a single low-impact region; expand after validation

# Faster rollout
MaxConcurrentPercentage: 25
FailureTolerancePercentage: 10
```

**Staged approach for large fleets:** Deploy to a handful of test accounts first. Verify, then expand to the full set. Only one StackSet operation runs at a time per StackSet, so overlapping operations will queue.

### Common StackSets use cases

- Deploy AWS Config rules, GuardDuty, Security Hub across all member accounts
- Provision baseline IAM roles (break-glass, cross-account access) org-wide
- Replicate shared networking (VPC endpoints, DNS resolvers) across regions
- Enforce tagging policies via CloudFormation Hooks

### Important StackSets gotchas

- Global resources (IAM roles, S3 buckets) can collide on names when deployed to multiple regions in the same account. Use `AWS::Region` in names to disambiguate.
- Stack instances in multiple regions count toward your per-region stack quota.
- Removing an account from an OU does not automatically delete its stack instances unless you explicitly enable automatic deployment with `RetainStacksOnAccountRemoval: false`.


## Custom Resources with Lambda

Custom resources let you run arbitrary code during stack create, update, and delete. Use them when no native CloudFormation resource exists, or when you need to call an external API, generate a value, or coordinate with something outside AWS.

### Minimal skeleton

```yaml
Resources:
  MyFunction:
    Type: AWS::Lambda::Function
    Properties:
      Handler: index.handler
      Runtime: python3.12
      Role: !GetAtt LambdaRole.Arn
      Timeout: 300
      Code:
        ZipFile: |
          import json, urllib3
          http = urllib3.PoolManager()

          def handler(event, context):
              response = {
                  "Status": "SUCCESS",
                  "RequestId": event["RequestId"],
                  "StackId": event["StackId"],
                  "LogicalResourceId": event["LogicalResourceId"],
                  "PhysicalResourceId": event.get("PhysicalResourceId", "my-resource-id"),
                  "Data": {"OutputKey": "OutputValue"},
              }
              # MUST PUT to the pre-signed S3 URL — not return from Lambda
              http.request("PUT", event["ResponseURL"],
                           body=json.dumps(response),
                           headers={"Content-Type": ""})

  MyCustomResource:
    Type: Custom::AcmeCertValidator
    Properties:
      ServiceToken: !GetAtt MyFunction.Arn
      ServiceTimeout: 300   # seconds; default is 3600
      DomainName: example.com
```

### Request format (what Lambda receives)

```json
{
  "RequestType": "Create",          // Create | Update | Delete
  "RequestId": "abc-123",
  "StackId": "arn:aws:cloudformation:...",
  "ResponseURL": "https://s3.presigned.url...",
  "ResourceType": "Custom::AcmeCertValidator",
  "LogicalResourceId": "MyCustomResource",
  "PhysicalResourceId": null,       // present on Update and Delete
  "ResourceProperties": { "DomainName": "example.com" },
  "OldResourceProperties": {}       // only on Update
}
```

### Response format (what Lambda PUTs to S3)

```json
{
  "Status": "SUCCESS",              // or "FAILED"
  "Reason": "optional error text",  // required when Status=FAILED
  "RequestId": "abc-123",
  "StackId": "arn:aws:cloudformation:...",
  "LogicalResourceId": "MyCustomResource",
  "PhysicalResourceId": "my-resource-id",  // required
  "Data": { "OutputKey": "OutputValue" },
  "NoEcho": false                   // set true to hide Data from DescribeStackEvents
}
```

### Critical gotchas

- **Always respond to the pre-signed URL.** If Lambda times out without responding, CloudFormation will wait up to `ServiceTimeout` seconds then mark the resource FAILED. Set `ServiceTimeout` to a value shorter than your Lambda timeout plus a buffer.
- **`PhysicalResourceId` controls update vs. replace.** If your Update handler returns a different `PhysicalResourceId` than the one from Create, CloudFormation will call Delete on the old ID. This can cause unintended resource deletion.
- **Handle Delete gracefully.** On stack delete, your function receives `RequestType: Delete`. Always respond SUCCESS (even if the resource is already gone) to avoid the stack getting stuck.
- **Idempotency.** CloudFormation can retry requests. Make Create and Delete idempotent — check if the resource already exists before creating it, and ignore "not found" on delete.
- **VPC access.** Custom resources must be able to reach the pre-signed S3 response URL. In a private subnet, add a VPC endpoint for S3 or use a NAT gateway.
- **Use the `cfn-response` module** (included in the Lambda Node.js and Python runtimes) to avoid hand-rolling the PUT logic.


## SAM — Serverless Application Model

SAM is a CloudFormation macro (`AWS::Serverless` transform) that adds higher-level shorthand resource types for serverless applications. SAM templates are valid CloudFormation templates.

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Transform: AWS::Serverless-2016-10-31

Globals:
  Function:
    Runtime: python3.12
    MemorySize: 256
    Timeout: 30
    Environment:
      Variables:
        TABLE_NAME: !Ref Table

Resources:
  ApiFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: src/
      Handler: app.handler
      Events:
        ApiEvent:
          Type: Api
          Properties:
            Path: /items
            Method: get

  Table:
    Type: AWS::Serverless::SimpleTable
    Properties:
      PrimaryKey:
        Name: id
        Type: String
```

SAM resource types and what they expand to:

| SAM type | Expands to |
| --- | --- |
| `AWS::Serverless::Function` | Lambda Function + IAM Role + optional event sources |
| `AWS::Serverless::Api` | API Gateway RestAPI + Stage + Deployment |
| `AWS::Serverless::HttpApi` | API Gateway v2 HttpApi |
| `AWS::Serverless::SimpleTable` | DynamoDB Table (single hash key) |
| `AWS::Serverless::StateMachine` | Step Functions state machine |
| `AWS::Serverless::LayerVersion` | Lambda Layer |
| `AWS::Serverless::Application` | Nested SAR application |

Use `sam build` and `sam deploy` instead of `aws cloudformation deploy` — SAM handles packaging (uploading code to S3/ECR) before deploying.


## Change Sets: Review Before You Apply

Never update a production stack directly. Always create a change set first to see what CloudFormation will do.

```bash
# Create the change set
aws cloudformation create-change-set \
  --stack-name my-stack \
  --template-body file://template.yaml \
  --parameters ParameterKey=Environment,ParameterValue=prod \
  --change-set-name preview-$(date +%Y%m%d-%H%M%S) \
  --capabilities CAPABILITY_NAMED_IAM

# Review what will change
aws cloudformation describe-change-set \
  --stack-name my-stack \
  --change-set-name preview-...

# Apply only when satisfied
aws cloudformation execute-change-set \
  --stack-name my-stack \
  --change-set-name preview-...
```

Pay close attention to the `Replacement` field in the change set output. A value of `True` means the resource will be **deleted and recreated** — data loss risk for stateful resources like RDS instances, DynamoDB tables, and EFS volumes.

To protect critical resources from replacement, apply a stack policy:

```json
{
  "Statement": [
    {
      "Effect": "Deny",
      "Principal": "*",
      "Action": ["Update:Replace", "Update:Delete"],
      "Resource": "LogicalResourceId/ProductionDatabase"
    },
    {
      "Effect": "Allow",
      "Principal": "*",
      "Action": "Update:*",
      "Resource": "*"
    }
  ]
}
```


## Common Gotchas and Anti-Patterns

### Circular dependencies

CloudFormation cannot deploy if two resources depend on each other. The typical case: a Lambda function references a security group, and the security group references the Lambda function's ARN.

Fix: break the cycle with a separate resource. For security group rules, use `AWS::EC2::SecurityGroupIngress` and `AWS::EC2::SecurityGroupEgress` standalone resources instead of inline rules:

```yaml
  SGAIngressFromSGB:
    Type: AWS::EC2::SecurityGroupIngress
    Properties:
      GroupId: !Ref SecurityGroupA
      IpProtocol: tcp
      FromPort: 443
      ToPort: 443
      SourceSecurityGroupId: !Ref SecurityGroupB
```

### Update replacement surprises

Some property changes force resource replacement (delete + create). For example, changing an RDS instance's `DBName`, a DynamoDB table's `TableName`, or an S3 bucket's `BucketName`. Check the CloudFormation resource documentation — properties marked `Update requires: Replacement` are the danger zone.

### Stack limits

- 500 resources per stack (hard limit)
- 2,000 stacks per region per account (default; can request increase)
- 200 outputs per stack
- 60 parameters per stack

When you hit resource limits, split into nested stacks or separate independent stacks.

### DependsOn vs. implicit dependencies

CloudFormation infers dependencies from `Ref` and `Fn::GetAtt`. Only use explicit `DependsOn` when there is a real ordering requirement that cannot be expressed through resource references — for example, waiting for an RDS cluster to be available before creating a custom resource that seeds it.

Overusing `DependsOn` slows deployments and can create false circular dependency errors.

### Retain on delete

By default, deleting a stack deletes all its resources. For stateful resources (S3, RDS, DynamoDB) set `DeletionPolicy: Retain` or `DeletionPolicy: Snapshot`:

```yaml
  ProductionDB:
    Type: AWS::RDS::DBInstance
    DeletionPolicy: Snapshot
    UpdateReplacePolicy: Snapshot
    Properties: ...
```

`UpdateReplacePolicy` applies the same logic during replacement, not just deletion.

### Hardcoded region and account IDs

Never hardcode `us-east-1` or `123456789012` in a template. Use `${AWS::Region}` and `${AWS::AccountId}` via `!Sub`. This is the single biggest reason templates fail when reused across environments.

### Template size limits

- Inline template body: 51,200 bytes
- Template in S3: 1 MB
- For large templates, always use S3. Structure with nested stacks to keep individual templates small and focused.


## 2024–2025 Notable Additions

- **Optimistic stabilization (2024):** CloudFormation can start provisioning dependent resources when the upstream resource reaches `CONFIGURATION_COMPLETE` rather than waiting for full stabilization. Up to 40% faster stack creation for stacks with many resources.
- **Stack refactoring (early 2025):** Move resources between stacks, rename logical IDs, and split monolithic stacks without deleting and recreating resources.
- **CloudFormation Hooks enhancements (late 2024):** Hooks can now validate entire templates and cross-resource relationships (architectural patterns), not just individual resources. Useful for org-wide guardrails.
- **IaC Generator:** Generate a CloudFormation template from existing live resources — useful for bringing manually created infrastructure under IaC control.


---


# AWS SQS & SNS Messaging Patterns

## SQS: Standard vs FIFO Queues

### Standard Queue

- **Delivery**: At-least-once — a message may be delivered more than once; consumers must be idempotent.
- **Ordering**: Best-effort; messages can arrive out of sequence under load or after failure recovery.
- **Throughput**: Nearly unlimited API calls per second. Supports up to ~120,000 in-flight messages.
- **Use cases**: Background jobs, media processing, task distribution where order doesn't matter and duplicate handling is cheap.

### FIFO Queue

- **Delivery**: Exactly-once within a 5-minute deduplication window.
- **Ordering**: Strict per message group (`MessageGroupId`). Use multiple message groups to parallelize while preserving per-group order.
- **Throughput**:
  - Standard mode: 300 API calls/second per action, or 3,000 messages/second with batching (10 messages/batch).
  - High-throughput mode: up to 30,000 TPS, but with relaxed (non-strict) ordering within groups.
- **Use cases**: Financial transactions, sequential state machines, command pipelines where order and exactly-once matter.

**FIFO high-throughput mode trade-off**: enabling it sacrifices strict global ordering within a group for throughput. Only enable when strict order within a group is not required.


## Dead Letter Queues (DLQ)

A DLQ captures messages that fail processing more than `maxReceiveCount` times. It isolates poison-pill messages so they don't clog the source queue.

### Configuration

- Create a separate SQS queue (same type — standard or FIFO) to act as the DLQ.
- Attach a **redrive policy** to the source queue specifying:
  - `deadLetterTargetArn`: ARN of the DLQ.
  - `maxReceiveCount`: how many receive attempts before moving to DLQ. A value of 1 means any single failure immediately moves the message — too aggressive for most cases. A value between 3–10 is typical.
- Keep the DLQ in the **same AWS account and region** as the source queue.

### Retention

- Standard queues: the DLQ preserves the **original enqueue timestamp**, so message age keeps counting. Set the DLQ's retention period **longer than the source queue** to avoid expiration before you can investigate.
- FIFO queues: the timestamp **resets** when the message moves to the DLQ.

### Monitoring

- Create a **CloudWatch alarm** on `ApproximateNumberOfMessagesVisible` for the DLQ. Any message in the DLQ is a signal that processing failed — it should never be silently ignored.

### Reprocessing

- Use **SQS dead-letter queue redrive** (console or API) to replay messages back to the source queue after fixing the bug.
- Automate investigation: log the raw message body and exception to CloudWatch Logs or a separate store before the DLQ alarm fires.

### Caveat for FIFO

- Avoid DLQs when strict message ordering is critical across the entire queue (e.g., EDL video editing sequences) because moving a message to DLQ removes it from the ordered stream.


## Message Batching

The `SendMessageBatch`, `ReceiveMessage` (up to 10 at once), and `DeleteMessageBatch` APIs process up to **10 messages per call**.

- Batching reduces API calls by 10×, directly cutting cost.
- For FIFO, batching with `SendMessageBatch` is the primary way to reach 3,000 messages/second.
- On the consumer side, receive 10 messages per poll cycle and delete in a single `DeleteMessageBatch` call after processing.
- Failed deletions in a batch are reported per-message; handle them individually rather than re-queuing the whole batch.


## SNS Message Filtering

By default, every subscriber to a topic receives every message. Filter policies reduce noise and cost by delivering only relevant messages to each subscriber.

### Filter policy scopes

- **MessageAttributes** (attribute-based): filter on key/value pairs attached as message attributes.
- **MessageBody** (payload-based): filter on the JSON message body itself. Requires a well-formed JSON payload.

### Supported operators

| Operator | Example use |
| --- | --- |
| Exact string match | `"event_type": ["order_created"]` |
| Anything-but | Exclude specific values |
| Prefix match | Strings starting with a prefix |
| Suffix match | Strings ending with a suffix |
| Equals-ignore-case | Case-insensitive string comparison |
| Numeric range/exact | `"price": [{"numeric": [">=", 100]}]` |
| IP address | CIDR range matching |
| Exists / does not exist | Attribute presence check |
| AND / OR logic | Combine conditions within a filter |

### Operational note

Filter policy changes take **up to 15 minutes** to propagate due to eventual consistency. Do not rely on instant enforcement after a policy update.


## At-Least-Once Delivery Handling

Standard queues guarantee delivery but not uniqueness. Designing for idempotency is non-negotiable.

Patterns:

- **Idempotency key in the message**: include a UUID. The consumer checks a fast store (DynamoDB, Redis) before processing. If the key exists, skip; otherwise process and write the key.
- **Conditional writes**: use DynamoDB conditional expressions (`attribute_not_exists`) to make the processing operation itself atomic and idempotent.
- **Natural idempotency**: design operations that are safe to repeat (e.g., `SET balance = X` rather than `INCREMENT balance BY X`).
- **Message deduplication table TTL**: set TTL on idempotency records to match or exceed the SQS message retention period (up to 14 days).

Even FIFO queues require downstream idempotency for DLQ redrive scenarios.


## Cost Optimization

- **Long polling** (WaitTimeSeconds=20) is the single highest-impact change for reducing API call volume on lightly loaded queues.
- **Batch sends and deletes** (10 messages/call) reduce costs by up to 10× on high-volume queues.
- **Message size**: SQS charges in 64 KB chunks. A 65 KB message costs 2 units. Compress large payloads or store the payload in S3 and put only the S3 reference in the message (extended client pattern).
- **SNS + SQS filtering**: use subscription filter policies so SNS does not deliver irrelevant messages to queues. Fewer SQS receives = lower cost and less Lambda invocation.
- **FIFO vs Standard**: FIFO queues cost more per API call. Only use FIFO when exactly-once or ordering semantics are a hard requirement.
- **Retention period**: default is 4 days. Reduce if messages not consumed within that window are useless — shorter retention reduces storage cost marginally but more importantly limits DLQ accumulation.
- **Message deduplication**: for FIFO, content-based deduplication avoids the overhead of generating and tracking explicit deduplication IDs in application code.

