#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]


pub mod edge;
mod languages;
pub mod outcome;
pub mod output;
pub mod resources;
pub mod timings;

