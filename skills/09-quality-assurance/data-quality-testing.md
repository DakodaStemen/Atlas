---
name: data-quality-testing
description: Patterns and implementation guide for data quality testing using Great Expectations (GX), Soda Core (SodaCL), and dbt tests. Covers expectation suites, checkpoints, SodaCL YAML syntax, custom checks, CI/CD integration, alerting on failures, and data contracts.
domain: data
category: quality
tags: [Great-Expectations, Soda, data-quality, validation, expectations, data-contracts, dbt, SodaCL, checkpoints, CI-CD]
triggers: ["data quality", "Great Expectations", "Soda Core", "SodaCL", "data validation", "expectation suite", "data contracts", "pipeline testing", "dbt tests", "data checks"]
---

# Data Quality Testing

## Tool Selection Framework

Three tools dominate the modern data stack for quality checks. They are not mutually exclusive—most production pipelines layer all three.

| | dbt tests | Great Expectations | Soda Core |
| --- | --- | --- | --- |
| Language | YAML + SQL | Python | YAML (SodaCL) |
| Primary use | Validate during transformation | Deep validation at any pipeline stage | Continuous monitoring and observability |
| Learning curve | Low (SQL teams) | High (Python required) | Low to moderate |
| Custom logic | SQL macros | Full Python | SQL expressions + Python UDFs |
| Data Docs | No | Yes (HTML) | Yes (functional) |
| Anomaly detection | No | Limited | Yes |
| Data contracts | No | Yes (suites as contracts) | Yes (experimental, v3.3+) |

**Use dbt tests** when you already use dbt for transformation and need not-null, uniqueness, accepted-values, and referential integrity checks defined alongside your models.

**Use Great Expectations** when you need cross-batch comparison, statistical distribution checks, rich HTML documentation, complex business rules expressed in Python, or validation at ingestion (pre-transformation).

**Use Soda** when your team is SQL-native, you need continuous scheduled monitoring, anomaly detection, or you want checks written by analysts without Python knowledge.

A common production layering: dbt for development-time transformation tests → GX for rigorous ingestion-point validation → Soda for scheduled production monitoring and alerting.

---

## Great Expectations

### Installation and Context

```bash
pip install great_expectations
```

```python
import great_expectations as gx

# File-system context (local/CI) or cloud context (GX Cloud)
context = gx.get_context()
```

The `DataContext` manages all configuration: datasources, expectation suites, checkpoints, and result stores. For production, point it at S3/GCS/Azure Blob so results persist across runs.

### Datasource Setup

```python
datasource = context.sources.add_or_update_sql(
    name="warehouse",
    connection_string="postgresql+psycopg2://user:pass@host:5432/db",
)

# Define a data asset (table or query)
asset = datasource.add_table_asset(name="orders", table_name="public.orders")

# Batch request — full table
batch_request = asset.build_batch_request()

# Batch request — partitioned by date
batch_request = asset.build_batch_request(
    options={"year": "2024", "month": "03"}
)
```

For Pandas (file-based):

```python
datasource = context.sources.add_pandas_filesystem(
    name="local_csv",
    base_directory="./data",
)
asset = datasource.add_csv_asset(name="customers", batching_regex=r"customers_(?P<date>\d{8})\.csv")
```

### Expectation Suites

An Expectation Suite is a named, versioned collection of expectations. Store it in Git and treat changes like code reviews.

Naming convention: `{domain}.{table}_{environment}` — e.g., `ecommerce.orders_staging`.

```python
suite = context.add_or_update_expectation_suite(
    expectation_suite_name="ecommerce.orders_staging"
)

validator = context.get_validator(
    batch_request=batch_request,
    expectation_suite_name="ecommerce.orders_staging",
)
```

### Built-in Expectations

#### Not null / missing values

```python
validator.expect_column_values_to_not_be_null("order_id")
validator.expect_column_values_to_not_be_null("customer_id")
```

#### Uniqueness

```python
validator.expect_column_values_to_be_unique("order_id")
```

#### Value set (accepted values)

```python
validator.expect_column_values_to_be_in_set(
    "currency", ["USD", "EUR", "GBP", "BRL"]
)
validator.expect_column_values_to_be_in_set(
    "status", ["pending", "shipped", "delivered", "cancelled"]
)
```

#### Numeric range

```python
validator.expect_column_values_to_be_between("amount", min_value=0, max_value=10000)
validator.expect_column_min_to_be_between("amount", min_value=0)
```

#### Row count

