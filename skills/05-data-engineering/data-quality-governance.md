---
name: data-quality-governance
description: Comprehensive data quality, governance, lineage, privacy/PII handling, cost optimization, and migration patterns. Use when implementing data quality monitoring, building lineage tracking, handling PII compliance, optimizing data costs, or planning data migrations.
domain: data-engineering
tags: [data-quality, governance, lineage, pii, privacy, gdpr, cost-optimization, migration, profiling, anomaly-detection]
triggers: data quality, data governance, data lineage, PII handling, data privacy, GDPR, CCPA, data cost, cost optimization, data migration, data profiling, freshness monitoring, data compliance
---

# Data Quality and Governance

## 1. Data Quality Monitoring

### Quality Dimensions

- **Freshness**: Is data arriving on time? Track max event timestamp per table/partition. Alert when gap exceeds SLA.
- **Completeness**: Are expected records/fields present? Monitor null rates and row counts.
- **Accuracy**: Do values match reality? Cross-validate against source systems.
- **Consistency**: Do related datasets agree? Compare aggregates across tables.
- **Uniqueness**: Are there unexpected duplicates? Monitor distinct counts on key columns.
- **Validity**: Do values conform to expected formats and ranges?

### Volume Monitoring

- Track row counts per load, per partition, per source. Establish baselines (rolling averages). Alert on >30% drop or >200% spike. Zero-row loads always trigger alerts.

### Schema Monitoring

- Detect unexpected column additions, removals, type changes. Compare current schema against registered schema. Alert on schema drift before downstream failures.

### Distribution Monitoring

- Profile column value distributions: null rates, distinct counts, min/max, mean/stddev. Detect anomalies using z-score or IQR. Focus on columns driving business logic or downstream models.

### Quality Gates

- Block data promotion (bronze→silver, silver→gold) when quality checks fail. Log all quality check results for audit and trend analysis.

### Alerting Strategy

- **Critical** (data missing, pipeline broken): on-call. **Warning** (anomaly, minor quality issue): team channel. Include context in alerts: table, partition, expected vs actual. Tune thresholds quarterly to avoid alert fatigue.

### Tooling

- **Open source**: Great Expectations, Soda Core, dbt tests, Elementary.
- **Managed**: Monte Carlo, Bigeye, Anomalo.

## 2. Data Governance and Lineage

### Lineage Tracking

- **Column-level lineage**: Track which source columns produce each target column. Essential for impact analysis and debugging.
- **Pipeline lineage**: Map source→transformation→destination for every pipeline. Auto-capture via orchestrator metadata where possible.
- **Business lineage**: Map business terms to physical columns. Maintain in a business glossary accessible to non-technical stakeholders.

### Data Catalog

- Register all datasets with metadata: schema, ownership, classification, update frequency, quality scores. Enable search and discovery. Tag with business domains. Require ownership for every dataset.

### Classification

- **Public**: No restrictions. **Internal**: Employee access. **Confidential**: Need-to-know basis. **Restricted**: PII, financial, health data with regulatory requirements. Apply classification at column level, not just table level.

### Ownership

- Every dataset has an owner (team, not individual). Owners are responsible for quality, documentation, access approval, and incident response. Review ownership quarterly.

### Access Control

- Implement least-privilege. Use role-based access with data classification. Audit all access to confidential/restricted data. Automate access reviews. Require justification for elevated access.

### Retention Policies

- Define retention per dataset based on regulatory requirements and business need. Automate enforcement. Distinguish between "must keep" (regulatory) and "nice to have" (analytics).

## 3. Privacy and PII Handling

### PII Identification

- **Direct identifiers**: Name, email, phone, SSN, passport, address. **Quasi-identifiers**: ZIP code, birth date, gender (can identify when combined). **Sensitive data**: Health records, financial data, biometrics, political/religious affiliation.

### Protection Techniques

