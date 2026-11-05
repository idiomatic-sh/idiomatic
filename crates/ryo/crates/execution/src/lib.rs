//! Ryo's sole execution owner; public callers enter through its inbound use case.
mod application;
mod domain;
pub mod inbound;
mod outbound;
