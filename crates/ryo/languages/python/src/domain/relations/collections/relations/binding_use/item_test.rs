//! Canonical typed-reader contract and explicit frontend refusal.
use super::super::fixtures::contract::check;
use super::item::BindingUseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::proposed;
use ryo::languages::contract::Reading;
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty module has complete empty binding facts"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("malformed source does not prove absence"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["binding analysis requires complete supported syntax"],
        }),
    },
];
#[test]
fn succeeds() {
    check::<BindingUseItem>();
    run(CASES, Word::Succeeds, |input| {
        Ok(proposed::<BindingUseItem>(input))
    });
}

/// The `binding-use` rows of `source` read under `path` at Python 3.10: each as its reference's and its definition's text.
fn uses_of(path: &str, source: &'static str) -> Vec<String> {
    let report = ryo::languages::contract::reading::<BindingUseItem>(
        &crate::linked_with_settings(crate::settings::PythonParserSettings::new(
            crate::settings::PythonTargetVersion::Py310,
        )),
        ryo::languages::SourcePath(path),
        ryo::languages::SampleText(source),
    )
    .expect("Python source is recognized");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    report.rows.iter().map(|row| format!("{row:?}")).collect()
}

/// A stub is never run: its annotations are postponed, as under the future import, so an annotation reads the class
/// defined after it; a module's annotation evaluated where it stands does not.
#[test]
fn stub_annotations_read_definitions_that_follow_them() {
    const LATER: &str = "def f(value: Later) -> Later: ...\nclass Later: ...\n";
    const FUTURE: &str =
        "from __future__ import annotations\ndef f(value: Later) -> Later: ...\nclass Later: ...\n";
    let counts = (
        uses_of("case.py", LATER).len(),
        uses_of("case.pyi", LATER).len(),
        uses_of("case.py", FUTURE).len(),
    );
    assert_eq!(counts, (0, 2, 2), "{:?}", uses_of("case.pyi", LATER));
}
