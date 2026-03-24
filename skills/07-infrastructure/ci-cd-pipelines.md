---
name: ci-cd-pipelines
description: CI/CD pipeline patterns covering GitHub Actions (workflows, composite actions, reusable workflows, matrix strategies, caching, secrets) and GitLab CI/CD (pipelines, stages, rules, artifacts, environments, runners, caching). Use when building or optimizing CI/CD workflows.
domain: infrastructure
tags: [github-actions, gitlab-ci, cicd, pipelines, workflows, automation, continuous-integration, continuous-deployment]
triggers: github actions, gitlab ci, CI/CD, pipeline, workflow, composite action, reusable workflow, gitlab runner
---


# GitHub Actions — CI/CD Patterns and Best Practices

## 1. Workflow File Structure

Workflows live under `.github/workflows/` and are YAML files. Every workflow
needs a name, at least one trigger (`on:`), and at least one job.

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

# Default token permissions for all jobs — tighten further per-job.
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683  # v4.2.2
      - name: Run tests
        run: npm test
```


## 3. Job Dependencies and Execution Order

### needs

`needs` declares upstream jobs. A job waits for all listed jobs to succeed
before starting. Fan-out and fan-in patterns are expressed naturally.

```yaml
jobs:
  lint:
    runs-on: ubuntu-latest
    steps: [...]

  test:
    runs-on: ubuntu-latest
    steps: [...]

  build:
    needs: [lint, test]     # fan-in: runs after both pass
    runs-on: ubuntu-latest
    steps: [...]

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps: [...]
```

To run a downstream job even when an upstream job fails, use `always()`:

```yaml
  notify:
    needs: [build, deploy]
    if: ${{ always() }}
    runs-on: ubuntu-latest
```

### outputs

Pass data between jobs through job outputs.

```yaml
jobs:
  compute:
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.get_version.outputs.version }}
    steps:
      - id: get_version
        run: echo "version=$(cat VERSION)" >> "$GITHUB_OUTPUT"

  use:
    needs: compute
    runs-on: ubuntu-latest
    steps:
      - run: echo "Building version ${{ needs.compute.outputs.version }}"
```

### concurrency

Prevent duplicate runs for the same ref. Cancel in-progress runs on PRs;
queue on the main branch.

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```


## 5. Caching Dependencies

Use `actions/cache` when setup actions (e.g., `setup-node`, `setup-python`)
don't cover your package manager. Cache key includes OS and a hash of the
lockfile so the cache auto-invalidates on dependency changes.

```yaml
- name: Cache npm dependencies
  uses: actions/cache@d4323d4df104b026a6aa633fdb11d772146be0bf  # v4.1.2
  with:
    path: ~/.npm
    key: ${{ runner.os }}-npm-${{ hashFiles('**/package-lock.json') }}
    restore-keys: |
      ${{ runner.os }}-npm-

- name: Install dependencies
  run: npm ci
```

**Setup-action cache shortcut** (preferred when available):

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: '22'
    cache: 'npm'             # handles cache key + restore automatically
