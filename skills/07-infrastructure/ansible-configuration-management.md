---
name: ansible-configuration-management
description: Comprehensive patterns for Ansible configuration management covering inventory design, role and playbook layout, idempotency, vault secrets, tags, molecule testing, collections, and Windows/Linux cross-platform targeting.
domain: devops
category: configuration
tags: [Ansible, playbooks, roles, inventory, vault, idempotency, molecule, collections, Windows, Linux]
triggers: [ansible, playbook, role, inventory, vault, ansible-galaxy, molecule, configuration management, idempotency, automation]
---

# Ansible Configuration Management

## Directory Layout

The canonical project structure separates inventory from playbooks, and playbooks from roles. For multi-environment projects, use per-environment inventory directories rather than a flat hosts file.

```text
project/
├── inventories/
│   ├── production/
│   │   ├── hosts                   # static INI or YAML hosts file
│   │   ├── group_vars/
│   │   │   ├── all.yml
│   │   │   └── webservers.yml
│   │   └── host_vars/
│   │       └── web01.yml
│   └── staging/
│       ├── hosts
│       ├── group_vars/
│       └── host_vars/
├── roles/
│   └── common/
│       ├── tasks/main.yml
│       ├── handlers/main.yml
│       ├── templates/              # .j2 files
│       ├── files/                  # static files
│       ├── vars/main.yml           # high-precedence constants
│       ├── defaults/main.yml       # low-precedence user-facing defaults
│       ├── meta/main.yml           # dependencies, Galaxy metadata
│       └── molecule/               # Molecule test scenarios
├── collections/
│   └── requirements.yml
├── library/                        # custom modules
├── filter_plugins/                 # custom filters
├── ansible.cfg
├── site.yml                        # master orchestration playbook
├── webservers.yml                  # tier-specific playbook
└── dbservers.yml
```

The `group_vars/` and `host_vars/` directories inside each inventory environment keep variable files co-located with the inventory they describe. One file per group/host prevents merge conflicts in Git and makes the variable scope self-documenting.

---

## Inventory: Static vs Dynamic

**Static inventory** suits stable, manually managed infrastructure. Use YAML format over INI for complex nested groups.

```yaml
# inventories/production/hosts
all:
  children:
    webservers:
      hosts:
        web01.example.com:
          ansible_user: deploy
        web02.example.com:
      vars:
        http_port: 80
    dbservers:
      hosts:
        db01.example.com:
```

**Dynamic inventory** is required for cloud and ephemeral environments. Use inventory plugins (not scripts) — they are first-class in Ansible and support caching, filtering, and composition.

```yaml
# inventories/production/aws_ec2.yml
plugin: amazon.aws.aws_ec2
regions:
  - us-east-1
filters:
  tag:Env: production
keyed_groups:
  - key: tags.Role
    prefix: role
  - key: placement.region
    prefix: region
compose:
  ansible_host: public_ip_address
```

Run with: `ansible-inventory -i inventories/production/ --list`

**Combining sources**: Drop both a static `hosts` file and a plugin YAML into the same inventory directory. Ansible merges them at runtime.

**Single Source of Truth principle**: Treat cloud APIs or CMDBs as authoritative for host existence. Use static files only for data that has no external source. Never duplicate host facts that can be auto-discovered.

---

## Playbook Structure

Keep playbooks thin. A playbook should read like a list of roles, not contain logic.

```yaml
# site.yml
---
- name: Apply common baseline
  hosts: all
  become: true
  roles:
    - role: common
      tags: [common]

- name: Configure web tier
  hosts: webservers
  become: true
  roles:
    - role: nginx
      tags: [nginx, web]
    - role: app_deploy
      tags: [app, deploy]
```

Do not mix `roles:` and `tasks:` in the same play — execution order is ambiguous and confusing. Use either the `roles:` section or `tasks:` with `import_role` / `include_role`, not both.

