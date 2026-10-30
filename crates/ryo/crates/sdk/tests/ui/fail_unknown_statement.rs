//! unknown_statement: a declaration is made of the statements `reads!` names, so the compiler rejects this at the statement.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        write { id: node(at), name: text(at), file: file, span: span(at) }
    }
}

fn main() {}
