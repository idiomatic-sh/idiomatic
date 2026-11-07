//! relation_defined_by_a_language: the compiler rejects this.

use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{Class, CellKind, Relation};

/// A language that defines a relation of its own, past the catalog.
struct Mine;

impl RelationsItem for Mine {
    type Row = ();
    type Column = ();
    type Words = [u64; 0];

    const CLASS: Class = Class::Code;

    fn relation() -> Relation {
        Relation::Function
    }

    fn columns() -> Vec<()> {
        Vec::new()
    }

    fn kinds() -> &'static [CellKind] {
        &[]
    }

    fn kind(_column: ()) -> CellKind {
        CellKind::Node
    }

    fn words(_row: ()) -> [u64; 0] {
        []
    }
}

fn main() {}
