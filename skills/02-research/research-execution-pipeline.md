---
name: research-execution-pipeline
description: End-to-end research execution covering data capture, literature review, synthesis techniques, and converting research findings to actionable recommendations. Use when conducting literature reviews, synthesizing multiple sources, building decision matrices, or writing research-backed recommendations.
domain: research
tags: [research, execution, synthesis, literature-review, data-capture, recommendations, decision-matrix]
triggers: research notes, literature review, synthesize research, research to action, decision matrix, research recommendations, capture research, note-taking
---

# Research Execution Pipeline

## Data Capture

### Structured Note Template

Use this template per source, capturing at the point of reading (not after):

```text
Source: [title, author, URL/DOI]
Date accessed: [YYYY-MM-DD]
Type: [primary/secondary/tertiary]
Relevance: [High/Medium/Low]
Key claims:
  1. [claim] -- [page/section reference]
  2. [claim] -- [page/section reference]
Methodology: [how the source reached its conclusions]
Limitations: [stated or inferred limitations]
Quotes: [verbatim quotes worth preserving, with page numbers]
My assessment: [what I think about reliability and applicability]
Connections: [links to other sources or themes]
```

### Key Practices

- **Separate fact from interpretation.** Direct quotes in quotation marks, paraphrased facts as plain text, your own analysis in brackets or a separate section.
- **Tag and categorize as you go.** Apply 2-5 tags per note covering topic, method, confidence level, and relevance.
- **Track provenance rigorously.** Every data point should trace to a specific source, page, or section.
- **Use progressive summarization.** First pass: highlight key passages. Second pass: bold the most critical highlights. Third pass: write a brief synthesis.
- **Maintain a running questions list.** As you capture data, questions will emerge. Log them immediately.
- **Archive raw data.** Keep original sources separate from processed notes.
- **Review and consolidate regularly.** End of each session: 5-10 minutes reviewing notes, adding cross-references, updating questions list.

---

## Literature Review & Synthesis

### Search Strategy

- **Define search boundaries.** Specify databases, repositories, documentation sites, and date ranges before searching.
- **Use multiple search strategies.** Keyword search, citation chaining (follow references from good papers), and expert recommendation.
- **Screen systematically.** Read titles and abstracts first. Apply inclusion/exclusion criteria before reading full texts. Track why sources were excluded.
- **Extract data consistently.** Use the standard note template for each source.

### Review Process

1. Define the review question and scope
2. Identify 3+ search channels (web, academic, internal docs, expert networks)
3. Execute searches and collect candidate sources (aim for 15-30 initial candidates)
4. Screen sources using inclusion/exclusion criteria
5. Extract structured data from each included source (8-15 sources typical)
6. Code findings into themes (3-7 themes is typical)
7. Identify and document contradictions between sources
8. Write a synthesis narrative organized by theme, not by source
9. Include a limitations section noting gaps in available literature

### Key Principles

- **Identify themes, not just summaries.** Group findings by theme rather than by source. Common themes: consensus areas, active debates, gaps in knowledge.
- **Flag contradictions explicitly.** Document the disagreement, likely reasons, and your assessment of credibility.
- **Synthesize, do not stack.** A literature review is not a list of summaries. Present an integrated narrative.
- **Know when to stop.** You have reached saturation when new sources repeat themes you have already captured.

---

## Synthesis Techniques

### Method Selection

| Method | Best For |
| --- | --- |
| **Narrative synthesis** | Heterogeneous sources without common metrics. Most versatile. |
| **Meta-analysis** | Multiple studies measuring the same outcome with comparable methods. |
| **Framework synthesis** | Existing theoretical framework can organize diverse findings. |
| **Best-fit synthesis** | Candidate framework needs modification based on findings. |
| **Realist synthesis** | Understanding "what works, for whom, in what circumstances." |

### Narrative Synthesis Process

1. Develop a preliminary synthesis by organizing findings thematically
2. Explore relationships between findings (agreement, contradiction, complement)
3. Assess the robustness of the synthesis (supported broadly or by a few sources?)
4. Write the integrated narrative with explicit cross-references

### Synthesis Tables

Before writing prose, create a matrix: rows are themes or questions, columns are sources. Fill in each cell with the source's contribution to that theme. Gaps in the matrix reveal where evidence is thin.

### Key Principles

- **Preserve dissent.** When sources genuinely disagree, present the disagreement, explain possible reasons, and state which position you find more credible and why.
- **Layer your synthesis.** Headline finding (one sentence), then summary (one paragraph), then detailed analysis.
- **Use conceptual frameworks.** When building a new model, start with observed patterns, abstract to concepts, then test the framework against each source.
- **Cite throughout.** Every claim should reference its supporting sources.
- **Write the "so what."** Synthesis without implication is incomplete.

---

## Research-to-Action Pipeline

### Decision Matrix Construction

1. List all viable options as columns
2. Define evaluation criteria as rows (derived from research, not invented ad-hoc)
3. Weight criteria by importance (must-have vs nice-to-have)
4. Score each option against each criterion using a consistent scale (1-5)
5. Calculate weighted scores and identify the winner
6. Sensitivity check: would changing the weights change the outcome?

### Writing Recommendations

A recommendation has three parts: **what to do**, **why** (supported by evidence), and **what the risks are**.

- "We found that X, Y, and Z" is a **summary**.
- "We recommend X because Y, with the caveat that Z" is a **recommendation**.

### Audience-Specific Structuring

- **Executive summary** (1 paragraph) -- The recommendation, the key reason, and the main risk. No jargon.
- **Decision brief** (1 page) -- The question, options considered, recommendation with rationale, trade-offs, and next steps.
- **Full report** (detailed) -- Complete methodology, findings, analysis, and appendices.

### Impact-Effort Prioritization

- **Quick wins** -- High impact, low effort. Do these first.
- **Strategic bets** -- High impact, high effort. Plan and resource carefully.
- **Fill-ins** -- Low impact, low effort. Do if capacity allows.
- **Avoid** -- Low impact, high effort. Explicitly deprioritize.

### Recommendation Best Practices

- **Address counterarguments proactively.** Identify the strongest argument against each recommendation and address it directly.
- **Define next steps concretely.** "Run a 2-week proof of concept with Technology X using our production dataset, led by [person], reporting results by [date]" is actionable. "Further investigation needed" is not.
- **Include a confidence statement.** "High confidence assuming our traffic grows less than 3x in the next 12 months."
- **Separate the recommendation from the decision.** Research teams recommend; decision-makers decide.

### Action Checklist

- [ ] Build a decision matrix if comparing multiple options
- [ ] Weight criteria and perform sensitivity analysis
- [ ] Write a clear recommendation with evidence and risk acknowledgment
- [ ] Prepare executive summary, decision brief, and full report as needed
- [ ] Prioritize recommended actions using impact-effort framework
- [ ] Address the strongest counterargument
- [ ] Define concrete, assigned, time-bound next steps
- [ ] Include confidence statement with conditions that would change the recommendation
- [ ] Document the decision made, reasoning, and date
- [ ] Schedule a follow-up to assess whether the action produced expected outcomes
