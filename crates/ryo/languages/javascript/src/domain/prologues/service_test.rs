//! The `prologues` service's cases: which leading string statements are directives, as written, and the empty argument.

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
        name: CaseName("the_leading_strings_are_the_prologue"),
        word: Word::Succeeds,
        input: || "\"use strict\"; \"other\"; x; \"late\";",
        expect: Expect::Value("use strict other - - argument=[]"),
    },
    Case {
        name: CaseName("an_escape_is_kept_as_written"),
        word: Word::Succeeds,
        input: || "\"use\\x20strict\";",
        expect: Expect::Value("use\\x20strict argument=[]"),
    },
    Case {
        name: CaseName("a_function_body_has_its_own_prologue"),
        word: Word::Succeeds,
        input: || "function f() { \"use strict\"; }",
        expect: Expect::Value("use strict argument=[]"),
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

/// Parses the row's source and writes out what the service answers over its tree.
fn subject(source: Input) -> Result<Value, Failure> {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Goal::Script,
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
        let mut answers: Vec<String> = languages::positions(tree.elements)
            .filter(|at| kind(tree, *at) == Kind::Node(Node::ExpressionStatement))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|at| {
                super::service::directive(tree, symbols, at)
                    .map_or_else(|| String::from("-"), |name| text(tree, symbols, name))
            })
            .collect();
        let argument = super::service::argument(tree, symbols);
        answers.push(format!("argument=[{}]", text(tree, symbols, argument)));
        answers.join(" ")
    };
    Ok(Box::leak(answer.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
