//! Pure request declarations and executable-program contracts.
pub mod contributor_rows;
pub mod coverage;
pub mod derived_views;
pub mod executable_program;
pub mod features;
pub mod kind_check;
pub mod kind_sets;
pub mod observation_schedule;
pub mod operating;
pub mod operation_schema;
pub mod operator;
pub mod placement;
pub mod prepared_request;
pub mod table_model;

pub mod trust;

pub(crate) mod describing;
pub mod finish;
pub mod hosted;
pub mod rules;
pub mod stored_graph;
pub mod tree_walk;

pub mod spelling;

pub mod completion;
pub mod frontier_coverage;
pub mod output_coverage;

pub mod source_partitions;

pub mod frontier_capture;
pub mod row_emission;
pub mod rule_intent;
