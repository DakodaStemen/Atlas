---
name: MEGA-ENGINEERING-PROCESSES
description: Consolidated engineering processes - ADRs, RFCs, PRDs, conventional commits, changelog, progress tracking, task creation, onboarding, post-deploy checks, vendor evaluation, technical debt prioritization.
domain: orchestration
triggers: adr, rfc, prd, conventional commits, changelog, progress tracking, task creation, onboarding, post-deploy, vendor evaluation, technical debt, implementation plan
---

# MEGA-ENGINEERING-PROCESSES

Consolidated engineering workflow patterns: ADRs, RFCs, PRDs, conventional commits, changelogs, progress tracking, task creation, onboarding, deployment checks, vendor evaluation, and technical debt management.


---

<!-- merged from: standard-implementation-plan-template.md -->

﻿---
name: Standard Implementation Plan Template
description: # Standard Implementation Plan Template
 
 Use this template for most feature implementations.
---

# Standard Implementation Plan Template

Use this template for most feature implementations.

```markdown
# Implementation Plan: [Feature Name]

## Overview
[1-2 sentence feature description and business value]

## Linked Specification
<mention-page url="...">Original Specification</mention-page>

## Requirements Summary

### Functional Requirements
- [Requirement 1]
- [Requirement 2]
- [Requirement 3]

### Non-Functional Requirements
- **Performance**: [Targets]
- **Security**: [Requirements]
- **Scalability**: [Needs]

### Acceptance Criteria
- [ ] [Criterion 1]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

## Technical Approach

### Architecture
[High-level architectural decisions]

### Technology Stack
- Backend: [Technologies]
- Frontend: [Technologies]
- Infrastructure: [Technologies]

### Key Design Decisions
1. **[Decision]**: [Rationale]
2. **[Decision]**: [Rationale]

## Implementation Phases

### Phase 1: Foundation (Week 1)
**Goal**: Set up core infrastructure

**Tasks**:
- [ ] <mention-page url="...">Database schema design</mention-page>
- [ ] <mention-page url="...">API scaffolding</mention-page>
- [ ] <mention-page url="...">Authentication setup</mention-page>

**Deliverables**: Working API skeleton
**Estimated effort**: 3 days

### Phase 2: Core Features (Week 2-3)
**Goal**: Implement main functionality

**Tasks**:
- [ ] <mention-page url="...">Feature A implementation</mention-page>
- [ ] <mention-page url="...">Feature B implementation</mention-page>

**Deliverables**: Core features working
**Estimated effort**: 1 week

### Phase 3: Integration & Polish (Week 4)
**Goal**: Complete integration and refinement

**Tasks**:
- [ ] <mention-page url="...">Frontend integration</mention-page>
- [ ] <mention-page url="...">Testing & QA</mention-page>

**Deliverables**: Production-ready feature
**Estimated effort**: 1 week

## Dependencies

### External Dependencies
- [Dependency 1]: [Status]
- [Dependency 2]: [Status]

### Internal Dependencies
- [Team/component dependency]

### Blockers
- [Known blocker] or None currently

## Risks & Mitigation

### Risk 1: [Description]
- **Probability**: High/Medium/Low
- **Impact**: High/Medium/Low
- **Mitigation**: [Strategy]

### Risk 2: [Description]
- **Probability**: High/Medium/Low
- **Impact**: High/Medium/Low
- **Mitigation**: [Strategy]

## Timeline

| Milestone | Target Date | Status |
| ----------- | ------------- | -------- |
| Phase 1 Complete | [Date] | ⏳ Planned |
| Phase 2 Complete | [Date] | ⏳ Planned |
| Phase 3 Complete | [Date] | ⏳ Planned |
| Launch | [Date] | ⏳ Planned |

## Success Criteria

### Technical Success
- [ ] All acceptance criteria met
- [ ] Performance targets achieved
- [ ] Security requirements satisfied
- [ ] Test coverage > 80%

### Business Success
- [ ] [Business metric 1]
- [ ] [Business metric 2]

## Resources

### Documentation
- <mention-page url="...">Design Doc</mention-page>
- <mention-page url="...">API Spec</mention-page>

### Related Work
- <mention-page url="...">Related Feature</mention-page>

## Progress Tracking

[This section updated regularly]

### Phase Status
- Phase 1: ⏳ Not Started
- Phase 2: ⏳ Not Started
- Phase 3: ⏳ Not Started

**Overall Progress**: 0% complete

### Latest Update: [Date]
[Brief status update]
```


---

<!-- merged from: progress-tracking.md -->

﻿---
name: Progress Tracking
description: # Progress Tracking
 
 ## Update Frequency
---

# Progress Tracking

## Update Frequency (Progress Tracking)

### Daily Updates

For active implementation work:

#### What to update

- Task status if changed
- Add progress note to task
- Update blockers

#### When

- End of work day
- After completing significant work
- When encountering blockers

### Milestone Updates

For phase/milestone completion:

#### What to update (Milestone Updates)

- Mark phase complete in plan
- Add milestone summary
- Update timeline if needed
- Report to stakeholders

#### When (Milestone Updates)

- Phase completion
- Major deliverable ready
- Sprint end
- Release

### Status Change Updates

For task state transitions:

#### What to update (Status Change Updates)

