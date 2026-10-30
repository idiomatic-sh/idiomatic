//! each_without_an_item: `each` names the item it binds, so the compiler rejects this at the block.

use ryo::languages::{self, KindBits};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        each (languages::children(tree.elements, at)) {
            row { id: node(at), name: text(at), file: file, span: span(at) }
        }
    }
}

fn main() {}
