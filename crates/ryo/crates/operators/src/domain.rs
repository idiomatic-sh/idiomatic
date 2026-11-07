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
pub(crate) mod replace;
pub(crate) mod select;
pub mod source_matches;
pub mod words;

pub mod reduction;
