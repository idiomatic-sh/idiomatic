//! One collection: the interface every item implements and its items in order.

pub(crate) mod components;
pub(crate) mod directed_walk;
pub(crate) mod distinct;
pub(crate) mod extend;
pub(crate) mod filter;
pub(crate) mod group;
pub(crate) mod hash_index;
#[cfg(test)]
pub(crate) mod interface;
pub(crate) mod lookup;
pub(crate) mod ordered_walk;
pub(crate) mod probe;
pub(crate) mod project;
pub(crate) mod reach;
pub(crate) mod scan_facts;
pub(crate) mod scan_node;
pub(crate) mod scan_roots;
