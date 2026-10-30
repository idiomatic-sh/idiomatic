//! expression_column: a column is given by a form, and `{ expression }` is no form, so the compiler rejects this.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        row { id: node(at), name: text(at), file: { tree.file }, span: span(at) }
    }
}

fn main() {}