| Technique | Description | Reversible | Use Case |
|-----------|-------------|------------|----------|
| **Encryption** | AES-256 at rest, TLS in transit | Yes (with key) | Storage, transmission |
| **Tokenization** | Replace with random token, store mapping securely | Yes (with vault) | Payment card data |
| **Hashing** | One-way hash (SHA-256 + salt) | No | Pseudonymization for analytics |
| **Masking** | Partial redaction (***@email.com) | No | Display, logging |
| **Generalization** | Reduce precision (exact age → age range) | No | Analytics, k-anonymity |
| **Differential privacy** | Add calibrated noise to aggregates | No | Statistical queries |

### Compliance Requirements

- **GDPR**: Right to erasure, data portability, consent management, DPO, 72-hour breach notification, legitimate basis for processing.
- **CCPA/CPRA**: Right to know, delete, opt-out of sale, non-discrimination.
- **HIPAA**: PHI protection, minimum necessary rule, BAA with vendors.
- Implement data subject access requests (DSAR) as automated pipelines.

### Pipeline Considerations

- Identify PII at ingestion (bronze layer). Apply masking/hashing at silver layer. Gold layer should contain no raw PII unless explicitly justified. Log all PII access. Test that PII doesn't leak into logs, error messages, or analytics.

## 4. Data Cost Optimization

### Storage Costs

- Implement storage tiering: hot (SSD), warm (HDD), cold (archive/Glacier). Automate lifecycle policies. Delete or archive data past retention. Compress aggressively (Parquet/ORC reduce 5-10x vs JSON/CSV). Deduplicate redundant copies.

### Compute Costs

- Right-size clusters for workload. Use spot/preemptible instances for batch processing. Auto-scale based on queue depth. Schedule heavy jobs during off-peak. Shut down idle clusters.

### Query Costs (Pay-per-scan)

- Partition and cluster tables to minimize scan volume. Use column pruning (SELECT only needed columns). Cache frequent query results. Set per-user/per-team cost budgets. Review top-10 expensive queries weekly.

### Data Copy Reduction

- Eliminate unnecessary ETL copies. Use views or virtual datasets instead of materializing everything. Consolidate overlapping pipelines. Track data lineage to find redundant transformations.

### Monitoring

- Tag resources by team/project for cost allocation. Set alerts on spending anomalies. Review cost trends monthly. Calculate cost-per-query and cost-per-GB for benchmarking.

## 5. Data Migration Playbook

### Planning

- Inventory all source objects (tables, views, procedures, jobs). Map source-to-target schema transformations. Identify data volume and migration window. Define success criteria (row counts, checksums, business validation).

### Migration Strategies

- **Big bang**: Migrate everything in one window. Simpler but higher risk. Requires downtime.
- **Trickle/incremental**: Migrate in phases (by table, by domain, by priority). Lower risk, longer timeline. Requires dual-write or sync mechanism.
- **Parallel run**: Run old and new systems simultaneously. Compare outputs. Switch when confidence is high. Most expensive but safest.

### Execution Checklist

- [ ] Source and target schemas documented and mapped
- [ ] Data volume estimated for each object
- [ ] Migration scripts tested against production-sized data in staging
- [ ] Rollback plan documented and tested
- [ ] Validation queries prepared (row counts, checksums, sample records)
- [ ] Stakeholders informed of timeline and potential impact
- [ ] Post-migration validation completed before decommissioning source
- [ ] Monitoring configured for target system performance

### Common Pitfalls

- Underestimating data volume and migration time. Not testing with production-sized data. Missing dependent objects (views, triggers, stored procedures). Character encoding mismatches. Timezone handling differences. Foreign key constraint ordering during load.

## Master Checklist

- [ ] Quality dimensions defined for all critical datasets
- [ ] Volume and freshness baselines with anomaly alerting
- [ ] Quality gates block pipeline promotion on failure
- [ ] Data catalog with ownership, classification, and search
- [ ] Column-level lineage tracked for critical pipelines
- [ ] PII identified and protection applied at silver layer
- [ ] Compliance requirements mapped and automated (DSAR pipeline)
- [ ] Storage tiering and lifecycle policies active
- [ ] Cost allocation by team/project with monthly review
- [ ] Migration validation (row counts, checksums) before decommission