```yaml
# Acceptable alternative using import_role
- name: Configure web tier
  hosts: webservers
  tasks:
    - name: Apply nginx role
      import_role:
        name: nginx
```

Use `import_role` (static, parsed at load time) when the role name is fixed and you want tags to propagate. Use `include_role` (dynamic, evaluated at runtime) when the role name is a variable or conditionally loaded.

---

## Role Layout and Design

Generate scaffolding with: `ansible-galaxy init roles/myrole`

### Variable placement

| Location | Precedence | Purpose |
| --- | --- | --- |
| `defaults/main.yml` | Low | User-facing defaults — override freely |
| `vars/main.yml` | High | Internal constants the role controls |

Never put user-adjustable defaults in `vars/main.yml`. They sit above most inventory variables in the precedence chain and will silently override what users set.

#### Naming conventions

- All role variables must be prefixed with the role name: `nginx_port`, `nginx_worker_processes`
- Internal, non-public variables use double-underscore: `__nginx_derived_value`
- Role names use underscores, never hyphens — hyphens break collections
- Tags on role tasks use the role name as prefix: `nginx_install`, `nginx_config`

**Role argument specification** (Ansible 2.11+): Define `meta/argument_specs.yml` to get fail-fast input validation instead of cryptic mid-run errors.

```yaml
# roles/nginx/meta/argument_specs.yml
argument_specs:
  main:
    short_description: Install and configure nginx
    options:
      nginx_port:
        type: int
        default: 80
        description: Port nginx listens on
      nginx_worker_processes:
        type: str
        default: auto
```

**Task file naming**: Prefix sub-task names with their filename for readable output.

```yaml
# roles/nginx/tasks/install.yml
- name: install | Ensure nginx package is present
  ansible.builtin.package:
    name: nginx
    state: present
```

Output becomes: `TASK [nginx : install | Ensure nginx package is present]`

---

## Handlers

Handlers run once at the end of a play, regardless of how many tasks notify them. Use them for service restarts triggered by configuration changes.

```yaml
# roles/nginx/handlers/main.yml
- name: Restart nginx
  ansible.builtin.service:
    name: nginx
    state: restarted

- name: Reload nginx
  ansible.builtin.service:
    name: nginx
    state: reloaded
```

```yaml
# roles/nginx/tasks/config.yml
- name: config | Deploy nginx.conf
  ansible.builtin.template:
    src: nginx.conf.j2
    dest: /etc/nginx/nginx.conf
    mode: "0644"
  notify: Reload nginx
```

Handlers only fire when a task reports `changed`. If a configuration file is already correct, the handler does not run — this is correct behavior. Use `Reload` over `Restart` wherever the daemon supports it to minimise downtime.

To force handler execution immediately rather than waiting for play end: `meta: flush_handlers`.

---

## Idempotency

An idempotent playbook produces the same result whether run once or ten times. This is not automatic — you have to design for it.

**Use declarative modules over imperative commands.** Prefer `ansible.builtin.package`, `ansible.builtin.service`, `ansible.builtin.template`, `ansible.builtin.copy`, `ansible.builtin.user` over `command` and `shell`.

**Always state the desired state explicitly**, even when it is the module default:

```yaml
- name: Ensure nginx is installed
  ansible.builtin.package:
    name: nginx
    state: present         # explicit, not implicit

- name: Ensure nginx is running and enabled
  ansible.builtin.service:
    name: nginx
    state: started
    enabled: true
```

**When `command` or `shell` is unavoidable**, add `changed_when` and `creates`/`removes` to restore idempotency:

```yaml
- name: Compile app binary
  ansible.builtin.command:
    cmd: make install
    chdir: /opt/app
    creates: /usr/local/bin/myapp   # skip if target already exists
  changed_when: false               # or a register + condition

- name: Run database migration
  ansible.builtin.command: /opt/app/migrate.sh
  register: migration_result
  changed_when: "'Applied' in migration_result.stdout"
```

