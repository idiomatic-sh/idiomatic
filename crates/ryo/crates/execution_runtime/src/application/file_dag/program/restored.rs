//! Engine-issued lazy restored input: immutable bytes retain their validated
//! root, coverage, partition set and owning generation until actual demand.
pub(crate) mod service;
pub(crate) mod types;
