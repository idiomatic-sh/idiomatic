//! The `runs` service's cases: which elements are read as prose, whether each joins the run around it or holds its own, and the text of each run.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind, types::Kind};
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
        name: CaseName("a_paragraph_holds_a_run_its_inline_markup_joins"),
        word: Word::Succeeds,
        input: || "<p>Hello <b>bold</b>  world</p>",
        expect: Expect::Value(
            "Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@0:Prose/Block/Words/owner Node(SourceFile)@0/[Hello bold world] Node(Element)@9:Prose/Inline/Silent/owner Node(SourceFile)@0/[bold]",
        ),
    },
    Case {
        name: CaseName("script_and_style_are_not_read"),
        word: Word::Succeeds,
        input: || "<div>text<script>x()</script><style>p{}</style></div>",
        expect: Expect::Value(
            "Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@0:Prose/Block/Words/owner Node(SourceFile)@0/[text] Node(Element)@9:Excluded/Block/Silent/owner Node(SourceFile)@0/[x()] Node(Element)@29:Excluded/Block/Silent/owner Node(SourceFile)@0/[p{}]",
        ),
    },
    Case {
        name: CaseName("code_is_kept_apart_from_words"),
        word: Word::Succeeds,
        input: || "<p>call <code>f()</code> now</p>",
        expect: Expect::Value(
            "Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@4294967295:Prose/Block/Silent/owner Node(SourceFile)@0/[] Node(Element)@0:Prose/Block/Words/owner Node(SourceFile)@0/[call now] Node(Element)@8:Prose/Inline/Silent/owner Node(SourceFile)@0/[f()]",
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
    factory::swc_html_parser().read(
        SourceText(source.as_bytes()),
        languages::SourcePath("page.html"),
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
            .filter(|each| kind(tree, *each) == Kind::Node(Node::Element))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|element| {
                let name = super::service::run_name(tree, symbols, element);
                format!(
                    "{}:{:?}/{:?}/{:?}/owner {}/[{}]",
                    at(tree, element),
                    super::service::readable(tree, element),
                    super::service::flow(tree, element),
                    super::service::run_words(tree, element),
                    at(tree, super::service::run_owner(tree, element)),
                    text(tree, symbols, name),
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
