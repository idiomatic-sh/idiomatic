//! One private runtime owner for admission, scheduling, capability issuance and release.
#[cfg(test)]
mod lifecycle;
pub(in crate::application::file_dag) mod service;
pub(in crate::application::file_dag) mod types;
