---
name: agent-evaluation-evals
description: Comprehensive guide to LLM and RAG evaluation frameworks (Braintrust, PromptFoo, RAGAS), RAGAS metric internals, LLM-as-judge design, golden dataset construction, regression evals in CI, and metric selection by task type.
domain: ai-ml
category: evaluation
tags: [evals, RAGAS, PromptFoo, Braintrust, LLM-evaluation, benchmarks, RAG-metrics, LLM-as-judge, golden-dataset, CI-evals]
triggers: ["eval", "evaluation", "evals", "RAGAS", "PromptFoo", "Braintrust", "LLM-as-judge", "golden dataset", "RAG metrics", "faithfulness", "context recall", "answer relevancy", "regression testing", "benchmark", "hallucination detection"]
---

# LLM and Agent Evaluation — Framework Guide

## Framework Landscape

Three tools dominate the practical LLM evaluation space. They solve different problems and are most powerful when composed rather than treated as alternatives.

### PromptFoo — Prompt Testing and CI Gate

PromptFoo is a CLI and library whose primary job is systematic prompt testing and LLM red teaming. It is fully open-source (MIT), runs 100% locally (prompts never leave the machine), and uses declarative YAML configuration files. Evaluation runs with `promptfoo eval`; results are viewed with `promptfoo view`.

**Architecture**: Node.js-based, YAML-driven. Assertions are defined inline with test cases. A GitHub Action provides native CI integration.

**Assertion types**: exact match, contains, regex, LLM-as-judge rubric (`llm-rubric`), JSON schema validation, cost threshold, latency threshold, toxicity, bias, factuality.

**Red teaming**: PromptFoo includes an automated red-team agent that performs reconnaissance, attack planning, and vulnerability scanning aligned with OWASP LLM Top 10 and NIST presets. It generates a security report and can scan pull requests for LLM-related compliance issues.

**Model support**: OpenAI, Anthropic, Azure, AWS Bedrock, Ollama, Gemini, and others — enabling side-by-side comparison grids.

**Best for**: Teams that need a lightweight, portable, developer-friendly eval loop; prompt regression testing before every deploy; security validation and red teaming; Node.js shops that want CI-native evals without a SaaS dependency.

**Limitations**: Results live in CI artifacts or local runs — no persistent experiment tracking, no shared quality dashboard, no production monitoring. Not a standalone eval management system.

---

### RAGAS — RAG-Specific Reference Metrics

RAGAS (Retrieval Augmented Generation Assessment) is an open-source Python library providing research-backed metrics specifically for RAG pipelines and, more recently, agentic tool-use workflows. It is designed to be embedded inside a broader evaluation system rather than used as a management platform.

**Architecture**: Python library, async-first API, pluggable LLM backends (uses an LLM internally for claim decomposition and verification). Integrates with LangChain, LlamaIndex, and any pipeline that can emit (question, context, response) tuples.

**Core RAG metrics** (all score 0–1, higher is better):

| Metric | What it measures | Inputs required |
| --- | --- | --- |
| **Faithfulness** | Factual consistency of response with retrieved context | question, response, contexts |
| **Answer Relevancy** | How directly the response addresses the question | question, response |
| **Context Precision** | Signal-to-noise ratio of retrieved chunks (are the right chunks ranked first?) | question, contexts, ground truth |
| **Context Recall** | How much of the ground-truth answer is coverable from retrieved docs | question, contexts, ground truth |
| **Context Entities Recall** | Entity-level coverage of ground truth in retrieved context | question, contexts, ground truth |
| **Noise Sensitivity** | How much irrelevant context degrades the response | question, response, contexts |

**How Faithfulness is computed**: The response is decomposed into atomic claims. Each claim is checked against the retrieved context. Score = claims supported / total claims. A score of 1.0 means every statement in the response is grounded in the source material; anything below 0.8 warrants investigation for hallucination. Vectara's HHEM-2.1-Open can substitute for the LLM in the verification step.

**Agent / tool-use metrics**: Topic Adherence, Tool Call Accuracy, Tool Call F1, Agent Goal Accuracy.

**General-purpose metrics**: Factual Correctness, Semantic Similarity, Aspect Critic, Rubrics-based scoring (custom criteria), BLEU, ROUGE, CHRF, Exact Match, String Presence.

