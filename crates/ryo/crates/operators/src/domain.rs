//! Portable meanings and algorithms for primitive operators.
pub(crate) mod append;
pub(crate) mod project;
pub mod set_rows;
pub(crate) mod span_between;
pub(crate) mod span_bounds;

pub mod equality_join;

pub(crate) mod aggregate;
pub mod capture;
pub mod contract;
pub mod ordered_existence;
pub mod ordered_match;
pub(crate) mod rank;
pub mod replace;
pub mod select;
pub mod source_matches;
pub mod words;

pub mod reduction;

pub mod axis;
pub mod closure;
pub mod comparing;
pub mod cycles;
pub mod evaluation;
pub mod fits_rendered;
pub mod functions;
pub(crate) mod numeric_values;
pub(crate) mod operands;
pub mod ordered_dfs;
pub mod patterns;
pub mod physical_plan;
pub mod render_batch;
pub mod table_rows;
pub mod table_scratch;
pub mod table_texts;
pub mod tree_cells;
