//! The `declarations` service's cases, on values alone: what a file of code declares, read from the rows any language
//! gives of the shared relations, and how a file's declarations compare with its template's.

use super::service::declarations;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::comments::types::ByteSpan;
use crate::domain::contracts::service::{compare, facts, shown};
use crate::domain::contracts::types::{
    DataRow, RowKey, RowNode, RowPlace, Syntax, SyntaxKind, WrittenValue,
};
use crate::domain::definitions::types::Exactness;
use crate::domain::findings::types::ByteOffset;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The declarations of a file of a class with a method whose body declares a local, shown.
    Nested,
    /// The mismatches of a file whose decoration passes more arguments than its template's.
    MoreArguments,
    /// The mismatches of a file whose module, and whose function, carries a decoration its template's does not.
    ExtraDecoration,
    /// The mismatches of a file whose declaration lacks its template's visibility.
    LostVisibility,
    /// The mismatches of a file whose constant has another type than its template's.
    OtherType,
    /// The mismatches of a file whose own scope carries a decoration its template's does not.
    FileDecoration,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_declaration_is_named_within_the_declarations_whose_scopes_hold_it"),
        word: Word::Succeeds,
        input: || Question::Nested,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_decoration_is_held_one_argument_at_a_time"),
        word: Word::Succeeds,
        input: || Question::MoreArguments,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_decoration_beside_a_modules_own_is_a_mismatch_and_beside_a_functions_is_not",
        ),
        word: Word::Succeeds,
        input: || Question::ExtraDecoration,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_visibility_is_part_of_what_is_declared"),
        word: Word::Succeeds,
        input: || Question::LostVisibility,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_value_is_declared_with_its_type"),
        word: Word::Succeeds,
        input: || Question::OtherType,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_may_decorate_its_own_scope_beyond_its_template"),
        word: Word::Succeeds,
        input: || Question::FileDecoration,
        expect: Expect::Value(String::new()),
    },
];

/// A row of `syntax`, at `start..end`.
fn row(
    syntax: Syntax,
    (id, owner): (&str, &str),
    (key, kind, text): (&str, &str, &str),
    (start, end): (usize, usize),
) -> DataRow {
    DataRow {
        syntax,
        id: RowNode(id.into()),
        owner: RowNode(owner.into()),
        key: RowKey(key.into()),
        kind: SyntaxKind(kind.into()),
        text: WrittenValue(text.into()),
        span: RowPlace::At(ByteSpan {
            start: ByteOffset(start),
            end: ByteOffset(end),
        }),
    }
}

/// A file's module scope.
fn module() -> DataRow {
    row(Syntax::Scopes, ("0", "0"), ("", "", ""), (0, 100))
}

/// A file of a class `C` with a method `m` whose body declares a local `x`.
fn nested() -> Vec<DataRow> {
    vec![
        module(),
        row(Syntax::Defines, ("1", "0"), ("C", "class", "class"), (6, 7)),
        row(Syntax::Scopes, ("1", "0"), ("", "", ""), (0, 60)),
        row(
            Syntax::Defines,
            ("2", "1"),
            ("m", "method", "method"),
            (14, 15),
        ),
        row(Syntax::Scopes, ("2", "1"), ("", "", ""), (10, 50)),
        row(
            Syntax::Defines,
            ("3", "2"),
            ("x", "variable", "variable"),
            (20, 21),
        ),
    ]
}

/// A file of one function `f`, with the decorations, visibility and other rows given.
fn function(more: &[DataRow]) -> Vec<DataRow> {
    let mut rows = vec![
        module(),
        row(
            Syntax::Defines,
            ("1", "0"),
            ("f", "function", "function"),
            (14, 15),
        ),
        row(Syntax::Scopes, ("1", "0"), ("", "", ""), (10, 40)),
    ];
    rows.extend_from_slice(more);
    rows
}

/// A file of one module `m`, declared with no body, with the decorations given.
fn namespace(more: &[DataRow]) -> Vec<DataRow> {
    let mut rows = vec![
        module(),
        row(
            Syntax::Defines,
            ("1", "0"),
            ("m", "namespace", "module"),
            (14, 15),
        ),
    ];
    rows.extend_from_slice(more);
    rows
}

/// A decoration `target(arguments)` of the declaration `of`.
fn decoration(id: &str, of: &str, target: &str, arguments: &str) -> DataRow {
    row(Syntax::Decorates, (id, of), (target, "", arguments), (2, 9))
}

/// A constant `K` annotated with `annotation`.
fn constant(annotation: &str) -> Vec<DataRow> {
    vec![
        module(),
        row(
            Syntax::Defines,
            ("1", "0"),
            ("K", "constant", "const"),
            (6, 7),
        ),
        row(Syntax::Annotates, ("2", "1"), ("", "", annotation), (9, 12)),
    ]
}

/// The mismatches of `found` with the template's `expected`, each shown.
fn mismatches(expected: &[DataRow], found: &[DataRow], exact: Exactness) -> String {
    compare(
        &declarations(&facts(expected)),
        &declarations(&facts(found)),
        exact,
    )
    .iter()
    .map(|mismatch| format!("{mismatch:?}"))
    .collect::<Vec<_>>()
    .join("; ")
}

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Nested => (
            declarations(&facts(&nested()))
                .iter()
                .map(|declaration| shown(declaration).0)
                .collect::<Vec<_>>()
                .join("; "),
            "; class C; method m in C".to_owned(),
        ),
        Question::MoreArguments => (
            mismatches(
                &function(&[decoration("5", "1", "derive", "A")]),
                &function(&[decoration("5", "1", "derive", "B, A")]),
                Exactness::Exact,
            ),
            String::new(),
        ),
        Question::ExtraDecoration => {
            let count = |declared: fn(&[DataRow]) -> Vec<DataRow>| {
                mismatches(
                    &declared(&[decoration("5", "1", "path", "\"x\"")]),
                    &declared(&[
                        decoration("5", "1", "path", "\"x\""),
                        decoration("6", "1", "cfg", "any()"),
                    ]),
                    Exactness::Contains,
                )
                .split("; ")
                .filter(|said| !said.is_empty())
                .count()
            };
            (
                format!("{} {}", count(namespace), count(function)),
                "1 0".to_owned(),
            )
        }
        Question::LostVisibility => (
            mismatches(
                &function(&[row(
                    Syntax::Visibility,
                    ("5", "1"),
                    ("", "", "pub"),
                    (10, 13),
                )]),
                &function(&[]),
                Exactness::Contains,
            )
            .split("; ")
            .filter(|said| !said.is_empty())
            .count()
            .to_string(),
            "1".to_owned(),
        ),
        Question::OtherType => (
            mismatches(&constant("T"), &constant("U"), Exactness::Exact)
                .split("; ")
                .filter(|said| !said.is_empty())
                .count()
                .to_string(),
            "2".to_owned(),
        ),
        Question::FileDecoration => (
            mismatches(
                &function(&[]),
                &function(&[decoration("5", "0", "allow", "x")]),
                Exactness::Contains,
            ),
            String::new(),
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