**SQL evaluation**: Datacompy scoring (execution-based), SQL Query Equivalence.

**Best for**: Any team building or debugging a RAG pipeline; getting fast, reference-free retrieval and generation diagnostics; embedding retrieval quality checks into a broader eval harness.

**Limitations**: No CI orchestration, no experiment tracking UI, no production monitoring, no human collaboration interface. Needs external infrastructure (Braintrust, LangSmith, Arize, etc.) for lifecycle management.

---

### Braintrust — Full Evaluation Lifecycle Platform

Braintrust is a SaaS (with self-hosting option) that covers the full eval lifecycle: dataset management, experiment tracking, CI/CD gating, production monitoring, prompt management, and team collaboration.

**Architecture**: TypeScript-first, purpose-built "Brainstore" backend (claimed 80x faster than traditional databases for AI logs). SOC 2 Type II compliant. Supports Python SDK as well.

#### Key capabilities

- Evals run against both offline datasets and live production traffic
- Detailed traces across every agent step
- Shared UI where PMs and engineers collaborate on prompt iteration
- Automated Loop agent: auto-generates evaluation datasets, refines scorers, optimizes prompts
- Plugs into CI/CD as a deployment gate

**Best for**: Teams that need persistent experiment history; production-monitoring-to-eval feedback loops; collaboration between non-engineers and engineers on prompt quality; enterprise governance requirements.

**Limitations**: SaaS with pricing; TypeScript-first means Python teams need adaptation. Opinionated platform means some lock-in.

---

### Framework Selection Guide

| Need | Reach for |
| --- | --- |
| Prompt regression in CI, red teaming, security scanning | PromptFoo |
| RAG retrieval and generation diagnostics | RAGAS |
| Full lifecycle: datasets, experiments, prod monitoring, collaboration | Braintrust |
| Open-source observability with self-hosting | Langfuse or Arize Phoenix |
| LangChain-heavy Python stack, rapid prototyping | LangSmith |

In practice, a mature setup combines all three: RAGAS metrics embedded inside a Braintrust experiment, with PromptFoo running the pre-deploy CI gate.

---

## RAGAS Metrics — Interpretation and Thresholds

Scores above **0.8** generally indicate strong performance, but thresholds are domain-dependent. A knowledge-base Q&A system should set a stricter faithfulness bar (0.9+) than a creative assistant.

**Faithfulness < 0.7**: The model is likely hallucinating facts not present in retrieved context. Audit the retrieval chunks first — often the retriever is returning irrelevant material, causing the LLM to fill gaps.

**Context Recall < 0.6**: The retriever is missing relevant documents. Review chunking strategy, embedding model, and top-k settings.

**Context Precision < 0.6**: The retriever is returning noise alongside signal. High-precision retrieval (reranking, metadata filtering) is needed.

**Answer Relevancy < 0.7**: The response is verbose, off-topic, or answering a different question. Inspect system prompt and prompt structure.

**Improving scores**: Retrieval-side issues (recall, precision, noise sensitivity) are fixed in the retrieval pipeline, not by prompt engineering. Generation-side issues (faithfulness, answer relevancy) respond to prompt changes, chain-of-thought instructions, and model selection.

---

## LLM-as-Judge Pattern

LLM-as-judge uses a strong LLM (GPT-4o, Claude Sonnet, Gemini 1.5 Pro) as an automated scorer, replacing expensive human annotation for most test cases. Research shows high correlation with human judgment when implemented correctly.

### Design Principles

**Write explicit rubrics.** Vague criteria ("is the answer good?") produce inconsistent scores. Define each score level with concrete examples:

```text
Score 1: Response contradicts or ignores provided context.
Score 2: Response is partially grounded but introduces unsupported claims.
Score 3: Response is fully grounded in context and answers the question directly.
```

**Decompose into atomic evaluations.** A single prompt asking "rate quality, safety, relevance, and accuracy" is unreliable. One evaluation criterion per prompt produces more consistent results.

