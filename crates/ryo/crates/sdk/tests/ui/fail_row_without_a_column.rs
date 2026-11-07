//! row_without_a_column: a row has every column of its relation, so the compiler rejects this.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        row { id: node(at), file: file, span: span(at) }
    }
}

fn main() {}
