#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

pub mod application;
pub mod outbound;
/// Spike (ryo-r6ux): the facts export of the engine bake-off.
pub(crate) mod spike_export;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
