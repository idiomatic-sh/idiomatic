//! Opening a scope's relation owner: the derived access an edge hands down is turned into the generation's derived-view
//! owner here, inside the derived concept, so the relation owner holds that owner and never the access.
pub(crate) mod permit;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