```python
validator.expect_table_row_count_to_be_between(min_value=1000, max_value=None)
# Dynamic threshold: ±10% of rolling average via evaluation parameters
validator.expect_table_row_count_to_be_between(
    min_value={"$PARAMETER": "row_count_lower"},
    max_value={"$PARAMETER": "row_count_upper"},
)
```

#### Regex pattern

```python
validator.expect_column_values_to_match_regex("email", r"^[^@]+@[^@]+\.[^@]+$")
validator.expect_column_values_to_match_regex("order_id", r"^ORD-\d{8}$")
```

#### String length

```python
validator.expect_column_value_lengths_to_be_between("country_code", min_value=2, max_value=3)
```

#### Column existence and type

```python
validator.expect_column_to_exist("order_id")
validator.expect_column_values_to_be_of_type("amount", "FLOAT")
```

#### Cross-column / business rules

```python
# delivered_at must be >= shipped_at
validator.expect_column_pair_values_A_to_be_greater_than_or_equal_to_B(
    column_A="delivered_at", column_B="shipped_at"
)
```

#### Distribution / statistical

```python
validator.expect_column_mean_to_be_between("amount", min_value=50, max_value=500)
validator.expect_column_stdev_to_be_between("amount", min_value=0, max_value=200)
validator.expect_column_quantile_values_to_be_between(
    "amount",
    quantile_ranges={
        "quantiles": [0.25, 0.5, 0.75],
        "value_ranges": [[5, 100], [20, 300], [50, 800]],
    },
)
```

#### Save the suite after building it

```python
validator.save_expectation_suite(discard_failed_expectations=False)
```

### Custom Expectations

When built-ins don't cover domain logic, subclass `ColumnMapExpectation`, `ColumnAggregateExpectation`, or `BatchExpectation`.

```python
from great_expectations.expectations.expectation import ColumnMapExpectation

class ExpectColumnValuesToBeValidIsbn(ColumnMapExpectation):
    """Validates ISBN-13 check digit."""
    map_metric = "column_values.custom.valid_isbn"
    examples = [...]

    def validate_configuration(self, configuration):
        super().validate_configuration(configuration)

    # Register the metric using @column_condition_partial decorator
```

For simpler SQL-expressible rules, use `expect_column_values_to_match_regex` or build a `QueryExpectation` that runs raw SQL and asserts the result.

### Checkpoints

A Checkpoint is the runnable artifact: it wires one or more (batch_request, suite) pairs together, runs validation, and fires actions.

```python
checkpoint = context.add_or_update_checkpoint(
    name="orders_staging_checkpoint",
    run_name_template="%Y%m%d-%H%M-orders-staging",
    validations=[
        {
            "batch_request": batch_request,
            "expectation_suite_name": "ecommerce.orders_staging",
            "action_list": [
                {
                    "name": "store_validation_result",
                    "action": {"class_name": "StoreValidationResultAction"},
                },
                {
                    "name": "update_data_docs",
                    "action": {"class_name": "UpdateDataDocsAction"},
                },
                {
                    "name": "send_slack_notification_on_failure",
                    "action": {
                        "class_name": "SlackNotificationAction",
                        "slack_webhook": "${SLACK_WEBHOOK_URL}",
                        "notify_on": "failure",
                        "renderer": {
                            "module_name": "great_expectations.render.renderer.slack_renderer",
                            "class_name": "SlackRenderer",
                        },
                    },
                },
            ],
        }
    ],
    evaluation_parameters={
        "row_count_lower": 900,
        "row_count_upper": 1100,
    },
)
```

Running a checkpoint:

```python
result = context.run_checkpoint(checkpoint_name="orders_staging_checkpoint")

if not result.success:
    raise ValueError("Data quality checkpoint failed — see Data Docs for details.")
```

### Validation Actions Reference

| Action class | Effect |
| --- | --- |
| `StoreValidationResultAction` | Persists results to configured store (local, S3, GCS) |
| `StoreEvaluationParametersAction` | Saves computed parameters for reuse across runs |
| `UpdateDataDocsAction` | Rebuilds HTML Data Docs site |
| `SlackNotificationAction` | Posts pass/fail summary to Slack channel |
| `EmailAction` | Sends email on failure (or always) |
| `SNSNotificationAction` | Publishes to AWS SNS topic |
| Custom `ValidationAction` subclass | Any Python logic: PagerDuty, webhook, quarantine table write |

### Data Docs

Data Docs are auto-generated HTML pages showing every expectation, its observed value, and pass/fail status per run. Publish them to S3 or GCS for team-wide visibility.

```python
context.build_data_docs()
context.open_data_docs()  # opens browser locally
```

For S3 publishing, configure a `site` in `great_expectations.yml`:

```yaml
data_docs_sites:
  s3_site:
    class_name: SiteBuilder
    store_backend:
      class_name: TupleS3StoreBackend
      bucket: my-data-docs-bucket
      prefix: gx/
    site_index_builder:
      class_name: DefaultSiteIndexBuilder
```

### Evaluation Parameters (Dynamic Thresholds)

Hard-coded thresholds become brittle. Use evaluation parameters to pass runtime values — rolling averages, yesterday's row count, or business-defined SLAs — into expectations at run time.

```python
# Store a metric from a previous run
validator.expect_table_row_count_to_be_between(
    min_value={"$PARAMETER": "prev_row_count * 0.9"},
    max_value={"$PARAMETER": "prev_row_count * 1.1"},
)
```

### CI/CD Integration

Run a checkpoint as a pipeline step; fail the step on non-success.

#### GitHub Actions example

```yaml
- name: Run GX data quality checkpoint
  run: |
    python -c "
    import great_expectations as gx
    ctx = gx.get_context()
    result = ctx.run_checkpoint('orders_staging_checkpoint')
    if not result.success:
        raise SystemExit('GX checkpoint failed')
    "
  env:
    SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK_URL }}
```

#### Airflow

```python
from great_expectations_provider.operators.great_expectations import GreatExpectationsOperator

validate_orders = GreatExpectationsOperator(
    task_id="validate_orders",
    data_context_root_dir="/opt/airflow/gx",
    checkpoint_name="orders_staging_checkpoint",
    fail_task_on_validation_failure=True,
)
```

---

## Soda Core

### Installation

```bash
pip install soda-core-postgres     # or soda-core-snowflake, soda-core-bigquery, etc.
```

### Datasource Configuration (`configuration.yml`)

```yaml
data_source my_warehouse:
  type: postgres
  host: ${PGHOST}
  port: "5432"
  username: ${PGUSER}
  password: ${PGPASSWORD}
  database: analytics
  schema: public
```

For Snowflake:

```yaml
data_source snowflake_prod:
  type: snowflake
  username: ${SNOWFLAKE_USER}
  password: ${SNOWFLAKE_PASSWORD}
  account: myaccount.us-east-1
  database: ANALYTICS
  schema: PUBLIC
  warehouse: COMPUTE_WH
  role: SODA_ROLE
```

### SodaCL Check Syntax (`checks.yml`)

The basic structure is `checks for <table_name>:` followed by a list of check expressions.

#### Row count checks

```yaml
checks for orders:
  - row_count > 0
  - row_count between 1000 and 500000
```

#### Missing values

```yaml
checks for orders:
  - missing_count(order_id) = 0
  - missing_count(customer_id) = 0:
      name: No missing customer IDs
  - missing_percent(shipped_at) < 5%
```

#### Uniqueness / duplicates

```yaml
checks for orders:
  - duplicate_count(order_id) = 0:
      name: Order IDs must be unique
  - duplicate_count(email) = 0
```

#### Value validity

```yaml
checks for orders:
  - invalid_count(email) = 0:
      name: Valid email format
      valid format: email
  - invalid_count(status) = 0:
      valid values: [pending, shipped, delivered, cancelled]
  - invalid_percent(phone) < 10%:
      valid format: phone number
  - invalid_count(amount) = 0:
      valid min: 0
      valid max: 99999.99
```

#### Numeric range and aggregates

```yaml
checks for orders:
  - min(amount) >= 0
  - max(amount) < 100000
  - avg(amount) between 50 and 5000
```

#### Schema checks

```yaml
checks for orders:
  - schema:
      name: Schema must match contract
      fail:
        when required column missing: [order_id, customer_id, amount, status, created_at]
        when wrong column type:
          order_id: varchar
          amount: numeric
        when wrong column index:
          order_id: 0
```

#### Freshness

```yaml
checks for orders:
  - freshness(created_at) < 2h:
      name: Orders table updated within last 2 hours
  - freshness(updated_at) < 24h
```

#### Warn vs. fail thresholds

```yaml
checks for orders:
  - missing_percent(email):
      warn: when > 5%
      fail: when > 20%
  - row_count:
      warn: when < 500
      fail: when < 100
```

#### Custom SQL check

```yaml
checks for orders:
  - failed rows:
      name: delivered_at must be after shipped_at
      fail condition: delivered_at < shipped_at AND delivered_at IS NOT NULL
  - failed rows:
      name: Amount must be positive for completed orders
      fail condition: status = 'delivered' AND amount <= 0
```

#### Reference check (cross-table)

```yaml
checks for orders:
  - values in (customer_id) must exist in customers (id):
      name: All order customer_ids exist in customers table
```

#### User-defined metric (custom SQL aggregate)