**Templates and idempotency**: Never put `{{ ansible_date_time.iso8601 }}` or timestamps in managed templates. The file will differ on every run, triggering spurious changes and handler fires. Use `{{ ansible_managed | comment }}` for the managed-by header.

**Check mode**: Run `ansible-playbook site.yml --check --diff` before applying to production. Registered variables from `command`/`shell` tasks are not populated in check mode — tasks that depend on those registers will fail. Guard with:

```yaml
- name: Use registered value safely
  ansible.builtin.debug:
    msg: "{{ migration_result.stdout }}"
  when: not ansible_check_mode
```

---

## Tags for Partial Runs

Tags let you run a targeted subset of tasks without touching the rest of the play.

```yaml
- name: Deploy application
  ansible.builtin.copy:
    src: app.jar
    dest: /opt/app/app.jar
  tags:
    - app
    - deploy

- name: Configure app
  ansible.builtin.template:
    src: app.conf.j2
    dest: /etc/app/app.conf
  tags:
    - app
    - configure
```

Run only tagged tasks: `ansible-playbook site.yml --tags deploy`
Skip tagged tasks: `ansible-playbook site.yml --skip-tags deploy`
Preview what would run: `ansible-playbook site.yml --tags deploy --list-tasks`

### Tag design rules

- Each tag must be safe to run independently. Never design tags that require other tags to have run first in the same invocation.
- Use a consistent taxonomy before the project grows: e.g., `install`, `configure`, `deploy`, plus role-name tags for full role control.
- Do not over-tag — tagging every individual task creates noise and breaks the independence guarantee.

---

## Ansible Vault

Vault encrypts sensitive data at rest. Never store plaintext passwords, API keys, or certificates in version control.

### Encrypting a whole file

```bash
ansible-vault encrypt group_vars/production/vault.yml
ansible-vault edit group_vars/production/vault.yml
ansible-vault decrypt group_vars/production/vault.yml   # only for key rotation
```

**Encrypting individual variables** (preferred for mixed files):

```bash
ansible-vault encrypt_string 'mysecretpassword' --name 'db_password'
```

Produces an inline-encrypted string you paste into a vars file:

```yaml
db_password: !vault |
  $ANSIBLE_VAULT;1.1;AES256
  66386439653236336462626566653...
```

**Vault variable separation pattern** — the most maintainable approach at scale:

```text
group_vars/
└── production/
    ├── vars.yml        # plain variables, reference vault variables
    └── vault.yml       # all secrets, fully encrypted
```

```yaml
# vars.yml
db_host: db01.example.com
db_user: appuser
db_pass: "{{ vault_db_pass }}"   # reference, not the secret

# vault.yml (encrypted)
vault_db_pass: supersecret
```

This lets you `git diff` `vars.yml` freely. The vault file is opaque but you can see which secrets exist by their variable names in `vars.yml`.

**Vault IDs** let you use different passwords per environment or team:

```bash
ansible-vault encrypt --vault-id production@prompt group_vars/production/vault.yml
ansible-playbook site.yml --vault-id production@prompt --vault-id staging@~/.vault_staging
```

**Password files and CI/CD**: Store the vault password in a secret manager (HashiCorp Vault, AWS Secrets Manager, GitHub Actions secret). At runtime, write it to a temp file or use a script source:

```bash
# vault-password-script.sh
#!/bin/bash
aws secretsmanager get-secret-value --secret-id ansible/vault-password \
  --query SecretString --output text

ansible-playbook site.yml --vault-password-file vault-password-script.sh
```

Set in `ansible.cfg` to avoid passing the flag each time:

```ini
[defaults]
vault_password_file = ~/.ansible_vault_pass
```

**`no_log: true`** on any task that handles secrets to prevent them appearing in output or callback logs:

```yaml
- name: Create database user
  community.mysql.mysql_user:
    name: "{{ db_user }}"
    password: "{{ db_pass }}"
    state: present
  no_log: true
```

---

## Collections vs Roles

**Roles** are the unit of reusable task logic for a single concern. They live inside a project or in Galaxy.

