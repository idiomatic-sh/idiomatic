//! Cohesive application operations, composed by use cases.

pub mod cross_file;
pub mod file_analysis;
pub(crate) mod project_context;
pub mod rule_compilation;

pub(crate) mod file_dag;

pub(crate) mod job_execution;
pub(crate) mod repository_listing;
