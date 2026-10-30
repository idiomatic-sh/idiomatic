//! reader_writes_another_relation: the compiler rejects this.

use ryo_language::domain::relations::collections::relations::function::item::FunctionRow;
use ryo_language::domain::relations::collections::relations::scope::item::ScopeRow;
use ryo_language::domain::relations::types::{FileId, NodeId, Offset, Position, Rows, Span};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::elements::types::Tree;

/// The reader of `function`, which is given the rows of `function` and writes one of `scope`.
fn visit(
    tree: &Tree<'_>,
    _model: &(),
    _symbols: &mut Symbols,
    _at: Position,
    out: &mut dyn Rows<FunctionRow>,
) {
    out.write(ScopeRow {
        parent: NodeId(0),
        child: NodeId(0),
        file: FileId(0),
        span: Span {
            start: Offset(0),
            end: Offset(0),
        },
    });
}

fn main() {}
