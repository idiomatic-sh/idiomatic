#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

#[macro_use]
pub mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
