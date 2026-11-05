//! The `toml_parser` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxTomlParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{self, ByteSlice, Elements, Position, Symbols};
use std::path::{Path, PathBuf};

/// What a row reads: a text whose elements are listed, a text held to a dump written out, or every file pinned.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of the text.
    Tree(&'static str),
    /// A text and a dump of the standard's tree of it, written as the reference tool prints it: our tree has the standard's, or not.
    Standard(&'static str, &'static str),
    /// Every relation's input, every file of the corpus and of the suite, and every file of the allocation test, whose tree the
    /// reference tool printed, pinned in `standard/`.
    Pinned,
}
type Input = Given;
/// The elements the adapter read the text into, in order, each as its kind; what differs from the standard's tree; or the pinned
/// files whose tree is not the standard's.
type Value = &'static str;
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_value_is_a_node_with_its_key_and_its_value"),
        word: Word::Succeeds,
        input: || Given::Tree("a = 1"),
        expect: Expect::Value("toml keyval key unquoted-key ws keyval-sep ws integer"),
    },
    Case {
        name: CaseName("a_header_opens_a_table_that_holds_the_entries_below_it"),
        word: Word::Succeeds,
        input: || Given::Tree("[a . b]\nc = 'x'\n"),
        expect: Expect::Value(
            "toml std-table std-table-open key unquoted-key ws dot-sep ws unquoted-key std-table-close newline keyval key unquoted-key ws keyval-sep ws literal-string newline",
        ),
    },
    Case {
        name: CaseName("arrays_and_inline_tables_are_nodes_inside_the_key_value"),
        word: Word::Succeeds,
        input: || Given::Tree("a = [1, {b = true}] # c"),
        expect: Expect::Value(
            "toml keyval key unquoted-key ws keyval-sep ws array array-open integer array-sep ws inline-table inline-table-open keyval key unquoted-key ws keyval-sep ws boolean inline-table-close array-close ws comment",
        ),
    },
    Case {
        name: CaseName("text_the_parser_cannot_read_is_still_in_the_tree"),
        word: Word::Succeeds,
        input: || Given::Tree("a = \nb = hello\n"),
        expect: Expect::Value(
            "toml keyval key unquoted-key ws keyval-sep ws error newline keyval key unquoted-key ws keyval-sep ws error newline",
        ),
    },
    Case {
        name: CaseName("nesting_past_the_limit_is_read_no_deeper"),
        word: Word::Succeeds,
        input: || Given::Tree(NESTED),
        expect: Expect::Value(NESTED_TREE),
    },
    Case {
        name: CaseName("a_tree_that_is_not_the_standards_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "a = 1\n",
                "toml 0-6\n expression 0-5\n  ws 0-1\n newline 5-6",
            )
        },
        expect: Expect::Value("/toml/keyval: we have `keyval 0-5`, the standard has `ws 0-1`"),
    },
    Case {
        name: CaseName(
            "every_pinned_relation_corpus_suite_and_allocation_file_has_the_standards_tree",
        ),
        word: Word::Succeeds,
        input: || Given::Pinned,
        expect: Expect::Value(""),
    },
];

/// Arrays nested deeper than the library reads.
const NESTED: &str = "a = [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]";

/// What the adapter reads `NESTED` into: eighty arrays, then the rest as text the parser could not read, and their closings.
const NESTED_TREE: &str = "toml keyval key unquoted-key ws keyval-sep ws array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open array array-open error error error error error error error error error error error array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close array-close";

/// One piece of a tree held to the standard's: its rule's name, the bytes it spans and the pieces under it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shape {
    name: String,
    start: u32,
    end: u32,
    under: Vec<Shape>,
}

fn leaf(name: &str, start: u32, end: u32) -> Shape {
    Shape {
        name: name.to_owned(),
        start,
        end,
        under: Vec::new(),
    }
}

/// `under` with the white space that touches joined: a run of it is one piece in the tree, whatever the rules that matched it.
fn merged(under: Vec<Shape>) -> Vec<Shape> {
    let mut joined: Vec<Shape> = Vec::new();
    for each in under {
        match joined.last_mut() {
            Some(last) if last.name == "ws" && each.name == "ws" && last.end == each.start => {
                last.end = each.end;
            }
            Some(_) | None => joined.push(each),
        }
    }
    joined
}

/// What the dump printed, read back: `<rule> <start>-<end>` a line, a rule's parts indented one more than it. Nothing when the text
/// is no dump.
fn dumped(dump: &str) -> Option<Shape> {
    let mut open: Vec<(usize, Shape)> = Vec::new();
    let mut root = None;
    let close = |stack: &mut Vec<(usize, Shape)>, built: &mut Option<Shape>| {
        let (_, done) = stack.pop()?;
        match stack.last_mut() {
            Some((_, parent)) => parent.under.push(done),
            None => *built = Some(done),
        }
        Some(())
    };
    for line in dump.lines() {
        let text = line.trim_start();
        let depth = line.len() - text.len();
        let (name, span) = text.split_once(' ')?;
        let (start, end) = span.split_once('-')?;
        while open.last().is_some_and(|(at, _)| *at >= depth) {
            close(&mut open, &mut root)?;
        }
        open.push((depth, leaf(name, start.parse().ok()?, end.parse().ok()?)));
    }
    while !open.is_empty() {
        close(&mut open, &mut root)?;
    }
    root
}

