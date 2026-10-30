//! when_branch_without_a_block: a branch of `when` is a block of statements, so the compiler rejects this at the branch.

use ryo::languages::{self, KindBits, Position, Tree};

const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

fn kind(tree: &Tree<'_>, at: Position) -> u16 {
    languages::kind_at(tree.elements, at).0
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1] as kind_bits,
        when kind(tree, at) {
            1 => row { id: node(at), name: text(at), file: file, span: span(at) }
            _ => {}
        }
    }
}

fn main() {}
