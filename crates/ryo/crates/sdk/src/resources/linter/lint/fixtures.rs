//! The test data of `linter lint`, compiled only for tests and under the `fixtures` feature.
#![cfg(any(test, feature = "fixtures"))]

#[cfg(test)]
pub(crate) mod outcome;
#[cfg(test)]
pub(crate) mod queries;
#[cfg(test)]
pub(crate) mod capability_provider;
#[cfg(test)]
pub(crate) mod rules;
#[cfg(test)]
pub(crate) mod directory;
#[cfg(test)]
pub(crate) mod observations;
#[cfg(test)]
pub(crate) mod shared_calls;
#[cfg(test)]
pub(crate) mod capability_cache;
#[cfg(test)]
pub(crate) mod permission;
#[cfg(test)]
pub(crate) mod hosted_text_identity;
#[cfg(test)]
pub(crate) mod zero_finding_producer;
#[cfg(test)]
pub(crate) mod re_exported_publication;
#[cfg(test)]
pub(crate) mod refused_and_empty_producers;
#[cfg(test)]
pub(crate) mod producer_without_query;
#[cfg(test)]
pub(crate) mod unbound_export;