```

### Cache limits

- Per-repository: 10 GB total.
- Eviction: caches unused for 7 days are removed (oldest-first within the limit).
- Rate limits: 200 uploads/min, 1500 downloads/min per repository.
- Scope: cache entries are isolated by branch; the default branch cache is
  readable from all branches.


## 7. Reusable Workflows (workflow_call)

A reusable workflow is a standard `.github/workflows/*.yml` file that declares
`on: workflow_call`. It is called at the **job level** in the consuming
workflow, not inside a step.

### Defining a reusable workflow

```yaml
# .github/workflows/deploy.yml
on:
  workflow_call:
    inputs:
      environment:
        required: true
        type: string
      image_tag:
        required: true
        type: string
    secrets:
      deploy_key:
        required: true
    outputs:
      deployed_url:
        description: "The URL of the deployed service"
        value: ${{ jobs.deploy.outputs.url }}

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: ${{ inputs.environment }}
    outputs:
      url: ${{ steps.deploy_step.outputs.url }}
    steps:
      - id: deploy_step
        run: |
          echo "Deploying ${{ inputs.image_tag }} to ${{ inputs.environment }}"
          echo "url=https://${{ inputs.environment }}.example.com" >> "$GITHUB_OUTPUT"
        env:
          DEPLOY_KEY: ${{ secrets.deploy_key }}
```

### Calling a reusable workflow

```yaml
# Same repository
jobs:
  call-deploy:
    uses: ./.github/workflows/deploy.yml
    with:
      environment: staging
      image_tag: ${{ needs.build.outputs.tag }}
    secrets:
      deploy_key: ${{ secrets.STAGING_DEPLOY_KEY }}

# Different repository (pin to SHA)
jobs:
  call-deploy:
    uses: org/infra-workflows/.github/workflows/deploy.yml@a1b2c3d4e5f6
    with:
      environment: prod
      image_tag: ${{ needs.build.outputs.tag }}
    secrets: inherit      # pass all caller secrets implicitly
```

### Limitations

- Maximum nesting depth: 10 levels (caller + 9 called workflows).
- Secrets only flow to directly called workflows; they must be explicitly
  forwarded by intermediate workflows.
- Environment secrets from the caller cannot be passed through `secrets:`.
- Permissions can be reduced but never elevated through the call chain.


## 9. OIDC for Cloud Authentication (No Long-Lived Secrets)

OIDC lets GitHub issue short-lived tokens that cloud providers verify directly.
No static credentials stored as secrets.

### Workflow configuration

```yaml
permissions:
  id-token: write    # required to request the OIDC JWT
  contents: read

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # AWS
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789012:role/GitHubActionsRole
          aws-region: us-east-1

      # GCP
      - uses: google-github-actions/auth@v2
        with:
          workload_identity_provider: projects/123/locations/global/workloadIdentityPools/github/providers/github
          service_account: deploy@project.iam.gserviceaccount.com

      # Azure
      - uses: azure/login@v2
        with:
          client-id: ${{ secrets.AZURE_CLIENT_ID }}
          tenant-id: ${{ secrets.AZURE_TENANT_ID }}
          subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}
```

The trust relationship on the cloud provider side scopes tokens to specific
repos, branches, or environments — an attacker who steals the JWT cannot reuse
it outside the workflow's context.


## 11. Self-Hosted Runners

Use for private network access, custom hardware, or cost optimization on
high-volume workloads. Carry significant security obligations.

- **Never attach self-hosted runners to public repositories.** Any fork can
  trigger a workflow and execute arbitrary code on the runner.
- Use just-in-time (JIT) runners that register, execute one job, then terminate:

  ```shell
  ./run.sh --jitconfig "${ENCODED_JIT_CONFIG}"
  ```

- Isolate runner pools by sensitivity: production runners should not share
  infrastructure with runners that build untrusted PRs.
- Run runners as a non-root, low-privilege OS user.
- Harden the base image — remove unused tools and network access paths.
- Prefer ephemeral VMs or containers (Actions Runner Controller on Kubernetes)
  over persistent machines to eliminate state bleed between jobs.


## 13. Workflow Patterns Reference

### CI pipeline (build → test → lint in parallel, then build artifact)

```yaml
name: CI
on:
  pull_request:
    branches: [main]

permissions:
  contents: read

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
      - uses: actions/setup-node@v4
        with: { node-version: '22', cache: 'npm' }
      - run: npm ci && npm test

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
      - uses: actions/setup-node@v4
        with: { node-version: '22', cache: 'npm' }
      - run: npm ci && npm run lint

  build:
    needs: [test, lint]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
      - uses: actions/setup-node@v4
        with: { node-version: '22', cache: 'npm' }
      - run: npm ci && npm run build
      - uses: actions/upload-artifact@v4
        with:
          name: dist
          path: dist/
          retention-days: 3
```

### Release workflow with OIDC deploy to AWS

```yaml
name: Release
on:
  push:
    tags: ['v*']

permissions:
  contents: read
  id-token: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: production
      url: https://app.example.com
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
      - uses: actions/download-artifact@v4
        with: { name: dist, path: dist/ }
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789012:role/GitHubActionsRole
          aws-region: us-east-1
      - run: aws s3 sync dist/ s3://my-bucket/ --delete
```


---


# GitLab CI/CD

## Pipeline Structure

A GitLab pipeline is defined entirely in `.gitlab-ci.yml` at the repository root (or in files included from it). The top-level structure has global keywords and job definitions.

```yaml
# Global defaults applied to all jobs unless overridden
default:
  image: node:20-alpine
  retry:
    max: 2
    when: [runner_system_failure, stuck_or_timeout_failure]
  interruptible: true

# Defines stage order; jobs in the same stage run in parallel
stages:
  - build
  - test
  - security
  - deploy

# Global variables available to all jobs
variables:
  NODE_ENV: production
  DOCKER_DRIVER: overlay2
```

Jobs are the atomic unit. Every job needs at minimum a `script`:

```yaml
build:app:
  stage: build
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - dist/
    expire_in: 1 day
```


## rules vs only/except

`only` and `except` are deprecated and no longer maintained. Always use `rules`. Rules evaluate top-to-bottom and the first match wins.

```yaml
# Bad — deprecated
deploy:
  only:
    - main
  except:
    - tags

# Good — use rules
deploy:
  rules:
    - if: $CI_COMMIT_TAG
      when: never
    - if: $CI_COMMIT_BRANCH == "main"
      when: on_success
    - when: never
```

Common `rules` conditions:

```yaml
rules:
  # Run only on MR pipelines
  - if: $CI_PIPELINE_SOURCE == "merge_request_event"

  # Run only when specific files changed
  - if: $CI_COMMIT_BRANCH
    changes:
      - src/**/*
      - package.json

  # Run only when a file exists
  - exists:
      - Dockerfile

  # Manual trigger with variable pre-fill
  - when: manual
    variables:
      DEPLOY_TARGET: staging
