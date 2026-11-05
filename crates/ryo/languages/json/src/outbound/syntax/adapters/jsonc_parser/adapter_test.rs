// the adapter's rows and the one comparison they rest on: its trees held to the JSON standard's pinned dumps (read back, mapped onto the language's kinds, compared piece by piece), which only these rows use; the adapter's fixtures roles (oracle, walk) hold the library's own readings, not the standard's
//! The `jsonc_parser` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxJsoncParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::language::service::dialect;
use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::domain::syntax::types::{Dialect, Kind};
use crate::outbound::syntax::fixtures::contract::{check, elements_of};
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{self, ByteSlice, Elements, Position, SourcePath, Symbols};
use std::path::{Path, PathBuf};

/// What a row reads: a text whose elements are listed, a text whose reports are listed, a text held to a dump written out, or every
/// file pinned.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of the text read as json with comments.
    Tree(&'static str),
    /// What the adapter reports of the text read in a dialect.
    Reports(&'static str, Dialect),
    /// A text and a dump of the standard's tree of it, written as the reference tool prints it: our tree has the standard's, or not.
    Standard(&'static str, &'static str),
    /// Every relation's input, every file of the corpus and of the suite, and every file of the allocation test, whose tree the
    /// reference tool printed, pinned in `standard/`.
    Pinned,
}
type Input = Given;
/// The elements the adapter read the text into, in order, each as its kind; what it reported; what differs from the standard's tree;
/// or the pinned files whose tree is not the standard's.
type Value = &'static str;
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_member_is_a_node_with_its_name_its_colon_and_its_value"),
        word: Word::Succeeds,
        input: || Given::Tree("{\"a\": 1}"),
        expect: Expect::Value(
            "json-text object begin-object member string name-separator ws number end-object",
        ),
    },
    Case {
        name: CaseName("an_array_is_a_node_around_its_values_and_commas"),
        word: Word::Succeeds,
        input: || Given::Tree("[true, null]"),
        expect: Expect::Value("json-text array begin-array true value-separator ws null end-array"),
    },
    Case {
        name: CaseName("comments_are_tokens_where_they_stand"),
        word: Word::Succeeds,
        input: || Given::Tree("// c\n[1, /* b */ 2]"),
        expect: Expect::Value(
            "json-text line-comment ws array begin-array number value-separator ws block-comment ws number end-array",
        ),
    },
    Case {
        name: CaseName("a_trailing_comma_is_a_token_of_its_array"),
        word: Word::Succeeds,
        input: || Given::Tree("[1,]"),
        expect: Expect::Value("json-text array begin-array number value-separator end-array"),
    },
    Case {
        name: CaseName("text_the_library_cannot_read_is_still_in_the_tree"),
        word: Word::Succeeds,
        input: || Given::Tree("{\"a\": ]"),
        expect: Expect::Value("json-text begin-object string name-separator ws end-array"),
    },
    Case {
        name: CaseName("a_comment_in_json_is_reported"),
        word: Word::Succeeds,
        input: || Given::Reports("{} // c", Dialect::Strict),
        expect: Expect::Value(
            "comments are not allowed in JSON (RFC 8259): only a file read as json with comments has them 3-7",
        ),
    },
    Case {
        name: CaseName("a_trailing_comma_in_json_is_reported"),
        word: Word::Succeeds,
        input: || Given::Reports("[1,]", Dialect::Strict),
        expect: Expect::Value(
            "a trailing comma is not allowed in JSON (RFC 8259): only a file read as json with comments has one 2-3",
        ),
    },
    Case {
        name: CaseName("a_file_with_no_value_in_json_is_reported"),
        word: Word::Succeeds,
        input: || Given::Reports("\n", Dialect::Strict),
        expect: Expect::Value("expected a value: a JSON text is one value (RFC 8259) 0-0"),
    },
    Case {
        name: CaseName("comments_trailing_commas_and_no_value_are_json_with_comments"),
        word: Word::Succeeds,
        input: || Given::Reports("// c\n[1,]\n", Dialect::Jsonc),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("what_the_library_cannot_read_is_reported_in_both_dialects"),
        word: Word::Succeeds,
        input: || Given::Reports("{\"a\": [1, 2}", Dialect::Jsonc),
        expect: Expect::Value("Unexpected close brace on line 1 column 12 11-12"),
    },
    Case {
        name: CaseName("a_control_character_in_a_string_is_reported_in_both_dialects"),
        word: Word::Succeeds,
        input: || Given::Reports("[\"a\tb\"]", Dialect::Jsonc),
        expect: Expect::Value(
            "a control character in a string must be escaped (RFC 8259, section 7) 3-4",
        ),
    },
    Case {
        name: CaseName("a_tree_that_is_not_the_standards_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "[1]",
                "json-text 0-3\n value 0-3\n  object 0-3\n   begin-object 0-1\n   end-object 2-3",
            )
        },
        expect: Expect::Value(
            "/json-text/array: we have `array 0-3`, the standard has `object 0-3`",
        ),
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

