// this test holds the whole comparison with the standard's print: the reader of the print, our tree projected onto it and the cases; a split would be a file no adapter template places
//! The `cssparser` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxCssparserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::{
    ByteSlice, Elements, Position, Symbols, children as children_at, end_at, kind_at, start_at,
    table_builder,
};

/// What a row reads: CSS text whose elements are listed, a file and the tree the standard's tool printed for it, or every file
/// whose tree is pinned.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of the text.
    Tree(&'static str),
    /// A file and what tinycss2 printed for it: our tree has the standard's nodes and tokens.
    Standard(&'static str, &'static str),
    /// Every relation's input, and every file of the corpus and of the suite, whose tree is pinned in `standard/`.
    Pinned,
}

type Input = Given;
/// The kind of each element the adapter read the text into, in order; what differs from the standard's tree; or the files whose tree
/// is not the standard's, by path below `tests/fixtures`.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kinds of the elements, in order.
    Elements(&'static str),
    /// The first place our tree differs from the standard's; nothing when it does not.
    Standard(Option<&'static str>),
    /// The pinned files whose tree differs from the standard's.
    Pinned(&'static [&'static str]),
}

/// A node or a token of a tree, by its kind's name, with what it holds under it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shape {
    /// The kind's name.
    label: String,
    /// What it holds, in order.
    children: Vec<Shape>,
}

fn shape(label: &str, children: Vec<Shape>) -> Shape {
    Shape {
        label: label.to_owned(),
        children,
    }
}

/// A line of the tool's print and the lines indented under it.
struct Line {
    head: String,
    below: Vec<Line>,
}

/// The standard's token that a `literal` of tinycss2 is, or the two delim-tokens of an attribute matcher.
fn literal(value: &str) -> Vec<Shape> {
    let one = |name: &str| vec![shape(name, Vec::new())];
    match value {
        ":" => one("colon-token"),
        ";" => one("semicolon-token"),
        "," => one("comma-token"),
        "<!--" => one("cdo-token"),
        "-->" => one("cdc-token"),
        "~=" | "|=" | "^=" | "$=" | "*=" | "||" => {
            vec![shape("delim-token", Vec::new()); 2]
        }
        _ => one("delim-token"),
    }
}

/// What a line of the print is in the standard's names, with what is under it: the nodes of the parsing results and the tokens of
/// the tokenizer. White space and comments are dropped, a rule's block flattened into the rule, a declaration given its name and
/// colon and its `!important` as the one token `important`.
fn standard_of(line: &Line) -> Vec<Shape> {
    let (word, argument) = line.head.split_once(' ').unwrap_or((&line.head, ""));
    let below = || -> Vec<Shape> { line.below.iter().flat_map(standard_of).collect() };
    let first = |name: &str| {
        let mut children = vec![shape(name, Vec::new())];
        children.extend(below());
        children
    };
    match (word, argument) {
        ("whitespace" | "comment", _) => Vec::new(),
        ("stylesheet" | "qualified-rule", _) => vec![shape(word, below())],
        ("at-rule", _) => vec![shape(word, first("at-keyword-token"))],
        ("function", _) => vec![shape(word, first("function-token"))],
        ("block", "") => first("{-token"),
        ("block", opener) => vec![shape("simple-block", first(&format!("{opener}-token")))],
        ("declaration", name) => {
            let mut children = vec![
                shape("ident-token", Vec::new()),
                shape("colon-token", Vec::new()),
            ];
            children.extend(below());
            if name.ends_with(" important") {
                children.push(shape("important", Vec::new()));
            }
            vec![shape(word, children)]
        }
        ("error", "bad-string" | "bad-url") => {
            vec![shape(&format!("{argument}-token"), Vec::new())]
        }
        ("error", _) => vec![shape("error", Vec::new())],
        ("literal", value) => literal(value),
        (other, _) => vec![shape(&format!("{other}-token"), Vec::new())],
    }
}

