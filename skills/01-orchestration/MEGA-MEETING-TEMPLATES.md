---
name: Meeting Template Selection Guide
description: # Meeting Template Selection Guide
 
 Choose the right template for your meeting type.
---

# Meeting Template Selection Guide

Choose the right template for your meeting type.

## Template Overview

| Meeting Type | Use This Template | When to Use |
| -------------- | ------------------- | ------------- |
| Make a decision | [Decision Meeting](decision-meeting-template.md) | Need to evaluate options and reach a decision |
| Project update | [Status Update](status-update-template.md) | Regular check-ins, progress reviews |
| Generate ideas | [Brainstorming](brainstorming-template.md) | Creative ideation, problem-solving |
| Sprint planning | [Sprint Planning](sprint-planning-template.md) | Planning agile sprint work |
| Sprint retro | [Retrospective](retrospective-template.md) | Reflecting on completed work |
| Manager/report | [1:1 Meeting](one-on-one-template.md) | Regular one-on-one check-ins |
| Weekly team sync | [Status Update](status-update-template.md) (simplified) | Routine team synchronization |

## Quick Decision Tree

```text
What's the primary purpose?

├─ Make a decision
│  └─ Use: Decision Meeting Template
│
├─ Update on progress
│  └─ Use: Status Update Template
│
├─ Generate ideas
│  └─ Use: Brainstorming Template
│
├─ Plan sprint work
│  └─ Use: Sprint Planning Template
│
├─ Reflect on past work
│  └─ Use: Retrospective Template
│
└─ Manager/report check-in
   └─ Use: 1:1 Meeting Template
```

## Template Customization

All templates can be customized:

- **Simplify** for shorter meetings
- **Add sections** for specific needs
- **Combine elements** from multiple templates
- **Adapt language** for your team culture

## Best Practices

1. **Choose template first**: Select before gathering context
2. **Gather Notion content**: Search and fetch relevant pages
3. **Enrich with research**: Add Codex insights where valuable
4. **Customize as needed**: Adapt template to specific situation
5. **Share early**: Give attendees time to review

## When to use

Use when the user asks about or needs: Meeting Template Selection Guide.
﻿---
name: Example: Customer Meeting Prep
description: # Example: Customer Meeting Prep
 
 **User Request**: "Prep for sales meeting with Acme Corp on Thursday"
---

# Example: Customer Meeting Prep

**User Request**: "Prep for sales meeting with Acme Corp on Thursday"

## Workflow

### 1. Search Customer Info

```yaml
Notion:notion-search
query: "Acme Corp"
```

Found:

- CRM notes from initial call
- Product demo feedback
- Pricing proposal
- Competitor analysis

### 2. Fetch Details

```text
Notion:notion-fetch (4 pages)
```

#### Key context

- **Company**: 500-person fintech startup, Series B
- **Pain Points**: Manual workflows, data silos, scaling issues
- **Decision Makers**: CTO (technical), VP Product (user needs), CFO (budget)
- **Timeline**: Need solution by Q1
- **Budget**: $50-75K/year range
- **Competition**: Evaluating us vs. Competitor A and Competitor B

### 3. Add Research Context

Codex research added:

- Fintech industry compliance requirements
- Common integration needs for financial products
- Typical procurement processes for Series B companies

### 4. Create Internal Pre-Read

```text
Notion:notion-create-pages
pages: [{
  properties: {
    title: "Acme Corp Meeting - Pre-Read (Internal)"
  },
  content: "# Acme Corp Meeting - Pre-Read (Internal)

## Company Overview
**Company**: Acme Corp (500-person fintech, Series B)
**Decision Makers**: CTO, VP Product, CFO
Source: <mention-page url='...'>CRM Notes</mention-page>

## Their Pain Points → Our Solutions
- Manual workflows → Workflow automation
- Data silos → API integrations
- Scaling issues → Enterprise features

## Competitive Position
**Our advantages over Competitor A/B**:
- Better integration ecosystem (Stripe, Plaid)
- Stronger compliance features
- Faster implementation (4 weeks vs 12 weeks)

## Pricing Strategy
Recommend: $60K/year (mid-range of their budget)

## Potential Objections & Responses
..."
}]
```

### 5. Create Meeting Agenda

