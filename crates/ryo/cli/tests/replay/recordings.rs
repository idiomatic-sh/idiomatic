//! What a recording must hold for the replay to compare it, each way a failure could otherwise pass as a match: a stale
//! `pending.txt` (H1), a declared parameter with no value (H2), a malformed table (H3), a missing `expected.toml` or `checked.toml`
//! (H4), an `undecided.txt` with no reason (H5), a finding too short or under no live rule (H6), a sample that could not be made
//! (H7), a missing cell read as empty text (H8), a recorded difference with no reason (H9), an undecided rule giving rows nobody
//! checked (F2), a pending recording failing on another ground or for other relations than its `pending.txt` names (F3), and a
//! recording without its query or scope (F5). Each fails; the goldens' own failures are in `goldens`.

use super::answers::{by_rule, expectation_at, extra_rows_in, unchecked_rows};
use super::base::{ABSENT, Pending, Recording, does_not_run_in, finding_texts, recording_at};
use std::path::Path;

/// A query over the syntax tree that every Rust file gives rows to.
const TREE: &str = r#"let rows = select l."id" as "id", l."file" as "file", l."span" as "span" from "syntax-tree" as l;
emit rows subject "file" key("span");"#;

/// A recording directory holding `files`, each a name and its text.
fn recorded(files: &[(&str, &str)]) -> tempfile::TempDir {
    let at = tempfile::tempdir().expect("a directory for the recording");
    for (name, body) in files {
        std::fs::write(at.path().join(name), body).expect("a file of the recording");
    }
    at
}

/// Whether reading the recording at `at` fails.
fn refused(at: &Path) -> bool {
    std::panic::catch_unwind(|| recording_at(at)).is_err()
}

/// Whether reading what the recording at `at` expects fails.
fn unexpected(at: &Path) -> bool {
    std::panic::catch_unwind(|| expectation_at(at)).is_err()
}

#[test]
fn a_pending_txt_naming_a_relation_the_rust_language_provides_fails() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", TREE),
        ("scope.toml", "include = [\"**\"]\n"),
        (
            "pending.txt",
            "reads syntax-tree, which the Rust language of ryo does not provide\n",
        ),
    ]);
    assert!(refused(at.path()), "a stale pending.txt passes");
}

#[test]
fn a_pending_txt_not_in_its_one_form_fails() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", TREE),
        ("scope.toml", "include = [\"**\"]\n"),
        ("pending.txt", "waits for something\n"),
    ]);
    assert!(
        refused(at.path()),
        "a pending.txt naming no relation passes"
    );
}

#[test]
fn a_parameter_the_query_declares_without_a_value_fails() {
    let _beside = crate::beside();
    let query = format!(
        "param names: set<string>;\n{}",
        TREE.replace(
            r#"from "syntax-tree" as l;"#,
            r#"from "syntax-tree" as l where l."text" in $names;"#,
        )
    );
    let at = recorded(&[
        ("query.ryoql", &query),
        ("scope.toml", "include = [\"**\"]\n"),
    ]);
    assert!(
        refused(at.path()),
        "a declared parameter is bound to an empty list"
    );
}

#[test]
fn a_malformed_scope_or_parameters_table_fails() {
    let _beside = crate::beside();
    for (file, body) in [
        ("scope.toml", "include = ["),
        ("parameters.toml", "names = "),
    ] {
        let mut files = vec![
            ("query.ryoql", TREE),
            ("scope.toml", "include = [\"**\"]\n"),
        ];
        files.retain(|(name, _)| *name != file);
        files.push((file, body));
        let at = recorded(&files);
        assert!(
            refused(at.path()),
            "a malformed {file} reads as an empty table"
        );
    }
}

#[test]
fn a_recording_without_expected_or_checked_rows_fails() {
    let without_expected = recorded(&[("plan", "query-1")]);
    assert!(
        unexpected(without_expected.path()),
        "a recording without expected.toml matches no rows"
    );
    let without_checked = recorded(&[
        ("plan", "query-1"),
        ("expected.toml", "rows = []\n"),
        ("undecided.txt", "Today's ryo could not decide this rule.\n"),
    ]);
    assert!(
        unexpected(without_checked.path()),
        "an undecided recording without checked.toml is checked to give none"
    );
}

#[test]
fn an_undecided_txt_without_a_reason_fails() {
    let at = recorded(&[
        ("plan", "query-1"),
        ("expected.toml", "rows = []\n"),
        ("undecided.txt", "  \n"),
        ("checked.toml", "rows = []\n"),
    ]);
    assert!(
        unexpected(at.path()),
        "an undecided.txt naming no reason passes"
    );
}

#[test]
fn a_finding_too_short_or_under_no_live_rule_fails() {
    let row = |cells: &[&str]| {
        cells
            .iter()
            .map(|cell| (*cell).to_owned())
            .collect::<Vec<String>>()
    };
    assert!(
        by_rule(&[row(&["rule", "file.rs"])], &["rule"]).is_err(),
        "a short finding is dropped"
    );
    assert!(
        by_rule(&[row(&["other", "file.rs", "1..2"])], &["rule"]).is_err(),
        "a finding under a rule that is not live is dropped"
    );
    let kept =
        by_rule(&[row(&["rule", "file.rs", "1..2"])], &["rule"]).expect("a finding of a live rule");
    assert_eq!(kept.get("rule"), Some(&vec!["file.rs\t1..2".to_owned()]));
}

#[test]
fn a_sample_that_could_not_be_made_is_a_failure_not_a_run() {
    let recording = Recording {
        name: "sample".into(),
        query: TREE.into(),
        parameters: Vec::new(),
        include: vec!["**".into()],
        exclude: Vec::new(),
        pending: Pending::Missing,
    };
    assert!(
        does_not_run_in(Err(std::io::Error::other("no directory")), &recording).is_some(),
        "a sample that could not be made reads as a rule that runs"
    );
}

