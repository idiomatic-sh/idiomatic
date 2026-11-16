//! What a run keeps of the steps of the rules that cross files, with the real Rust language, the real fact store and the
//! real store file: a file added changes the answer, and a definition removed is found from what the file that calls it handed,
//! without reading that file again. The rows of the cross-file steps that have no place leave in one order, however the workers
//! made them.

/// The same query of rows with no place, over the core's sources, gives the same bytes every time it runs: the order of the rows
/// does not follow which worker read which file first.
#[test]
fn rows_with_no_place_leave_in_one_order() {
    let query = r#"let c = group-count(from="function", keys=[name], count=n); emit c subject name key(name);"#;
    let run = || {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_ryo"))
            .args([
                "query",
                "run",
                "--root",
                "../crates/core/src",
                "--json",
                "--limit",
                "100000",
                "--text",
                query,
            ])
            .output()
            .expect("the command runs");
        assert!(output.status.success(), "the command succeeds");
        output.stdout
    };
    let first = run();
    assert!(first.len() > 1000, "the query answers rows");
    for _ in 0_u8..5 {
        assert!(run() == first, "two runs of one query answered other bytes");
    }
}

use crate::cache::{sorted, write};
use crate::unbounded;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;

/// A rule that reads the files the run lists, across files: it answers when there are three or more.
fn counting_files() -> Vec<Rule> {
    vec![crate::tools::rule_from_query(
        "three-files",
        "let files = project(from=\"filesystem.file\", columns=[\"path\"]);\nlet ones = extend(from=files, values={\"one\": 1});\nlet counted = group-count(from=ones, keys=[\"one\"], count=\"files\");\nlet enough = select * from counted where files >= 3;\nemit enough subject one key(files);",
        &["**/*.rs".to_owned()],
    )]
}

/// What a run of `rules` over `root` that keeps nothing finds.
fn uncached(root: &std::path::Path, rules: Vec<Rule>) -> Vec<Vec<String>> {
    sorted(
        unbounded::lint_counting(
            root,
            rules,
            Arc::new(ryo_core::outbound::lint_store::factory::memory()),
        )
        .expect("a run that keeps nothing")
        .rows,
    )
}

#[test]
fn a_file_added_that_holds_no_node_named_x_changes_the_answer() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    let store = crate::base::store_at(root.path());
    write(
        root.path(),
        &[
            ("one.rs", "pub fn y() {}\n".to_owned()),
            ("two.rs", "pub fn z() {}\n".to_owned()),
        ],
    );
    let before = unbounded::lint_counting(root.path(), counting_files(), Arc::clone(&store))
        .expect("a cold run");
    assert!(
        before.rows.is_empty(),
        "two files are not three: {:?}",
        before.rows
    );
    // The file added holds no function at all, but it is a file more: what the files show one another changed.
    write(root.path(), &[("three.rs", "// nothing here\n".to_owned())]);
    let after = unbounded::lint_counting(root.path(), counting_files(), store)
        .expect("a run after the file is added");
    assert_eq!(after.reads, 1, "only the file added is read");
    assert_eq!(after.rows.len(), 1, "three files are: {:?}", after.rows);
    assert_eq!(sorted(after.rows), uncached(root.path(), counting_files()));
}

/// A rule that finds the calls of a function nothing defines, across files.
fn calls_without_a_definition() -> Vec<Rule> {
    vec![crate::tools::rule_from_query(
        "undefined-calls",
        "let calls = project(from=\"static-call\", columns=[\"target\"]);\nlet defined = project(from=\"function\", columns=[\"name\"]);\nlet unresolved = antijoin(left=calls, right=defined, left-on=[\"target\"], right-on=[\"name\"]);\nemit unresolved subject target key(target);",
        &["**/*.rs".to_owned()],
    )]
}

#[test]
fn removing_the_definition_is_found_from_what_the_file_that_calls_handed() {
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    write(
        root.path(),
        &[
            ("define.rs", "pub fn beta() {}\n".to_owned()),
            ("call.rs", "pub fn caller() { beta(); }\n".to_owned()),
        ],
    );
    let first = unbounded::lint_counting(
        root.path(),
        calls_without_a_definition(),
        crate::base::store_at(root.path()),
    )
    .expect("a cold run");
    assert!(first.rows.is_empty(), "beta is defined: {:?}", first.rows);
    // A fresh adapter over the same root stands for a fresh process: nothing is remembered but the store on disk.
    std::fs::remove_file(root.path().join("define.rs")).expect("the definition removed");
    let after = unbounded::lint_counting(
        root.path(),
        calls_without_a_definition(),
        crate::base::store_at(root.path()),
    )
    .expect("a run after the removal");
    assert_eq!(after.reads, 0, "the file that calls is not read again");
    assert_eq!(
        after.rows.len(),
        1,
        "and its call of beta is now unresolved: {:?}",
        after.rows
    );
    assert_eq!(
        sorted(after.rows),
        uncached(root.path(), calls_without_a_definition())
    );
}