```text
Notion:notion-create-pages
pages: [{
  properties: {
    title: "Acme Corp - Product Demo & Q&A"
  },
  content: "# Acme Corp - Product Demo & Q&A

## Meeting Details
**Date**: Thursday, Oct 17, 2025 @ 2pm PST
**Attendees**: CTO, VP Product, CFO (Acme) | Sales Lead, Solutions Engineer (Us)
**Duration**: 60 minutes

## Objective
Demonstrate how our platform solves Acme's workflow automation and integration challenges.

## Agenda

**1. Introductions** (5 min)

**2. Understand Current Workflow** (10 min)
- Current pain points
- Integration requirements
- Success criteria

**3. Product Demo** (25 min)
- Workflow automation capabilities
- API integrations (Stripe, Plaid)
- Security & compliance features

**4. Pricing & Implementation** (10 min)

**5. Next Steps** (10 min)
"
}]
```

### 6. Link Resources

Connected agenda to CRM page, pricing sheet, and technical integration docs.

## Outputs

**Internal Pre-Read**: Full context for sales team
**Customer Agenda**: Professional meeting structure
**Both in Notion** with links to supporting materials

## Key Success Factors

- Understood customer's specific pain points
- Researched industry context (fintech compliance)
- Mapped features to their needs
- Prepared competitive differentiators
- Structured demo around their use cases
- Pre-planned objection responses
- Clear next steps in agenda

## When to use

Use when the user asks about or needs: Example: Customer Meeting Prep.
﻿---
name: Example: Sprint Planning Meeting Prep
description: # Example: Sprint Planning Meeting Prep
 
 **User Request**: "Prepare for tomorrow's sprint planning meeting"
---

# Example: Sprint Planning Meeting Prep

**User Request**: "Prepare for tomorrow's sprint planning meeting"

## Workflow

### 1. Search for Context

```yaml
Notion:notion-search
query: "sprint planning" + "product backlog"
teamspace_id: "engineering-team"
```

Found:

- Last sprint retrospective
- Product backlog (prioritized)
- Current sprint progress
- Team capacity notes

### 2. Fetch Details

```text
Notion:notion-fetch (4 pages)
```

#### Key context

- **Last Sprint**: Completed 32/35 story points (91%)
- **Velocity**: Consistent 30-35 points over last 3 sprints
- **Team**: 5 engineers, 1 on vacation next sprint (80% capacity)
- **Top Backlog Items**: User auth improvements, API performance, mobile responsive fixes

### 3. Query Current Sprint Tasks

```sql
Notion:notion-query-data-sources
query: "SELECT * FROM tasks WHERE Sprint = 'Sprint 24' AND Status != 'Done'"
```

3 tasks carrying over (technical debt items)

### 4. Create Pre-Read (Internal)

```yaml
Notion:notion-create-pages
title: "Sprint 25 Planning - Pre-Read (Internal)"
```

#### Pre-read included

- Sprint 24 summary (velocity, what carried over)
- Team capacity for Sprint 25
- Top backlog candidates with story points
- Technical dependencies
- Risk items (auth changes need QA time)

### 5. Create Agenda

```yaml
Notion:notion-create-pages  
title: "Sprint 25 Planning - Agenda"
```

#### Agenda

- Review Sprint 24 completion (5 min)
- Discuss carryover items (5 min)
- Review capacity (28 points available)
- Select backlog items (30 min)
- Identify dependencies & risks (10 min)
- Confirm commitments (10 min)

### 6. Link Documents

Cross-linked pre-read and agenda, referenced last retro and backlog.

## Output Summary

**Internal Pre-Read**: Team context, capacity, blockers
**External Agenda**: Meeting structure, discussion topics
**Both saved to Notion** and linked to project pages

## Key Success Factors

- Gathered sprint history for velocity trends
- Calculated realistic capacity (account for PTO)
- Identified carryover items upfront
- Pre-read gave team context before meeting
- Agenda kept meeting focused and timeboxed

## When to use

Use when the user asks about or needs: Example: Sprint Planning Meeting Prep.
﻿---
name: Example: Project Decision Meeting Prep
description: # Example: Project Decision Meeting Prep
 
 ## User Request
---

# Example: Project Decision Meeting Prep

## User Request (Example: Project Decision Meeting Prep)

> "Prep for tomorrow's architecture meeting where we need to decide on database migration strategy"

## Workflow Execution

### Step 1: Search for Context

