#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

//! Ryo's sole execution owner; public callers enter through its inbound use case.
pub mod application;
mod domain;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
