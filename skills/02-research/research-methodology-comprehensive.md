---
name: research-methodology-comprehensive
description: Comprehensive research methodology covering planning, hypothesis formulation, qualitative and quantitative methods, evidence ranking, source validation, and validation gates. Use when planning research, defining hypotheses, choosing methods, evaluating evidence, or quality-checking research outputs.
domain: research
tags: [research, methodology, hypothesis, qualitative, quantitative, evidence, validation, planning]
triggers: research plan, hypothesis, qualitative research, quantitative research, evidence ranking, source validation, research QA, survey design, thematic analysis
---

# Research Methodology Comprehensive

## Planning & Scoping

### When to Use

- Starting a research initiative spanning more than a single session
- A stakeholder asks for findings on an uninvestigated topic
- Comparing multiple approaches, tools, or strategies before recommending
- Prior ad-hoc research produced inconclusive or contradictory results

### Key Practices

- **Define the question first.** Write the research question as a single, unambiguous sentence. If you cannot do this, the scope is not clear enough.
- **Scope ruthlessly.** List what is explicitly in-scope and out-of-scope. Time-box the effort: most research tasks should have a hard stop at 2-4 hours of active work.
- **Choose methods before starting.** Decide upfront whether you need literature review, competitive analysis, primary data collection, expert interviews, or some combination.
- **Identify known unknowns.** Before you search, write down what you already know and what you expect to find. This prevents confirmation bias.
- **Set deliverable format early.** Comparison table, written summary, decision matrix, or recommendation memo? Knowing the format shapes what you collect.
- **Track provenance.** Every claim must link back to its source. Use structured notes from the beginning, not retroactively.
- **Plan iteration.** Most research needs at least two passes: a broad sweep to map the landscape, then a focused dive into the most promising areas.
- **Define "done."** Set explicit completion criteria: number of sources reviewed, confidence threshold reached, or stakeholder sign-off obtained.

### Planning Checklist

- [ ] Write a single-sentence research question
- [ ] List 3-5 sub-questions that decompose the main question
- [ ] Document in-scope and out-of-scope boundaries
- [ ] Select research methods (literature review, interviews, data analysis, etc.)
- [ ] Identify at least 3 initial sources or search strategies
- [ ] Set a time budget and deadline
- [ ] Define the deliverable format and audience
- [ ] Write down current assumptions and expected findings
- [ ] Establish completion criteria
- [ ] Schedule a midpoint review to assess progress and pivot if needed

---

## Hypothesis Formulation

### When to Use

- A research question is too broad or vague to investigate effectively
- You need to decide between competing theories and want a fair test
- Technical evaluation requires defining measurable outcomes before testing
- Stakeholders disagree on expected outcomes and you need an objective frame

### Key Practices

- **Start with an observation.** Every hypothesis begins with something noticed: a performance anomaly, a user complaint pattern, a market signal.
- **Structure the hypothesis clearly.** Use the form: "If [independent variable changes in this way], then [dependent variable will change in this way], because [reasoning]."
- **Define variables explicitly:**
  - **Independent variable** -- What you are changing or comparing (the cause)
  - **Dependent variable** -- What you are measuring (the effect)
  - **Control variables** -- What you hold constant to ensure a fair comparison
- **Ensure falsifiability.** "Technology X is better" is not a hypothesis. "Technology X will process 10K requests/second with p99 latency under 50ms on our standard benchmark" is falsifiable.
- **Set success criteria before testing.** Define what result would confirm, weaken, or refute the hypothesis. Write thresholds down before collecting data.
- **Distinguish hypothesis types:**
  - **Descriptive** -- "System X currently handles N requests/second" (establishes baseline)
  - **Comparative** -- "System X handles more requests than System Y under identical conditions"
  - **Causal** -- "Adding a cache layer will reduce p99 latency by at least 30%"
- **Limit scope.** One hypothesis per investigation. Compound hypotheses produce ambiguous results.

### Hypothesis Checklist

- [ ] Document the initial observation or question
- [ ] Write the hypothesis in "If X, then Y, because Z" format
- [ ] Identify independent, dependent, and control variables
- [ ] Verify the hypothesis is falsifiable
- [ ] Set quantitative success criteria and thresholds
- [ ] Define what "inconclusive" looks like
- [ ] Review for bias: does the hypothesis assume its own conclusion?

---

## Qualitative Research Methods

### When to Use

- Exploring a new problem space where you do not yet know the right questions
- Understanding user motivations, pain points, or workflows through interviews
- Analyzing unstructured data such as support tickets, forum posts, or open-ended survey responses
- Building theory or generating hypotheses before quantitative validation

### Method Selection

- **Semi-structured interviews** -- Best for exploring individual experiences and reasoning. Use a guide with open-ended questions but allow follow-up.
- **Focus groups** -- Best for understanding group dynamics and shared vs divergent views. Limit to 5-8 participants.
- **Observational studies** -- Best for understanding actual behavior vs reported behavior. Watch people work; do not just ask them.
- **Document analysis** -- Best for understanding organizational norms, historical decisions, or patterns in written artifacts.

### Interview Design

- Start with broad, open-ended questions; narrow gradually
- Use "tell me about a time when..." prompts to elicit concrete examples
- Avoid leading questions ("Don't you think X is a problem?")
- Prepare 8-12 core questions; expect to use 6-8 in a 45-minute session
- Always pilot the guide with one participant before the full study

### Thematic Analysis Process

