//! Complete typed-key acquisition and retention share the common resource owner.
pub(in crate::application::file_dag::derived::owner) mod service;
#[cfg(test)]
mod test;
pub(in crate::application::file_dag::derived::owner) mod types;
