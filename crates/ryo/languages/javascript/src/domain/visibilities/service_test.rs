//! The `visibilities` service's cases: the export keywords each exported declaration is written with, where they are and how they read.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Goal, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// What the service answered over it, written out.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_export_of_two_declarators"),
        word: Word::Succeeds,
        input: || "export const a = 1, b = 2;",
        expect: Expect::Value(
            "Token(Keyword)@0 on Node(VariableDeclarator)@13 Span { start: Offset(0), end: Offset(6) } [export] Token(Keyword)@0 on Node(VariableDeclarator)@20 Span { start: Offset(0), end: Offset(6) } [export]",
        ),
    },
    Case {
        name: CaseName("a_default_export"),
        word: Word::Succeeds,
        input: || "export default function f() {}",
        expect: Expect::Value(
            "Token(Keyword)@0 on Node(FunctionDeclaration)@15 Span { start: Offset(0), end: Offset(14) } [export default]",
        ),
    },
    Case {
        name: CaseName("an_exported_class"),
        word: Word::Succeeds,
        input: || "export class K {}",
        expect: Expect::Value(
            "Token(Keyword)@0 on Node(ClassDeclaration)@7 Span { start: Offset(0), end: Offset(6) } [export]",
        ),
    },
];

/// The text of a name, or `-` for the unknown one.
fn text(
    tree: &languages::Tree<'_>,
    symbols: &languages::Symbols,
    name: languages::Symbol,
) -> String {
    languages::text_of(symbols, tree.source, name)
        .map_or_else(|| String::from("-"), |name| name.0.to_owned())
}

/// An element written out: its kind and where it starts.
fn at(tree: &languages::Tree<'_>, element: languages::Position) -> String {
    format!(
        "{:?}@{}",
        kind(tree, element),
        languages::start_at(tree.elements, element).0
    )
}

/// Parses the row's source and writes out what the service answers over its tree.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Goal::Module,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let tree = &tree;
    let symbols = &mut symbols;
    let answer: String = {
        languages::positions(tree.elements)
            .filter(|each| {
                matches!(
                    kind(tree, *each),
                    Kind::Node(Node::ExportNamedDeclaration | Node::ExportDefaultDeclaration)
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|each| super::service::visible(tree, each).collect::<Vec<_>>())
            .map(|seen| {
                let written = super::service::written(tree, symbols, &seen);
                format!(
                    "{} on {} {:?} [{}]",
                    at(tree, seen.occurrence),
                    at(tree, seen.declaration),
                    super::service::span(&seen),
                    text(tree, symbols, written)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
