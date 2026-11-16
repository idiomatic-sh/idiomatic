//! One rule's replayed rows against its recording (`answers::compare`): the verdict and the listing read the one comparison, for a
//! decided rule against `expected.toml` and for an undecided one against `checked.toml`, an undecided rule giving rows nobody
//! checked failing (F2). Every row is listed with the source text at its key.

use super::answers::{Against, Expectation, compare, listing, source_text, verdict};

/// The rows `each`, as owned text.
fn rows(each: &[&str]) -> Vec<String> {
    each.iter().map(|row| (*row).to_owned()).collect()
}

/// What a decided recording of plan `query-1` expects: the rows `expected`.
fn decided(expected: &[&str]) -> Expectation {
    Expectation {
        plan: "query-1".into(),
        expected: rows(expected),
        undecided: false,
        checked: Vec::new(),
    }
}

/// What an undecided recording of plan `query-1` holds: the findings `checked` by hand.
fn undecided(checked: &[&str]) -> Expectation {
    Expectation {
        plan: "query-1".into(),
        expected: Vec::new(),
        undecided: true,
        checked: rows(checked),
    }
}

#[test]
fn an_undecided_rule_giving_rows_nobody_checked_fails() {
    let said = undecided(&["a.rs\t1..2"]);
    assert!(
        verdict(&compare(
            "r",
            &said,
            &[],
            &rows(&["a.rs\t1..2", "b.rs\t3..4"])
        ))
        .is_some(),
        "an extra row of an undecided rule passes"
    );
    assert_eq!(
        verdict(&compare(
            "r",
            &said,
            &rows(&["b.rs\t3..4"]),
            &rows(&["a.rs\t1..2", "b.rs\t3..4"])
        )),
        None,
        "an extra row listed with a reason is kept"
    );
}

#[test]
fn a_decided_rule_lists_exactly_what_its_verdict_counts() {
    let root = tempfile::tempdir().expect("a root with no files");
    let compared = compare(
        "r",
        &decided(&["a.rs\t1..2", "b.rs\t1..2", "c.rs\t1..2"]),
        &rows(&["d.rs\t1..2"]),
        &rows(&[
            "b.rs\t1..2",
            "b.rs\t1..2",
            "c.rs\t1..2",
            "d.rs\t1..2",
            "e.rs\t1..2",
        ]),
    );
    assert_eq!(compared.against, Against::Expected);
    assert_eq!(compared.missing, rows(&["a.rs\t1..2"]));
    assert_eq!(compared.extra, rows(&["e.rs\t1..2"]));
    assert!(compared.gone.is_empty());
    assert_eq!(
        compared.given,
        rows(&["b.rs\t1..2", "c.rs\t1..2", "e.rs\t1..2"])
    );
    let said = verdict(&compared).expect("a verdict");
    assert!(
        said.contains("1 recorded rows missing, 1 rows not recorded"),
        "{said}"
    );
    assert_eq!(
        listing(&compared, root.path()),
        rows(&[
            "r\tmissing\ta.rs\t1..2\t(outside the file)",
            "r\textra\te.rs\t1..2\t(outside the file)",
        ])
    );
}

#[test]
fn an_undecided_rule_lists_against_its_checked_rows() {
    let root = tempfile::tempdir().expect("a root with no files");
    let compared = compare(
        "orphan-rust-files",
        &undecided(&["src/orphan.rs\tsrc/orphan.rs"]),
        &[],
        &rows(&["src/orphan.rs\tsrc/orphan.rs", "src/other.rs\tsrc/other.rs"]),
    );
    assert_eq!(compared.against, Against::Checked);
    assert_eq!(compared.extra, rows(&["src/other.rs\tsrc/other.rs"]));
    assert!(compared.missing.is_empty());
    let said = verdict(&compared).expect("a verdict");
    assert!(said.contains("src/other.rs\tsrc/other.rs"), "{said}");
    assert_eq!(
        listing(&compared, root.path()),
        rows(&["orphan-rust-files\textra\tsrc/other.rs\tsrc/other.rs\t-"])
    );
}

#[test]
fn a_stale_difference_is_listed_beside_the_changed_rows() {
    let root = tempfile::tempdir().expect("a root with no files");
    for said in [decided(&["a.rs\t1..2"]), undecided(&["a.rs\t1..2"])] {
        let compared = compare(
            "r",
            &said,
            &rows(&["d.rs\t1..2"]),
            &rows(&["a.rs\t1..2", "e.rs\t1..2"]),
        );
        assert_eq!(compared.gone, rows(&["d.rs\t1..2"]));
        assert_eq!(compared.extra, rows(&["e.rs\t1..2"]));
        let failure = verdict(&compared).expect("a verdict");
        assert!(
            failure.contains("`d.rs\t1..2`") && failure.contains("no longer gives it"),
            "{failure}"
        );
        assert_eq!(
            listing(&compared, root.path()),
            rows(&[
                "r\tgone\td.rs\t1..2\t(outside the file)",
                "r\textra\te.rs\t1..2\t(outside the file)",
            ])
        );
    }
}

#[test]
fn duplicate_rows_are_one_finding() {
    let root = tempfile::tempdir().expect("a root with no files");
    let compared = compare(
        "r",
        &decided(&["a.rs\t1..2", "a.rs\t1..2", "b.rs\t1..2"]),
        &[],
        &rows(&["b.rs\t1..2", "a.rs\t1..2", "a.rs\t1..2"]),
    );
    assert_eq!(verdict(&compared), None);
    assert!(listing(&compared, root.path()).is_empty());
    assert_eq!(compared.given.len(), 2);
}

#[test]
fn the_source_text_of_a_row_is_the_text_at_its_span() {
    let root = tempfile::tempdir().expect("a root");
    std::fs::write(
        root.path().join("a.rs"),
        "let a =\n\"hello\tworld\"\nfn x() {\n}\n",
    )
    .expect("a file");
    let shown: Vec<String> = [
        "a.rs\t8..21",
        "a.rs\t22..32",
        "a.rs\ta.rs",
        "a.rs\t900..910",
    ]
    .iter()
    .map(|row| source_text(root.path(), row))
    .collect();
    assert_eq!(
        shown,
        rows(&["\"hello world\"", "fn x() {", "-", "(outside the file)"])
    );
}
