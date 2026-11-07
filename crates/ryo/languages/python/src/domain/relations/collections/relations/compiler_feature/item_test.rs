//! Compiler-mode rows follow the canonical frontend flag, never checker traversal snapshots.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::CompilerFeatureItem;
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, contract::Want};

/// Read the actual provider projection at an explicitly selected language version and source kind.
fn feature(source: &'static str, path: &'static str, target: PythonTargetVersion) -> bool {
    let report = languages::contract::reading::<CompilerFeatureItem>(
        &crate::linked_with_settings(PythonParserSettings::new(target)),
        languages::SourcePath(path),
        languages::SampleText(source),
    )
    .expect("Python source is recognized");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 1);
    let [Want::File(file), Want::Name(name), Want::Flag(enabled)] = *report.rows[0] else {
        panic!("file, feature and explicit flag");
    };
    assert_eq!(file, path);
    assert_eq!(name, "python.future-annotations");
    enabled
}

fn only_valid_absolute_module_prefix_directives_enable_the_feature() {
    check::<CompilerFeatureItem>();
    for (source, enabled) in [
        ("", false),
        ("x = 1\n", false),
        ("from __future__ import annotations\nx = 1\n", true),
        ("'doc'\nfrom __future__ import annotations\nx = 1\n", true),
        (
            "from __future__ import division\nfrom __future__ import annotations\n",
            true,
        ),
        ("from __future__ import annotations as postponed\n", true),
        ("from .__future__ import annotations\n", false),
        ("x = 1\nfrom __future__ import annotations\n", false),
        ("def f():\n from __future__ import annotations\n", false),
        ("class C:\n from __future__ import annotations\n", false),
        ("if True:\n from __future__ import annotations\n", false),
        (
            "try:\n pass\nexcept Exception:\n from __future__ import annotations\n",
            false,
        ),
        (
            "from __future__ import annotations, unknown_feature\n",
            false,
        ),
        ("lazy from __future__ import annotations\n", false),
        (
            "from __future__ import annotations\nx = 1\nfrom __future__ import division\n",
            false,
        ),
    ] {
        for path in ["case.py", "case.pyi"] {
            assert_eq!(
                feature(source, path, PythonTargetVersion::Py313),
                enabled,
                "{path}: {source}"
            );
        }
    }
}

fn native_deferred_annotations_do_not_invent_a_future_feature() {
    for target in [
        PythonTargetVersion::Py313,
        PythonTargetVersion::Py314,
        PythonTargetVersion::Py315,
    ] {
        assert!(!feature("def f(x: Missing): pass\n", "case.py", target));
        assert!(feature(
            "from __future__ import annotations\ndef f(x: Missing): pass\n",
            "case.py",
            target
        ));
    }
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("only_valid_absolute_module_prefix_directives_enable_the_feature"),
        word: Word::Succeeds,
        input: || only_valid_absolute_module_prefix_directives_enable_the_feature,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("native_deferred_annotations_do_not_invent_a_future_feature"),
        word: Word::Succeeds,
        input: || native_deferred_annotations_do_not_invent_a_future_feature,
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
