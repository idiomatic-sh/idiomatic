// ignore: idiomatic.arch/file-length - the adapter spec allows one adapter_test.rs, which holds the port contract rows, the cases of the adapter's tree and its sections, and the comparison of the whole tree with the pinned mdast prints
//! The `pulldown_cmark` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxPulldownCmarkAdapter;
use super::fixtures::walk::{Shape, differences, dumped, shape};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::{self, ByteSlice, Elements, Position, Symbols, TOKEN};

/// What a row reads: a text whose elements are listed, a text and what the standard's reference tool printed for it, or every file
/// pinned.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of the text.
    Tree(&'static str),
    /// The nodes of the text, each with the nodes it holds, its tokens left out.
    Nesting(&'static str),
    /// A text and the `unist-util-inspect` print of the tree `mdast-util-from-markdown` gave for it: our tree projected onto mdast has
    /// the same nodes.
    Standard(&'static str, &'static str),
    /// Every relation's input, and every file of the corpus and of the suite's valid half, whose print is pinned in `standard/`.
    Pinned,
}

/// Markdown text, or what is compared with the standard.
type Input = Given;
/// The elements the adapter read it into, in order, each as its kind; or what the comparison with the standard found.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kinds of the elements, in order.
    Elements(&'static str),
    /// What differs between our tree projected onto mdast and the standard's.
    Standard(Option<&'static str>),
    /// The pinned files whose tree differs from the standard's, by path below `tests/fixtures`.
    Pinned(&'static [&'static str]),
}

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_heading_holds_its_marker_and_its_text"),
        word: Word::Succeeds,
        input: || Given::Tree("## a\n"),
        expect: Expect::Value(Seen::Elements(
            "root section heading markup text whitespace",
        )),
    },
    Case {
        name: CaseName("a_setext_heading_holds_its_underline"),
        word: Word::Succeeds,
        input: || Given::Tree("a\n===\n"),
        expect: Expect::Value(Seen::Elements("root section heading text markup")),
    },
    Case {
        name: CaseName("a_fenced_block_holds_its_fences_and_its_code"),
        word: Word::Succeeds,
        input: || Given::Tree("```rust\nfn a() {}\n```\n"),
        expect: Expect::Value(Seen::Elements(
            "root code markup code-flow-value markup whitespace",
        )),
    },
    Case {
        name: CaseName("a_link_holds_its_text_and_its_destination_as_markup"),
        word: Word::Succeeds,
        input: || Given::Tree("[a](b)\n"),
        expect: Expect::Value(Seen::Elements(
            "root paragraph link markup text markup whitespace",
        )),
    },
    Case {
        name: CaseName("nested_lists_nest"),
        word: Word::Succeeds,
        input: || Given::Tree("- a\n  - b\n"),
        expect: Expect::Value(Seen::Elements(
            "root list list-item markup text whitespace list list-item markup text whitespace",
        )),
    },
    Case {
        name: CaseName("an_alert_is_a_quote_of_its_kind"),
        word: Word::Succeeds,
        input: || Given::Tree("> [!NOTE]\n> a\n"),
        expect: Expect::Value(Seen::Elements(
            "root blockquote markup paragraph text whitespace",
        )),
    },
    Case {
        name: CaseName("front_matter_is_a_node_at_the_start"),
        word: Word::Succeeds,
        input: || Given::Tree("---\ntitle: x\n---\n"),
        expect: Expect::Value(Seen::Elements("root yaml markup text markup whitespace")),
    },
    Case {
        name: CaseName("a_table_and_a_task"),
        word: Word::Succeeds,
        input: || Given::Tree("| a |\n|---|\n| b |\n\n- [x] c\n"),
        expect: Expect::Value(Seen::Elements(
            "root table table-row markup table-cell whitespace text whitespace markup markup table-row markup table-cell whitespace text whitespace markup whitespace list list-item markup task-checked whitespace text whitespace",
        )),
    },
    Case {
        name: CaseName("a_tree_that_is_not_mdast_s_is_a_difference"),
        word: Word::Succeeds,
        input: || Given::Standard("a\n", "root[1]\n└─0 thematicBreak\n"),
        expect: Expect::Value(Seen::Standard(Some(
            "/root: we have `paragraph`, mdast has `thematicBreak`",
        ))),
    },
    Case {
        name: CaseName("every_pinned_relation_corpus_and_suite_file_has_mdast_s_nodes_but_four"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        // The four exceptions, each a difference of the library: `pulldown-cmark` lists only the first definition of a label, so the
        // second is not a node (CommonMark examples 204 and 544); it reads the empty front matter `---` `---` as two thematic
        // breaks (example 98); and it reads the `[!NOTE]` line of an alert as the quote's marker, where mdast has a paragraph of
        // text for it.
        expect: Expect::Value(Seen::Pinned(&[
            "corpus/ruff_CONTRIBUTING.md",
            "suite/valid/link-reference-definitions/example-204.md",
            "suite/valid/links/example-544.md",
            "suite/valid/setext-headings/example-098.md",
        ])),
    },
    Case {
        name: CaseName("a_section_holds_its_heading_and_the_blocks_up_to_the_next_heading_as_deep"),
        word: Word::Succeeds,
        input: || Given::Nesting("# A\n\na\n\n## B\n\n- b\n\n# C\n\nc\n"),
        expect: Expect::Value(Seen::Elements(
            "root(section(heading paragraph section(heading list(list-item))) section(heading paragraph))",
        )),
    },
    Case {
        name: CaseName("a_section_ends_with_the_blocks_that_hold_its_heading"),
        word: Word::Succeeds,
        input: || Given::Nesting("> # Q\n> q\n\na\n"),
        expect: Expect::Value(Seen::Elements(
            "root(blockquote(section(heading paragraph)) paragraph)",
        )),
    },
    Case {
        name: CaseName("blocks_before_the_first_heading_are_the_document_s"),
        word: Word::Succeeds,
        input: || Given::Nesting("a\n\n## B\n"),
        expect: Expect::Value(Seen::Elements("root(paragraph section(heading))")),
    },
    Case {
        name: CaseName("a_paragraph_holds_its_text_and_its_line_end"),
        word: Word::Succeeds,
        input: || Given::Tree("a\n"),
        expect: Expect::Value(Seen::Elements("root paragraph text whitespace")),
    },
];

