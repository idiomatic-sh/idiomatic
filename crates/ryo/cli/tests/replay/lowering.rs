//! The steps of a plan that read only the tree of a file run over the positions of its elements (the `fragment` concept of the
//! core), the only way they run. These are determinism checks over ruff and HTML, not a lowering differential: the operator-form
//! switch was removed on purpose (its absence is the `compile_fail` doctest of `LintUseCaseImpl`), so no second path exists to
//! compare. The lowered rows are checked against the recorded rows by `behavior::every_recorded_rule_gives_the_recorded_rows`.

use std::collections::BTreeMap;
use std::sync::Arc;

use ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake;

use crate::counting;

/// The rows of every live rule over ruff.
fn rows_of() -> BTreeMap<String, Vec<Vec<String>>> {
    let rules: Vec<_> = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let run = counting::lint_counting(
        &crate::base::snapshot::ruff(),
        rules,
        Arc::new(ryo_core::outbound::lint_store::factory::memory()),
    );
    let mut by_rule: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for row in run.expect("the rules run").rows {
        by_rule
            .entry(row.first().cloned().unwrap_or_default())
            .or_default()
            .push(row);
    }
    for rows in by_rule.values_mut() {
        rows.sort();
    }
    by_rule
}

#[test]
fn every_rule_gives_the_same_rows_on_two_runs_over_ruff() {
    let _alone = crate::alone();
    let first = rows_of();
    let second = rows_of();
    let differing: Vec<&String> = first
        .keys()
        .chain(second.keys())
        .filter(|rule| first.get(*rule) != second.get(*rule))
        .collect();
    assert!(
        differing.is_empty(),
        "the rows of {differing:?} change between two runs"
    );
    assert!(
        first.values().map(Vec::len).sum::<usize>() > 10_000,
        "the rules give the rows of ruff"
    );
}

/// Native DOM order, scalar values and missing locations must survive every run.
#[cfg(feature = "html")]
#[test]
fn html_dom_and_absent_locations_are_stable_across_runs() -> Result<(), Box<dyn std::error::Error>>
{
    let _alone = crate::alone();
    let directory = tempfile::tempdir()?;
    std::fs::write(
        directory.path().join("page.html"),
        "<!doctype html><table>foster<tr><td>&amp;</td></tr></table><p id='a'>end</p>",
    )?;
    let queries = [
        (
            "tree",
            r#"let rows = select * from "syntax-tree"; emit rows subject file key(id,text,span);"#,
        ),
        (
            "spans",
            r#"let rows = select l."id" as "left", r."id" as "right", l."file" as "file", l."span" as "span" from "syntax-tree" as l join "syntax-tree" as r on l."span" = r."span"; emit rows subject file key(left,right,span);"#,
        ),
    ];
    let mut answers = Vec::new();
    for _run in 0..2 {
        let store = tempfile::tempdir()?;
        let html = ryo_html::linked();
        let result = crate::base::lint_report_with_languages(
            directory.path(),
            queries
                .iter()
                .map(|(name, query)| crate::rule_from_query(name, query, &["**/*.html".to_owned()]))
                .collect(),
            Arc::new(ryo_core::outbound::facts::factory::spill(
                store.path().to_path_buf(),
            )),
            Arc::new(LintStoreFake::default()),
            Vec::new(),
            false,
            vec![ryo_core::outbound::language::types::LanguageHandles::from(
                html,
            )],
        );
        let (mut rows, report) = result?;
        assert!(report.unread.is_empty(), "HTML is actually read");
        rows.sort();
        assert!(rows.iter().flatten().any(|cell| cell.contains("unlocated")));
        assert!(
            rows.iter()
                .flatten()
                .any(|cell| cell.contains("http://www.w3.org/1999/xhtml"))
        );
        answers.push(rows);
    }
    assert_eq!(answers[0], answers[1]);
    Ok(())
}