```

### Workflow rules

`workflow` controls whether a pipeline is created at all — evaluated before any job:

```yaml
workflow:
  rules:
    # Prevent duplicate pipelines when an MR is open for a branch push
    - if: $CI_COMMIT_BRANCH && $CI_OPEN_MERGE_REQUESTS
      when: never
    - if: $CI_COMMIT_BRANCH
    - if: $CI_COMMIT_TAG
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```


## Configuration Reuse

### extends (preferred)

`extends` supports multi-level inheritance and works across included files. Arrays do not merge — the child fully replaces the parent array.

```yaml
.base:job:
  image: python:3.12-slim
  before_script:
    - pip install -r requirements.txt

.rules:merge-request:
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"

test:
  extends:
    - .base:job
    - .rules:merge-request
  script: pytest
```

### YAML Anchors

Anchors are scoped to the current file only and cannot cross `include` boundaries. Use for array merging where `extends` falls short.

```yaml
.common_scripts: &common_scripts
  - echo "Setting up..."
  - source .env

job:
  script:
    - *common_scripts
    - echo "Running job"
```

### !reference

`!reference` enables reuse of specific sections across included files, unlike anchors:

```yaml
# shared/setup.yml
.setup:
  script:
    - echo "common setup"
  after_script:
    - echo "common teardown"

# .gitlab-ci.yml
include:
  - local: shared/setup.yml

test:
  script:
    - !reference [.setup, script]
    - pytest
```

### Hidden Jobs (templates)

Any job name beginning with `.` is not executed. Use them as pure templates:

```yaml
.deploy:template:
  image: bitnami/kubectl:latest
  before_script:
    - kubectl config use-context $KUBE_CONTEXT
  script:
    - kubectl apply -f k8s/

deploy:staging:
  extends: .deploy:template
  environment: staging

deploy:production:
  extends: .deploy:template
  environment: production
  when: manual
```


## CI/CD Components and Catalog

Components are the modern, parameterized replacement for shared templates. A component project has this layout:

```text
templates/
  go-build.yml
  go-test.yml
README.md
.gitlab-ci.yml   ← tests the components themselves
```

Each component file has a `spec` block followed by `---` and the actual job definitions:

```yaml
# templates/go-build.yml
spec:
  inputs:
    go_version:
      description: Go toolchain version
      default: "1.22"
    stage:
      default: build

## Artifacts

Artifacts pass build outputs to downstream jobs in the same pipeline. They are guaranteed delivery (unlike cache).

```yaml
build:
  script: npm run build
  artifacts:
    name: "${CI_JOB_NAME}-${CI_COMMIT_SHORT_SHA}"
    paths:
      - dist/
      - coverage/
    exclude:
      - dist/**/*.map
    expire_in: 3 days
    when: always          # upload even on failure

test:
  needs: [build]
  dependencies: [build]   # only download artifacts from build, not all prior jobs
  script: npm test
```

Use `reports:` sub-keywords for structured report integration:

```yaml
test:
  script: pytest --junitxml=report.xml --cov-report xml:coverage.xml
  artifacts:
    reports:
      junit: report.xml
      coverage_report:
        coverage_format: cobertura
        path: coverage.xml
```

Limit `dependencies` to jobs whose artifacts are actually needed. Downloading unnecessary artifacts slows jobs.


## GitLab Runners

Runners execute jobs. Three types:

- **Shared runners** — GitLab.com-managed; SaaS convenience but shared infrastructure, Docker executor.
- **Group runners** — Registered to a group; available to all projects in it.
- **Project runners** — Registered to a single project; highest isolation.

Self-hosted runners are required for: private network access, custom hardware, compliance requirements, or cost control at scale.

Select a runner with `tags`:

```yaml
build:gpu:
  tags:
    - gpu
    - linux
  script: ./train.sh
```

Common executor types:

