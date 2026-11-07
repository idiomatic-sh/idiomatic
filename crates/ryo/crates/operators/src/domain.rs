//! Portable meanings and algorithms for primitive operators.
pub(crate) mod append;
pub(crate) mod project;
pub(crate) mod set_rows;
pub(crate) mod span_between;
pub(crate) mod span_bounds;

pub(crate) mod equality_join;

pub(crate) mod aggregate;
pub(crate) mod capture;
pub(crate) mod contract;
pub(crate) mod ordered_existence;
pub(crate) mod ordered_match;
pub(crate) mod rank;
pub(crate) mod replace;
pub(crate) mod select;
pub(crate) mod source_matches;
pub(crate) mod words;

pub(crate) mod reduction;

pub mod cycles;
pub mod fits_rendered;
pub mod render_batch;
pub mod table_rows;
