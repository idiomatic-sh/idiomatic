//! The `catalog` port's types: what the port hands back.

use ryo_language::domain::relations::types::Relation;

/// The relations the catalog lists, in the order it lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relations(pub Vec<Relation>);
