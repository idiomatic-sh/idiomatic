//! The engine's domain, one module per concept.

pub mod code;
pub mod counting;
pub mod encoding;
#[cfg(test)]
mod executing;
pub mod findings;
pub mod indexes;
pub mod lowered;
pub mod lowering;
#[cfg(test)]
pub mod memos;
#[cfg(any(test, feature = "fixtures"))]
pub mod made_tables;
#[cfg(test)]
mod needs;
pub mod packs;
pub mod passes;
pub mod programs;
pub mod scratch;
pub mod stages;
pub mod statistics;
pub mod steps;