- Task status property
- Add transition note
- Notify relevant people

#### When (Status Change Updates)

- Start work (To Do → In Progress)
- Ready for review (In Progress → In Review)
- Complete (In Review → Done)
- Block (Any → Blocked)

## Progress Note Format

### Daily Progress Note

```markdown
## Progress: [Date]

### Completed
- [Specific accomplishment with details]
- [Specific accomplishment with details]

### In Progress
- [Current work item]
- Current status: [Percentage or description]

### Next Steps
1. [Next planned action]
2. [Next planned action]

### Blockers
- [Blocker description and who/what needed to unblock]
- Or: None

### Decisions Made
- [Any technical/product decisions]

### Notes
[Additional context, learnings, issues encountered]
```

Example:

```markdown
## Progress: Oct 14, 2025

### Completed
- Implemented user authentication API endpoints (login, logout, refresh)
- Added JWT token generation and validation
- Wrote unit tests for auth service (95% coverage)

### In Progress
- Frontend login form integration
- Currently: Form submits but need to handle error states

### Next Steps
1. Complete error handling in login form
2. Add loading states
3. Implement "remember me" functionality

### Blockers
None

### Decisions Made
- Using HttpOnly cookies for refresh tokens (more secure than localStorage)
- Session timeout set to 24 hours based on security review

### Notes
- Found edge case with concurrent login attempts, added to backlog
- Performance of auth check is good (<10ms)
```

### Milestone Summary

```markdown
## Phase [N] Complete: [Date]

### Overview
[Brief description of what was accomplished in this phase]

### Completed Tasks
- <mention-page url="...">Task 1</mention-page> ✅
- <mention-page url="...">Task 2</mention-page> ✅
- <mention-page url="...">Task 3</mention-page> ✅

### Deliverables
- [Deliverable 1]: [Link/description]
- [Deliverable 2]: [Link/description]

### Key Accomplishments
- [Major achievement]
- [Major achievement]

### Metrics
- [Relevant metric]: [Value]
- [Relevant metric]: [Value]

### Challenges Overcome
- [Challenge and how it was solved]

### Learnings
**What went well**:
- [Success factor]

**What to improve**:
- [Area for improvement]

### Impact on Timeline
- On schedule / [X days ahead/behind]
- Reason: [If deviation, explain why]

### Next Phase
- **Starting**: [Next phase name]
- **Target start date**: [Date]
- **Focus**: [Main objectives]
```

## Updating Implementation Plan

### Progress Indicators

Update plan page regularly:

```markdown
## Status Overview

**Overall Progress**: 45% complete

### Phase Status
- ✅ Phase 1: Foundation - Complete
- 🔄 Phase 2: Core Features - In Progress (60%)
- ⏳ Phase 3: Integration - Not Started

### Task Summary
- ✅ Completed: 12 tasks
- 🔄 In Progress: 5 tasks
- 🚧 Blocked: 1 task
- ⏳ Not Started: 8 tasks

**Last Updated**: [Date]
```

### Task Checklist Updates

Mark completed tasks:

```markdown
## Implementation Phases

### Phase 1: Foundation
- [x] <mention-page url="...">Database schema</mention-page>
- [x] <mention-page url="...">API scaffolding</mention-page>
- [x] <mention-page url="...">Auth setup</mention-page>

### Phase 2: Core Features
- [x] <mention-page url="...">User management</mention-page>
- [ ] <mention-page url="...">Dashboard</mention-page>
- [ ] <mention-page url="...">Reporting</mention-page>
```

### Timeline Updates

Update milestone dates:

```markdown
## Timeline

| Milestone | Original | Current | Status |
| ----------- | ---------- | --------- | -------- |
| Phase 1 | Oct 15 | Oct 14 | ✅ Complete (1 day early) |
| Phase 2 | Oct 30 | Nov 2 | 🔄 In Progress (3 days delay) |
| Phase 3 | Nov 15 | Nov 18 | ⏳ Planned (adjusted) |
| Launch | Nov 20 | Nov 22 | ⏳ Planned (adjusted) |

**Timeline Status**: Slightly behind due to [reason]
```

## Task Status Tracking

### Status Definitions

**To Do**: Not started

- Task is ready to begin
- Dependencies met
- Assigned (or available)

**In Progress**: Actively being worked

- Work has started
- Assigned to someone
- Regular updates expected

**Blocked**: Cannot proceed

- Dependency not met
- External blocker
- Waiting on decision/resource

**In Review**: Awaiting review

- Work complete from implementer perspective
- Needs code review, QA, or approval
- Reviewers identified

**Done**: Complete

- All acceptance criteria met
- Reviewed and approved
- Deployed/delivered

### Updating Task Status

When updating:

```text
1. Update Status property
2. Add progress note explaining change
3. Update related tasks if needed
4. Notify relevant people via comment

Example:
properties: { "Status": "In Progress" }

Content update:
## Progress: Oct 14, 2025
Started implementation. Set up basic structure and wrote initial tests.
```

## Blocker Tracking

### Recording Blockers

When encountering a blocker:

```markdown
## Blockers

### [Date]: [Blocker Description]
**Status**: 🚧 Active
**Impact**: [What's blocked]
**Needed to unblock**: [Action/person/decision needed]
**Owner**: [Who's responsible for unblocking]
**Target resolution**: [Date or timeframe]
```