/// The nodes under `at`, each written `kind(held held)`, its tokens left out.
fn nesting(elements: &Elements, at: Position) -> String {
    let held: Vec<String> = languages::children(elements, at)
        .filter(|each| languages::flags_at(elements, *each).0 & TOKEN.0 == 0)
        .map(|each| nesting(elements, each))
        .collect();
    let name = kind_name(kind_of(languages::kind_at(elements, at))).0;
    if held.is_empty() {
        name.to_owned()
    } else {
        format!("{name}({})", held.join(" "))
    }
}

/// The type mdast gives a kind of our trees; nothing for the kinds mdast has no node for (tokens, markup, the task marks).
fn mdast_name(kind: Kind) -> Option<String> {
    match kind {
        Kind::Node(_) => {
            let mut camel = String::new();
            let mut upper = false;
            for each in kind_name(kind).0.chars() {
                if each == '-' {
                    upper = true;
                } else if upper {
                    camel.push(each.to_ascii_uppercase());
                    upper = false;
                } else {
                    camel.push(each);
                }
            }
            Some(camel)
        }
        Kind::Token(Token::LineEnding) => Some("text".to_owned()),
        Kind::Extra(Extra::Yaml) => Some("yaml".to_owned()),
        Kind::Extra(Extra::Toml) => Some("toml".to_owned()),
        Kind::Extra(Extra::Math) => Some("math".to_owned()),
        Kind::Extra(Extra::InlineMath) => Some("inlineMath".to_owned()),
        Kind::Extra(_) | Kind::Token(_) => None,
    }
}

/// Whether the element at `at` is part of a line of text: what mdast puts in a paragraph.
fn is_phrasing(elements: &Elements, at: Position) -> bool {
    match kind_of(languages::kind_at(elements, at)) {
        Kind::Node(
            Node::Text
            | Node::Emphasis
            | Node::Strong
            | Node::Delete
            | Node::Link
            | Node::LinkReference
            | Node::Image
            | Node::ImageReference
            | Node::InlineCode
            | Node::Break
            | Node::FootnoteReference,
        )
        | Kind::Extra(Extra::InlineMath)
        | Kind::Token(Token::LineEnding) => true,
        Kind::Node(Node::Html) => languages::flags_at(elements, at).0 & TOKEN.0 != 0,
        Kind::Node(
            Node::Root
            | Node::Paragraph
            | Node::Heading
            | Node::Blockquote
            | Node::Code
            | Node::List
            | Node::ListItem
            | Node::FootnoteDefinition
            | Node::Table
            | Node::TableRow
            | Node::TableCell
            | Node::Definition
            | Node::ThematicBreak,
        )
        | Kind::Token(_)
        | Kind::Extra(_) => false,
    }
}

/// Our tree at `at` projected onto mdast: the nodes it is, the tokens and the markup dropped, the text of a line and the lines of a
/// paragraph one `text` as mdast joins them, and the lines of a tight list item or a footnote in the `paragraph` mdast puts them in.
fn project(elements: &Elements, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(languages::kind_at(elements, at));
    let Some(name) = mdast_name(kind) else {
        return;
    };
    let mut children: Vec<Shape> = Vec::new();
    // mdast keeps the alternative text of an image as a field, `alt`, and the text of front matter as `value`, not as nodes under them.
    if matches!(
        kind,
        Kind::Node(Node::Image | Node::ImageReference) | Kind::Extra(Extra::Yaml | Extra::Toml)
    ) {
        out.push(shape(&name, children));
        return;
    }
    let wraps = matches!(kind, Kind::Node(Node::ListItem | Node::FootnoteDefinition));
    let mut lines: Vec<Shape> = Vec::new();
    for each in held(elements, at) {
        let mut one = Vec::new();
        project(elements, each, &mut one);
        if wraps && is_phrasing(elements, each) {
            lines.append(&mut one);
            continue;
        }
        if !one.is_empty() {
            if !lines.is_empty() {
                children.push(shape("paragraph", std::mem::take(&mut lines)));
            }
            children.append(&mut one);
        }
    }
    if !lines.is_empty() {
        children.push(shape("paragraph", lines));
    }
    out.push(Shape { name, children });
}