```text
Notion:notion-search
query: "database migration"
query_type: "internal"
teamspace_id: "engineering-teamspace-id"
filters: {
  created_date_range: {
    start_date: "2024-01-01"
  }
}
```

#### Results Found

- "Database Migration Proposal" (Engineering)
- "Current Database Performance Issues" (Engineering)
- "Migration Options Analysis" (Architecture)
- "Previous Migration Post-mortem" (Engineering)

### Step 2: Fetch Relevant Pages

```yaml
Notion:notion-fetch
id: "database-migration-proposal-page-id"
```

**Extracted**: Proposes migrating from MongoDB to PostgreSQL, outlines timeline and risks

```sql
Notion:notion-fetch
id: "database-performance-issues-page-id"
```

**Extracted**: Current pain points - slow queries, schema flexibility issues, transaction limitations

```yaml
Notion:notion-fetch
id: "migration-options-analysis-page-id"
```

**Extracted**: Three options analyzed:

1. Stay on MongoDB with optimizations
2. Migrate to PostgreSQL
3. Hybrid approach (MongoDB + PostgreSQL)

```sql
Notion:notion-fetch
id: "previous-migration-postmortem-page-id"
```

**Extracted**: Lessons from 2023 Redis migration - importance of rollback plan, testing strategy

### Step 3: Synthesize Pre-Read

#### Key Information Identified

- **Problem**: MongoDB performance degrading, schema flexibility causing data consistency issues
- **Options**: Three approaches with different trade-offs
- **Recommendation**: PostgreSQL migration favored by engineering team
- **Concerns**: Downtime requirements, data migration complexity, application changes needed
- **Historical context**: Previous migration took 6 weeks, had rollback issues

### Step 4: Create Meeting Agenda

```text
Notion:notion-create-pages

parent: { page_id: "meetings-parent-id" }

pages: [{
  properties: {
    "title": "Architecture Decision: Database Migration Strategy - Oct 15, 2025"
  },
  content: "[Decision meeting agenda]"
}]
```

#### Agenda Created

