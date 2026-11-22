//! The `finish_plans` concept: the plan the finish runs, the contract between the lowering and the executor. A plan is a
//! list of steps, each reading only steps before it, shared by every rule that lowers to an equal step; each rule names
//! the step it emits. The format is fixed by the plans of record (`build/vec/runs8/plan8.json`): one step kind per
//! operator of the step set, and the marks the planner passes add.

pub mod arguments;
pub(crate) mod constants;
pub mod expressions;
pub mod service;
pub mod types;

