#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

pub mod application;
pub mod domain;
pub mod inbound;
pub mod outbound;
pub mod spike;
pub mod codegen;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