```markdown
# Architecture Decision: Database Migration Strategy

## Meeting Details
**Date & Time**: October 15, 2025 at 2:00 PM  
**Duration**: 90 minutes  
**Location**: Conference Room B / Zoom  
**Facilitator**: Sarah Chen (Engineering Lead)  
**Attendees**:
- Sarah Chen (Engineering Lead)
- David Kim (CTO)
- Alice Wang (Backend Team Lead)
- Bob Martinez (DevOps Lead)
- Carol Johnson (Product Director)

## Pre-Read Summary

### Background

Our MongoDB database is experiencing performance degradation as we scale. Query response times have increased from 50ms to 400ms average over the past 6 months. The schema-less nature that initially provided flexibility is now causing data consistency issues and making application logic more complex.

**Source**: <mention-page url="...">Current Database Performance Issues</mention-page>

### Current Situation

**Performance metrics**:
- Average query time: 400ms (was 50ms 6 months ago)
- p95 query time: 1.2s (was 200ms)
- Database size: 500GB (growing 20GB/month)
- Connection pool exhaustion during peak traffic

**Technical debt**:
- 15+ application-layer validation rules compensating for lack of schema
- Complex data migration scripts for schema changes
- Limited transaction support causing race conditions

**Source**: <mention-page url="...">Database Migration Proposal</mention-page>

### Historical Context

We successfully migrated from Redis to Memcached in 2023, which took 6 weeks. Key learnings:
- Underestimated application code changes (3 weeks instead of 1 week)
- Rollback plan was crucial when we discovered compatibility issues
- Parallel running period (dual writes) was essential for safe migration

**Source**: <mention-page url="...">Previous Migration Post-mortem</mention-page>

## Decision Required

**Question**: Which database migration strategy should we adopt?

**Timeline**: Need decision by end of week to include in Q4 planning

**Impact**: 
- Engineering team (4-8 weeks of work)
- Application architecture
- Operations & monitoring
- Future feature development velocity

## Options Analysis

### Option A: Stay on MongoDB with Optimizations

**Description**: Invest in MongoDB performance tuning, add indexes, upgrade to latest version, implement better query patterns.

**Pros**:
- ✅ No migration complexity
- ✅ Team familiar with MongoDB
- ✅ Can implement immediately
- ✅ Lower risk
- ✅ Estimated 2 weeks effort

**Cons**:
- ❌ Doesn't solve fundamental schema flexibility issues
- ❌ Still limited transaction support
- ❌ Performance improvements may be temporary
- ❌ Continues technical debt accumulation

**Cost/Effort**: 2 weeks engineering + $5K/year additional MongoDB infrastructure

**Risk**: Medium - Improvements may not be sufficient

**Source**: <mention-page url="...">Migration Options Analysis</mention-page>

### Option B: Migrate to PostgreSQL

**Description**: Full migration from MongoDB to PostgreSQL. Redesign schema with proper constraints, implement dual-write period, then cut over.

**Pros**:
- ✅ Solves schema consistency issues
- ✅ Full ACID transactions
- ✅ Better performance for relational queries
- ✅ Lower long-term complexity
- ✅ Industry standard, easier hiring

**Cons**:
- ❌ High migration effort (6-8 weeks)
- ❌ Requires schema redesign
- ❌ Application code changes extensive
- ❌ Risk of data loss during migration
- ❌ Downtime required (4-6 hours estimated)

**Cost/Effort**: 8 weeks engineering + $8K migration costs - $15K/year MongoDB savings = net $7K/year savings

**Risk**: High - Complex migration, application changes required

**Recommendation**: ✅ **Favored by engineering team**

**Source**: <mention-page url="...">Database Migration Proposal</mention-page>

### Option C: Hybrid Approach

**Description**: Keep MongoDB for document-heavy data (logs, analytics), migrate transactional data to PostgreSQL. Run both databases.

**Pros**:
- ✅ Phased migration (lower risk)
- ✅ Use best tool for each data type
- ✅ Can migrate incrementally
- ✅ Smaller initial scope (4 weeks)

**Cons**:
- ❌ Increased operational complexity
- ❌ Two databases to maintain
- ❌ Data consistency between databases challenging
- ❌ Higher infrastructure costs
- ❌ Complex application logic

**Cost/Effort**: 4 weeks initial + ongoing complexity + $10K/year additional infrastructure

**Risk**: Medium - Operational complexity increases

**Source**: <mention-page url="...">Migration Options Analysis</mention-page>

### Option D: Do Nothing

**Description**: Accept current performance and continue with MongoDB as-is.

**Implications**:
- Performance continues to degrade
- Technical debt increases
- Feature development slows
- Customer experience suffers
- Eventually forced into emergency migration

**Not recommended**

## Discussion Topics

### Technical Feasibility
1. Can we achieve < 4 hours downtime for Option B?
2. What's the rollback plan if PostgreSQL migration fails?
3. How do we handle data migration for 500GB?
4. Schema design - what constraints do we need?

### Business Impact
5. What's the customer impact of 4-6 hours downtime?
6. Can we schedule migration during low-traffic period?
7. How does this affect Q4 feature roadmap?
8. Cost-benefit analysis over 2-year horizon?

### Risk Management
9. What are the biggest risks with Option B?
10. How do we test thoroughly before cutover?
11. What's the rollback procedure and time?
12. Do we have necessary expertise on team?

### Timeline & Resources
13. Can we allocate 2 engineers full-time for 8 weeks?
14. Do we need external consultants?
15. What's the impact on other Q4 projects?
16. When could we realistically complete this?

## Decision Framework

**Evaluation criteria**:
1. **Performance improvement**: Will this solve our performance issues?
2. **Technical debt**: Does this reduce or increase complexity?
3. **Risk**: What's the probability and impact of failure?
4. **Cost**: Total cost of ownership (effort + infrastructure)
5. **Timeline**: Can we complete in Q4?
6. **Reversibility**: Can we roll back if needed?

**Decision makers**:
- Primary: David Kim (CTO)
- Consulted: Sarah Chen (Engineering), Bob Martinez (DevOps)
- Informed: Product, Executive team

## Decision

*[To be filled during meeting]*

**Selected Option**: ___________________

**Rationale**:

**Timeline**:

**Owner**: ___________________

**Success Criteria**:

## Action Items

*[To be filled during meeting]*

- [ ] [Action item] - @[Owner] - Due: [Date]
- [ ] [Action item] - @[Owner] - Due: [Date]

## Next Steps

**Immediate**:
- Document decision rationale
- Create detailed implementation plan
- Schedule kickoff meeting

**Follow-up meeting**: [Date] to review migration plan

## Related Resources

### Primary Sources
- <mention-page url="...">Database Migration Proposal</mention-page>
- <mention-page url="...">Current Database Performance Issues</mention-page>
- <mention-page url="...">Migration Options Analysis</mention-page>
- <mention-page url="...">Previous Migration Post-mortem</mention-page>

### Supporting Documents
- MongoDB performance metrics dashboard
- PostgreSQL evaluation results
- Cost analysis spreadsheet
```

