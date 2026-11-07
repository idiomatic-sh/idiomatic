//! row_with_a_column_of_another_relation: a row is the relation's own, so the compiler rejects a column it does not have.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        row { id: node(at), name: text(at), parent: node(at), file: file, span: span(at) }
    }
}

fn main() {}
