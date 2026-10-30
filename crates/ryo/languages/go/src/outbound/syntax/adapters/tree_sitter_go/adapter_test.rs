//! The `tree_sitter_go` adapter runs the port's contract, and its own cases: the tree it writes for a file is the tree `go/ast` makes of
//! it, the tokens of the table spell the file, and what the library says of a broken file is reported.

use super::adapter::SyntaxTreeSitterGoAdapter;
use super::fixtures::walk::{
    Acceptance, acceptance, differs, parsed, pinned, refused_files, walked,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::outbound::syntax::fixtures::contract::check;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::contract::Walked;
use ryo::languages::{self, ByteSlice, Elements, Position, Symbols};

/// What a row reads.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The elements of a file, one line each: how deep, its kind and the bytes it spans.
    Outline(&'static [u8]),
    /// Whether the kinds of the elements of a file, white space left out, hold the kinds in a row, in order.
    Holds(&'static [u8], &'static str),
    /// How many elements of a file are of the kind, whatever the white space between them.
    Count(&'static [u8], &'static str),
    /// Files whose tokens, joined, must spell them.
    Spell(&'static [&'static [u8]]),
    /// How many things the library says of a file.
    Diagnoses(&'static [u8]),
    /// A file read twice.
    Twice(&'static [u8]),
    /// A file and a tree pinned for it: our tree is the pinned tree, or it differs from it.
    Standard(&'static str, &'static str),
    /// Every relation's input, every file of the corpus and every file of the suite, whose tree `go/parser` made is pinned in
    /// `standard/output.txt`.
    Pinned,
    /// Every file `go/parser` refuses that is held to a syntax error alone, in `suite/refused`.
    Refused,
}

type Input = Given;
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The lines of the outline, joined with ` | `.
    Outline(&'static str),
    /// Whether the kinds are in a row.
    Holds(bool),
    /// How many.
    Count(usize),
    /// Whether the tokens of each file, joined, are the file.
    Spelled(bool),
    /// How many things the library said.
    Diagnoses(usize),
    /// Whether the two readings are the same.
    Same(bool),
    /// What differs between our tree and the pinned one, the first place found; nothing when they agree.
    Standard(Option<&'static str>),
    /// The pinned files whose tree differs from the standard's, by path below `tests/fixtures`.
    Pinned(&'static [&'static str]),
    /// The refused files that `go/parser` accepts or the language reads without a syntax error, by path below `tests/fixtures`.
    Refused(&'static [&'static str]),
}

/// The table the adapter reads `bytes` into.
fn table(bytes: &[u8]) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = languages::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    SyntaxTreeSitterGoAdapter.read(SourceText(bytes), &mut builder);
    elements
}

/// How deep the element at `at` is: its distance from the root.
fn depth(elements: &Elements, at: Position) -> usize {
    let (mut up, mut depth) = (at, 0);
    while languages::parent_at(elements, up) != up {
        up = languages::parent_at(elements, up);
        depth += 1;
    }
    depth
}

/// The elements of the table, one line each: how deep, its kind and the bytes it spans.
fn outline(bytes: &[u8]) -> Vec<String> {
    let elements = table(bytes);
    languages::positions(&elements)
        .map(|at| {
            format!(
                "{}{} {}..{}",
                " ".repeat(depth(&elements, at)),
                kind_name(kind_of(languages::kind_at(&elements, at))).0,
                languages::start_at(&elements, at).0,
                languages::end_at(&elements, at).0
            )
        })
        .collect()
}

