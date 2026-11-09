//! Pure request declarations and executable-program contracts.
pub mod comparing;
pub mod coverage;
pub mod derived_views;
pub mod executable_program;
pub mod features;
pub mod file_program;
pub mod kind_check;
pub mod kind_sets;
pub mod matching;
pub(crate) mod numeric_values;
pub(crate) mod operands;
pub mod operating;
pub mod operation_schema;
pub mod operator;
pub mod placement;
pub mod prepared_request;
pub mod self_join_recogniser;
pub mod shared_walk;
pub mod table_cells;
pub(crate) mod table_dedup;
pub mod table_groups;
pub mod table_model;
pub mod table_reach;
pub mod table_scratch;
pub mod table_texts;

pub mod trust;

pub(crate) mod describing;
pub mod finish;
pub mod frontiers;
pub(crate) mod functions;
pub mod hosted;
pub mod replay;
pub mod rules;
pub mod step_classes;
pub mod stored_graph;
pub(crate) mod text_gate;
pub mod tree_walk;

pub mod spelling;

pub mod completion;
pub mod contributor_rows;
pub mod frontier_coverage;
pub mod output_coverage;

pub mod source_partitions;

pub mod frontier_capture;
pub mod readiness;
pub mod row_emission;
pub mod rule_intent;
