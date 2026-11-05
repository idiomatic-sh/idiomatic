//! One collection: the interface every item implements, its items in order, and the types, errors and contract it has.

pub(crate) mod antijoin;
pub(crate) mod axis;
pub(crate) mod capture;
pub(crate) mod closure;
pub(crate) mod cycles;
pub(crate) mod difference;
pub(crate) mod extend;
pub(crate) mod group_count;
pub(crate) mod group_digest;
pub(crate) mod interface;
pub(crate) mod intersection;
pub(crate) mod join;
pub(crate) mod project;
pub(crate) mod rank;
pub(crate) mod rows;
pub(crate) mod select;
pub(crate) mod union;
pub(crate) mod words;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;

pub(crate) mod fits_rendered;
pub(crate) mod ordered_match;
pub(crate) mod render_syntax;
pub(crate) mod replace;
pub(crate) mod source_matches;
pub(crate) mod span_between;
pub(crate) mod span_bounds;

pub(crate) mod ordered_dfs;

pub(crate) mod hosted;