/// The tree tinycss2 printed, read back in the standard's names. Nothing when the text is no print.
fn dumped(print: &str) -> Option<Shape> {
    let mut open: Vec<(usize, Line)> = Vec::new();
    let mut roots: Vec<Line> = Vec::new();
    for text in print.lines() {
        let depth = (text.len() - text.trim_start().len()) / 2;
        while open.last().is_some_and(|(each, _)| *each >= depth) {
            let (_, done) = open.pop()?;
            match open.last_mut() {
                Some((_, parent)) => parent.below.push(done),
                None => roots.push(done),
            }
        }
        open.push((
            depth,
            Line {
                head: text.trim_start().to_owned(),
                below: Vec::new(),
            },
        ));
    }
    while let Some((_, done)) = open.pop() {
        match open.last_mut() {
            Some((_, parent)) => parent.below.push(done),
            None => roots.push(done),
        }
    }
    let mut root = roots.pop()?;
    if !roots.is_empty() {
        return None;
    }
    // The library skips an `@charset` that is the first rule, and the comparison leaves it out of the standard's tree too.
    if let Some(first) = root
        .below
        .iter()
        .position(|each| !each.head.starts_with("whitespace") && !each.head.starts_with("comment"))
        && root.below[first]
            .head
            .eq_ignore_ascii_case("at-rule charset")
    {
        drop(root.below.remove(first));
    }
    standard_of(&root).pop()
}

/// The text of the element at `at`.
fn text_at<'a>(elements: &Elements, text: &'a str, at: Position) -> &'a str {
    let (from, to) = (start_at(elements, at).0, end_at(elements, at).0);
    text.get(usize::try_from(from).unwrap_or(0)..usize::try_from(to).unwrap_or(0))
        .unwrap_or_default()
}

/// Drops the `@charset` rule that is the first rule, from its at-keyword to its `;` or the `}` of its block: the library skips it
/// without a node, so its tokens lie in the stylesheet.
fn skip_charset(elements: &Elements, text: &str, kids: &mut Vec<Position>) {
    let of = |position: &Position| kind_of(kind_at(elements, *position));
    let mut words = kids.iter().copied().filter(|each| {
        !matches!(
            of(each),
            Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment | Extra::ByteOrderMark)
        )
    });
    if let Some(first) = words.next()
        && of(&first) == Kind::Token(Token::AtKeyword)
        && text_at(elements, text, first).eq_ignore_ascii_case("@charset")
        && let Some(last) =
            words.find(|each| matches!(of(each), Kind::Token(Token::Semicolon | Token::RightCurly)))
    {
        kids.retain(|each| *each < first || *each > last);
    }
}