/// A piece with the white space left out of it and its bytes those from its first to its last piece that is not white space: the
/// standard's separators own the white space around them (and its `end-object` the white space after a document's last value),
/// where our trees leave it to the piece around, so the two are compared by what they hold. Where the white space stands is held
/// by the rail that reads the tokens back byte for byte.
fn bare(mut piece: Shape) -> Shape {
    piece.under = piece
        .under
        .into_iter()
        .filter(|each| each.name != "ws")
        .map(bare)
        .collect();
    if let (Some(first), Some(last)) = (piece.under.first(), piece.under.last()) {
        (piece.start, piece.end) = (first.start, last.end);
    }
    piece
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
/// is that token, in the bytes its parts leave (`name-separator` is the `:`, the white space around it parts of it the language has
/// its own tokens for); any other rule (`value`) is only the way to its parts.
fn onto(raw: &Shape, out: &mut Vec<Shape>) {
    match kinds().find(|each| kind_name(*each).0 == raw.name) {
        Some(Kind::Node(_)) => {
            let mut under = Vec::new();
            raw.under.iter().for_each(|each| onto(each, &mut under));
            out.push(Shape {
                under,
                ..leaf(&raw.name, raw.start, raw.end)
            });
        }
        Some(Kind::Token(_) | Kind::Extra(_)) => {
            let mut cursor = raw.start;
            for each in &raw.under {
                if each.start > cursor {
                    out.push(leaf(&raw.name, cursor, each.start));
                }
                onto(each, out);
                cursor = each.end;
            }
            if cursor < raw.end {
                out.push(leaf(&raw.name, cursor, raw.end));
            }
        }
        None => raw.under.iter().for_each(|each| onto(each, out)),
    }
}

/// Our tree at `at` as the standard has it.
fn ours(elements: &Elements, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(languages::kind_at(elements, at));
    let mut under = Vec::new();
    for each in languages::children(elements, at) {
        ours(elements, each, &mut under);
    }
    out.push(Shape {
        under,
        ..leaf(
            kind_name(kind).0,
            languages::start_at(elements, at).0,
            languages::end_at(elements, at).0,
        )
    });
}

/// The table the adapter reads `bytes` into, as a file read in `dialect`.
fn read(bytes: &[u8], dialect: Dialect) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = languages::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    SyntaxJsoncParserAdapter.read(SourceText(bytes), dialect, &mut builder);
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

/// Whether the table says something is wrong with the file: a diagnostic, or a piece of text it could not read.
fn says_wrong(elements: &Elements) -> bool {
    languages::diagnostic_count(elements).0 > 0
        || languages::positions(elements)
            .any(|at| kind_of(languages::kind_at(elements, at)) == Kind::Extra(Extra::Error))
}

/// What differs between the tree of `bytes`, read in `dialect`, and the standard's `dump`. A file the standard refuses has no tree to
/// differ from: read as json it must say it is refused, with a piece of text it could not read or a diagnostic; read as json with
/// comments it may be read clean, which the standard does not describe, and must say nothing is wrong when the file is a clean one.
fn against(bytes: &[u8], dialect: Dialect, dump: &str) -> Option<String> {
    let elements = read(bytes, dialect);
    if dump.starts_with("refused") {
        return match dialect {
            Dialect::Strict => (!says_wrong(&elements))
                .then(|| "the standard refuses it and we read it clean".to_owned()),
            Dialect::Jsonc => None,
        };
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
                    differences(&bare(mine), &bare(standard), "")
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

/// What the adapter reports of `text` read in `dialect`: each report's message and the bytes it is about, one to a line.
fn reports(text: &str, dialect: Dialect) -> String {
    let elements = read(text.as_bytes(), dialect);
    languages::diagnostics(&elements)
        .map(|each| {
            format!(
                "{} {}-{}",
                String::from_utf8_lossy(each.message.0),
                each.start.0,
                each.end.0
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Tree(text) => {
            check(&SyntaxJsoncParserAdapter);
            let shown = elements_of(&SyntaxJsoncParserAdapter, text, Dialect::Jsonc)
                .iter()
                .map(|(kind, _)| kind_name(*kind).0)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Box::leak(shown.into_boxed_str()))
        }
        Given::Reports(text, dialect) => Ok(Box::leak(reports(text, dialect).into_boxed_str())),
        Given::Standard(text, dump) => {
            let found = against(text.as_bytes(), Dialect::Strict, dump).unwrap_or_default();
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
                // Joined by `/` on every platform, as the engine hands a language every path.
                let shown = below
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if let Some(found) = against(&bytes, dialect(SourcePath(&shown)), &printed) {
                    differing.push(format!("{shown}: {found}"));
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
