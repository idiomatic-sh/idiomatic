//! Pure request declarations and executable-program contracts.
pub(crate) mod comparing;
pub(crate) mod coverage;
pub(crate) mod derived_views;
pub(crate) mod executable_program;
pub(crate) mod features;
pub(crate) mod file_program;
pub(crate) mod kind_check;
pub(crate) mod kind_sets;
pub(crate) mod lowering;
pub(crate) mod matching;
pub(crate) mod native_positions;
pub(crate) mod numeric_values;
pub(crate) mod operands;
pub(crate) mod operating;
pub(crate) mod operation_schema;
pub(crate) mod operator;
pub(crate) mod self_join_recogniser;
pub(crate) mod placement;
pub(crate) mod positions;
pub(crate) mod prepared_request;
pub(crate) mod selections;
pub(crate) mod shared_walk;
pub(crate) mod stored_selection;
pub(crate) mod table_cells;
pub(crate) mod table_dedup;
pub(crate) mod table_groups;
pub(crate) mod table_model;
pub(crate) mod table_reach;
pub(crate) mod table_scratch;
pub(crate) mod table_texts;

pub(crate) mod tree_join;
pub(crate) mod trust;
pub(crate) mod walks;

pub(crate) mod column_kernel;
pub(crate) mod crossing;
pub(crate) mod describing;
pub(crate) mod finish;
pub(crate) mod frontiers;
pub(crate) mod functions;
pub(crate) mod hosted;
pub(crate) mod replay;
pub(crate) mod rules;
pub(crate) mod step_classes;
pub(crate) mod stored_graph;
pub(crate) mod text_gate;
pub(crate) mod tree_walk;

pub(crate) mod spelling;

pub(crate) mod completion;
pub(crate) mod contributor_rows;
pub(crate) mod frontier_coverage;
pub(crate) mod output_coverage;

pub(crate) mod source_partitions;

pub(crate) mod readiness;
