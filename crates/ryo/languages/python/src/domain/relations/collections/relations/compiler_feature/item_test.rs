//! Compiler-mode rows follow the canonical frontend flag, never checker traversal snapshots.
use super::super::fixtures::contract::check;
use super::item::CompilerFeatureItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::{self, contract::Want};

/// Read the actual provider projection at an explicitly selected language version and source kind.
fn feature(source: &'static str, path: &str, target: PythonTargetVersion) -> bool {
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

/// Every annotation context of the language fixture (`tests/fixtures/compiler-features`) enables the future-annotations feature
/// exactly where the fixture says the validated module future flag is on, at the fixture's own parser target and file name.
fn annotation_context_fixture_flags_follow_the_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/compiler-features/annotation-contexts.json");
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(path).expect("the fixture")).expect("JSON");
    assert!(cases.len() >= 40, "the fixture stays nonvacuous");
    for case in &cases {
        let text = |key: &str| case[key].as_str().expect("a text field").to_owned();
        let target = match text("target").as_str() {
            "py313" => PythonTargetVersion::Py313,
            "py314" => PythonTargetVersion::Py314,
            "py315" => PythonTargetVersion::Py315,
            other => panic!("unknown fixture target {other}"),
        };
        // The sample text of a provider reading is `'static`: the fixture's 51 sources are leaked for this one test.
        let source: &'static str = Box::leak(text("source").into_boxed_str());
        assert_eq!(
            feature(source, &text("file"), target),
            case["feature"].as_bool().expect("a flag"),
            "{}",
            text("name")
        );
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
    Case {
        name: CaseName("annotation_context_fixture_flags_follow_the_fixture"),
        word: Word::Succeeds,
        input: || annotation_context_fixture_flags_follow_the_fixture,
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