/// Our tree at `at` in the standard's terms: what the print has. The print drops white space, comments and the byte order mark, a
/// rule's `}` and `;` and a block's or function's closing token, a stylesheet's `<!--` and `-->`; it has no rule for a qualified rule
/// with no block (an error), and `!important` is a flag of a declaration.
fn project(elements: &Elements, text: &str, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(kind_at(elements, at));
    let node = match kind {
        Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment | Extra::ByteOrderMark) => {
            return;
        }
        Kind::Token(Token::RightParenthesis | Token::RightSquare | Token::RightCurly) => {
            return out.push(shape("error", Vec::new()));
        }
        Kind::Node(node) => node,
        Kind::Token(_) | Kind::Extra(_) => {
            return out.push(shape(kind_name(kind).0, Vec::new()));
        }
    };
    let mut kids: Vec<Position> = children_at(elements, at).collect();
    let of = |position: &Position| kind_of(kind_at(elements, *position));
    let closer = match (node, kids.first().map(of)) {
        (Node::Function, _) => Some(Token::RightParenthesis),
        (Node::SimpleBlock, Some(Kind::Token(opener))) => mirror(opener),
        _ => None,
    };
    let mut important = false;
    match node {
        Node::Stylesheet => {
            skip_charset(elements, text, &mut kids);
            kids.retain(|each| {
                !matches!(
                    of(each),
                    Kind::Token(Token::Cdo | Token::Cdc | Token::Semicolon)
                )
            });
        }
        Node::QualifiedRule | Node::AtRule => {
            if node == Node::QualifiedRule
                && !kids
                    .iter()
                    .any(|each| of(each) == Kind::Token(Token::LeftCurly))
            {
                return out.push(shape("error", Vec::new()));
            }
            kids.retain(|each| {
                !matches!(of(each), Kind::Token(Token::Semicolon | Token::RightCurly))
            });
        }
        Node::Declaration => {
            let words: Vec<&Position> = kids
                .iter()
                .filter(|each| {
                    !matches!(
                        of(each),
                        Kind::Token(Token::Whitespace) | Kind::Extra(Extra::Comment)
                    )
                })
                .collect();
            if let [.., bang, word] = words.as_slice()
                && of(bang) == Kind::Token(Token::Delim)
                && text_at(elements, text, **bang) == "!"
                && of(word) == Kind::Token(Token::Ident)
                && text_at(elements, text, **word).eq_ignore_ascii_case("important")
            {
                let from = **bang;
                kids.retain(|each| *each < from);
                important = true;
            }
        }
        Node::Function | Node::SimpleBlock => {
            if let (Some(last), Some(closer)) = (kids.last(), closer)
                && of(last) == Kind::Token(closer)
            {
                let _: Option<Position> = kids.pop();
            }
        }
    }
    let mut children = Vec::new();
    for each in kids {
        project(elements, text, each, &mut children);
    }
    if important {
        children.push(shape("important", Vec::new()));
    }
    out.push(shape(kind_name(kind).0, children));
}

/// Our tree of `text` in the standard's terms.
fn projected(text: &str) -> Option<Shape> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes()));
    SyntaxCssparserAdapter.read(
        SourceText(text.as_bytes()),
        SourcePath("a.css"),
        &mut builder,
    );
    let mut out = Vec::new();
    project(&elements, text, Position(0), &mut out);
    out.pop()
}

