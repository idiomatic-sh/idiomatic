//! The `manifest_diagnostic` item's cases, every row in full: where a manifest the tool cannot read or whose metadata it refuses is
//! wrong, and why.

use super::item::ManifestDiagnosticItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{Reading, Show, Shown, WITH_A_BROKEN_MANIFEST, WORKSPACE, reading};
use ryo::languages;

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// A repository whose one manifest is valid toml that names its project with a number.
const A_NUMBER_FOR_A_NAME: &[(&str, &str)] =
    &[("pyproject.toml", "[project]\nname = 3\n"), ("a.py", "")];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("valid_manifests_have_no_diagnostic"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_has_a_parse_diagnostic_at_its_span"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[
                "b/pyproject.toml at 8..8 \"span\" \"parse\" \"python-project-metadata\" \"invalid-toml\" \"unclosed table, expected `]`\"",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("metadata_the_schema_refuses_has_a_validation_diagnostic"),
        word: Word::Succeeds,
        input: || A_NUMBER_FOR_A_NAME,
        expect: Expect::Value(Reading {
            rows: &[
                "pyproject.toml at 17..18 \"span\" \"validation\" \"python-project-metadata\" \"invalid-project-metadata\" \"invalid type: integer `3`, expected a string\"",
            ],
            causes: &[],
        }),
    },
];

impl Show for languages::ManifestDiagnosticRow {
    /// The file and span, then each column of text.
    fn show(&self, shown: &Shown<'_>) -> String {
        format!(
            "{} at {}..{} {} {} {} {} {}",
            shown.file(self.file),
            self.span.start.0,
            self.span.end.0,
            shown.name(self.location_kind),
            shown.name(self.phase),
            shown.name(self.schema),
            shown.name(self.code),
            shown.name(self.message)
        )
    }
}

fn subject(files: Input) -> Value {
    reading::<ManifestDiagnosticItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