### Step 5: Share Agenda

```text
Notion:notion-create-comment
parent: { page_id: "agenda-page-id" }
rich_text: [{
  type: "text",
  text: { content: "📋 Meeting agenda ready! Please review the pre-read before tomorrow's meeting. cc: @Sarah @David @Alice @Bob @Carol" }
}]
```

## Summary Provided to User

```markdown
## Meeting Prep Complete ✅

### Meeting Details
- **Topic**: Database Migration Strategy Decision
- **Date**: October 15, 2025 at 2:00 PM
- **Duration**: 90 minutes
- **Attendees**: 5 (Engineering leads + CTO + Product)

### Agenda Created
<mention-page url="...">Architecture Decision: Database Migration Strategy</mention-page>

### Pre-Read Summary

**Problem**: MongoDB performance degrading (50ms → 400ms queries), schema flexibility causing issues

**Decision needed**: Choose between:
1. Stay on MongoDB (optimizations) - 2 weeks, low risk
2. Migrate to PostgreSQL - 8 weeks, high value, high risk ✅ Recommended
3. Hybrid approach - 4 weeks, medium complexity
4. Do nothing - Not recommended

**Recommendation**: Engineering team favors PostgreSQL migration

### Key Information Included

✅ **Background context** from 4 relevant pages
✅ **Options analysis** with pros/cons for each
✅ **Cost-benefit** breakdown
✅ **Risk assessment** for each option
✅ **Discussion topics** (16 questions organized by theme)
✅ **Decision framework** with evaluation criteria
✅ **Historical context** from previous migration
✅ **Empty sections** for decision and action items

### Discussion Topics Prepared

**Technical** (4 questions):
- Downtime achievability
- Rollback planning
- Data migration approach
- Schema design

**Business** (4 questions):
- Customer impact
- Scheduling
- Roadmap impact
- Cost analysis

**Risk** (4 questions):
- Risk identification
- Testing strategy
- Rollback procedure
- Expertise availability

**Resources** (4 questions):
- Team allocation
- External help needs
- Project trade-offs
- Timeline realism

### Success Factors

1. **Comprehensive research**: 4 relevant pages analyzed
2. **Clear options**: Pros/cons for each approach
3. **Structured discussion**: 16 questions organized by theme
4. **Decision framework**: Clear criteria for evaluation
5. **Historical context**: Lessons from previous migration
6. **Ready for outcomes**: Sections prepared for decision and actions

**Meeting is well-prepared for productive decision-making! 📊**
```

## Key Features Demonstrated

### Context Gathering

- **Targeted search**: Engineering teamspace, recent documents
- **Multiple perspectives**: Performance data, proposals, analysis, historical lessons
- **Comprehensive synthesis**: Combined technical, business, and historical context

### Options Analysis

- **Structured comparison**: Pros/cons for each option
- **Cost-benefit**: Effort and infrastructure costs included
- **Risk assessment**: Probability and impact noted
- **Recommendation**: Clear engineering preference stated

### Decision Support

- **Discussion topics**: 16 questions organized by theme
- **Decision framework**: Evaluation criteria defined
- **Decision makers**: Roles and responsibilities clear
- **Outcome capture**: Sections ready for decision and actions

### Meeting Structure

- **Pre-read**: Comprehensive background (can be read in 10 minutes)
- **Options**: Clear comparison for quick decision
- **Discussion**: Structured topics prevent rambling
- **Capture**: Templates for decision and actions

Perfect for: Architecture decisions, technical trade-offs, strategic choices

## When to use

Use when the user asks about or needs: Example: Project Decision Meeting Prep.
﻿---
name: Example: Executive Review Prep
description: # Example: Executive Review Prep
 
 **User Request**: "Prep for quarterly executive review on Friday"
---

# Example: Executive Review Prep

