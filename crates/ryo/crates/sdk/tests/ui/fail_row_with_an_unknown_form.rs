//! row_with_an_unknown_form: a column is given by a form `reads!` names, so the compiler rejects this at the form.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        row { id: node(at), name: frobnicate(at), file: file, span: span(at) }
    }
}

fn main() {}