/// The kinds of the elements, white space left out.
fn kinds(bytes: &[u8]) -> Vec<String> {
    outline(bytes)
        .into_iter()
        .map(|line| {
            line.trim_start()
                .split(' ')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .filter(|kind| kind != "whitespace")
        .collect()
}

/// Our tree of the file, written as the Go program writes the standard's and read back the same way.
fn ours(bytes: &[u8]) -> Walked {
    let elements = table(bytes);
    let lines: Vec<String> = languages::positions(&elements)
        .map(|at| {
            format!(
                "tree {} {} {} {} {}",
                depth(&elements, at),
                if languages::flags_at(&elements, at).0 & languages::TOKEN.0 == 0 {
                    "N"
                } else {
                    "T"
                },
                kind_name(kind_of(languages::kind_at(&elements, at))).0,
                languages::start_at(&elements, at).0,
                languages::end_at(&elements, at).0
            )
        })
        .collect();
    parsed(&lines.join("\n"))
}

/// Whether the tokens of the table the adapter reads `bytes` into, joined, are `bytes`.
fn spelled(bytes: &[u8]) -> bool {
    let elements = table(bytes);
    let mut joined = Vec::new();
    for at in languages::positions(&elements) {
        if languages::flags_at(&elements, at).0 & languages::TOKEN.0 != 0 {
            let (start, end) = (
                usize::try_from(languages::start_at(&elements, at).0).unwrap_or(0),
                usize::try_from(languages::end_at(&elements, at).0).unwrap_or(0),
            );
            joined.extend_from_slice(bytes.get(start..end).unwrap_or_default());
        }
    }
    joined == bytes
}

/// Leaks `text` for the life of the process: the value of a row is `'static`, and a test is short.
fn leaked(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_is_a_func_decl_over_a_func_type_that_starts_at_func"),
        word: Word::Succeeds,
        input: || Given::Outline(b"package p\nfunc f(a int) {}\n"),
        expect: Expect::Value(Seen::Outline(
            "file 0..27 |  package-token 0..7 |  whitespace 7..8 |  ident 8..9 |   ident-token 8..9 |  whitespace 9..10 |  func-decl 10..26 |   func-type 10..23 |    func 10..14 |    whitespace 14..15 |    ident 15..16 |     ident-token 15..16 |    field-list 16..23 |     lparen 16..17 |     field 17..22 |      ident 17..18 |       ident-token 17..18 |      whitespace 18..19 |      ident 19..22 |       ident-token 19..22 |     rparen 22..23 |   whitespace 23..24 |   block-stmt 24..26 |    lbrace 24..25 |    rbrace 25..26 |  whitespace 26..27",
        )),
    },
    Case {
        name: CaseName("the_tokens_of_a_file_spell_it"),
        word: Word::Succeeds,
        input: || {
            Given::Spell(&[
                b"package p\n\n// c\nfunc f() { return }\n",
                b"package p\nvar a = `r\r\nw`\n",
                b"",
                b"   \n",
                b"package p\nfunc (",
                b"\xef\xbb\xbfpackage p\n",
                b"package p\n\xff\xfe",
            ])
        },
        expect: Expect::Value(Seen::Spelled(true)),
    },
    Case {
        name: CaseName("a_byte_order_mark_is_white_space"),
        word: Word::Succeeds,
        input: || Given::Outline(b"\xef\xbb\xbfpackage p\n"),
        expect: Expect::Value(Seen::Outline(
            "file 0..13 |  whitespace 0..3 |  package-token 3..10 |  whitespace 10..11 |  ident 11..12 |   ident-token 11..12 |  whitespace 12..13",
        )),
    },
    Case {
        name: CaseName("bytes_that_are_not_utf8_are_an_error_token_and_a_report"),
        word: Word::Succeeds,
        input: || Given::Outline(b"package p\n\xff\xfe"),
        expect: Expect::Value(Seen::Outline(
            "file 0..12 |  package-token 0..7 |  whitespace 7..8 |  ident 8..9 |   ident-token 8..9 |  whitespace 9..10 |  error 10..12",
        )),
    },
    Case {
        name: CaseName("bytes_that_are_not_utf8_are_reported_once"),
        word: Word::Succeeds,
        input: || Given::Diagnoses(b"package p\n\xff\xfe"),
        expect: Expect::Value(Seen::Diagnoses(1)),
    },
    Case {
        name: CaseName("a_file_that_is_well_formed_has_nothing_reported"),
        word: Word::Succeeds,
        input: || Given::Diagnoses(b"package p\nfunc f() {}\n"),
        expect: Expect::Value(Seen::Diagnoses(0)),
    },
    Case {
        name: CaseName("a_file_that_is_broken_still_has_its_tree_and_a_report"),
        word: Word::Succeeds,
        input: || Given::Diagnoses(b"package p\nfunc f( {\n"),
        expect: Expect::Value(Seen::Diagnoses(1)),
    },
    Case {
        name: CaseName("reading_a_file_twice_gives_the_same_tree"),
        word: Word::Succeeds,
        input: || Given::Twice(b"package p\n\nfunc f() { x := []int{1, 2}; _ = x }\n"),
        expect: Expect::Value(Seen::Same(true)),
    },
    Case {
        name: CaseName("a_group_of_parameters_that_share_a_type_is_one_field_however_long"),
        word: Word::Succeeds,
        input: || {
            Given::Count(
                b"package p\nfunc f(a, b, c, d, e, f, g, h, i, j, k, l *byte) {}\n",
                "field",
            )
        },
        expect: Expect::Value(Seen::Count(1)),
    },
    Case {
        name: CaseName("a_declaration_after_a_label_is_a_statement"),
        word: Word::Succeeds,
        input: || {
            Given::Holds(
                b"package p\nfunc f() {\nL:\n\tvar a int\n}\n",
                "labeled-stmt ident ident-token colon decl-stmt gen-decl",
            )
        },
        expect: Expect::Value(Seen::Holds(true)),
    },
    Case {
        name: CaseName("the_star_of_a_conversion_to_a_pointer_is_the_star_of_the_conversion"),
        word: Word::Succeeds,
        input: || {
            Given::Holds(
                b"package p\nvar a = *(*[8]byte)(s)\n",
                "star-expr mul call-expr paren-expr",
            )
        },
        expect: Expect::Value(Seen::Holds(true)),
    },
    Case {
        name: CaseName("a_label_with_no_statement_has_the_empty_statement_go_parser_makes"),
        word: Word::Succeeds,
        input: || {
            Given::Holds(
                b"package p\nfunc f() {\nL:\n}\n",
                "labeled-stmt ident ident-token colon empty-stmt rbrace",
            )
        },
        expect: Expect::Value(Seen::Holds(true)),
    },
    Case {
        name: CaseName("a_default_clause_in_a_select_is_a_comm_clause"),
        word: Word::Succeeds,
        input: || {
            Given::Holds(
                b"package p\nfunc f() {\n\tselect {\n\tdefault:\n\t}\n}\n",
                "select-stmt select block-stmt lbrace comm-clause default",
            )
        },
        expect: Expect::Value(Seen::Holds(true)),
    },
    Case {
        name: CaseName("a_type_switch_has_the_assignment_and_the_assertion_go_parser_makes"),
        word: Word::Succeeds,
        input: || {
            Given::Holds(
                b"package p\nfunc f() {\n\tswitch t := x.(type) {\n\t}\n}\n",
                "type-switch-stmt switch assign-stmt ident ident-token define type-assert-expr",
            )
        },
        expect: Expect::Value(Seen::Holds(true)),
    },
    Case {
        name: CaseName("a_tree_that_is_not_go_parsers_is_a_difference"),
        word: Word::Succeeds,
        input: || {
            Given::Standard(
                "package p\n",
                "tree 0 N file 0 10\ntree 1 T package-token 0 7\n",
            )
        },
        expect: Expect::Value(Seen::Standard(Some(
            "root: we nest 4 elements under file, the standard nests 1",
        ))),
    },
    Case {
        name: CaseName("every_pinned_relation_corpus_and_suite_file_has_go_parsers_tree"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        expect: Expect::Value(Seen::Pinned(&[])),
    },
    Case {
        name: CaseName(
            "every_file_go_parser_refuses_and_the_language_does_not_read_to_its_tree_has_a_syntax_error",
        ),
        word: Word::Succeeds,
        input: || Given::Refused,
        expect: Expect::Value(Seen::Refused(&[])),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    Ok(match given {
        Given::Outline(bytes) => Seen::Outline(leaked(outline(bytes).join(" | "))),
        Given::Holds(bytes, window) => {
            let window: Vec<&str> = window.split(' ').collect();
            Seen::Holds(
                kinds(bytes)
                    .windows(window.len())
                    .any(|each| each.iter().eq(window.iter())),
            )
        }
        Given::Count(bytes, kind) => {
            Seen::Count(kinds(bytes).iter().filter(|each| *each == kind).count())
        }
        Given::Spell(files) => Seen::Spelled(files.iter().all(|bytes| spelled(bytes))),
        Given::Diagnoses(bytes) => Seen::Diagnoses(languages::diagnostic_count(&table(bytes)).0),
        Given::Twice(bytes) => Seen::Same(outline(bytes) == outline(bytes)),
        Given::Standard(text, block) => {
            Seen::Standard(differs(&ours(text.as_bytes()), &parsed(block)).map(leaked))
        }
        Given::Refused => {
            let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let files = refused_files(&fixtures);
            if files.is_empty() {
                return Err(());
            }
            let mut failing = Vec::new();
            for path in files {
                let bytes = std::fs::read(&path).map_err(drop)?;
                let reported = languages::diagnostic_count(&table(&bytes)).0 > 0;
                let refused = acceptance(&String::from_utf8_lossy(&bytes)) == Acceptance::Refused;
                if !(reported && refused) {
                    failing.push(leaked(
                        path.strip_prefix(&fixtures)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .into_owned(),
                    ));
                }
            }
            Seen::Refused(Box::leak(failing.into_boxed_slice()))
        }
        Given::Pinned => {
            let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            if !fixtures.join("standard").join("output.txt").is_file() {
                return Err(());
            }
            let files = pinned(&fixtures);
            if files.is_empty() {
                return Err(());
            }
            let mut differing = Vec::new();
            for path in files {
                let bytes = std::fs::read(&path).map_err(drop)?;
                let text = String::from_utf8_lossy(&bytes);
                if differs(&ours(&bytes), &walked(&text)).is_some() {
                    differing.push(leaked(
                        path.strip_prefix(&fixtures)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .into_owned(),
                    ));
                }
            }
            Seen::Pinned(Box::leak(differing.into_boxed_slice()))
        }
    })
}

#[test]
fn succeeds() {
    check(&SyntaxTreeSitterGoAdapter);
    run(CASES, Word::Succeeds, subject);
}