/// The elements under `at` as mdast holds them: mdast has no section, so the elements a section holds are held by what holds it.
fn held(elements: &Elements, at: Position) -> Vec<Position> {
    let mut found = Vec::new();
    for each in languages::children(elements, at) {
        if kind_of(languages::kind_at(elements, each)) == Kind::Extra(Extra::Section) {
            found.extend(held(elements, each));
        } else {
            found.push(each);
        }
    }
    found
}

/// Our tree of `text`, projected onto mdast.
fn projected(text: &str) -> Option<Shape> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxPulldownCmarkAdapter.read(
        SourceText(text.as_bytes()),
        SourcePath("a.md"),
        &mut languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes())),
    );
    let mut out = Vec::new();
    project(&elements, Position(0), &mut out);
    out.pop().map(joined)
}

/// The shape with the `text` nodes that follow one another joined into one, at every depth: mdast has one `text` for a run of text,
/// where the library has one for each line, entity and escape.
fn joined(node: Shape) -> Shape {
    let mut children: Vec<Shape> = Vec::new();
    for child in node.children.into_iter().map(joined) {
        let after_text = children.last().is_some_and(|last| last.name == "text");
        if !(after_text && child.name == "text") {
            children.push(child);
        }
    }
    Shape {
        name: node.name,
        children,
    }
}

/// The prints pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: for the corpus
/// and the suite at the same path, and for each relation's input beside the page `standard/<relation>.mdast`.
fn pinned(
    standard: &std::path::Path,
    fixtures: &std::path::Path,
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut found = Vec::new();
    let mut folders = vec![standard.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
                continue;
            }
            if path.extension().is_some_and(|each| each == "mdast")
                && let Ok(below) = path.strip_prefix(standard)
            {
                let input = if below
                    .parent()
                    .is_some_and(|each| each.as_os_str().is_empty())
                {
                    std::path::Path::new("relations")
                        .join(below.file_stem().unwrap_or_default())
                        .join("input.md")
                } else {
                    below.with_extension("md")
                };
                if fixtures.join(&input).exists() {
                    found.push((below.to_path_buf(), input));
                }
            }
        }
    }
    found.sort();
    found
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Tree(text) => {
            check(&SyntaxPulldownCmarkAdapter);
            let shown = elements_of(&SyntaxPulldownCmarkAdapter, text)
                .iter()
                .map(|(kind, _)| kind_name(*kind).0)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Seen::Elements(Box::leak(shown.into_boxed_str())))
        }
        Given::Nesting(text) => {
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            SyntaxPulldownCmarkAdapter.read(
                SourceText(text.as_bytes()),
                SourcePath("a.md"),
                &mut languages::table_builder(
                    &mut elements,
                    &mut symbols,
                    ByteSlice(text.as_bytes()),
                ),
            );
            Ok(Seen::Elements(Box::leak(
                nesting(&elements, Position(0)).into_boxed_str(),
            )))
        }
        Given::Standard(text, print) => {
            let found = match (projected(text), dumped(print)) {
                (Some(ours), Some(theirs)) => differences(&ours, &theirs, ""),
                (None, _) => Some("our tree has no root".to_owned()),
                (_, None) => Some("the print does not read back".to_owned()),
            };
            Ok(Seen::Standard(
                found.map(|each| &*Box::leak(each.into_boxed_str())),
            ))
        }
        Given::Pinned => {
            let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let files = pinned(&fixtures.join("standard"), &fixtures);
            let mut differing = Vec::new();
            for (print, input) in &files {
                let (Ok(text), Ok(printed)) = (
                    std::fs::read_to_string(fixtures.join(input)),
                    std::fs::read_to_string(fixtures.join("standard").join(print)),
                ) else {
                    return Err(());
                };
                let agrees = match (projected(&text), dumped(&printed)) {
                    (Some(ours), Some(theirs)) => differences(&ours, &theirs, "").is_none(),
                    _ => false,
                };
                if !agrees {
                    differing.push(&*Box::leak(
                        input
                            .components()
                            .map(|part| part.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/")
                            .into_boxed_str(),
                    ));
                }
            }
            if files.is_empty() {
                return Err(());
            }
            Ok(Seen::Pinned(Box::leak(differing.into_boxed_slice())))
        }
    }
}

#[test]
fn succeeds() {
    check(&SyntaxPulldownCmarkAdapter);
    run(CASES, Word::Succeeds, subject);
}
