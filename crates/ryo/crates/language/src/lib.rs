#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]



#[macro_use]
pub mod domain;
/// Engine-owned preparation and completion of one file generation.
pub mod application;
pub mod inbound;

pub mod cases;
