//! The `language` domain's rules, one module per concept. It is public because the catalog of
//! relations is the vocabulary every language writes in.

#[macro_use]
pub mod answers;
pub mod builders;
pub mod completion;
pub mod decoded;
pub mod diagnostics;
pub mod elements;
pub mod facts;
pub mod files;
pub mod fillers;
pub mod fixed_names;
pub mod flow;
pub mod flow_table;
pub mod keys;
pub mod kind_index;
pub mod names;
pub mod node_table;
pub mod notes;
pub mod project_context;
pub mod reasons;
pub mod semantics;
pub mod shapes;
pub mod symbols;
pub mod targets;
pub mod trees;
pub mod words;