| Executor | When to use |
| --- | --- |
| Docker | Isolated jobs with clean environments; most common |
| Kubernetes | Cloud-native; auto-scales pods per job |
| Shell | Direct host access; least isolation; avoid for untrusted code |
| Docker Machine (deprecated) | Legacy autoscaling; migrate to Kubernetes or fleeting |

Runner autoscaling on Kubernetes uses the GitLab Runner Helm chart with `config.toml` pointing to the cluster. The `fleeting` plugin is the new autoscaling approach for VM-based fleets (AWS EC2, GCP GCE, Azure VMs).


## OIDC Cloud Authentication

Instead of storing long-lived cloud credentials, use GitLab's OIDC support to request short-lived tokens at job time.

### AWS

```yaml
deploy:aws:
  id_tokens:
    GITLAB_OIDC_TOKEN:
      aud: https://gitlab.example.com
  script:
    - >
      STS=$(aws sts assume-role-with-web-identity
      --role-arn "${AWS_ROLE_ARN}"
      --role-session-name "gitlab-${CI_PROJECT_ID}-${CI_PIPELINE_ID}"
      --web-identity-token "${GITLAB_OIDC_TOKEN}"
      --duration-seconds 3600
      --query 'Credentials.[AccessKeyId,SecretAccessKey,SessionToken]'
      --output text)
    - export AWS_ACCESS_KEY_ID=$(echo $STS | cut -f1)
    - export AWS_SECRET_ACCESS_KEY=$(echo $STS | cut -f2)
    - export AWS_SESSION_TOKEN=$(echo $STS | cut -f3)
    - aws sts get-caller-identity
    - ./deploy.sh
```

IAM trust policy to restrict access to a specific branch:

```json
{
  "Effect": "Allow",
  "Principal": { "Federated": "arn:aws:iam::ACCOUNT_ID:oidc-provider/gitlab.example.com" },
  "Action": "sts:AssumeRoleWithWebIdentity",
  "Condition": {
    "StringEquals": {
      "gitlab.example.com:sub": "project_path:group/project:ref_type:branch:ref:main"
    }
  }
}
```

The `sub` claim format is: `project_path:{group}/{project}:ref_type:{branch|tag}:ref:{name}`.

GCP uses Workload Identity Federation with the same `id_tokens` approach; Azure uses federated credentials on a managed identity or app registration.


## Security Scanning

All security templates produce GitLab Security Report artifacts in a standard JSON schema. Results surface in MR widgets and the Security Dashboard (Ultimate tier). Include templates in the relevant stage; they are auto-configured for the language detected in the repo.

### SAST

```yaml
include:
  - template: Jobs/SAST.gitlab-ci.yml

variables:
  SAST_EXCLUDED_PATHS: "spec,test,tests,tmp,docs,vendor"
  SEARCH_MAX_DEPTH: 10
  # Ultimate: enable cross-file/cross-function analysis
  GITLAB_ADVANCED_SAST_ENABLED: "true"
```

Analyzers used automatically based on detected language: Semgrep (most languages), SpotBugs (Java/Groovy/Kotlin/Scala), PMD-Apex (Salesforce Apex), Sobelow (Elixir/Phoenix), Kubesec (Kubernetes manifests). Pin analyzer versions with `SAST_ANALYZER_IMAGE_TAG` to prevent unexpected upgrades.

### Secret Detection

```yaml
include:
  - template: Jobs/Secret-Detection.gitlab-ci.yml
```

Scans commits for leaked credentials, API keys, and tokens. Runs as a historical scan on default branch; incremental on MRs. Findings appear in the Security tab of the MR.

### Dependency Scanning

```yaml
include:
  - template: Jobs/Dependency-Scanning.gitlab-ci.yml

variables:
  DS_EXCLUDED_PATHS: "tests/"
```

Analyzes package manifests (package-lock.json, Gemfile.lock, go.sum, pom.xml, etc.) for known CVEs. Pairs with License Scanning to surface license compliance issues.

### Container Scanning

```yaml
container_scanning:
  stage: security

include:
  - template: Jobs/Container-Scanning.gitlab-ci.yml

variables:
  CS_IMAGE: $CI_REGISTRY_IMAGE:$CI_COMMIT_SHORT_SHA
  CS_SEVERITY_THRESHOLD: HIGH
```

Scans a built Docker image against vulnerability databases (Trivy by default). Run after the image is built and pushed.

### DAST (Browser-Based)

DAST tests a running application by simulating real browser-driven attacks. The proxy-based analyzer was removed in GitLab 17.3 — use the browser-based analyzer only.

DAST requires a deployed target, so it typically runs against a review app or a staging environment.

