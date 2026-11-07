//! read_item_written_by_hand: a reader is declared with `ryo::reads!`, never implemented, so the compiler rejects this.

use std::marker::PhantomData;

use ryo::languages::{self, Position, Reader, Reads, Rows, Symbols, Tree};

struct ByHand;

fn visit(
    _tree: &Tree<'_>,
    _model: &(),
    _symbols: &mut Symbols,
    _at: Position,
    _out: &mut dyn Rows<languages::FunctionRow>,
) {
}

impl languages::ReadItem for ByHand {
    type Relation = languages::Function;

    const READER: Reader<languages::Function, ()> = Reader {
        reads: Reads::Every,
        visit,
        expansions: false,
        relation: PhantomData,
    };
}

fn main() {}