/// The standard's rule `raw` as the kinds of the language: a rule the language has a node for is that node; one it has a token for
/// is that token, in the bytes its parts leave (`keyval-sep` is the `=`, the white space around it parts of it the language has its
/// own tokens for); the white space is `ws`; any other rule (`expression`, `val`, `simple-key`...) is only the way to its parts.
fn onto(raw: &Shape, out: &mut Vec<Shape>) {
    let name = if raw.name == "wschar" {
        "ws"
    } else {
        &raw.name
    };
    match kinds().find(|each| kind_name(*each).0 == name) {
        Some(Kind::Node(_)) => {
            let mut under = Vec::new();
            raw.under.iter().for_each(|each| onto(each, &mut under));
            out.push(Shape {
                under: merged(under),
                ..leaf(name, raw.start, raw.end)
            });
        }
        Some(Kind::Token(_) | Kind::Extra(_)) => {
            let mut cursor = raw.start;
            for each in &raw.under {
                if each.start > cursor {
                    out.push(leaf(name, cursor, each.start));
                }
                onto(each, out);
                cursor = each.end;
            }
            if cursor < raw.end {
                out.push(leaf(name, cursor, raw.end));
            }
        }
        None => raw.under.iter().for_each(|each| onto(each, out)),
    }
}

/// Our tree at `at` as the standard has it: the entries below a header are in the table the header opens, and the standard's table
/// is the header alone, so they are put after it.
fn ours(elements: &Elements, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(languages::kind_at(elements, at));
    let (mut end, mut under, mut after) =
        (languages::end_at(elements, at).0, Vec::new(), Vec::new());
    let header = matches!(kind, Kind::Node(Node::StdTable | Node::ArrayTable));
    let mut hoisting = false;
    for each in languages::children(elements, at) {
        if hoisting {
            ours(elements, each, &mut after);
        } else {
            ours(elements, each, &mut under);
            let closes = matches!(
                kind_of(languages::kind_at(elements, each)),
                Kind::Token(Token::StdTableClose | Token::ArrayTableClose)
            );
            if header && closes {
                hoisting = true;
                end = languages::end_at(elements, each).0;
            }
        }
    }
    out.push(Shape {
        under: merged(under),
        ..leaf(kind_name(kind).0, languages::start_at(elements, at).0, end)
    });
    out.extend(after);
}

/// The table the adapter reads `bytes` into.
fn read(bytes: &[u8]) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = languages::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    SyntaxTomlParserAdapter.read(SourceText(bytes), &mut builder);
    elements
}

/// What differs between our tree and the standard's, the first place found; nothing when they agree.
fn differences(ours: &Shape, theirs: &Shape, path: &str) -> Option<String> {
    let here = format!("{path}/{}", ours.name);
    if (&ours.name, ours.start, ours.end) != (&theirs.name, theirs.start, theirs.end) {
        return Some(format!(
            "{here}: we have `{} {}-{}`, the standard has `{} {}-{}`",
            ours.name, ours.start, ours.end, theirs.name, theirs.start, theirs.end
        ));
    }
    ours.under
        .iter()
        .zip(&theirs.under)
        .find_map(|(mine, yours)| differences(mine, yours, &here))
        .or_else(|| {
            (ours.under.len() != theirs.under.len()).then(|| {
                format!(
                    "{here}: we have {} pieces, the standard has {}",
                    ours.under.len(),
                    theirs.under.len()
                )
            })
        })
}

/// What differs between the tree of `bytes` and the standard's `dump`. A file the standard refuses has no tree to differ from: ours
/// must say it is refused, with a piece of text it could not read or a diagnostic.
fn against(bytes: &[u8], dump: &str) -> Option<String> {
    let elements = read(bytes);
    if dump.starts_with("refused") {
        let says = languages::diagnostic_count(&elements).0 > 0
            || languages::positions(&elements)
                .any(|at| kind_of(languages::kind_at(&elements, at)) == Kind::Extra(Extra::Error));
        return (!says).then(|| "the standard refuses it and we read it clean".to_owned());
    }
    let mut tree = Vec::new();
    ours(&elements, Position(0), &mut tree);
    match (tree.pop(), dumped(dump)) {
        (Some(mine), Some(raw)) => {
            let mut theirs = Vec::new();
            onto(&raw, &mut theirs);
            theirs
                .pop()
                .map_or(Some("the dump has no tree".to_owned()), |standard| {
                    differences(&mine, &standard, "")
                })
        }
        (None, _) => Some("our tree has no root".to_owned()),
        (_, None) => Some("the dump does not read back".to_owned()),
    }
}

/// The dumps pinned below `standard`, each with the file it was printed for, both by path below `fixtures`, sorted: a dump is its
/// file's path and `.tree`.
fn pinned(standard: &Path) -> Vec<PathBuf> {
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
            if path.extension().is_some_and(|each| each == "tree") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Tree(text) => {
            check(&SyntaxTomlParserAdapter);
            let shown = elements_of(&SyntaxTomlParserAdapter, text)
                .iter()
                .map(|(kind, _)| kind_name(*kind).0)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Box::leak(shown.into_boxed_str()))
        }
        Given::Standard(text, dump) => {
            let found = against(text.as_bytes(), dump).unwrap_or_default();
            Ok(Box::leak(found.into_boxed_str()))
        }
        Given::Pinned => {
            let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let standard = fixtures.join("standard");
            let dumps = pinned(&standard);
            if dumps.is_empty() {
                return Err(());
            }
            let mut differing = Vec::new();
            for dump in dumps {
                let below = dump
                    .strip_prefix(&standard)
                    .map_err(drop)?
                    .with_extension("");
                let (Ok(bytes), Ok(printed)) = (
                    std::fs::read(fixtures.join(&below)),
                    std::fs::read_to_string(&dump),
                ) else {
                    return Err(());
                };
                if let Some(found) = against(&bytes, &printed) {
                    differing.push(format!("{}: {found}", below.display()));
                }
            }
            Ok(Box::leak(differing.join("\n").into_boxed_str()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