**User Request**: "Prep for quarterly executive review on Friday"

## Workflow

### 1. Search for Context

```yaml
Notion:notion-search
query: "Q4 objectives" + "KPIs" + "quarterly results"
```

Found:

- Q4 OKRs and progress
- Product metrics dashboard
- Engineering velocity reports
- Customer feedback summary

### 2. Fetch & Analyze

```text
Notion:notion-fetch (5 pages)
```

#### Key metrics

- **Revenue**: $2.4M ARR (96% of Q4 target)
- **Customer Growth**: 145 new customers (exceeds 120 target)
- **Churn**: 3.2% (below 5% target)
- **Product**: 3 major features shipped, 2 in beta
- **Engineering**: 94% uptime (above 95% SLA)

### 3. Add Codex Research Context

Added context on:

- Industry benchmarks for SaaS metrics
- Typical Q4 sales patterns
- Best practices for executive presentations

### 4. Create Pre-Read (Internal)

```yaml
Notion:notion-create-pages
title: "Q4 Review - Pre-Read (Internal)"
```

#### Pre-read sections

- **Executive Summary**: Strong quarter, missed revenue by 4% but exceeded customer growth
- **Detailed Metrics**: All KPIs with trend lines
- **Wins**: Product launches, key customer acquisitions
- **Challenges**: Sales pipeline conversion, engineering hiring
- **Q1 Preview**: Strategic priorities

### 5. Create Presentation Agenda

```yaml
Notion:notion-create-pages
title: "Q4 Executive Review - Agenda"
```

**Agenda** (90 min):

- Q4 Results Overview (15 min)
- Revenue & Growth Deep Dive (20 min)
- Product & Engineering Update (20 min)
- Customer Success Highlights (15 min)
- Q1 Strategic Plan (15 min)
- Discussion & Questions (15 min)

### 6. Link Supporting Docs

Connected to OKRs, metrics dashboards, and Q1 planning docs.

## Outputs

**Internal Pre-Read**: Comprehensive context with honest assessment
**Executive Agenda**: Structured 90-min presentation
**Both in Notion** with links to supporting data

## Key Success Factors

- Synthesized data from multiple sources (OKRs, metrics, feedback)
- Added industry context and benchmarks
- Created honest internal assessment (not just wins)
- Structured agenda with time allocations
- Linked to source data for drill-down during Q&A

## When to use

Use when the user asks about or needs: Example: Executive Review Prep.
﻿---
name: Sprint Planning Template
description: # Sprint Planning Template
 
 Use this template for agile sprint planning meetings.
---

# Sprint Planning Template

Use this template for agile sprint planning meetings.

```markdown
# Sprint [#] Planning - [Date]

## Meeting Details
**Date**: [Date]
**Team**: [Team name]
**Sprint Duration**: [Dates]

## Sprint Goal

[Clear statement of what this sprint aims to accomplish]

## Capacity

| Team Member | Availability | Capacity (points) |
| ------------- | -------------- | ------------------- |
| [Name] | [%] | [#] |
| **Total** | | [#] |

## Backlog Review

### High Priority Items

[From product backlog, linked from task database]

- <mention-page url="...">Task 1</mention-page> - [Points]
- <mention-page url="...">Task 2</mention-page> - [Points]

## Sprint Backlog

### Committed Items

- [x] <mention-page url="...">Task</mention-page> - [Points] - @[Owner]
- [ ] <mention-page url="...">Task</mention-page> - [Points] - @[Owner]

**Total committed**: [Points]

### Stretch Goals

- [ ] <mention-page url="...">Task</mention-page> - [Points]

## Dependencies & Risks

**Dependencies**:
- [Dependency]

**Risks**:
- [Risk]

## Definition of Done

- [ ] Code complete and reviewed
- [ ] Tests written and passing
- [ ] Documentation updated
- [ ] Deployed to staging
- [ ] QA approved

## Next Steps

- Team begins sprint work
- Daily standups at [Time]
- Sprint review on [Date]
```

## When to use

Use when the user asks about or needs: Sprint Planning Template.
﻿---
name: Retrospective Template
description: # Retrospective Template  Use this template for sprint retrospectives and team retrospectives.
---

# Retrospective Template

Use this template for sprint retrospectives and team retrospectives.

