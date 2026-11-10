//! reader_visits_with_any_writer: the compiler rejects this.

use ryo_relations::domain::relations::collections::relations::function::item::Function;
use ryo_relations::domain::relations::types::{Position, RowWriter};
use ryo_language::domain::answers::types::Visiting;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::elements::types::Tree;

/// A reader of `function` whose visit is given a writer of every relation, which the type of a reader refuses: it is given the rows of its own.
struct Mine;

impl Visiting<Function, ()> for Mine {
    fn visit(
        &self,
        _tree: &Tree<'_>,
        _model: &(),
        _symbols: &mut Symbols,
        _at: Position,
        _out: &mut dyn RowWriter,
    ) {
    }
}

fn main() {}
