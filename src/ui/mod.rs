//! UI utilities: design tokens (colors, typography) for CLI output.
//!
//! Re-exports the `theme` submodule, which parses design data (e.g. `docs/design/data/Colors.csv`,
//! `Typography.csv`) and provides ANSI color helpers and fallbacks when CSVs are missing.

pub mod theme;
