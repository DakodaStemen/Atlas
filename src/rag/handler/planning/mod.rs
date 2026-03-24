//! Task planning and decomposition: plan_task generates structured execution plans
//! with substeps, success criteria, and constraints. Used by the control loop to
//! break complex objectives into verifiable units of work.

mod tools;

pub use tools::plan_task_impl;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single step in a plan.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PlanStep {
    /// Step number (1-indexed).
    pub step: u32,
    /// What to do.
    pub action: String,
    /// Tool to use (if applicable).
    #[serde(default)]
    pub tool: Option<String>,
    /// How to verify this step succeeded.
    pub success_criterion: String,
}

/// Output of plan_task.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    /// Original objective.
    pub objective: String,
    /// Ordered steps.
    pub steps: Vec<PlanStep>,
    /// Constraints that apply to all steps.
    pub constraints: Vec<String>,
    /// Overall success criteria for the entire plan.
    pub success_criteria: Vec<String>,
    /// Estimated complexity: "low", "medium", "high".
    pub complexity: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// PlanTaskParams.
pub struct PlanTaskParams {
    /// The objective to decompose into a plan.
    pub objective: String,
    /// Optional constraints (e.g. "no new dependencies", "must pass clippy").
    #[serde(default)]
    pub constraints: Option<Vec<String>>,
    /// Optional max number of steps. Default 10.
    #[serde(default)]
    pub max_steps: Option<u32>,
}