**Collections** are the distribution and namespacing unit. A collection can contain multiple roles, modules, plugins, and playbooks under a namespace (`community.mysql`, `amazon.aws`).

Use collections when:

- You are distributing multiple related roles as a package
- You need custom modules or plugins available to those roles
- You want proper namespace isolation (`namespace.collection.module_name`)

Install collections via `requirements.yml` and pin versions for reproducibility:

```yaml
# collections/requirements.yml
collections:
  - name: community.general
    version: ">=8.0.0,<9.0.0"
  - name: amazon.aws
    version: "7.6.0"
  - name: ansible.windows
    version: ">=2.0.0"
```

Install: `ansible-galaxy collection install -r collections/requirements.yml -p ./collections`

Install locally inside the project (`-p ./collections`) rather than globally so all project contributors use the same versions and the project is self-contained.

Reference collection modules with their FQCN (Fully Qualified Collection Name) in tasks — it avoids ambiguity and is required in some execution environments:

```yaml
- name: Create S3 bucket
  amazon.aws.s3_bucket:
    name: my-bucket
    state: present
```

For roles from Galaxy, use a separate `roles/requirements.yml`:

```yaml
# roles/requirements.yml
roles:
  - name: geerlingguy.docker
    version: 6.1.0
```

Install: `ansible-galaxy role install -r roles/requirements.yml -p ./roles`

---

## Windows vs Linux Targets

Ansible manages Windows hosts over WinRM (or SSH since Ansible 2.8+). The connection type and module family differ.

### Inventory for Windows hosts

```yaml
windows_servers:
  hosts:
    win01.example.com:
  vars:
    ansible_connection: winrm
    ansible_winrm_transport: ntlm       # or kerberos, credssp
    ansible_winrm_server_cert_validation: validate   # never ignore in production
    ansible_user: Administrator
    ansible_password: "{{ vault_win_password }}"
    ansible_port: 5985                  # HTTP; use 5986 for HTTPS
```

**Use `ansible.windows.*` and `community.windows.*` modules for Windows**; Linux modules do not work on Windows:

| Task | Linux module | Windows module |
| --- | --- | --- |
| Copy file | `ansible.builtin.copy` | `ansible.windows.win_copy` |
| Run command | `ansible.builtin.command` | `ansible.windows.win_command` |
| Manage service | `ansible.builtin.service` | `ansible.windows.win_service` |
| Install package | `ansible.builtin.package` | `chocolatey.chocolatey.win_chocolatey` |
| Template file | `ansible.builtin.template` | `ansible.windows.win_template` |
| User management | `ansible.builtin.user` | `ansible.windows.win_user` |

**`become` on Windows** uses a different mechanism. For `runas` elevation:

```yaml
become: true
become_method: runas
become_user: SYSTEM
```

**Separate plays by OS** rather than using complex conditionals in every task:

```yaml
- name: Configure Linux hosts
  hosts: linux_servers
  roles:
    - role: common_linux

- name: Configure Windows hosts
  hosts: windows_servers
  roles:
    - role: common_windows
```

For roles that genuinely support both platforms, use `include_vars` and `include_tasks` to load the right file per OS family:

```yaml
# tasks/main.yml
- name: Load OS-specific variables
  ansible.builtin.include_vars: "{{ ansible_facts['os_family'] }}.yml"

- name: Run OS-specific setup
  ansible.builtin.include_tasks:
    file: "{{ lookup('first_found', __setup_files) }}"
  vars:
    __setup_files:
      - "{{ ansible_facts['distribution'] }}_{{ ansible_facts['distribution_major_version'] }}.yml"
      - "{{ ansible_facts['os_family'] }}.yml"
      - default.yml
```

Always use `{{ role_path }}/vars/` absolute paths for variable file lookups to prevent Ansible picking up a same-named file from a parent role.

---

## Privilege Escalation (`become`)

