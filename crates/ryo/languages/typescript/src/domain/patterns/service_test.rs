//! The `patterns` service's cases: what a carrier says of the pattern it carries, and where each pattern element stands under it.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::{service::kind, service::kind_name, types::Dialect};
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
        name: CaseName("a_literal_carries_its_flags_and_its_elements"),
        word: Word::Succeeds,
        input: || "/a[b-c]/g;",
        expect: Expect::Value(
            "carrier@2 flags=[g] parsed -; pattern@5 under 2 a[b-c] body value=0 nested=Lowered; disjunction@6 under 2 a[b-c] body value=0 nested=Raised; alternative@7 under 2 a[b-c] body value=0 nested=Raised; character@8 under 2 a body value=97 nested=Raised; character-class@9 under 2 [b-c] body value=0 nested=Raised; character-class-range@10 under 2 b-c body value=0 nested=Raised; character@11 under 2 b min value=98 nested=Raised; character@12 under 2 c max value=99 nested=Raised",
        ),
    },
    Case {
        name: CaseName("a_refused_pattern_keeps_the_message_on_its_carrier"),
        word: Word::Succeeds,
        input: || "RegExp(\"(\");",
        expect: Expect::Value(
            "carrier@6 flags=[] malformed Invalid regular expression: Unterminated capturing group",
        ),
    },
];

/// The text of a name, or `-` for none.
fn text(
    tree: &languages::Tree<'_>,
    symbols: &languages::Symbols,
    name: languages::Symbol,
) -> String {
    languages::text_of(symbols, tree.source, name)
        .map_or_else(|| String::from("-"), |name| name.0.to_owned())
}

/// Parses the row's source and writes out what the service answers of each carrier and each pattern element.
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
    let mut answers: Vec<String> = Vec::new();
    for at in languages::positions(tree.elements) {
        if let Some(flags) = super::service::flags(tree, symbols, at) {
            let outcome = super::service::outcome(tree, symbols, at);
            let error = super::service::error(tree, symbols, at);
            answers.push(format!(
                "carrier@{} flags=[{}] {} {}",
                at.0,
                text(tree, symbols, flags),
                text(tree, symbols, outcome),
                text(tree, symbols, error)
            ));
        }
    }
    let embedded: Vec<languages::EmbeddedSource> =
        languages::embedded_expressions(tree.elements).collect();
    for source in embedded {
        for at in (source.root.0..source.end.0).map(languages::Position) {
            let shown = super::service::text(tree, symbols, at);
            let field = super::service::field(tree, symbols, at);
            answers.push(format!(
                "{}@{} under {} {} {} value={} nested={:?}",
                kind_name(kind(tree, at)).0,
                at.0,
                super::service::carrier(tree, at).0,
                text(tree, symbols, shown),
                text(tree, symbols, field),
                super::service::value(tree, at).0,
                super::service::nested(tree, at)
            ));
        }
    }
    Ok(Box::leak(answers.join("; ").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
