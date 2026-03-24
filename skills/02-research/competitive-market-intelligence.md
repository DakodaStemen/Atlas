---
name: competitive-market-intelligence
description: Competitive intelligence, market analysis, technology evaluation, and domain knowledge mapping. Use when performing competitor analysis, market sizing (TAM/SAM/SOM), evaluating technologies for adoption, or building domain knowledge maps.
domain: research
tags: [research, competitive-intelligence, market-analysis, technology-evaluation, domain-mapping, SWOT, TAM]
triggers: competitor analysis, market research, TAM SAM SOM, technology evaluation, compare frameworks, SWOT analysis, domain mapping, knowledge map, build vs buy
---

# Competitive & Market Intelligence

## Competitive Intelligence Workflow

### When to Use

- Evaluating whether to build, buy, or adopt an existing solution
- Preparing to position a product or feature against competitors
- Pricing decisions that require understanding market norms
- Identifying gaps in the market that represent opportunities

### Key Practices

- **Define the competitive set.** List direct competitors (same problem, same audience), indirect competitors (same problem, different approach), and potential future competitors (adjacent products likely to expand).
- **Build a feature matrix.** Columns are competitors; rows are capabilities. Use consistent ratings: Yes/No/Partial, or a 1-5 scale with defined criteria.
- **Analyze pricing models.** Document pricing tiers, usage limits, enterprise vs self-serve options, and hidden costs. Normalize pricing to a common unit (per user/month, per request, per GB).
- **Assess market positioning.** Map competitors on 2x2 axes relevant to your context (e.g., ease of use vs power, price vs features).
- **Conduct SWOT for each major competitor:**
  - **Strengths** -- What they do objectively well (features, scale, brand, ecosystem)
  - **Weaknesses** -- Known limitations, common complaints, architectural constraints
  - **Opportunities** -- Market trends they could capitalize on
  - **Threats** -- Risks to their position (new entrants, technology shifts, regulation)
- **Use multiple data sources.** Official documentation, changelog/release notes, user reviews (G2, Reddit, HN), job postings, GitHub activity, and direct product trials.
- **Track momentum, not just snapshots.** Release frequency, hiring patterns, funding rounds, and community growth matter more than current state.

### Competitive Intelligence Checklist

- [ ] Define the competitive set (direct, indirect, potential)
- [ ] Build a feature comparison matrix with consistent rating criteria
- [ ] Document pricing models normalized to common units
- [ ] Create a positioning map on relevant axes
- [ ] Complete SWOT analysis for top 3-5 competitors
- [ ] Review user feedback from 2+ independent channels per competitor
- [ ] Assess competitive momentum (release velocity, community growth, hiring)
- [ ] Identify market gaps not currently addressed
- [ ] Date-stamp all findings and note verification status

---

## Market Intelligence Gathering

### Market Sizing (TAM/SAM/SOM)

- **TAM (Total Addressable Market)** -- Total revenue opportunity at 100% market share. Calculate top-down (industry reports) and bottom-up (potential customers x average revenue). If the two approaches differ by more than 2x, investigate.
- **SAM (Serviceable Addressable Market)** -- Segment of TAM reachable with your product, distribution, and geography. Typically 10-40% of TAM.
- **SOM (Serviceable Obtainable Market)** -- Portion of SAM realistically capturable in 1-3 years. SOM > 10% of SAM requires strong justification.

### Trend Identification Framework

- **Technology trends** -- Emerging tools, platforms, architectures gaining adoption (GitHub stars, Stack Overflow activity, job postings)
- **Market trends** -- Shifts in buyer behavior, pricing models, consolidation (M&A activity, funding rounds, analyst reports)
- **Regulatory trends** -- New or proposed regulations, compliance requirements, standards
- **Workforce trends** -- Skill demand shifts, remote work patterns, talent availability

### Data Sources for Triangulation

- Industry analyst reports (Gartner, Forrester, IDC) for market sizing
- SEC filings and earnings calls for public company revenue data
- Crunchbase/PitchBook for funding and M&A trends
- Job postings for technology adoption signals
- Patent filings for future technology direction
- Reddit, HN, and developer communities for grassroots sentiment

### Key Principles

- **Distinguish signal from noise.** Apply a relevance filter: Does this affect your market segment? Is adoption accelerating or plateauing?
- **Date-stamp everything.** A TAM estimate from 18 months ago may be significantly outdated.
- **Separate facts from projections.** Current market size is measurable. Growth projections are estimates with assumptions.

