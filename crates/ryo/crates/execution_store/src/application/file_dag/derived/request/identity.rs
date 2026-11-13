//! Exact typed static request equality; evaluated only during request preparation.
//! Its hash counterpart is `prepare::identity::hash_request`; both must follow the same fields.
mod service;
pub(crate) mod types;