```yaml
checks for orders:
  - revenue_last_7d > 10000:
      revenue_last_7d query: |
        SELECT SUM(amount)
        FROM orders
        WHERE created_at >= NOW() - INTERVAL '7 days'
```

### Running Scans

#### Test connection

```bash
soda test-connection -d my_warehouse -c configuration.yml
```

#### Execute scan

```bash
soda scan -d my_warehouse -c configuration.yml checks.yml
```

#### Python API

```python
from soda.scan import Scan

scan = Scan()
scan.set_scan_definition_name("orders_daily")
scan.set_data_source_name("my_warehouse")
scan.add_configuration_yaml_file(file_path="configuration.yml")
scan.add_sodacl_yaml_file(file_path="checks.yml")
scan.set_verbose(True)
scan.execute()

if scan.has_check_failures():
    raise SystemExit("Soda scan failed — data quality issues detected.")
```

### CI/CD Integration (Soda)

#### GitHub Actions

```yaml
- name: Soda data quality scan
  run: soda scan -d my_warehouse -c configuration.yml checks/orders.yml
  env:
    PGHOST: ${{ secrets.PGHOST }}
    PGUSER: ${{ secrets.PGUSER }}
    PGPASSWORD: ${{ secrets.PGPASSWORD }}
```

Exit code is non-zero on failures, so the CI step fails automatically — no extra logic needed.

#### Airflow (BashOperator)

```python
soda_scan = BashOperator(
    task_id="soda_scan_orders",
    bash_command="soda scan -d my_warehouse -c /opt/airflow/soda/config.yml /opt/airflow/soda/checks/orders.yml",
)
```

#### Airflow (PythonOperator with programmatic scan)

```python
def run_soda_scan(**context):
    from soda.scan import Scan
    scan = Scan()
    scan.set_data_source_name("my_warehouse")
    scan.add_configuration_yaml_file("/opt/airflow/soda/config.yml")
    scan.add_sodacl_yaml_file("/opt/airflow/soda/checks/orders.yml")
    scan.execute()
    if scan.has_check_failures():
        raise AirflowFailException("Soda scan detected data quality failures.")
```

### Alerting

Soda Cloud (the managed layer) provides built-in alerting to Slack, PagerDuty, and email. For Soda Core (open-source), wire alerting through the Python API:

```python
scan.execute()
if scan.has_check_failures():
    import requests
    requests.post(SLACK_WEBHOOK, json={
        "text": f"Data quality failure in {scan.scan_definition_name}: "
                f"{scan.get_checks_fail_count()} checks failed."
    })
```

---

## dbt Tests

### Built-in Generic Tests

Defined in `schema.yml` alongside model definitions:

```yaml
models:
  - name: orders
    columns:
      - name: order_id
        tests:
          - not_null
          - unique
      - name: status
        tests:
          - not_null
          - accepted_values:
              values: [pending, shipped, delivered, cancelled]
      - name: customer_id
        tests:
          - not_null
          - relationships:
              to: ref('customers')
              field: id
```

### Singular (Custom SQL) Tests

```sql
-- tests/assert_positive_amounts.sql
SELECT order_id
FROM {{ ref('orders') }}
WHERE amount < 0
```

Any rows returned = test failure.

### dbt-expectations (GX-style checks in dbt)

```yaml
- name: amount
  tests:
    - dbt_expectations.expect_column_values_to_be_between:
        min_value: 0
        max_value: 99999
    - dbt_expectations.expect_column_values_to_match_regex:
        regex: "^[0-9]+(\\.[0-9]{1,2})?$"
```

### When dbt Tests Are Sufficient

- Checking not_null and unique on model output columns
- Referential integrity between models
- Accepted value sets that are stable
- Tests that run as part of `dbt build` in CI

dbt tests run as SQL queries against the warehouse, so they are fast and require no extra infrastructure. Their limitation is they only run on dbt models (post-transformation) and have limited support for cross-batch, statistical, or freshness checks.

---

## Data Contracts Pattern

A data contract is a formal agreement between a data producer (e.g., an upstream team or system) and consumers, specifying schema, semantics, and SLAs. Both GX and Soda support encoding contracts as machine-executable checks.

### Contract as GX Expectation Suite

Define the producer's obligations as an expectation suite and version it in Git:

```python
# contracts/orders_v1_contract.py
suite = context.add_or_update_expectation_suite("contracts.orders_v1")
validator = context.get_validator(batch_request=br, expectation_suite_name="contracts.orders_v1")

# Schema obligations
validator.expect_column_to_exist("order_id")
validator.expect_column_to_exist("amount")
validator.expect_column_values_to_be_of_type("amount", "FLOAT")

# Semantic obligations
validator.expect_column_values_to_not_be_null("order_id")
validator.expect_column_values_to_be_unique("order_id")
validator.expect_column_values_to_be_between("amount", min_value=0)

# SLA obligations
validator.expect_table_row_count_to_be_between(min_value=1)
validator.save_expectation_suite()
```

Any breaking schema change — dropped column, type change — fails the checkpoint before downstream jobs consume bad data.

### Contract as SodaCL Checks

```yaml
# contracts/orders_v1.yml
checks for orders:
  - schema:
      fail:
        when required column missing: [order_id, customer_id, amount, status, created_at]
        when wrong column type:
          order_id: varchar
          amount: numeric
  - missing_count(order_id) = 0
  - duplicate_count(order_id) = 0
  - min(amount) >= 0
  - freshness(created_at) < 4h:
      name: SLA — orders table must be refreshed within 4 hours
```

Version this file in Git, owned by the producing team. Consumer teams reference it for integration tests.

### Contract Enforcement in CI

Run contract checks on every merge to the producer's pipeline:

```yaml
# .github/workflows/contract_check.yml
- name: Validate data contract
  run: soda scan -d prod_warehouse -c config.yml contracts/orders_v1.yml
```

Breaking the contract fails the pipeline before the change ships.

---

## Alerting on Data Quality Failures

### Severity Levels

Map check outcomes to response urgency:

| Severity | Trigger | Response |
| --- | --- | --- |
| Warning | Row count 10% below average, missing rate 1–5% | Slack notification to data team |
| Failure | Missing primary keys, schema change, SLA breach | PagerDuty alert + pipeline halt |
| Critical | Zero rows, wrong table | Immediate page + block downstream |

### Soda warn/fail pattern

```yaml
checks for orders:
  - row_count:
      warn: when < 900
      fail: when < 500
  - missing_percent(email):
      warn: when > 2%
      fail: when > 10%
```

### GX action-based routing

```python
action_list = [
    {
        "name": "slack_on_failure",
        "action": {
            "class_name": "SlackNotificationAction",
            "slack_webhook": "${SLACK_WEBHOOK_URL}",
            "notify_on": "failure",
        },
    },
    {
        "name": "pagerduty_on_critical",
        "action": {
            "class_name": "PagerdutyAlertAction",  # custom action subclass
            "api_key": "${PAGERDUTY_KEY}",
            "notify_on": "failure",
        },
    },
]
```

### Pipeline halting pattern

Never let bad data flow silently downstream. In Airflow, set upstream quality tasks to trigger downstream tasks only on success:

```python
validate_orders >> transform_orders >> load_to_mart
```

If `validate_orders` fails, the entire DAG branch stops. In dbt, use `dbt build` which runs models and tests together and stops on test failure.

---

## Organizing Checks Across Environments

### GX store configuration per environment

Use separate stores and suite prefixes per environment. In CI, point `GX_CLOUD_ORGANIZATION_ID` or the local config to a staging store so prod expectations are never contaminated.

```text
expectations/
  ecommerce.orders_dev.json
  ecommerce.orders_staging.json
  ecommerce.orders_prod.json
```

### Soda checks file structure

```text
soda/
  config/
    configuration_dev.yml
    configuration_prod.yml
  checks/
    ingestion/
      raw_orders.yml
      raw_customers.yml
    staging/
      stg_orders.yml
    marts/
      fct_orders.yml
  contracts/
    orders_v1.yml
    customers_v1.yml
```

Run the correct config file per environment:

```bash
soda scan -d prod_warehouse -c soda/config/configuration_prod.yml soda/checks/marts/fct_orders.yml
```

---

## Common Pitfalls

**Hard-coded thresholds go stale.** Row counts change as business grows. Use rolling averages, evaluation parameters, or warn/fail bands instead of absolute numbers.

**Full table scans on large tables.** Validate the most recent partition only; use SQL `WHERE created_at >= CURRENT_DATE - 1` or GX batch partitioning rather than scanning everything.

**Too many expectations on day one.** Start with a small, high-value set (not_null on PKs, row count bounds, schema checks) and expand as you learn the data's failure modes.

**Silent failures.** A check that runs but whose failure is ignored is worse than no check. Wire every check to a pipeline gate or an alert that someone actually reads.

**Expectations owned by nobody.** Assign data owners to suites and contract files. Put them in Git with CODEOWNERS entries so schema changes require review.

**Tests that pass by design.** Never edit an expectation to match observed bad data — fix the source or the pipeline. An expectation is a claim about what the data should be, not a description of what it currently is.