```markdown
# Sprint [#] Retrospective - [Date]

## Meeting Details
**Date**: [Date]
**Team**: [Team]
**Sprint**: [Sprint dates]
**Facilitator**: [Name]

## Sprint Summary

**Sprint Goal**: [Goal]
**Goal Met**: Yes / Partially / No

**Completed**: [#] points
**Velocity**: [#] points
**Planned**: [#] points

## Pre-Read

**Sprint Metrics**:
- Tasks completed: [#]
- Tasks carried over: [#]
- Bugs found: [#]
- Blockers encountered: [#]

## Discussion

### What Went Well (Keep)

[Team input during meeting]

### What Didn't Go Well (Stop)

[Team input during meeting]

### What To Try (Start)

[Team input during meeting]

### Shout-outs

[Team recognition]

## Action Items

- [ ] [Improvement to implement] - @[Owner] - Due: [Date]
- [ ] [Process change] - @[Owner] - Due: [Date]

## Follow-up

**Review actions in**: [Next retro date]
```

## When to use

Use when the user asks about or needs: Retrospective Template.
﻿---
name: 1:1 Meeting Template
description: # 1:1 Meeting Template  Use this template for manager/report one-on-one meetings.
---

# 1:1 Meeting Template

Use this template for manager/report one-on-one meetings.

```markdown
# 1:1: [Manager] & [Report] - [Date]

## Meeting Details
**Date**: [Date]
**Last meeting**: <mention-page url="...">Previous 1:1</mention-page>

## Agenda

### [Report]'s Topics
1. [Topic to discuss]
2. [Question or concern]

### [Manager]'s Topics
1. [Topic to cover]
2. [Feedback or update]

## Discussion Notes

### [Topic 1]
[Discussion points]

**Action items**:
- [ ] [Action] - @[Owner]

### [Topic 2]
[Discussion points]

## Career Development

**Current focus**: [Development goal]
**Progress**: [Update on progress]

## Feedback

**What's going well**:
- [Positive feedback]

**Areas for growth**:
- [Developmental feedback]

## Action Items

- [ ] [Action] - @[Report] - Due: [Date]
- [ ] [Action] - @[Manager] - Due: [Date]

## Next Meeting

**Date**: [Date]
**Topics to cover**:
- [Carry-over topic]
- [Upcoming topic]
```

## When to use

Use when the user asks about or needs: 1:1 Meeting Template.
﻿---
name: Brainstorming Meeting Template
description: # Brainstorming Meeting Template
 
 Use this template for creative ideation and brainstorming sessions.
---

# Brainstorming Meeting Template

Use this template for creative ideation and brainstorming sessions.

```markdown
# [Topic] Brainstorming - [Date]

## Meeting Details
**Date**: [Date]
**Facilitator**: [Name]
**Note-taker**: [Name]
**Attendees**: [List]

## Objective

[Clear statement of what we're brainstorming]

**Success looks like**: [How we'll know brainstorming was successful]

## Background & Context

[Context from research - 2-3 paragraphs]

**Related Pages**:
- <mention-page url="...">Context Page 1</mention-page>
- <mention-page url="...">Context Page 2</mention-page>

## Constraints

- [Constraint]
- [Constraint]
- [Constraint]

## Seed Ideas

[Starting ideas from research to spark discussion]:

1. **[Idea]**: [Brief description]
2. **[Idea]**: [Brief description]

## Ground Rules

- No criticism during ideation
- Build on others' ideas
- Quantity over quality initially
- Wild ideas welcome

## Brainstorming Notes

### Ideas Generated

[To be filled during meeting]

1. [Idea with brief description]
2. [Idea with brief description]

### Themes/Patterns

[Groupings that emerge]

## Evaluation

[If time permits, evaluate top ideas]

### Top Ideas

| Idea | Feasibility | Impact | Effort | Score |
| ------ | ------------- | --------- | -------- | ------- |
| [Idea] | [H/M/L] | [H/M/L] | [H/M/L] | [#] |

## Next Steps

- [ ] [Action to explore idea]
- [ ] [Action to prototype]
- [ ] [Action to research]

## Follow-up

**Next meeting**: [Date to reconvene]
```

## When to use

Use when the user asks about or needs: Brainstorming Meeting Template.
﻿---
name: Decision Meeting Template
description: # Decision Meeting Template  Use this template when you need to make an important decision with your team.
---

# Decision Meeting Template