```yaml
include:
  - template: DAST.gitlab-ci.yml

variables:
  DAST_WEBSITE: https://$CI_ENVIRONMENT_SLUG.review.example.com
  DAST_BROWSER_SCAN: "true"
  DAST_AUTH_URL: https://$CI_ENVIRONMENT_SLUG.review.example.com/login
  DAST_AUTH_USERNAME: $DAST_TEST_USER
  DAST_AUTH_PASSWORD: $DAST_TEST_PASSWORD
  DAST_AUTH_USERNAME_FIELD: "input[name=email]"
  DAST_AUTH_PASSWORD_FIELD: "input[name=password]"
  DAST_EXCLUDE_URLS: "/logout,/admin"

dast:
  needs: [deploy:review]
  environment:
    name: review/$CI_COMMIT_REF_SLUG
    action: verify
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

Store `DAST_TEST_USER` and `DAST_TEST_PASSWORD` as masked CI/CD variables. Use `action: verify` on the environment so DAST doesn't register as a new deployment.

### Full security stage example

```yaml
stages:
  - build
  - test
  - security
  - deploy

include:
  - template: Jobs/SAST.gitlab-ci.yml
  - template: Jobs/Secret-Detection.gitlab-ci.yml
  - template: Jobs/Dependency-Scanning.gitlab-ci.yml
  - template: Jobs/Container-Scanning.gitlab-ci.yml
  - template: DAST.gitlab-ci.yml

# Override the stage for all security jobs
.override-security-stage:
  stage: security

semgrep-sast:
  extends: .override-security-stage

secret_detection:
  extends: .override-security-stage

dependency_scanning:
  extends: .override-security-stage

container_scanning:
  extends: .override-security-stage
  needs: [build:image]
  variables:
    CS_IMAGE: $CI_REGISTRY_IMAGE:$CI_COMMIT_SHORT_SHA
```


## Common Anti-Patterns

**Pipeline duplication** — Extract shared jobs into `.hidden` templates and use `extends`. Centralize in a separate project only after the third identical use case (added complexity isn't worth it earlier).

**Monolithic jobs** — One 200-line `script` block is undebuggable. Split into focused jobs with proper artifact handoff. You lose visibility; the runner log limit is a hard ceiling.

**Downloading all artifacts** — Without `dependencies`, every job downloads artifacts from all prior jobs. Specify `dependencies: []` or list only what you need.

**Cache key collisions** — Using `$CI_COMMIT_REF_NAME` as the cache key means every branch gets its own cache cold start. Key on the lockfile hash + image for warmest hits.

**child/parent pipelines for modularization** — The UI is clunky, the parent can't access child security reports, and dynamic generation is complex. Prefer `include: local` with `rules` scoped to changed paths for mono-repo job filtering.

**Long-lived cloud credentials in variables** — Use OIDC `id_tokens` for AWS/GCP/Azure. If you must store credentials, mark them masked and protected.

**Multi-stage Docker builds instead of CI jobs** — Traditional CI jobs with artifact handoff give better per-step log visibility and easier debugging than a single opaque `docker build` with multi-stage Dockerfile.


## Complete Minimal Pipeline

```yaml
default:
  image: node:20-alpine
  interruptible: true

workflow:
  rules:
    - if: $CI_COMMIT_BRANCH && $CI_OPEN_MERGE_REQUESTS
      when: never
    - if: $CI_COMMIT_BRANCH
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"

stages: [build, test, security, deploy]

include:
  - template: Jobs/SAST.gitlab-ci.yml
  - template: Jobs/Secret-Detection.gitlab-ci.yml

variables:
  SAST_EXCLUDED_PATHS: "tests,node_modules"

.node:cache:
  cache:
    key:
      files: [package-lock.json]
      prefix: node
    paths: [node_modules/]

install:
  extends: .node:cache
  stage: build
  script: npm ci
  cache:
    policy: push

build:
  extends: .node:cache
  stage: build
  needs: [install]
  script: npm run build
  artifacts:
    paths: [dist/]
    expire_in: 1 day
  cache:
    policy: pull

test:unit:
  extends: .node:cache
  stage: test
  needs: [install]
  script: npm test -- --ci --reporters=jest-junit
  artifacts:
    reports:
      junit: junit.xml
  cache:
    policy: pull

deploy:staging:
  stage: deploy
  needs: [build, test:unit]
  script: ./scripts/deploy.sh staging
  environment:
    name: staging
    url: https://staging.example.com
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH

deploy:production:
  stage: deploy
  needs: [deploy:staging]
  script: ./scripts/deploy.sh production
  environment:
    name: production
    url: https://example.com
  when: manual
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```