### Resolving Blockers

When unblocked:

```markdown
## Blockers

### [Date]: [Blocker Description]
**Status**: ✅ Resolved on [Date]
**Resolution**: [How it was resolved]
**Impact**: [Any timeline/scope impact]
```

### Escalating Blockers

If blocker needs escalation:

```text
1. Update blocker status in task
2. Add comment tagging stakeholder
3. Update plan with blocker impact
4. Propose mitigation if possible
```

## Metrics Tracking

### Velocity Tracking

Track completion rate:

```markdown
## Velocity

### Week 1
- Tasks completed: 8
- Story points: 21
- Velocity: Strong

### Week 2
- Tasks completed: 6
- Story points: 18
- Velocity: Moderate (1 blocker)

### Week 3
- Tasks completed: 9
- Story points: 24
- Velocity: Strong (blocker resolved)
```

### Quality Metrics

Track quality indicators:

```markdown
## Quality Metrics

- Test coverage: 87%
- Code review approval rate: 95%
- Bug count: 3 (2 minor, 1 cosmetic)
- Performance: All targets met
- Security: No issues found
```

### Progress Metrics

Quantitative progress:

```markdown
## Progress Metrics

- Requirements implemented: 15/20 (75%)
- Acceptance criteria met: 42/56 (75%)
- Test cases passing: 128/135 (95%)
- Code complete: 80%
- Documentation: 60%
```

## Stakeholder Communication

### Weekly Status Report

```markdown
## Weekly Status: [Week of Date]

### Summary
[One paragraph overview of progress and status]

### This Week's Accomplishments
- [Key accomplishment]
- [Key accomplishment]
- [Key accomplishment]

### Next Week's Plan
- [Planned work]
- [Planned work]

### Status
- On track / At risk / Behind schedule
- [If at risk or behind, explain and provide mitigation plan]

### Blockers & Needs
- [Active blocker or need for help]
- Or: None

### Risks
- [New or evolving risk]
- Or: None currently identified
```

### Executive Summary

For leadership updates:

```markdown
## Implementation Status: [Feature Name]

**Overall Status**: 🟢 On Track / 🟡 At Risk / 🔴 Behind

**Progress**: [X]% complete

**Key Updates**:
- [Most important update]
- [Most important update]

**Timeline**: [Status vs original plan]

**Risks**: [Top 1-2 risks]

**Next Milestone**: [Upcoming milestone and date]
```

## Automated Progress Tracking

### Query-Based Status

Generate status from task database:

```sql
Query task database:
SELECT 
  "Status",
  COUNT(*) as count
FROM "collection://tasks-uuid"
WHERE "Related Tasks" CONTAINS 'plan-page-id'
GROUP BY "Status"

Generate summary:
- To Do: 8
- In Progress: 5
- Blocked: 1
- In Review: 2
- Done: 12

Overall: 44% complete (12/28 tasks)
```

### Timeline Calculation

Calculate projected completion:

```text
Average velocity: 6 tasks/week
Remaining tasks: 14
Projected completion: 2.3 weeks from now

Compares to target: [On schedule/Behind/Ahead]
```

## Best Practices

1. **Update regularly**: Don't let updates pile up
2. **Be specific**: "Completed login" vs "Made progress"
3. **Quantify progress**: Use percentages, counts, metrics
4. **Note blockers immediately**: Don't wait to report blockers
5. **Link to work**: Reference PRs, deployments, demos
6. **Track decisions**: Document why, not just what
7. **Be honest**: Report actual status, not optimistic status
8. **Update in one place**: Keep implementation plan as source of truth


---

<!-- merged from: changelog-release-notes.md -->

# Changelog and Release Notes


---

<!-- merged from: conventional-commits.md -->

# Conventional Commits

