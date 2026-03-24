---
name: r-data-science
description: Expert guidance for Data Science in R with Tidyverse, Tidymodels, renv, and Quarto.
domain: research
tags: [r, tidyverse, ggplot2, dplyr, tidymodels, renv, quarto, data-science]
triggers: r data science, tidyverse, ggplot2, tidymodels, renv, quarto, rstats
---

# R Data Science

Expert-level patterns for data manipulation, visualization, machine learning, and reproducible reporting using the R programming language and its modern ecosystem.

## When to Use

- Performing exploratory data analysis (EDA) and data cleaning.
- Creating high-quality, publication-ready visualizations with `ggplot2`.
- Building and deploying machine learning models with `tidymodels` and `vetiver`.
- Developing reproducible research documents, dashboards, and websites with `Quarto`.
- Managing project-specific package dependencies with `renv`.
- Handling large-scale data processing pipelines with `targets`.

## Core Patterns

### Data Manipulation & Visualization (Tidyverse)

- **Native Pipe (`|>`):** Use the native pipe for chaining operations (R 4.1+). It is more performant and reduces external dependencies compared to `%>%`.
- **dplyr & tidyr:** Use for intuitive data munging (filtering, selecting, mutating, pivoting).
- **ggplot2:** Follow the "Grammar of Graphics" to build layers. Use `patchwork` for combining multiple plots and `ggtext` for rich text formatting.
- **Functional Programming (purrr):** Use `map()` functions instead of `for` loops for cleaner, more maintainable code.

### Machine Learning (Tidymodels)

- **Workflows:** Use the `workflows` package to bundle preprocessing (recipes) and models (parsnip) together to prevent data leakage.
- **Recipes:** Define data preprocessing steps (scaling, encoding, imputation) separately from model training.
- **Yardstick:** Use for robust model evaluation and metric calculation.
- **Vetiver:** Use for versioning and deploying models as APIs (via `plumber`) or into production environments.

### Reproducibility & Pipeline Management

- **renv:** Always initialize your project with `renv::init()` to create a `renv.lock` file that captures exact package versions.
- **targets:** Use for long-running or complex pipelines to ensure only modified steps are re-run, saving time and ensuring consistency.
- **Project Structure:** Follow a consistent folder structure: `data/`, `R/` (functions), `output/` (plots/models), and `scripts/`.

## Critical Rules / Gotchas

- **Avoid Global Environment Pollution:** Keep your workspace clean. Use functions for reusable logic and source them from the `R/` directory.
- **Vectorized Operations:** Always prefer vectorized functions over explicit loops for performance.
- **Data Leakage:** Be extremely careful not to include information from the test set in your training/preprocessing phase. Tidymodels workflows help prevent this.
- **Quarto Over R Markdown:** For new projects, use Quarto (`.qmd`) as it is the modern, multi-language successor to R Markdown.
- **Native Pipe Limitations:** The native pipe `|>` is not a 1:1 replacement for `%>%` in all cases (e.g., the `.` placeholder behaves differently); use `\(x)` for anonymous functions if needed.

## Key Commands / APIs

- `renv::init()`: Initialize a project-local environment.
- `renv::snapshot()`: Save current package versions to the lockfile.
- `install.packages("pak")`: Install the `pak` manager for faster package installations.
- `library(tidyverse)`: Load the core Tidyverse packages.
- `quarto render doc.qmd`: Render a Quarto document to its output format.
- `tar_make()`: Run a `targets` pipeline.
- `vetiver_pin_write(board, model)`: Version and store a model.

## References

- [R for Data Science (2e)](https://r4ds.hadley.nz/)
- [Tidyverse Documentation](https://www.tidyverse.org/)
- [Tidymodels Documentation](https://www.tidymodels.org/)
- [Quarto Documentation](https://quarto.org/)
- [renv Documentation](https://rstudio.github.io/renv/)
- [targets R Package User Guide](https://books.ropensci.org/targets/)
- [R-bloggers (Community Aggregator)](https://www.r-bloggers.com/)