Apply `become` at the lowest scope that satisfies the requirement. Play-level `become: true` is appropriate when most tasks need root. Task-level is better when only one or two tasks need elevation.

```yaml
- name: Deploy application
  hosts: appservers
  become: false         # default for the play
  tasks:
    - name: Deploy app files
      ansible.builtin.copy:
        src: app.jar
        dest: /opt/app/app.jar
      # no become needed — deploy user owns /opt/app

    - name: Restart app service
      ansible.builtin.service:
        name: myapp
        state: restarted
      become: true      # only this task needs root
```

When `become_user` is a non-root user (e.g., `postgres`), you still need `become: true`:

```yaml
- name: Run psql as postgres user
  ansible.builtin.command: psql -c "SELECT 1"
  become: true
  become_user: postgres
```

Do not store `become_pass` in playbooks. Pass it via vault or prompt: `ansible-playbook site.yml --ask-become-pass`

---

## Connection Types

Ansible defaults to `ssh` for Linux/Unix targets. Set explicitly in inventory or `ansible.cfg` when needed.

```ini
# ansible.cfg
[defaults]
transport = ssh

[ssh_connection]
ssh_args = -o ControlMaster=auto -o ControlPersist=60s -o StrictHostKeyChecking=accept-new
pipelining = true      # significant performance improvement; requires requiretty disabled in sudoers
```

**Pipelining** (`pipelining = true`) reduces SSH round-trips substantially on large inventories. It requires that `requiretty` is not set for the Ansible user in `/etc/sudoers`.

Connection type options by target:

| Target | `ansible_connection` | Notes |
| --- | --- | --- |
| Linux/Unix (default) | `ssh` | Key-based auth preferred |
| Windows (traditional) | `winrm` | Requires WinRM setup on target |
| Windows (modern) | `ssh` | OpenSSH on Windows Server 2019+ |
| Local machine | `local` | For plays targeting the controller |
| Network devices | `network_cli` | For routers/switches |
| Docker containers | `community.docker.docker` | Via Docker API |

---

## Molecule for Role Testing

Molecule provides a testing framework for Ansible roles. It runs a role against a real or containerised instance, verifies idempotency, and optionally runs linting.

Install: `pip install molecule molecule-plugins[docker]`

Initialise tests in an existing role: `molecule init scenario --driver-name docker`

Default scenario structure inside a role:

```text
roles/nginx/
└── molecule/
    └── default/
        ├── molecule.yml        # driver, platforms, provisioner config
        ├── converge.yml        # playbook that applies the role
        ├── verify.yml          # assertion playbook (or use testinfra)
        └── prepare.yml         # optional pre-role setup
```

```yaml
# molecule/default/molecule.yml
dependency:
  name: galaxy
driver:
  name: docker
platforms:
  - name: instance-ubuntu22
    image: geerlingguy/docker-ubuntu2204-ansible:latest
    pre_build_image: true
  - name: instance-rocky9
    image: geerlingguy/docker-rockylinux9-ansible:latest
    pre_build_image: true
provisioner:
  name: ansible
  config_options:
    defaults:
      diff: true
verifier:
  name: ansible
```

```yaml
# molecule/default/converge.yml
---
- name: Converge
  hosts: all
  become: true
  roles:
    - role: nginx
      vars:
        nginx_port: 8080
```

Key test commands:

```bash
molecule test           # full lifecycle: create, converge, idempotency, verify, destroy
molecule converge       # apply role without destroying (fast iteration)
molecule idempotency    # run converge a second time, fail if any task reports changed
molecule verify         # run assertions only
molecule lint           # run ansible-lint and yamllint
```

The idempotency step is the most valuable — it catches `command`/`shell` tasks missing `changed_when`, templates with dynamic content, and other idempotency violations before they reach production.

---

## `ansible.cfg` Reference

Keep `ansible.cfg` in the project root so it applies automatically when you run from that directory.

