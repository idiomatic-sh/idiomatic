//! One positive and one negative compile control per acquiring entry, and one forgery control
//! per authority a consumer may hold. Every source is inserted into the unmodified derived root.

pub(super) struct Negative {
    pub(super) source: &'static str,
    pub(super) code: &'static str,
    pub(super) first: &'static str,
    pub(super) second: &'static str,
}
pub(super) struct Acquire {
    pub(super) positive: &'static str,
    pub(super) negative: Negative,
}
const fn refuses(
    source: &'static str,
    code: &'static str,
    first: &'static str,
    second: &'static str,
) -> Negative {
    Negative {
        source,
        code,
        first,
        second,
    }
}

/// Reads of leased views and handles: the accessor is nameable from a consumer.
pub(super) fn read(path: &str, owner_type: &str, method: &str) -> String {
    format!(
        "mod census_read_{owner_type}_{method} {{ use {path}::{owner_type}; const _: () = {{ let _ = {owner_type}::{method}; }}; }}\n"
    )
}

pub(super) mod admission;
pub(super) mod graph;
pub(super) mod authority;