/// The first place the two trees differ, with the path to it; nothing when they are the same.
fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    if ours.label != theirs.label {
        return Some(format!(
            "{path}: we have `{}`, the standard has `{}`",
            ours.label, theirs.label
        ));
    }
    let path = format!("{path}/{}", ours.label);
    ours.children
        .iter()
        .zip(&theirs.children)
        .enumerate()
        .find_map(|(place, (mine, yours))| differences(mine, yours, &format!("{path}[{place}]")))
        .or_else(|| {
            (ours.children.len() != theirs.children.len()).then(|| {
                format!(
                    "{path}: we have {} parts, the standard has {}",
                    ours.children.len(),
                    theirs.children.len()
                )
            })
        })
}

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_holds_its_prelude_and_its_declarations"),
        word: Word::Succeeds,
        input: || Given::Tree("a { color: red; }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule ident-token whitespace-token {-token whitespace-token declaration ident-token colon-token whitespace-token ident-token semicolon-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("an_import_is_an_at_rule_whose_prelude_is_its_url"),
        word: Word::Succeeds,
        input: || Given::Tree("@import url(\"a.css\") screen;"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token function function-token string-token )-token whitespace-token ident-token semicolon-token",
        )),
    },
    Case {
        name: CaseName("important_is_two_tokens_of_the_declaration"),
        word: Word::Succeeds,
        input: || Given::Tree(".a { color: red !important }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule delim-token ident-token whitespace-token {-token whitespace-token declaration ident-token colon-token whitespace-token ident-token whitespace-token delim-token ident-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_conditional_rule_holds_rules"),
        word: Word::Succeeds,
        input: || Given::Tree("@media screen { a {} }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token ident-token whitespace-token {-token whitespace-token qualified-rule ident-token whitespace-token {-token }-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_declaration_holding_a_block_is_a_nested_rule"),
        word: Word::Succeeds,
        input: || Given::Tree(".a { b:hover {} }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule delim-token ident-token whitespace-token {-token whitespace-token qualified-rule ident-token colon-token ident-token whitespace-token {-token }-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("an_unknown_at_rule_holds_component_values"),
        word: Word::Succeeds,
        input: || Given::Tree("@foo { bar }"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet at-rule at-keyword-token whitespace-token {-token whitespace-token ident-token whitespace-token }-token",
        )),
    },
    Case {
        name: CaseName("a_rule_without_a_block_is_still_in_the_tree"),
        word: Word::Succeeds,
        input: || Given::Tree("foo 4"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule ident-token whitespace-token number-token",
        )),
    },
    Case {
        name: CaseName("a_byte_order_mark_is_a_token_of_its_own"),
        word: Word::Succeeds,
        input: || Given::Tree("\u{feff}a{}"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet byte-order-mark qualified-rule ident-token {-token }-token",
        )),
    },
    Case {
        name: CaseName("an_attribute_matcher_is_two_delim_tokens_in_a_simple_block"),
        word: Word::Succeeds,
        input: || Given::Tree("[a~=b]{}"),
        expect: Expect::Value(Seen::Elements(
            "stylesheet qualified-rule simple-block [-token ident-token delim-token delim-token ident-token ]-token {-token }-token",
        )),
    },
    Case {
        name: CaseName("a_tree_that_is_not_the_standard_s_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "a { color: red }",
                "stylesheet\n  at-rule media\n    block\n",
            )
        },
        expect: Expect::Value(Seen::Standard(Some(
            "/stylesheet[0]: we have `qualified-rule`, the standard has `at-rule`",
        ))),
    },
    Case {
        name: CaseName("every_pinned_relation_corpus_and_suite_file_has_the_standard_s_tree"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        // The one exception: `@foo; @charset 4 {}`. An `@charset` that is not the first rule is refused by the library, which recovers
        // from it and keeps its tokens without a node, where tinycss2 keeps the at-rule.
        expect: Expect::Value(Seen::Pinned(&["suite/invalid/stylesheet/05.css"])),
    },
];

/// The trees pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: for the corpus
/// and the suite at the same path, and for each relation's input beside the page `standard/<relation>.tree`.
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
            if path.extension().is_some_and(|each| each == "tree")
                && let Ok(below) = path.strip_prefix(standard)
            {
                let input = if below
                    .parent()
                    .is_some_and(|each| each.as_os_str().is_empty())
                {
                    std::path::Path::new("relations")
                        .join(below.file_stem().unwrap_or_default())
                        .join("input.css")
                } else {
                    below.with_extension("css")
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

/// Whether our tree of `text` is the standard's `print`.
fn agrees(text: &str, print: &str) -> bool {
    match (projected(text), dumped(print)) {
        (Some(ours), Some(theirs)) => differences(&ours, &theirs, "").is_none(),
        _ => false,
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Tree(text) => {
            check(&SyntaxCssparserAdapter);
            let shown = elements_of(&SyntaxCssparserAdapter, text)
                .iter()
                .map(|(kind, _)| kind_name(*kind).0)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Seen::Elements(Box::leak(shown.into_boxed_str())))
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
            let found = pinned(&fixtures.join("standard"), &fixtures);
            if found.is_empty() {
                return Err(());
            }
            let mut differing = Vec::new();
            for (print, input) in found {
                let (Ok(text), Ok(printed)) = (
                    std::fs::read_to_string(fixtures.join(&input)),
                    std::fs::read_to_string(fixtures.join("standard").join(&print)),
                ) else {
                    return Err(());
                };
                if !agrees(&text, &printed) {
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
            Ok(Seen::Pinned(Box::leak(differing.into_boxed_slice())))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
