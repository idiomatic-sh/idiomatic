//! A compiled project emit defers at file scope while real file frontiers remain available.
mod applicability;
mod axis;
#[cfg(test)]
pub(crate) mod consumers;
#[cfg(test)]
mod declared_empty;
#[cfg(test)]
pub(crate) mod fixtures;
mod grouping;
mod members;
mod merged;
mod named_selection;
pub(crate) mod negative;
mod population_reads;
mod restoration;
mod rewrites;
mod skipped_frontier;
pub(crate) mod source_facts;
mod text_identity;
mod union;
