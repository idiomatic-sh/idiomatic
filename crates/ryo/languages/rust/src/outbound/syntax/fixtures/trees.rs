//! The trees of the sources the tests of the domain read, written by hand in the language's kinds, so those tests depend on no parser.
//! The adapter of the port is held to them: its test makes the table of each source and compares it element by element.

use super::script::Piece;

mod bindings;
mod control;
mod expressions;
mod items;
mod modules;
mod signatures;
mod uses;

/// Every source that has a tree here.
pub(crate) const SOURCES: [&str; 61] = [
    "fn f() {}",
    "fn f() { g(); }",
    "fn f() { a::b(); }",
    "pub fn f() {}",
    "#[a] fn f() {}",
    "fn f<'a>() {}",
    "fn f() { 'a: loop {} }",
    "extern \"C\" fn f() {}",
    "use a as b;",
    "fn f<T>() {}",
    "fn f() { g::<u8>(); }",
    "fn f() where T: A {}",
    "fn f(a: u8) {}",
    "fn f() -> u8 {}",
    "fn f() { g(1); }",
    "mod m { fn f() {} }",
    "impl T { fn f() {} }",
    "struct S { a: u8 }",
    "struct S(u8);",
    "enum E { A }",
    "fn f() { match x { _ => 1 } }",
    "use a::{b};",
    "fn f() { S { a: 1 }; }",
    "fn f() { let S { a } = s; }",
    "use a;",
    "fn f() { let a = b else { return }; }",
    "fn f() { let a = 1; }",
    "fn f() { 1 }",
    "fn f() { x.y; }",
    "fn f<T: A>() {}",
    "fn f() { if a {} }",
    "fn f() { if a {} else {} }",
    "fn f() { loop {} }",
    "fn f() { for a in b {} }",
    "fn f() { a.b(); }",
    "fn f() { a + b; }",
    "fn f() { a[b]; }",
    "fn f() { a..b; }",
    "fn f() { S { ..a }; }",
    "fn f() { match x { _ if y => 1 } }",
    "impl A for B {}",
    "fn f<T = u8>() {}",
    "fn f<const N: usize = 1>() {}",
    "#[cfg(unix)] fn f() {}",
    "fn one() {}\nstruct S;\nimpl S {\n    fn two(&self) {}\n}\n",
    "fn run() {}",
    "fn run(value: u8) {}",
    "fn run() -> u8 { 0 }",
    "fn f() { let x = 1; }",
    "fn f(a: u32) {}",
    "fn f((a, b): (u32, u32)) {}",
    "fn f() { let MAX = 1; }",
    "use a::{b, c, d};",
    "use a::b;",
    "fn f() { a::g(); }",
    "mod a;\nmod b { mod c; }\n#[path = \"x/y.rs\"] mod d;\n",
    "mod r#type;\nuse crate::r#type::r#enum;\n",
    "use crate::a::{b, c as d, self};\nuse super::*;\nfn f() { crate::m::g(); Self::x(); single(); <T as Tr>::f(); }\n",
    "mod outer { mod inner { fn f() { self::a::b(); } } }\n",
    "fn f( {",
    "use crate::a::{b, c};\nfn f() { crate :: m\n    ::g::<u8>(); }\n",
];

/// The tree of `source`; none when no tree is written for it.
#[must_use]
pub(crate) fn tree_of(source: &str) -> Option<Piece> {
    signatures::tree_of(source)
        .or_else(|| expressions::tree_of(source))
        .or_else(|| modules::tree_of(source))
        .or_else(|| control::tree_of(source))
        .or_else(|| uses::tree_of(source))
        .or_else(|| items::tree_of(source))
        .or_else(|| bindings::tree_of(source))
}
