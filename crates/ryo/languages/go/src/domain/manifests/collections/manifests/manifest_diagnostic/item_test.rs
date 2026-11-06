//! The `manifest_diagnostic` item's cases, every row in full.

use super::item::ManifestDiagnosticItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{
    Reading, WITH_A_BROKEN_MANIFEST, WITH_A_BROKEN_WORKSPACE, WORKSPACE, reading,
};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("manifests_the_tool_reads_say_nothing"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_mod_the_tool_refuses_is_said_with_its_reason"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[
                "b/go.mod at 0..0 \"file-origin\" \"parse\" \"go-module\" \"invalid-go-mod\" \"line 2: the `require (` block is not closed\"",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_work_the_tool_refuses_is_said_with_its_reason"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[
                "go.work at 0..0 \"file-origin\" \"parse\" \"go-workspace\" \"invalid-go-work\" \"line 3: the `use (` block is not closed\"",
            ],
            causes: &[],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<ManifestDiagnosticItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