#[test]
fn a_missing_cell_reads_apart_from_empty_text() {
    use ryo_values::domain::row_texts::types::Cell;
    let absent = finding_texts(&[None]);
    let empty = finding_texts(&[Some(Cell::Text("".into()))]);
    assert_ne!(absent, empty);
    assert_eq!(absent, vec![ABSENT.to_owned()]);
}

#[test]
fn a_recorded_difference_without_a_reason_fails() {
    let path = Path::new("differences.toml");
    let listed = |why: &str| -> toml::Table {
        format!("[rule.r]\nwhy = \"{why}\"\nextra = [{{ path = \"a.rs\", key = \"1..2\" }}]\n")
            .parse()
            .expect("a table of differences")
    };
    let without = listed(" ");
    assert!(
        std::panic::catch_unwind(|| extra_rows_in(&without, "r", path)).is_err(),
        "a difference with no reason is taken out"
    );
    assert_eq!(
        extra_rows_in(&listed("ryo sees more"), "r", path),
        vec!["a.rs\t1..2".to_owned()]
    );
}

/// A query that reads a relation ryo does not own, a host's.
const HOSTED: &str = r#"let rows = select l."path" as "file" from "tastemaker.placed" as l;
emit rows subject "file" key("file");"#;

#[test]
fn an_undecided_rule_giving_rows_nobody_checked_fails() {
    let rows = |each: &[&str]| {
        each.iter()
            .map(|row| (*row).to_owned())
            .collect::<Vec<String>>()
    };
    assert!(
        unchecked_rows(
            "r",
            &rows(&["a.rs\t1..2"]),
            &[],
            &rows(&["a.rs\t1..2", "b.rs\t3..4"])
        )
        .is_some(),
        "an extra row of an undecided rule passes"
    );
    assert_eq!(
        unchecked_rows(
            "r",
            &rows(&["a.rs\t1..2"]),
            &rows(&["b.rs\t3..4"]),
            &rows(&["a.rs\t1..2", "b.rs\t3..4"])
        ),
        None,
        "an extra row listed with a reason is kept"
    );
}

#[test]
fn a_pending_recording_that_fails_on_another_ground_fails() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", "let rows = selec;\n"),
        ("scope.toml", "include = [\"**\"]\n"),
        (
            "pending.txt",
            "reads tastemaker.placed, which the Rust language of ryo does not provide\n",
        ),
    ]);
    assert!(
        refused(at.path()),
        "a pending.txt stands while the query does not parse"
    );
}

#[test]
fn a_pending_txt_naming_other_relations_than_the_refusal_fails() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", HOSTED),
        ("scope.toml", "include = [\"**\"]\n"),
        (
            "pending.txt",
            "reads tastemaker.placed, tastemaker.other, which the Rust language of ryo does not provide\n",
        ),
    ]);
    assert!(
        refused(at.path()),
        "a pending.txt names more than the query is refused for"
    );
}

#[test]
fn a_pending_txt_naming_exactly_the_refused_relation_waits() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", HOSTED),
        ("scope.toml", "include = [\"**\"]\n"),
        (
            "pending.txt",
            "reads tastemaker.placed, which the Rust language of ryo does not provide\n",
        ),
    ]);
    assert!(matches!(
        recording_at(at.path()).pending,
        Pending::Because(_)
    ));
}

#[test]
fn a_recording_without_its_query_or_scope_fails() {
    let _beside = crate::beside();
    let without_query = recorded(&[("scope.toml", "include = [\"**\"]\n")]);
    assert!(
        refused(without_query.path()),
        "a recording without query.ryoql reads an empty query"
    );
    let without_scope = recorded(&[("query.ryoql", TREE)]);
    assert!(
        refused(without_scope.path()),
        "a recording without scope.toml reads every file"
    );
}

#[test]
fn a_scope_without_include_or_with_other_keys_fails() {
    let _beside = crate::beside();
    for scope in [
        "exclude = [\"**/build.rs\"]\n",
        "include = [\"**\"]\nincludes = [\"src/**\"]\n",
        "include = [1]\n",
    ] {
        let at = recorded(&[("query.ryoql", TREE), ("scope.toml", scope)]);
        assert!(
            refused(at.path()),
            "the scope `{scope}` widens to every file"
        );
    }
}

#[test]
fn a_parameter_the_query_does_not_declare_fails() {
    let _beside = crate::beside();
    let at = recorded(&[
        ("query.ryoql", TREE),
        ("scope.toml", "include = [\"**\"]\n"),
        ("parameters.toml", "names = [\"a\"]\n"),
    ]);
    assert!(
        refused(at.path()),
        "a parameter no query declares is passed unseen"
    );
}

#[test]
fn rows_nothing_compares_fail() {
    let undecided_with_rows = recorded(&[
        ("plan", "query-1"),
        (
            "expected.toml",
            "rows = [{ path = \"a.rs\", key = \"1..2\" }]\n",
        ),
        ("undecided.txt", "Today's ryo could not decide this rule.\n"),
        ("checked.toml", "rows = []\n"),
    ]);
    assert!(
        unexpected(undecided_with_rows.path()),
        "an undecided recording's expected rows are ignored"
    );
    let checked_without_undecided = recorded(&[
        ("plan", "query-1"),
        ("expected.toml", "rows = []\n"),
        (
            "checked.toml",
            "rows = [{ path = \"a.rs\", key = \"1..2\" }]\n",
        ),
    ]);
    assert!(
        unexpected(checked_without_undecided.path()),
        "checked.toml of a decided recording is ignored"
    );
}
