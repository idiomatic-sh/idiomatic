//! The generic target row follows parser configuration and is independent of source syntax.

use super::super::fixtures::contract::check;
use super::item::LanguageTargetItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, ReadItem, contract::Want};

fn target(version: PythonTargetVersion, source: &'static str) -> (i64, i64) {
    let language = crate::linked_with_settings(PythonParserSettings::new(version));
    let report = languages::contract::reading::<LanguageTargetItem>(
        &language,
        languages::SourcePath("item.py"),
        languages::SampleText(source),
    )
    .expect("an empty source is read");
    assert!(report.causes.is_empty());
    assert_eq!(report.rows.len(), 1);
    let [Want::File(file), Want::Number(major), Want::Number(minor)] = *report.rows[0] else {
        panic!("target row has file and numeric version columns");
    };
    assert_eq!(file, "item.py");
    (major, minor)
}

fn target_is_attached_to_source_even_when_source_is_empty() {
    check::<LanguageTargetItem>();
    for source in ["", "if:\n"] {
        assert_eq!(target(PythonTargetVersion::Py310, source), (3, 10));
        assert_eq!(target(PythonTargetVersion::Py311, source), (3, 11));
    }
}

/// A file holds one target row however many modules, classes, functions and lambdas nest in it: the row is of the file, so
/// `file` is the relation's key.
fn one_target_row_per_file_however_many_scopes_nest_in_it() {
    let nested = "import os\nclass C:\n    def f(self):\n        def g():\n            return lambda: (yield)\n        return g\n\nclass D(C):\n    x = [y for y in range(3)]\n";
    assert_eq!(target(PythonTargetVersion::Py312, nested), (3, 12));
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("target_is_attached_to_source_even_when_source_is_empty"),
        word: Word::Succeeds,
        input: || target_is_attached_to_source_even_when_source_is_empty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one_target_row_per_file_however_many_scopes_nest_in_it"),
        word: Word::Succeeds,
        input: || one_target_row_per_file_however_many_scopes_nest_in_it,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