Use this template when you need to make an important decision with your team.

```markdown
# [Decision Topic] - [Date]

## Meeting Details
**Date & Time**: [Date and time]
**Duration**: [Length]
**Attendees**: [List of attendees with roles]
**Location**: [Physical location or video link]
**Facilitator**: [Name]

## Pre-Read Summary

### Background
[2-3 sentences providing context from related project pages]

**Related Pages**:
- <mention-page url="...">Project Overview</mention-page>
- <mention-page url="...">Previous Discussion</mention-page>

### Current Situation
[What brings us to this decision point]

## Decision Required

**Question**: [Clear statement of decision needed]

**Timeline**: [When decision needs to be made]

**Impact**: [Who/what is affected by this decision]

## Options Analysis

### Option A: [Name]
**Description**: [What this option entails]

**Pros**:
- [Advantage]
- [Advantage]

**Cons**:
- [Disadvantage]
- [Disadvantage]

**Cost/Effort**: [Estimate]
**Risk**: [Risk assessment]

### Option B: [Name]
[Repeat structure]

### Option C: Do Nothing
**Description**: What happens if we don't decide
**Implications**: [Consequences]

## Recommendation

[If there is a recommended option, state it with rationale]

## Discussion Topics

1. [Topic to discuss]
2. [Clarification needed on]
3. [Trade-offs to consider]

## Decision Framework

**Criteria for evaluation**:
- [Criterion 1]
- [Criterion 2]
- [Criterion 3]

## Decision

[To be filled during meeting]

**Selected Option**: [Option chosen]
**Rationale**: [Why]
**Owner**: [Who will implement]
**Timeline**: [When]

## Action Items

- [ ] [Action] - @[Owner] - Due: [Date]
- [ ] [Action] - @[Owner] - Due: [Date]

## Follow-up

**Next review**: [Date]
**Success metrics**: [How we'll know this worked]
```

## When to use

Use when the user asks about or needs: Decision Meeting Template.
﻿---
name: Status Update Meeting Template
description: # Status Update Meeting Template
 
 Use this template for regular project status updates and check-ins.
---

# Status Update Meeting Template

Use this template for regular project status updates and check-ins.

```markdown
# [Project Name] Status Update - [Date]

## Meeting Details
**Date**: [Date and time]
**Attendees**: [List]
**Project**: <mention-page url="...">Project Page</mention-page>

## Executive Summary

**Status**: 🟢 On Track / 🟡 At Risk / 🔴 Behind

**Progress**: [Percentage] complete
**Timeline**: [Status vs original plan]

## Progress Since Last Meeting

### Completed
- [Accomplishment with specifics]
- [Accomplishment with specifics]

### In Progress
- [Work item and status]
- [Work item and status]

## Metrics

| Metric | Current | Target | Status |
| -------- | --------- | -------- | -------- |
| [Metric] | [Value] | [Value] | [Icon] |
| [Metric] | [Value] | [Value] | [Icon] |

## Upcoming Work

**Next 2 Weeks**:
- [Planned work]
- [Planned work]

**Next Month**:
- [Milestone or major work]

## Blockers & Risks

### Active Blockers
- **[Blocker]**: [Description and impact]
  - Action: [What's being done]

### Risks
- **[Risk]**: [Description]
  - Mitigation: [Strategy]

## Discussion Topics

1. [Topic requiring input]
2. [Topic for alignment]

## Decisions Needed

- [Decision] or None

## Action Items

- [ ] [Action] - @[Owner] - Due: [Date]

## Next Meeting

**Date**: [Date]
**Focus**: [What next meeting will cover]
```

## When to use

Use when the user asks about or needs: Status Update Meeting Template.
﻿---
name: Milestone Summary Template
description: # Milestone Summary Template  Use this when completing major phases or milestones.
---

# Milestone Summary Template

Use this when completing major phases or milestones.

```markdown
## Phase [N] Complete: [Date]

### Accomplishments
- [Major item delivered]
- [Major item delivered]

### Deliverables
- <mention-page url="...">Deliverable 1</mention-page>
- [Link to PR/deployment]

### Metrics
- [Relevant metric]
- [Relevant metric]

### Learnings
- [What went well]
- [What to improve]

### Next Phase
Starting [Phase name] on [Date]
```

## When to use

Use when the user asks about or needs: Milestone Summary Template.
