//! The `bindings` service's cases: for each reference of a source, how it uses its name, what it is bound to, and the declarations it
//! resolves to.

use super::service::{candidates, context, indexed, state};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Dialect, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;

/// The source a row parses.
type Input = &'static str;
/// What the service answered over it: one line for each reference, written out.
type Value = &'static str;
/// Reading a tree is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_class_binds_a_type_reference_in_a_sibling_function"),
        word: Word::Succeeds,
        input: || "class A {} function f(x: A) {}",
        expect: Expect::Value("A type unique-declaration [\"Node(ClassDeclaration)\"]"),
    },
    Case {
        name: CaseName("a_type_parameter_binds_inside_its_declaration"),
        word: Word::Succeeds,
        input: || "function g<T>(y: T) {}",
        expect: Expect::Value("T type unique-declaration [\"Node(TSTypeParameter)\"]"),
    },
    Case {
        name: CaseName("an_unknown_name_is_unbound_and_a_global_object_property_builtin"),
        word: Word::Succeeds,
        input: || "let w: Missing = Math;",
        expect: Expect::Value("Missing type unbound []; Math value builtin []"),
    },
    Case {
        name: CaseName("a_name_declared_twice_resolves_to_both_declarations"),
        word: Word::Succeeds,
        input: || "var x; var x; x = 1;",
        expect: Expect::Value(
            "x write alternative-declarations [\"Node(VariableDeclarator)\", \"Node(VariableDeclarator)\"]",
        ),
    },
    Case {
        name: CaseName(
            "a_catch_parameter_and_a_function_expression_name_bind_where_they_are_written",
        ),
        word: Word::Succeeds,
        input: || "try {} catch (e) { e; } (function g() { g; });",
        expect: Expect::Value(
            "e value unique-declaration [\"Node(CatchClause)\"]; g value unique-declaration [\"Node(FunctionExpression)\"]",
        ),
    },
    Case {
        name: CaseName("a_shorthand_destructured_name_has_one_declaration"),
        word: Word::Succeeds,
        input: || "const { b } = o; b;",
        expect: Expect::Value(
            "o value unbound []; b value unique-declaration [\"Node(VariableDeclarator)\"]",
        ),
    },
    Case {
        name: CaseName("a_signature_parameter_binds_no_declaration"),
        word: Word::Succeeds,
        input: || "type F = (p: number) => typeof p; interface I { m(q: number): typeof q }",
        expect: Expect::Value("p type unique-declaration []; q type unique-declaration []"),
    },
];

/// The text of the name `symbol` stands for, or `-` for none.
fn text(
    tree: &languages::Tree<'_>,
    symbols: &languages::Symbols,
    symbol: languages::Symbol,
) -> String {
    languages::text_of(symbols, tree.source, symbol)
        .map_or_else(|| String::from("-"), |name| name.0.to_owned())
}

/// Parses the row's source and writes out what the service answers for each reference of its tree.
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
    let symbols = &mut symbols;
    let cell = std::cell::RefCell::default();
    let index = indexed(tree, &cell);
    let mut lines = Vec::new();
    for at in languages::positions(tree.elements) {
        if kind(tree, at) != Kind::Node(Node::Identifier) {
            continue;
        }
        let Some(used) = context(tree, symbols, at) else {
            continue;
        };
        let name = crate::domain::names::service::identifier(tree, symbols, at);
        let bound = state(tree, symbols, &index, at);
        let resolved: Vec<String> = candidates(tree, &index, at)
            .iter()
            .map(|candidate| format!("{:?}", kind(tree, candidate.at)))
            .collect();
        lines.push(format!(
            "{} {} {} {resolved:?}",
            text(tree, symbols, name),
            text(tree, symbols, used),
            text(tree, symbols, bound),
        ));
    }
    Ok(Box::leak(lines.join("; ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
