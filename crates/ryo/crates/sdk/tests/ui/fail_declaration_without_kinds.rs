//! declaration_without_kinds: a reader says which kinds it looks at, or `every`, so the compiler rejects this.

use ryo::languages;

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        row { id: node(at), name: text(at), file: file, span: span(at) }
    }
}

fn main() {}