```ini
[defaults]
inventory          = inventories/staging
roles_path         = roles
collections_paths  = ./collections:~/.ansible/collections
host_key_checking  = True
retry_files_enabled = False
stdout_callback    = yaml          # readable output; or 'debug' for verbose
callbacks_enabled  = profile_tasks  # shows task timing
interpreter_python = auto_silent
gathering          = smart          # cache facts between plays
fact_caching       = jsonfile
fact_caching_connection = /tmp/ansible_facts
fact_caching_timeout = 3600

[ssh_connection]
pipelining         = True
ssh_args           = -o ControlMaster=auto -o ControlPersist=60s

[privilege_escalation]
become             = False
become_method      = sudo
become_user        = root
become_ask_pass    = False
```

---

## Common Gotchas

**Variable precedence surprises**: Ansible has 22 precedence levels. The most common trap: `vars/main.yml` inside a role ranks above `group_vars` and `host_vars`, so role internal constants silently override inventory variables. Keep user-facing defaults in `defaults/main.yml`.

**Loops and `include_tasks`**: Variables defined inside a loop iteration are not available after the loop ends. If you need the loop item in a subsequent task, use `set_fact` inside the included file.

**`include_*` vs `import_*`**: `import_*` is static (parsed at load time — tags, when conditions apply correctly). `include_*` is dynamic (evaluated at runtime — tags may not propagate as expected). Tags on `include_tasks` do not apply to the included tasks themselves by default; use `apply` to force it.

```yaml
- name: Include tasks with tag propagation
  ansible.builtin.include_tasks:
    file: setup.yml
    apply:
      tags: [setup]
  tags: [always]
```

**Handlers and failures**: If a play fails mid-run, pending handlers do not execute. Use `--force-handlers` if you need them to run even on failure, or use `meta: flush_handlers` earlier in the play to run them before a risky task.

**`serial` and rolling updates**: `serial: 1` runs the entire play against one host before moving to the next. Set `max_fail_percentage: 0` to abort the run immediately if any host fails, preventing a bad config from rolling through the fleet.

```yaml
- name: Rolling deploy
  hosts: webservers
  serial: "25%"
  max_fail_percentage: 0
  roles:
    - role: app_deploy
```

**WinRM certificate errors**: Using `ansible_winrm_server_cert_validation: ignore` is a common quick fix that should never reach production. Configure proper certificates or Kerberos authentication instead.

**`gather_facts` in large inventories**: With hundreds of hosts, fact gathering is slow. Set `gathering: smart` in `ansible.cfg` and enable fact caching (jsonfile or redis). For plays that genuinely need no facts, disable explicitly: `gather_facts: false`.

**Jinja2 type coercion**: Ansible variables loaded from YAML are typed, but variables passed via `-e` are always strings. `"{{ my_port | int }}"` and explicit `| bool` filters prevent silent type mismatches.

**Namespace collisions**: Role variables without a role-name prefix will silently overwrite same-named variables from other roles or inventory. Always prefix.

**`no_log` and debugging**: `no_log: true` suppresses all output including error messages. When debugging a failing task that uses `no_log`, temporarily remove it locally — never commit the change.

**Ansible Lint**: Run `ansible-lint` in CI on every PR. It catches deprecated module names, `command` usage where a module exists, missing `name` fields, and YAML style issues before they become habits.

---

## Execution Environments

For reproducible CI/CD and team environments, package ansible-core, collections, and Python dependencies into an execution environment container image using `ansible-builder`:

```yaml
# execution-environment.yml
version: 3
dependencies:
  galaxy: collections/requirements.yml
  python: requirements.txt
  system: bindep.txt
images:
  base_image:
    name: registry.redhat.io/ansible-automation-platform/ee-minimal-rhel9:latest
```

Build: `ansible-builder build -t myorg/ee-myproject:1.0 -f execution-environment.yml`

Run playbooks inside the EE: `ansible-runner run . -p site.yml --container-image myorg/ee-myproject:1.0`

This eliminates "works on my machine" problems across Ansible controller, CI runners, and developer laptops.
