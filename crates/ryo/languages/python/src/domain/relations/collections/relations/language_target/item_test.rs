//! The generic target row follows parser configuration and is independent of source syntax.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::LanguageTargetItem;
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
    let [Want::File(file), Want::Number(major), Want::Number(minor)] = **report.rows[0] else {
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

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("target_is_attached_to_source_even_when_source_is_empty"),
        word: Word::Succeeds,
        input: || target_is_attached_to_source_even_when_source_is_empty,
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
