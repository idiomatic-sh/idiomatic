//! The `visibilities` service's cases: the visibility keywords each node writes, and the declaration each is written on.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::{service::kind, types::Dialect};
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
        name: CaseName("an_exported_class_and_its_member_accessibilities"),
        word: Word::Succeeds,
        input: || "export class A { private x = 1; public y() {} }",
        expect: Expect::Value(
            "Token(Keyword)/Node(ClassDeclaration) Token(Keyword)/Node(PropertyDefinition) Token(Keyword)/Node(MethodDefinition)",
        ),
    },
    Case {
        name: CaseName("an_export_of_two_declarators"),
        word: Word::Succeeds,
        input: || "export const a = 1, b = 2;",
        expect: Expect::Value(
            "Token(Keyword)/Node(VariableDeclarator) Token(Keyword)/Node(VariableDeclarator)",
        ),
    },
    Case {
        name: CaseName("an_unexported_function_writes_none"),
        word: Word::Succeeds,
        input: || "function f() {}",
        expect: Expect::Value(""),
    },
];

/// Parses the row's source and writes out what the service answers over its tree.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Dialect::TypeScript,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let tree = &tree;
    let answer: String = {
        languages::positions(tree.elements)
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|at| super::service::visible(tree, at).collect::<Vec<_>>())
            .map(|seen| {
                format!(
                    "{:?}/{:?}",
                    kind(tree, seen.occurrence),
                    kind(tree, seen.declaration)
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
