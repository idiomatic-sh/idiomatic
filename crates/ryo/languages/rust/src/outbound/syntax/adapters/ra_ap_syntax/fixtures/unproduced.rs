//! Test support split from adapter_test: kind_of_bits, PROBES, UNPRODUCED and more.

use super::super::adapter::SyntaxRaApSyntaxAdapter;
use super::super::schemas::kind_of;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::kinds::sample;
use crate::outbound::syntax::fixtures::contract::{Told, expanding, made_by};
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::trees::{SOURCES, tree_of};
use ra_ap_syntax::SyntaxKind;
use ryo::languages::Sample;
use ryo::languages::{Elements, Position, kind_at};

pub(in super::super) fn kind_of_bits(elements: &Elements, position: Position) -> Kind {
    crate::domain::syntax::service::kind_of(kind_at(elements, position))
}

/// The sources the parser was probed with: the constructs the grammar names, nightly ones included, and the files with errors.
pub(in super::super) const PROBES: &[&str] = &[
    "fn main() {}",
    "fn f() { builtin # asm(\"\", out(reg) x, options(pure, att_syntax), clobber_abi(\"C\"), label { }) }",
    "fn f() { builtin # offset_of(S, f); builtin # format_args(\"{}\", x); let _ = builtin # include_bytes(\"f\"); let builtin # deref(x) = y; }",
    "type T = builtin # pattern_type(u32 is !null);",
    "---cargo\n[dependencies]\n---\nfn main() {}",
    "fn f() { do yeet 1; become g(); gen { yield 1 }; try { 1 }; let box x = y; match x { const { 1 } => {} } }",
    "impl(crate) trait T {}\nstruct S { pub mut(crate) x: u8 }\nauto trait A {}\nfn f() where T::foo(..): Send {}",
    "fn f( {\nlet abstract = 1;\n",
    "#!/usr/bin/env run\nmacro_rules! m { () => {} }\nunion U { a: u8 }\nunsafe extern \"C\" { safe fn g(); }\n",
];

/// The kinds the grammar defines and the parser never gives to a tree built from a file (their reasons are in `sample`).
pub(in super::super) const UNPRODUCED: [Kind; 6] = [
    Kind::Extra(Extra::Other),
    Kind::Node(Node::MacroItems),
    Kind::Node(Node::MacroStmts),
    Kind::Token(Token::Newline),
    Kind::Token(Token::BikeshedKw),
    Kind::Token(Token::NullKw),
];

/// How many of the kinds never produced are an element of a source anyway: of the probes, or of the sample of a kind.
pub(in super::super) fn unproduced_kinds_a_source_gives() -> usize {
    let adapter = SyntaxRaApSyntaxAdapter::new(expanding(Told::Standard));
    let samples = crate::domain::syntax::service::kinds().filter_map(|kind| match sample(kind) {
        Sample::Node(source) | Sample::Token(source) => Some(source.0),
        Sample::Unread(_) => None,
    });
    samples
        .chain(PROBES.iter().copied())
        .flat_map(|source| made_by(&adapter, source))
        .filter(|made| UNPRODUCED.contains(&made.element.kind))
        .count()
}

/// Whether every tree written by hand for a source of the domain's tests is the table this adapter makes of it: the first source for
/// which they differ, or `agreed`.
pub(in super::super) fn trees_agree() -> String {
    let adapter = SyntaxRaApSyntaxAdapter::new(expanding(Told::Nothing));
    for source in SOURCES {
        let made = made_by(&adapter, source);
        let written = made_by(
            &SyntaxFake(tree_of(source).expect("a tree is written for the source")),
            source,
        );
        if made != written {
            return format!("the tree of {source:?} differs from the adapter's");
        }
    }
    "agreed".to_owned()
}

/// Whether every kind of the grammar but the parser's three markers maps to the kind of its own name, which is written the way the
/// language writes its kinds: lower case, `-` for `_`.
pub(in super::super) fn kinds() -> String {
    let markers = [SyntaxKind::TOMBSTONE, SyntaxKind::EOF, SyntaxKind::__LAST];
    for number in 0..=u16::from(SyntaxKind::__LAST) {
        let library = SyntaxKind::from(number);
        let ours = kind_of(library);
        let right = if markers.contains(&library) {
            ours == Kind::Extra(Extra::Other)
        } else {
            kind_name(ours).0 == format!("{library:?}").to_lowercase().replace('_', "-")
        };
        if !right {
            return format!("{library:?} maps to the kind of another name");
        }
    }
    "mapped".to_owned()
}
