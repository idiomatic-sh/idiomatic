//! The `language` domain's rules, one module per concept. It is public because the catalog of
//! relations is the vocabulary every language writes in.

#[macro_use]
pub mod answers;
pub mod builders;
pub mod completion;
pub mod decoded;
pub mod diagnostics;
pub mod elements;
pub mod expansions;
pub mod files;
pub mod kind_index;
pub mod location;
pub mod names;
pub mod notes;
pub mod project_context;
pub mod reasons;
pub mod relation_masks;
pub mod semantics;
pub mod symbols;
pub mod targets;
pub mod trees;
pub mod words;
