//! The `manifest_diagnostic` item's cases: the manifests refused become its rows, each cell read back.

use super::item::ManifestDiagnosticItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::{Diagnosed, Stage};
use crate::domain::workspaces::types::Reason;
use crate::fixtures::reading::SHOWN;
use ryo::languages;
use ryo::languages::contract::{Reading, want_file, want_name, want_span, written};

/// The manifests refused, as the domain says them.
type Input = fn() -> Vec<Diagnosed>;
/// The rows the item wrote, every cell of each.
type Value = Reading;
type Failure = ();

fn none() -> Vec<Diagnosed> {
    Vec::new()
}

fn unreadable() -> Vec<Diagnosed> {
    vec![Diagnosed {
        file: languages::FileId(1),
        stage: Stage::Parse,
        reason: Reason("unclosed table, expected `]`".into()),
    }]
}

fn stranded() -> Vec<Diagnosed> {
    vec![
        Diagnosed {
            file: languages::FileId(0),
            stage: Stage::Validation,
            reason: Reason("current package believes it's in a workspace when it's not".into()),
        },
        Diagnosed {
            file: languages::FileId(1),
            stage: Stage::Parse,
            reason: Reason("unclosed table, expected `]`".into()),
        },
    ]
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_manifest_refused_gives_no_row"),
        word: Word::Succeeds,
        input: || none,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_reader_refused_is_a_row_of_the_parse_at_the_file_origin"),
        word: Word::Succeeds,
        input: || unreadable,
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("b/Cargo.toml"),
                want_span((1, 1), (1, 1)),
                want_name("file-origin"),
                want_name("parse"),
                want_name("cargo-manifest"),
                want_name("unreadable-manifest"),
                want_name("unclosed table, expected `]`"),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_workspace_that_does_not_hold_the_package_is_a_row_of_the_validation"),
        word: Word::Succeeds,
        input: || stranded,
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_file("a/Cargo.toml"),
                    want_span((1, 1), (1, 1)),
                    want_name("file-origin"),
                    want_name("validation"),
                    want_name("cargo-manifest"),
                    want_name("inconsistent-workspace"),
                    want_name("current package believes it's in a workspace when it's not"),
                ],
                &[
                    want_file("b/Cargo.toml"),
                    want_span((1, 1), (1, 1)),
                    want_name("file-origin"),
                    want_name("parse"),
                    want_name("cargo-manifest"),
                    want_name("unreadable-manifest"),
                    want_name("unclosed table, expected `]`"),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(given: Input) -> Value {
    let diagnosed = given();
    let mut symbols = languages::Symbols::default();
    let mut out = languages::contract::Collect::default();
    ManifestDiagnosticItem::rows(&diagnosed, &mut symbols, &mut out);
    written(
        out,
        symbols,
        (&["a/Cargo.toml", "b/Cargo.toml"], &[]),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::ManifestDiagnostic,
        ),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