---

## Technology Evaluation Framework

### Maturity Scale

- **Experimental** -- Pre-1.0, breaking changes expected, limited production use
- **Early adoption** -- 1.0+, growing community, some production deployments, APIs stabilizing
- **Mainstream** -- Stable API, extensive documentation, broad production use
- **Mature** -- Well-understood trade-offs, rich ecosystem, declining innovation velocity
- **Declining** -- Maintenance mode, community shrinking, better alternatives emerging

### Ecosystem Health Evaluation

- **Documentation quality** -- Official docs, tutorials, API references, migration guides
- **Package/plugin ecosystem** -- Number and quality of extensions, integrations, middleware
- **Tooling support** -- IDE integration, debugging tools, profiling, testing frameworks
- **Cloud/platform support** -- Managed offerings, deployment options, monitoring integration

### Performance Benchmarking

Run benchmarks relevant to your workload. Generic benchmarks are misleading. Define your specific access patterns, data sizes, and concurrency requirements. Document hardware, configuration, and methodology.

### Migration Cost Estimation

- **Code changes** -- Lines of code affected, API surface to rewrite, abstraction layers needed
- **Data migration** -- Schema changes, data format conversion, migration downtime
- **Team learning** -- Training time, ramp-up productivity loss, hiring implications
- **Operational changes** -- Monitoring, alerting, runbooks, on-call procedures
- **Rollback plan** -- How to revert if the migration fails partway through

### Community Health Indicators

- Issue response time (median time to first response on GitHub issues)
- Release frequency (monthly, quarterly, annual)
- Contributor diversity (bus factor: how many core maintainers?)
- Corporate backing vs community-driven (both have risks)
- Stack Overflow answer rate for common questions

### Total Cost of Ownership

Include licensing, hosting, operational overhead, training, and opportunity cost. A free tool with high operational overhead may cost more than a paid managed service.

### Technology Evaluation Checklist

- [ ] Define the specific use case and requirements
- [ ] Assess maturity level using the defined scale
- [ ] Evaluate documentation quality and completeness
- [ ] Survey the ecosystem: packages, plugins, integrations, tooling
- [ ] Run benchmarks against actual workload patterns
- [ ] Estimate migration cost across all dimensions
- [ ] Assess community health
- [ ] Calculate total cost of ownership including hidden costs
- [ ] Test error handling, debugging experience, and failure modes
- [ ] Check license compatibility
- [ ] Set and meet a decision deadline

---

## Domain Knowledge Mapping

### Building the Map

1. Identify 10-20 foundational concepts. Write a one-sentence definition for each.
2. Map relationships between concepts:
   - **Is-a** -- Classification hierarchy (a REST API is a type of Web API)
   - **Has-a** -- Composition (a Kubernetes cluster has nodes)
   - **Depends-on** -- Prerequisites (CI/CD depends on version control)
   - **Influences** -- Causal or correlational (cache hit rate influences response latency)
   - **Contradicts** -- Tension between concepts (consistency vs availability)
3. Build layered maps:
   - Layer 1: Core concepts and relationships (fits on one page)
   - Layer 2: Sub-concepts within each core concept
   - Layer 3: Implementation details, tools, specific technologies

### Knowledge Gap Analysis

- **Unknown unknowns audit** -- For each concept, ask: "What don't we know about this that could hurt us?" Rate each gap by risk.
- **Expertise inventory** -- For each concept, identify who has working knowledge, deep expertise, or no exposure. Single points of expertise are risks.
- **Freshness check** -- When was knowledge last validated? In fast-moving domains, knowledge older than 12 months may be stale.

### Taxonomic Principles

- **Mutually exclusive** -- Categories should not overlap
- **Collectively exhaustive** -- Categories should cover the full domain
- **Consistent depth** -- All branches should reach similar levels of detail
- **Useful granularity** -- Stop subdividing when further detail does not aid understanding

### Domain Mapping Checklist

- [ ] Identify 10-20 core concepts with one-sentence definitions
- [ ] Map relationships between concepts
- [ ] Build Layer 1 map (one-page view)
- [ ] Conduct unknown unknowns audit
- [ ] Build expertise inventory (who knows what)
- [ ] Identify single points of expertise (knowledge bus factor)
- [ ] Perform freshness check on existing knowledge
- [ ] Validate with 2-3 domain experts
- [ ] Schedule regular review cadence (quarterly recommended)