**Use G-Eval framing** (from DeepEval's research): give the judge explicit evaluation steps as a chain-of-thought scaffold before the scoring instruction. This reduces judge variance significantly.

**Output structured scores.** Instruct the judge to return JSON with `score` (integer) and `reason` (string). Parse programmatically; log the reason for debugging.

#### Watch for judge bias

- *Verbosity bias*: Longer answers score higher even when wrong. Counter with explicit rubric language: "Length does not contribute to score."
- *Self-preference bias*: GPT-4o systematically scores GPT-4o outputs higher. Use a different judge than the model being evaluated, or average across multiple judges.
- *Position bias*: In pairwise comparisons, the first option wins more often. Randomize order and average both orderings.

**Calibrate against humans.** On a sample of 50–100 cases, compare judge scores to human annotations. Target Cohen's kappa > 0.6. If agreement is low, revise the rubric.

### Evaluator Type Hierarchy

1. **Deterministic** (regex, exact match, JSON schema): cheapest, most reliable, use wherever possible.
2. **Statistical** (BLEU, ROUGE, semantic similarity): good for translation/summarization, fragile for open-ended tasks.
3. **LLM-as-judge**: necessary for semantic quality, safety, and instruction-following where deterministic checks cannot reach.

---

## Golden Dataset Construction

A golden dataset is a versioned collection of (input, optional context, expected output, rubric) tuples that acts as the regression test suite for your AI system.

### Step-by-Step Build Process

**1. Define scope and metrics first.** Decide what you are evaluating (retrieval, generation, full pipeline, agent multi-step) and which metrics matter. Misaligned scope produces noisy, unactionable benchmarks.

#### 2. Source data from multiple origins

- *Production logs*: highest fidelity, represents actual user behavior. Filter for interesting/edge/failure cases. Strip PII.
- *SME-authored must-pass cases*: expert-written scenarios with explicit acceptance criteria. Anchors the dataset against real domain requirements.
- *Adversarial/red-team cases*: jailbreaks, prompt injections, harmful content requests. Ensure safety coverage.
- *Synthetic generation*: use an LLM to generate variations of real cases (Evol-Instruct methodology). Treat as "silver" — promote to "gold" only after human review.

**3. Write precise rubrics for each case.** Expected output should have an associated scoring schema: required entities, acceptable phrasings, disqualifying errors. Do not rely only on fuzzy "expected output" strings.

**4. Calibrate annotators.** Run a pilot labeling round, measure inter-annotator agreement (target Cohen's kappa > 0.7), resolve disagreements with a tie-breaker reviewer, document the decision rationale.

**5. Enforce decontamination.** Check for overlap with training data via exact match, substring search, and embedding similarity. Near-duplicate items inflate metrics without measuring real capability.

#### 6. Attach rich metadata to every item

- Scenario attributes: intent, persona, difficulty, language, safety category
- Provenance: source (production/synthetic/SME), timestamp, reviewer ID
- Governance: risk tags, NIST RMF alignment, audit trail

**7. Size the dataset correctly.** For an expected 80% pass rate with 5% margin of error at 95% confidence: ~246 samples per scenario. Start with 50–100 cases to uncover gross failures, then grow. More cases are not always better — diverse coverage beats raw volume.

**8. Version and gate.** Map dataset versions to prompt and model versions. Enforce a minimum aggregate score threshold as a release gate. Failing the gate blocks the deploy.

### Maintenance

Golden datasets decay. Common causes: prompt changes that obsolete old expected outputs, domain drift, new failure modes surfacing in production. Build a pipeline that:

- Monitors production for anomaly patterns and imports interesting failures into the dataset
- Schedules periodic human review passes (quarterly minimum)
- Flags when evaluation distributions shift significantly (score standard deviation spike)

---

## Regression Evals in CI

### Setup Principles

Run evals in CI on every pull request that touches prompt text, retrieval config, model version, or system architecture. This is not optional — model behavior changes without code changes.

#### Gate structure

1. *Unit-level eval* (fast, < 2 min): 20–30 critical must-pass cases. Hard fail blocks merge.
2. *Regression eval* (medium, < 10 min): Full golden dataset. Soft fail on score regression > 2% vs. main branch; requires human sign-off.
3. *Nightly eval* (slow, < 1 hr): Extended dataset including adversarial and edge cases. Results feed the monitoring dashboard.

**Treat actual model outputs as computed at eval time**, not pre-generated. Caching LLM responses leads to stale evals that miss regressions.

**Track score history, not just pass/fail.** A score drop from 0.92 to 0.87 is a signal even if it stays above the gate threshold.

### PromptFoo CI Example Pattern

```yaml
# promptfooconfig.yaml
prompts:
  - file://prompts/answer-v2.txt

providers:
  - openai:gpt-4o-mini

tests:
  - vars:
      question: "What is the refund policy?"
      context: "{{file://fixtures/refund_policy.txt}}"
    assert:
      - type: llm-rubric
        value: "Response cites the 30-day policy and does not invent conditions."
      - type: cost
        threshold: 0.002
      - type: latency
        threshold: 3000
```

Run in CI: `promptfoo eval --ci` exits non-zero on any assertion failure.

### Braintrust CI Pattern

Braintrust experiments are triggered programmatically via SDK. The experiment records every output, score, and trace, enabling diff views between branches. A GitHub Action can call the Braintrust API to block merge if the experiment score falls below a threshold.

---

## Metric Selection by Task Type

Not all tasks need the same metrics. Applying the wrong metrics produces misleading signals.

| Task type | Primary metrics | Secondary / diagnostic |
| --- | --- | --- |
| RAG Q&A | Faithfulness, Context Recall, Answer Relevancy | Context Precision, Noise Sensitivity |
| Summarization | Faithfulness, Factual Correctness, ROUGE-2 | Semantic Similarity, human brevity score |
| Code generation | Exact execution pass rate, test pass rate | Syntactic correctness, security scan |
| Classification | Accuracy, F1, confusion matrix | LLM-as-judge for borderline cases |
| Dialogue / chat | Answer Relevancy, safety scorer, instruction-following | User satisfaction proxy, topic adherence |
| Agent (tool use) | Tool Call Accuracy, Agent Goal Accuracy, Task Completion Rate | Trajectory efficiency (steps/task), cost |
| Translation | CHRF, COMET, human MQM | BLEU (low signal, use as sanity check only) |
| SQL generation | Execution match rate, Datacompy score | SQL Query Equivalence |

**General rule**: prefer reference-based metrics (those that compare against a ground truth) during development — they are stricter and more debuggable. Use reference-free metrics (RAGAS faithfulness, LLM-as-judge) in production monitoring where ground truth is unavailable.

---

## Human Annotation Workflow

Human annotation remains the ground truth calibration layer even when most scoring is automated.

### Annotation pipeline

1. Sample 5–10% of production traffic weekly (stratified by failure signal, not random — bias toward borderline and failing cases).
2. Route to annotators through a dedicated interface (Label Studio, Argilla, Scale AI, or a custom Retool app).
3. Annotators score on a rubric that mirrors your LLM-as-judge rubric. This enables direct calibration.
4. Resolve disagreements: cases with disagreement > 1 point get a second-pass review by a senior annotator.
5. Compute inter-annotator agreement weekly. Declining kappa is a rubric clarity problem, not an annotator skill problem.
6. Feed approved annotations back into the golden dataset and use them to recalibrate the judge LLM's few-shot examples.

**Annotation efficiency**: surface failed LLM-as-judge cases for human review first (judge uncertainty is a proxy for annotation value). Cases where the judge is confident and correct do not need human review.

---

## Common Pitfalls

**Overfitting the eval to the prompt.** When you iteratively optimize a prompt against a fixed eval set, you will eventually overfit. Reserve 20% of the golden dataset as a held-out test set; never use it during iteration.

**Using BLEU/ROUGE for open-ended tasks.** These n-gram metrics measure surface form, not meaning. A paraphrase scores 0; a near-copy scores high. Use semantic similarity or LLM-as-judge for generative tasks.

**Skipping retrieval evaluation.** Teams often only evaluate generation quality and miss that recall and precision issues are causing hallucination downstream. Always eval both retrieval and generation independently.

**Treating eval as a one-time activity.** Evals decay with model updates, prompt changes, and domain drift. Build them into the development loop from day one.

**Running evals only offline.** Production traffic surfaces failure modes that no synthetic dataset anticipates. Wire production monitoring to automatically flag and queue cases for eval review.

**Judge model equals target model.** If GPT-4o is generating answers and GPT-4o is judging them, self-preference bias inflates scores. Use a different judge family, or average across judges.
