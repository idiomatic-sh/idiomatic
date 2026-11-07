//! What an edit of a file's interface does to a run that keeps its store: an edit inside a body reads one file and keeps the findings
//! that cross files; an edit of what a file declares or imports makes the cross-file steps again, and the answer is the one a run
//! that keeps nothing gives.

use crate::cache::{sorted, write};
use crate::{counting, stores};
use std::path::Path;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;

/// The recorded rules named, in the order of the recordings.
fn rules_named(names: &[&str]) -> Vec<Rule> {
    let mut wanted = counting::runnable();
    wanted.retain(|(name, _)| names.contains(&name.as_str()));
    wanted.into_iter().map(|(_, rule)| rule).collect()
}

/// The rules an edit of a file's interface is proved against: two that read what other files declare and import (one through the
/// project), and three that stay in one file.
fn interface_rules() -> Vec<Rule> {
    rules_named(&[
        "duplicate-function-bodies",
        "module-cycles",
        "pub-item-unreferenced-elsewhere",
        "todo-comments",
        "unwrap-outside-tests",
    ])
}

/// A Cargo workspace of two packages, `b` depending on `a` by path: `a` declares a public function in a module of its own that `b`
/// never names. (The rule that finds it leaves the roots of the crates alone, so what the test edits is in a module.)
const INTERFACE_WORKSPACE: &[(&str, &str)] = &[
    ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
    (
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    ),
    (
        "a/src/lib.rs",
        "pub mod helpers;\n\npub fn a() -> u32 {\n    1\n}\n",
    ),
    (
        "a/src/helpers.rs",
        "pub fn helper() -> u32 {\n    let value = 2;\n    value + 40\n}\n",
    ),
    (
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\na = { path = \"../a\" }\n",
    ),
    (
        "b/src/lib.rs",
        "mod inner;\n\npub fn b() -> u32 {\n    a::a() + inner::inner()\n}\n",
    ),
    (
        "b/src/inner.rs",
        "pub fn inner() -> u32 {\n    crate::b()\n}\n",
    ),
];

/// A run over `root` with the interface rules, keeping its store in `store`: the rows sorted, and how many files the language read.
fn interface_run(root: &Path, store: &stores::Kept, step: &str) -> (Vec<Vec<String>>, usize) {
    let run = counting::lint_counting(root, interface_rules(), Arc::clone(&store.0)).expect(step);
    (sorted(run.rows), run.reads)
}

/// What a run that keeps nothing gives over `root` with the interface rules, sorted.
fn interface_oracle(root: &Path) -> Vec<Vec<String>> {
    sorted(
        counting::lint_counting(root, interface_rules(), stores::in_memory().0)
            .expect("the oracle runs")
            .rows,
    )
}

/// How many public items of `a/src/helpers.rs` the rows say no other file names.
fn unreferenced_helpers(rows: &[Vec<String>]) -> usize {
    rows.iter()
        .filter(|row| {
            row.first()
                .is_some_and(|rule| rule == "pub-item-unreferenced-elsewhere")
                && row.get(1).is_some_and(|path| path == "a/src/helpers.rs")
        })
        .count()
}

/// A run after an edit: it reads only `reads` files, and answers what a run that keeps nothing gives.
fn edited_run(root: &Path, store: &stores::Kept, step: &str, reads: usize) -> Vec<Vec<String>> {
    let (rows, read) = interface_run(root, store, step);
    assert_eq!(read, reads, "{step}: the files read");
    assert_eq!(
        rows,
        interface_oracle(root),
        "{step} answers what a run that keeps nothing gives"
    );
    rows
}

#[test]
fn an_edit_inside_a_body_reads_one_file_and_an_edit_of_what_a_file_declares_or_imports_makes_the_cross_file_steps_again()
 {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = stores::Kept(crate::base::store_at(root.path()));
    let owned: Vec<(&str, String)> = INTERFACE_WORKSPACE
        .iter()
        .map(|(path, text)| (*path, (*text).to_owned()))
        .collect();
    write(root.path(), &owned);
    let text_of = |path: &str| {
        std::fs::read_to_string(root.path().join(path)).expect("a file of the workspace")
    };
    let cold = edited_run(
        root.path(),
        &store,
        "a cold run reads every file: the Rust files and the manifests",
        INTERFACE_WORKSPACE.len(),
    );
    assert_eq!(
        unreferenced_helpers(&cold),
        1,
        "`helper` is public and named in no other file: {cold:?}"
    );

    // A body edit of the same length: nothing the file shows other files moves or changes.
    write(
        root.path(),
        &[(
            "a/src/helpers.rs",
            text_of("a/src/helpers.rs").replace("let value = 2;", "let value = 3;"),
        )],
    );
    let same_length = edited_run(root.path(), &store, "a body edit of the same length", 1);
    assert_eq!(
        same_length, cold,
        "and nothing a cross-file rule finds changed"
    );

    // A body edit that adds a line: what the file declares stays, where it is in the file moves.
    write(
        root.path(),
        &[(
            "a/src/helpers.rs",
            text_of("a/src/helpers.rs").replace(
                "    value + 40\n",
                "    let more = 1;\n    value + 40 - more\n",
            ),
        )],
    );
    let _shifted = edited_run(
        root.path(),
        &store,
        "a body edit that moves what follows",
        1,
    );

    // An import added: `b` names `helper` now, so what `a` declares is named elsewhere and the finding about it goes.
    write(
        root.path(),
        &[(
            "b/src/inner.rs",
            "use a::helpers::helper;\n\npub fn inner() -> u32 {\n    crate::b() + helper()\n}\n"
                .to_owned(),
        )],
    );
    let imported = edited_run(root.path(), &store, "an import added", 1);
    assert_eq!(
        unreferenced_helpers(&imported),
        0,
        "the cross-file steps were made again: `helper` is named in `b` now: {imported:?}"
    );

    // A declaration added: a new public function nobody names is found across files.
    write(
        root.path(),
        &[(
            "a/src/helpers.rs",
            format!(
                "{}\npub fn fresh() -> u32 {{\n    1\n}}\n",
                text_of("a/src/helpers.rs")
            ),
        )],
    );
    let declared = edited_run(root.path(), &store, "a declaration added", 1);
    assert_eq!(
        unreferenced_helpers(&declared),
        1,
        "the cross-file steps were made again: `fresh` is public and named in no other file: {declared:?}"
    );

    let warm = edited_run(root.path(), &store, "a warm run", 0);
    assert_eq!(warm, declared, "and answers what the last run did");
}
