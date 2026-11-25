//! The `sets` part of the tables: a comparison that holds a node's kind or field to one of a set of literals, however the
//! plan writes the set, as one pin of the cell for each literal: the union of the literals' buckets.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