1. **Familiarization** -- Read all data, note initial impressions
2. **Initial coding** -- Label meaningful segments with descriptive codes
3. **Theme development** -- Group related codes into broader themes
4. **Review themes** -- Check themes against the data; merge or split as needed
5. **Define and name themes** -- Write a clear definition and scope for each
6. **Report** -- Present themes with supporting quotes and examples

### Saturation

Continue data collection until new interviews or observations stop producing new codes or themes. For most topics, 8-15 interviews reach saturation.

---

## Quantitative Research Methods

### When to Use

- Measuring the prevalence or magnitude of something (adoption rates, error rates, performance metrics)
- Comparing two or more options with measurable outcomes (A/B testing, benchmark comparisons)
- Validating a hypothesis with statistical confidence
- Designing surveys to collect structured data from a population

### Survey Design Principles

- Keep surveys under 15 questions for adequate completion rates
- Use consistent scales (always 1-5 or always 1-7, never mix)
- Include one or two reverse-coded items to detect inattentive responses
- Avoid double-barreled questions ("Is this tool fast and reliable?")
- Pilot with 5-10 respondents before full distribution

### Sample Sizing Guidelines

- For population proportions with 95% confidence and 5% margin of error: ~385 responses for large populations
- For comparing two groups: use power analysis; typical minimum is 30 per group for parametric tests
- For benchmarks and performance tests: minimum 30 runs per configuration to establish stable distributions

### Statistical Test Selection

| Test | Use Case |
| --- | --- |
| **t-test** | Comparing means of two groups |
| **ANOVA** | Comparing means across 3+ groups |
| **Chi-square** | Testing association between categorical variables |
| **Correlation** | Measuring linear relationship between two continuous variables |
| **Regression** | Predicting one variable from one or more others |

### Key Principles

- **Statistical significance is not practical significance.** A p-value < 0.05 means unlikely due to chance, not that it matters. A 0.1ms improvement may be statistically significant with enough samples but practically irrelevant.
- **Report effect sizes alongside p-values.** Cohen's d for mean comparisons, r-squared for correlations.
- **Control for confounds.** Change one variable at a time.
- **Document collection methodology.** Record when data was collected, sampling method, response rate, exclusion criteria, and anomalies.

---

## Evidence Hierarchy & Ranking

### The 9-Tier Hierarchy (strongest to weakest)

1. **Reproducible primary research** -- Controlled experiments, benchmarks you ran yourself, A/B tests with statistical significance
2. **Peer-reviewed primary research** -- Published studies with methodology disclosure and peer review
3. **Replicated findings** -- Claims confirmed independently by 3+ unrelated sources
4. **Systematic reviews and meta-analyses** -- Rigorous aggregation of multiple primary studies
5. **Case studies and field reports** -- Real-world implementations with documented outcomes
6. **Expert analysis** -- Opinions from recognized domain experts with disclosed reasoning
7. **Vendor documentation** -- Authoritative for the vendor's own system but inherently biased
8. **Anecdotal reports** -- Individual experiences, blog posts, forum comments
9. **Unsubstantiated claims** -- Assertions without evidence, methodology, or attribution

### Handling Conflicts

1. Check if sources measured the same thing (apples-to-apples comparison)
2. Compare methodology rigor using the hierarchy
3. Consider contextual differences (scale, configuration, workload)
4. Look for a newer source that may supersede an older one
5. If still unresolved, present both positions with your assessment

### Confidence Levels

- **High** -- Multiple strong sources agree
- **Medium** -- One strong source or multiple moderate ones
- **Low** -- Single moderate source or contradicted by others
- **Speculative** -- Weak evidence only

---

## Source Validation & Credibility

### Key Practices

- **Classify source type.** Primary > secondary > tertiary. An AWS blog post about AWS superiority is marketing, not research.
- **Check author credentials.** Expertise? Financial or ideological interest in the conclusion?
- **Verify publication context.** Peer-reviewed > self-published. Conference proceedings > preprints.
- **Cross-reference key claims.** Important claims should appear in at least two independent sources.
- **Check the date.** In technology domains, a 2-year-old benchmark may be irrelevant.
- **Look for methodology disclosure.** Vague claims like "studies show" without citations are red flags.

### Common Bias Patterns

- **Survivorship bias** -- Only successful cases are reported
- **Confirmation bias** -- Author sought evidence supporting a predetermined conclusion
- **Vendor bias** -- Source has financial interest in the recommendation
- **Recency bias** -- Newest is assumed best without evidence
- **Authority bias** -- Accepted because of who said it, not the evidence

---

## Research Validation Gates

### Gate 1: Completeness Check

- Does the research address the original question fully?
- Are all sub-questions answered or explicitly marked as out-of-scope?
- Is the methodology section complete enough for replication?

### Gate 2: Source Audit

- Can every factual claim be traced to a specific source?
- Have all sources been validated for credibility?
- Is there over-reliance on a single source for critical claims?
- Have vendor-provided claims been cross-referenced?

### Gate 3: Bias Audit

- Review for confirmation bias: did the research seek disconfirming evidence?
- Review for selection bias: were sources chosen systematically or cherry-picked?
- Review for survivorship bias: are failures and negative results represented?

### Gate 4: Reproducibility Verification

- Can the search strategy be repeated to find the same sources?
- Are benchmarks documented well enough to reproduce?
- Would a different researcher reach similar conclusions?

### Gate 5: Peer Review

- Have findings been reviewed by someone who did not conduct the research?
- Has the reviewer challenged the strongest claims and weakest evidence?
- Is the language precise and unambiguous?

**Apply gates proportionally.** A 2-hour competitive scan needs Gates 1-2. A multi-week strategic research effort needs all five gates.