Rigorous formatting for commit messages so that history is machine-readable and changelogs can be generated automatically. Aligned with [Conventional Commits](https://www.conventionalcommits.org/) and the softaworks/agent-toolkit commit-work skill.

## When to Use This Skill

- Writing or editing commit messages in any repository.
- Setting up commit hooks or CI that validate commit format.
- Generating changelogs from git history.
- Enforcing team standards for commit types and scopes.

## Format

```html
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Type (required)

- **feat:** New feature or user-facing capability.
- **fix:** Bug fix.
- **docs:** Documentation only (no code change).
- **style:** Formatting, whitespace, semicolons; no logic change.
- **refactor:** Code change that neither fixes a bug nor adds a feature.
- **perf:** Performance improvement.
- **test:** Adding or updating tests.
- **build:** Build system, CI, or tooling.
- **ci:** CI configuration only.
- **chore:** Other changes (deps, config) that don’t touch src or docs.

### Scope (optional)

- Short noun: area of the codebase (e.g. `auth`, `api`, `ui`, `rag`).
- Use consistent scopes within a repo; document them in CONTRIBUTING or a commit guide.

### Description (required)

- Imperative, lowercase start: "add feature" not "added feature" or "Adds feature".
- No period at the end.
- ~50 characters or fewer when possible; use body for detail.

### Body (optional)

- Explain what and why, not how (code shows how).
- Wrap at 72 characters.
- Separate from subject with a blank line.

### Footer (optional)

- **Breaking changes:** `BREAKING CHANGE: <description>` or append `!` after type/scope (e.g. `feat!: remove deprecated API`).
- **Issue refs:** `Fixes #123`, `Refs #456`.

## Rules

1. One logical change per commit; split large changes into multiple commits by type/scope.
2. First line is the subject; it must stand alone and summarize the change.
3. Do not use past tense or passive voice in the subject.
4. When in doubt, use a type that matches the primary intent (e.g. refactor vs fix).

## Examples

```text
feat(api): add pagination to list endpoint
fix(auth): correct token expiry check in middleware
docs(readme): update install instructions for Windows
refactor(rag): extract chunking logic into separate module
```

## Checklist

- [ ] Type is one of the allowed values.
- [ ] Description is imperative and concise.
- [ ] Breaking changes documented in footer or with `!`.
- [ ] Body used when context or rationale is needed.

## Reference

- [Conventional Commits](https://www.conventionalcommits.org/).
- softaworks/agent-toolkit: commit-work skill.


---

<!-- merged from: rfc-lifecycle.md -->

## When to Use

Use this skill when proposing significant technical, organizational, or process changes that require feedback and consensus from a broad group of stakeholders.

## Core Patterns

### 1. RFC Lifecycle Stages

1. **Drafting (WIP):** Author defines the problem and proposes a solution.
2. **Peer Review:** Initial feedback from a small group of SMEs.
3. **Public Comment Period:** Shared with the wider team/org for a fixed time (e.g., 7–14 days).
4. **FCP (Final Comment Period):** A short window (3–5 days) for final objections before closure.
5. **Decision:** Marked as **Accepted**, **Rejected**, or **Withdrawn**.

### 2. Standard RFC Structure

* **Abstract:** 2-sentence summary of the change.
* **Motivation:** Why now? What pain point does this solve?
* **Goals vs. Non-Goals:** Define scope boundaries clearly.
* **Technical Design:** The core "how-to" of the proposal.
* **Alternatives Considered:** Proactive defense against the "why didn't you just..." question.
* **Risks & Drawbacks:** Honest assessment of technical debt or complexity.

### 3. Decision Criteria

* **Alignment:** Does this fit our long-term roadmap?
* **Consensus:** Aim for "lazy consensus" (no strong objections) rather than 100% agreement.
* **Trade-offs:** Is the value provided worth the maintenance burden?

## Critical Rules/Gotchas

* **Don't Bikeshed:** Focus on high-level design and architectural impact, not trivial implementation details.
* **Timeboxes:** Without a "Decision Due Date," RFCs can linger indefinitely.
* **Master vs. Sub-RFC:** For massive projects, write one high-level Master RFC and several smaller sub-proposals for specific modules.

## Key Commands/APIs

* **Templates:** Maintain a `TEMPLATE.md` in your `rfcs/` directory to ensure consistency.
* **PR Labels:** Use GitHub labels like `status/proposed`, `status/in-review`, and `status/accepted`.

## References

* [Rust RFC Process (The Gold Standard)](https://github.com/rust-lang/rfcs)
* [Google Engineering: Design Docs at Google](https://www.industrialempathy.com/posts/design-docs-at-google/)
* [The RFC Process (IETF)](https://www.ietf.org/about/introduction/)


---

<!-- merged from: rfc-process.md -->

# RFC Process


---

<!-- merged from: adr-architecture-decisions.md -->

# ADR — Architecture Decision Records


---

<!-- merged from: adr-templates.md -->

## When to Use

Use this skill when making significant technical decisions that are difficult or expensive to change later (e.g., choosing a database, adopting a new framework).

## Core Patterns

### 1. Nygard Format (The Original)

A minimalist structure focused on the context and the decision.

* **Title:** Short and descriptive.
* **Status:** Proposed, Accepted, Rejected, Superseded.
* **Context:** The problem/situation driving the decision.
* **Decision:** The specific path chosen.
* **Consequences:** Both positive and negative outcomes.

### 2. MADR (Markdown Architecture Decision Records)

A more structured, "checklist-style" template for complex decisions.

* **Decision Drivers:** Explicitly list what matters most (e.g., cost, performance, developer experience).
* **Considered Options:** Document the "runners-up."
* **Pros/Cons of Options:** Force a comparison of alternatives.

### 3. The Lifecycle of an ADR

* **Immutable:** Once an ADR is `Accepted`, it should not be edited.
* **Superseding:** If a decision changes, create a new ADR (e.g., ADR #10) and mark the old one (ADR #2) as `Superseded by ADR #10`.
* **Review via PR:** ADRs should be submitted as Pull Requests to ensure team alignment before finalization.

## Critical Rules/Gotchas

* **Architectural Significance:** Don't document trivial things (e.g., "we use 2 spaces for indentation"). Only document things that matter to the system's long-term health.
* **The "Why," Not the "What":** The code is the "what." The ADR is the "why" and the "what else we considered."
* **Linkage:** Always link between ADRs (e.g., "Amends ADR-004").

## Key Commands/APIs

* **adr-tools:** `adr new "Choose Postgres for main DB"`
* **Log4brains:** `log4brains adr new` (generates a static site with your records).

## References

* [Documenting Architecture Decisions (Michael Nygard)](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
* [MADR Project Home](https://adr.github.io/madr/)
* [Spotify's Engineering Culture: ADRs](https://engineering.atspotify.com/2020/04/when-should-i-write-an-adr/)


---

<!-- merged from: prd-writing.md -->

## When to Use

Use this skill when initiating a new project or feature to ensure clarity on the problem, the audience, and the specific requirements for success.

## Core Patterns

### 1. The Problem Statement

Focus on the "Why." Avoid proposing the solution here.

* **Pattern:** "Currently, [User] struggles with [Pain Point], which results in [Business Impact]."

### 2. User Stories (INVEST)

* **Independent:** Can be developed in any order.
* **Negotiable:** Leave room for developer creativity.
* **Valuable:** Provide clear value to the end user.
* **Estimable:** Developers can gauge the effort.
* **Small:** Fits in a single sprint.
* **Testable:** Has clear success criteria.

### 3. Acceptance Criteria (AC)

The "Definition of Done."

* **Given-When-Then:** "Given I'm on the login page, When I enter a valid email, Then I receive a reset link."
* **Checklist:** A simple list of functional requirements (e.g., "Must support .png and .jpg").

### 4. Edge Cases & Constraints

Document the "unhappy path."

* **Connectivity:** Offline behavior.
* **Data Limits:** Maximum characters/file sizes.
* **Empty States:** What happens when there is no data to show?

## Critical Rules/Gotchas

* **Outcomes over Outputs:** Describe what the user should be able to *achieve*, not which button should be blue.
* **Collaborate Early:** Involve Engineering and Design leads during the draft phase to catch technical blockers.
* **Living Document:** Update the PRD as technical constraints or user feedback emerge during development.

## Key Commands/APIs

* **Figma:** Link directly to specific mocks within the PRD sections.
* **Jira/Linear:** Map individual user stories to tickets in your task manager.

## References

* [The Anatomy of a Great PRD (Product School)](https://productschool.com/blog/product-management-2/how-to-write-a-good-prd-with-template)
* [Writing a Lean PRD (Aha!)](https://www.aha.io/roadmapping/guide/requirements-management/what-is-a-prd)
* [User Stories Applied (Mike Cohn)](https://www.mountaingoatsoftware.com/books/user-stories-applied)


---

<!-- merged from: onboarding-runbooks.md -->

# Onboarding and Runbooks


---

<!-- merged from: repo-contract-and-validation.md -->

﻿---
name: Repo Contract And Validation
description: # Repo Contract And Validation
 
 Load this reference when scaffolding or reviewing a generated ChatGPT app repo.
---

# Repo Contract And Validation

Load this reference when scaffolding or reviewing a generated ChatGPT app repo.

The goal is not “files were created.” The goal is “the repo is plausibly runnable and follows a stable working-app contract.”

## Minimum Working Repo Contract

Every generated repo should satisfy the relevant parts of this contract.

### 1. Shape

- The repo shape matches the chosen archetype.
- The repo structure is simple enough that a user can identify where the server and widget live.

### 2. Server

- There is a clear MCP server entry point.
- The server exposes `/mcp`.
- The server registers tools intentionally.
- If a UI exists, the server registers a resource/template with the MCP Apps UI MIME type.

### 3. Tools

- Each tool maps to one user intent.
- Descriptions help the model choose the tool.
- Required annotations are present and accurate.
- UI-linked tools use `_meta.ui.resourceUri`.
- `_meta["openai/outputTemplate"]` is treated as optional compatibility, not the primary contract.
- When the app is connector-like, data-only, sync-oriented, or intended for company knowledge or deep research, it implements standard `search` and `fetch` tools instead of custom substitutes.

### 4. Widget

- The widget initializes the MCP Apps bridge when needed.
- The widget can receive `ui/notifications/tool-result`.
- The widget renders from `structuredContent`.
- Interactive widgets use `tools/call`.
- Baseline follow-up messaging uses `ui/message`.
- `window.openai` is optional and additive.

### 5. Local Developer Experience

- There is a clear way to start the app locally.
- There is at least one low-cost check command when the stack supports it.
- The response explains how to connect the app in ChatGPT Developer Mode when relevant.

## Validation Ladder

Run the highest level you can without overfitting to a single stack.

### Level 0: Static contract review

Check for:

- chosen archetype is sensible
- repo shape matches archetype
- `/mcp` route is present
- tool/resource/widget responsibilities are coherent
- if the app is connector-like or sync-oriented, `search` and `fetch` are present with the expected standard shape

### Level 1: Syntax or compile checks

Use the stack-appropriate cheapest check available, for example:

- Python syntax check
- TypeScript compile check
- framework-specific lint or build sanity check if already installed

### Level 2: Local runtime sanity

If feasible:

- start the server
- confirm the health route or `/mcp` endpoint responds

### Level 3: Host loop validation

If feasible:

- inspect with MCP Inspector
- test through ChatGPT Developer Mode
- confirm widget updates after tool results

## Reporting Rule

Always say which validation level was reached and what was not run.

That makes the skill more reliable because it separates:

- “repo shape looks right”
- “syntax is valid”
- “server starts”
- “host integration was actually exercised”


---

<!-- merged from: post-deploy-checks.md -->

﻿---
name: Post-deploy checks
description: # Post-deploy checks
 
 Use this after any deploy or service creation. Keep it short; stop when a check fails.
---

# Post-deploy checks

Use this after any deploy or service creation. Keep it short; stop when a check fails.

## 1) Confirm deploy status

```text
list_deploys(serviceId: "<service-id>", limit: 1)
```

- Expect `status: "live"`.
- If status is failed, inspect build/runtime logs immediately.

## 2) Verify service health

- Hit the health endpoint (preferred) or `/` and confirm a 200 response.
- If there is no health endpoint, add one and redeploy.

## 3) Scan recent error logs

```text
list_logs(resource: ["<service-id>"], level: ["error"], limit: 50)
```

- If you see a clear error signature, jump to the matching fix in
  [troubleshooting-basics.md](troubleshooting-basics.md) or
  [error-patterns.md](error-patterns.md).

## 4) Verify env vars and port binding

- Confirm all required env vars are set (especially secrets marked `sync: false`).
- Ensure the app binds to `0.0.0.0:$PORT` (not localhost).

## 5) Redeploy only after fixing the first failure

- Avoid repeated deploys without changes; fix one issue at a time.


---

<!-- merged from: vendor-evaluation.md -->

# Vendor Evaluation


---

<!-- merged from: technical-debt-prioritization.md -->

# Technical Debt Prioritization


---

<!-- merged from: decision-log-database-adr-architecture-decision-records.md -->

﻿---
name: Decision Log Database (ADR - Architecture Decision Records)
description: # Decision Log Database (ADR - Architecture Decision Records)
 
 **Purpose**: Track important decisions with context and rationale.
---

# Decision Log Database (ADR - Architecture Decision Records)

**Purpose**: Track important decisions with context and rationale.

## Schema

| Property | Type | Options | Purpose |
| ---------- | ------ | --------- | --------- |
| **Decision** | title | - | What was decided |
| **Date** | date | - | When decision was made |
| **Status** | select | Proposed, Accepted, Superseded, Deprecated | Current decision status |
| **Domain** | select | Architecture, Product, Business, Design, Operations | Decision category |
| **Impact** | select | High, Medium, Low | Expected impact level |
| **Deciders** | people | - | Who made the decision |
| **Stakeholders** | people | - | Who's affected by decision |
| **Related Decisions** | relation | Links to other decisions | Context and dependencies |

## Usage

```sql
Create decision records with properties:
{
  "Decision": "Use PostgreSQL for Primary Database",
  "Date": "2025-10-15",
  "Status": "Accepted",
  "Domain": "Architecture",
  "Impact": "High",
  "Deciders": [tech_lead, architect],
  "Stakeholders": [eng_team]
}
```

## Content Template

Each decision page should include:

- **Context**: Why this decision was needed
- **Decision**: What was decided
- **Rationale**: Why this option was chosen
- **Options Considered**: Alternatives and trade-offs
- **Consequences**: Expected outcomes (positive and negative)
- **Implementation**: How decision will be executed

## Views

**Recent Decisions**: Sort by Date descending
**Active Decisions**: Filter where Status = "Accepted"
**By Domain**: Group by Domain
**High Impact**: Filter where Impact = "High"
**Pending**: Filter where Status = "Proposed"

## Best Practices

1. **Document immediately**: Record decisions when made, while context is fresh
2. **Include alternatives**: Show what was considered and why it wasn't chosen
3. **Track superseded decisions**: Update status when decisions change
4. **Link related decisions**: Use relations to show dependencies
5. **Review periodically**: Check if old decisions are still valid


---

<!-- merged from: example-decision-record-capture.md -->

﻿---
name: Example: Decision Record Capture
description: # Example: Decision Record Capture
 
 **User Request**: "Document our decision to move from REST to GraphQL API"
---

# Example: Decision Record Capture

**User Request**: "Document our decision to move from REST to GraphQL API"

## Workflow

### 1. Extract Decision from Context

From conversation, identified:

- **Decision**: Migrate customer-facing API from REST to GraphQL
- **Context**: REST endpoints becoming unwieldy (50+ endpoints)
- **Alternatives**: Keep REST, try gRPC, adopt GraphQL
- **Rationale**: Better client experience, type safety, single endpoint

### 2. Find Decision Log

```yaml
Notion:notion-search
query: "architecture decisions" or "ADR"
```

Found: "Architecture Decision Records" database

### 3. Fetch Schema

```text
Notion:notion-fetch
```

Got properties: Decision (title), Date, Status, Domain, Impact, Deciders, Stakeholders

### 4. Create Decision Record

```text
Notion:notion-create-pages
parent: { data_source_id: "decision-log-collection-id" }
pages: [{
  properties: {
    "Decision": "Migrate to GraphQL API",
    "date:Date:start": "2025-10-16",
    "date:Date:is_datetime": 0,
    "Status": "Accepted",
    "Domain": "Architecture",
    "Impact": "High"
  },
  content: "[Full decision record with context, rationale, alternatives...]"
}]
```

#### Content sample

```markdown
# Migrate Customer-Facing API to GraphQL

## Context
Our REST API has grown to 50+ endpoints with inconsistent patterns. Frontend and mobile teams request new endpoints frequently, leading to:
- API bloat and maintenance burden
- Over-fetching/under-fetching data
- Slow iteration on client features
- Inconsistent error handling

## Decision
Migrate customer-facing API from REST to GraphQL while maintaining REST for internal services.

## Rationale
**Why GraphQL**:
- Clients fetch exactly what they need (no over/under-fetching)
- Single endpoint, self-documenting schema
- Type safety with code generation
- Better developer experience
- Industry standard for client-facing APIs

## Options Considered

### Option A: Keep REST (Status Quo)
**Pros**: No migration cost, team familiar
**Cons**: Doesn't solve core problems, maintenance keeps growing
**Decision**: Rejected - doesn't address root cause

### Option B: gRPC
**Pros**: Performance, strong typing
**Cons**: Browser support issues, steeper learning curve, not designed for client-facing
**Decision**: Rejected - better for internal services

### Option C: GraphQL
**Pros**: Solves over/under-fetching, great DX, strong ecosystem
**Cons**: Learning curve, caching complexity, migration effort
**Decision**: **Accepted**

## Consequences

**Positive**:
- Faster frontend/mobile development
- Reduced API maintenance burden
- Better type safety and tooling
- Single endpoint simplifies deployment

**Negative**:
- 3-4 month migration timeline
- Team needs training
- Need to solve caching strategy
- Monitoring/debugging patterns different

## Implementation Plan
1. GraphQL server setup (Apollo Server)
2. Schema design workshop
3. Gradual migration (start with new features)
4. Dual-run period (both REST and GraphQL)
5. Deprecate old REST endpoints

**Timeline**: Start Q4 2025, complete Q1 2026
**Owner**: Backend team lead

## Success Metrics
- API response times improve 30%
- Client fetch efficiency (less data transferred)
- Reduced new endpoint requests
- Developer satisfaction scores
```

### 5. Make Discoverable

Added link from Architecture Wiki and notified team in Slack.

## Key Success Factors

- Captured decision while context fresh
- Documented alternatives considered
- Included both pros and cons
- Clear implementation plan
- Saved to decision log for future reference
- Made discoverable for team


---

<!-- merged from: task-creation-from-specs.md -->

﻿---
name: Task Creation from Specs
description: # Task Creation from Specs
 
 ## Finding the Task Database
---

# Task Creation from Specs

## Finding the Task Database (Task Creation from Specs)

Before creating tasks, locate the task database:

```yaml
1. Search for task database:
   Notion:notion-search
   query: "Tasks" or "Task Management" or "[Project] Tasks"
   
2. Fetch database schema:
   Notion:notion-fetch
   id: "database-id-from-search"
   
3. Identify data source:
   - Look for <data-source url="collection://..."> tags
   - Extract collection ID for parent parameter
   
4. Note schema:
   - Required properties
   - Property types and options
   - Relation properties for linking

Example:
Notion:notion-search
query: "Engineering Tasks"
query_type: "internal"

Notion:notion-fetch
id: "tasks-database-id"
```

Result: `collection://abc-123-def` for use as parent

## Task Breakdown Strategy

### Size Guidelines

#### Good task size

- Completable in 1-2 days
- Single clear deliverable
- Independently testable
- Minimal dependencies

#### Too large

- Takes > 3 days
- Multiple deliverables
- Many dependencies
- Break down further

#### Too small

- Takes < 2 hours
- Too granular
- Group with related work

### Granularity by Phase

**Early phases**: Larger tasks acceptable

- "Design database schema"
- "Set up API structure"

**Middle phases**: Medium-sized tasks

- "Implement user authentication"
- "Build dashboard UI"

**Late phases**: Smaller, precise tasks

- "Fix validation bug in form"
- "Add loading state to button"

## Task Creation Pattern

For each requirement or work item:

```text
1. Identify the work
2. Determine task size
3. Create task in database
4. Set properties
5. Write task description
6. Link to spec/plan
```

### Creating Task

```text
Use Notion:notion-create-pages:

parent: {
  type: "data_source_id",
  data_source_id: "collection://tasks-db-uuid"
}

properties: {
  "[Title Property]": "Task: [Clear task name]",
  "Status": "To Do",
  "Priority": "[High/Medium/Low]",
  "[Project/Related]": ["spec-page-id", "plan-page-id"],
  "Assignee": "[Person]" (if known),
  "date:Due Date:start": "[Date]" (if applicable),
  "date:Due Date:is_datetime": 0
}

content: "[Task description using template]"
```

## Task Description Template

```markdown
# [Task Name]

## Context
Implementation task for <mention-page url="...">Feature Spec</mention-page>

Part of <mention-page url="...">Implementation Plan</mention-page> - Phase [N]

## Objective
[What this task accomplishes]

## Requirements
Based on spec requirements:
- [Relevant requirement 1]
- [Relevant requirement 2]

## Acceptance Criteria
- [ ] [Specific, testable criterion]
- [ ] [Specific, testable criterion]
- [ ] [Specific, testable criterion]

## Technical Approach
[Suggested implementation approach]

### Components Affected
- [Component 1]
- [Component 2]

### Key Decisions
- [Decision point 1]
- [Decision point 2]

## Dependencies

### Blocked By
- <mention-page url="...">Prerequisite Task</mention-page> or None

### Blocks
- <mention-page url="...">Dependent Task</mention-page> or None

## Resources
- [Link to design mockup]
- [Link to API spec]
- [Link to relevant code]

## Estimated Effort
[Time estimate]

## Progress
[To be updated during implementation]
```

## Task Types

### Infrastructure/Setup Tasks

```text
Title: "Setup: [What's being set up]"
Examples:
- "Setup: Configure database connection pool"
- "Setup: Initialize authentication middleware"
- "Setup: Create CI/CD pipeline"

Focus: Getting environment/tooling ready
```

### Feature Implementation Tasks

```text
Title: "Implement: [Feature name]"
Examples:
- "Implement: User login flow"
- "Implement: File upload functionality"
- "Implement: Dashboard widget"

Focus: Building specific functionality
```

### Integration Tasks

```text
Title: "Integrate: [What's being integrated]"
Examples:
- "Integrate: Connect frontend to API"
- "Integrate: Add payment provider"
- "Integrate: Link user profile to dashboard"

Focus: Connecting components
```

### Testing Tasks

```text
Title: "Test: [What's being tested]"
Examples:
- "Test: Write unit tests for auth service"
- "Test: E2E testing for checkout flow"
- "Test: Performance testing for API"

Focus: Validation and quality assurance
```

### Documentation Tasks

```text
Title: "Document: [What's being documented]"
Examples:
- "Document: API endpoints"
- "Document: Setup instructions"
- "Document: Architecture decisions"

Focus: Creating documentation
```

### Bug Fix Tasks

```text
Title: "Fix: [Bug description]"
Examples:
- "Fix: Login error on Safari"
- "Fix: Memory leak in image processing"
- "Fix: Race condition in payment flow"

Focus: Resolving issues
```

### Refactoring Tasks

```text
Title: "Refactor: [What's being refactored]"
Examples:
- "Refactor: Extract auth logic to service"
- "Refactor: Optimize database queries"
- "Refactor: Simplify component hierarchy"

Focus: Code quality improvement
```

## Sequencing Tasks

### Critical Path

Identify must-happen-first tasks:

```text
1. Database schema
2. API foundation
3. Core business logic
4. Frontend integration
5. Testing
6. Deployment
```

### Parallel Tracks

Tasks that can happen simultaneously:

```text
Track A: Backend development
- API endpoints
- Business logic
- Database operations

Track B: Frontend development
- UI components
- State management
- Routing

Track C: Infrastructure
- CI/CD setup
- Monitoring
- Documentation
```

### Phase-Based Sequencing

Group by implementation phase:

```text
Phase 1 (Foundation):
- Setup tasks
- Infrastructure tasks

Phase 2 (Core):
- Feature implementation tasks
- Integration tasks

Phase 3 (Polish):
- Testing tasks
- Documentation tasks
- Optimization tasks
```

## Priority Assignment

### P0/Critical

- Blocks everything else
- Core functionality
- Security requirements
- Data integrity

### P1/High

- Important features
- User-facing functionality
- Performance requirements

### P2/Medium

- Nice-to-have features
- Optimizations
- Minor improvements

### P3/Low

- Future enhancements
- Edge case handling
- Cosmetic improvements

## Estimation

### Story Points

If using story points:

- 1 point: Few hours
- 2 points: Half day
- 3 points: Full day
- 5 points: 2 days
- 8 points: 3-4 days (consider breaking down)

### Time Estimates

Direct time estimates:

- 2-4 hours: Small task
- 1 day: Medium task
- 2 days: Large task
- 3+ days: Break down further

### Estimation Factors

Consider:

- Complexity
- Unknowns
- Dependencies
- Testing requirements
- Documentation needs

## Task Relationships

### Parent Task Pattern

For large features:

```text
Parent: "Feature: User Authentication"
Children:
- "Setup: Configure auth library"
- "Implement: Login flow"
- "Implement: Password reset"
- "Test: Auth functionality"
```

### Dependency Chain Pattern

For sequential work:

```text
Task A: "Design database schema"
↓ (blocks)
Task B: "Implement data models"
↓ (blocks)
Task C: "Create API endpoints"
↓ (blocks)
Task D: "Integrate with frontend"
```

### Related Tasks Pattern

For parallel work:

```text
Central: "Feature: Dashboard"
Related:
- "Backend API for dashboard data"
- "Frontend dashboard component"
- "Dashboard data caching"
```

## Bulk Task Creation

When creating many tasks:

```text
For each work item in breakdown:
  1. Determine task properties
  2. Create task page
  3. Link to spec/plan
  4. Set relationships

Then:
  1. Update plan with task links
  2. Review sequencing
  3. Assign tasks (if known)
```

## Task Naming Conventions

### Be specific

✓ "Implement user login with email/password"
✗ "Add login"

#### Include context

✓ "Dashboard: Add revenue chart widget"
✗ "Add chart"

#### Use action verbs

- Implement, Build, Create
- Integrate, Connect, Link
- Fix, Resolve, Debug
- Test, Validate, Verify
- Document, Write, Update
- Refactor, Optimize, Improve

## Validation Checklist

Before finalizing tasks:

☐ Each task has clear objective
☐ Acceptance criteria are testable
☐ Dependencies identified
☐ Appropriate size (1-2 days)
☐ Priority assigned
☐ Linked to spec/plan
☐ Proper sequencing
☐ Resources noted