//! What a file hands the steps after the files is bounded by its own tree: over all of ruff, no file hands one rule more than four
//! rows for each row of its syntax tree. The hard guard against a handover that grows with pairs of a file's rows.

use crate::counting;
use std::sync::Arc;

/// The most rows a file may hand one rule's steps after the files for each row of its syntax tree: a rule hands the rows of the
/// relations it reads, and each of those has at most a row for each node or token of the tree, so four inputs of the tree's size fit
/// and a step whose rows grow with pairs of them does not.
const HANDOVER_WIDTH_BOUND: u64 = 4;

/// The widest handover of each file, as the report of a counting run over `root` with every recorded rule gives it.
fn handovers_over(root: &std::path::Path) -> Vec<ryo_values::domain::runs::types::Handover> {
    let rules = counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    let handed = counting::lint_observing(
        root,
        rules,
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
        0,
        Some(crate::stores::counter),
        |_, _, report| report.handovers,
    );
    handed.expect("the rules run")
}

#[test]
fn no_file_of_ruff_hands_a_rule_more_than_four_rows_for_each_row_of_its_tree() {
    let _beside = crate::beside();
    let copy = crate::ruff::ruff_copy();
    let mut handovers = handovers_over(copy.path());
    // A run that counted nothing would hold every bound: the run is held to have counted a handover for most of ruff's files.
    assert!(
        handovers.len() > 10_000,
        "the counting run reports a handover for each file that handed something, not {}",
        handovers.len()
    );
    // Widest first: the hundredths of a row handed for each row of the tree, a tree of no rows counted as one row.
    let width = |each: &ryo_values::domain::runs::types::Handover| {
        u128::from(each.handed.0) * 100 / u128::from(each.parsed.0.max(1))
    };
    handovers.sort_by_key(|each| std::cmp::Reverse(width(each)));
    let at = |share: usize| {
        handovers
            .get(handovers.len() * share / 100)
            .map_or(0, width)
    };
    eprintln!(
        "HANDOVERS {} files, in hundredths of a row for each row of the tree: max {}, p99 {}, p90 {}, p50 {}",
        handovers.len(),
        at(0),
        at(1),
        at(10),
        at(50)
    );
    for each in handovers.iter().take(10) {
        eprintln!(
            "HANDOVER {} {} handed {} of a tree of {} ({})",
            each.path.0,
            each.rule.0,
            each.handed.0,
            each.parsed.0,
            width(each)
        );
    }
    let over = handovers
        .iter()
        .filter(|each| each.handed.0 > HANDOVER_WIDTH_BOUND * each.parsed.0.max(1))
        .map(|each| {
            format!(
                "{} {} {}/{}",
                each.path.0, each.rule.0, each.handed.0, each.parsed.0
            )
        })
        .collect::<Vec<_>>();
    assert!(
        over.is_empty(),
        "a file hands a rule more than {HANDOVER_WIDTH_BOUND} rows for each row of its tree: {over:?}"
    );
}
