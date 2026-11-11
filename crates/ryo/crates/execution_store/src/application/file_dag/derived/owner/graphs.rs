//! Bounded per-generation ownership of immutable compact graph access and successful memos.

pub mod handles;
#[cfg(test)]
mod nearest;
/// Graph-priority reclamation with live ownership checks.
pub(crate) mod reclamation;
pub(crate) mod reserve;
pub mod retained;
pub mod roots;
pub(crate) mod service;

#[cfg(test)]
mod fixtures;
