//! The `language` domain's rules, one module per concept. It is public because the catalog of
//! relations is the vocabulary every language writes in.

#[macro_use]
pub mod answers;
pub mod completion;
pub mod elements;
pub mod expansions;
pub mod files;
pub mod kind_index;
#[macro_use]
pub mod kinds;
pub mod reasons;
pub mod relations;
pub mod symbols;
pub mod targets;
pub mod terms;
pub mod trees;
pub mod words;
