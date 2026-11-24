//! Portable meanings and algorithms for primitive operators.
pub(crate) mod append;
pub(crate) mod project;
pub mod set_rows;
pub mod span_between;
pub mod span_bounds;

pub mod equality_join;

pub mod aggregate;
pub mod capture;
pub mod contract;
pub mod distinct_rows;
pub mod ordered_match;
pub mod rank;
pub mod replace;
pub mod select;
pub mod source_matches;
pub mod words;

pub mod reduction;

pub mod axis;
pub mod closure;
pub mod comparing;
pub mod cycles;
pub mod fits_rendered;
pub mod follow;
pub mod functions;
pub(crate) mod kept_room;
pub(crate) mod numeric_values;
pub(crate) mod operands;
pub mod ordered_dfs;
pub mod ordered_groups;
pub mod patterns;
pub mod physical_plan;
pub mod render_batch;
pub mod table_rows;
pub mod table_scratch;
pub mod table_texts;
pub mod tree_cells;
pub(crate) mod worklist;

pub mod row_predicates;

pub(crate) mod entity_queries;
