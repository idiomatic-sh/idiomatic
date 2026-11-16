//! What a per-file plan hands to the cross-file finish is reduced rows, never the tokens of a file:
//! `duplicate-function-bodies` over files whose functions hold ten times the tokens hands over the
//! same number of words, one row for each function large enough to compare.

use crate::base;
use crate::counting;
use crate::stores::Counting;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Files of `functions` functions each, each holding `statements` repeats of one statement of about eight tokens: more elements, the same
/// vocabulary, so what a rule hands over may depend on the first and never on the second.
fn corpus(files: usize, functions: usize, statements: usize) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for file in 0..files {
        let mut text = String::new();
        for function in 0..functions {
            writeln!(text, "fn work_{function}() {{").expect("writing to a String cannot fail");
            for _ in 0..statements {
                writeln!(text, "    let value = {file} + 1;")
                    .expect("writing to a String cannot fail");
            }
            text.push_str("}\n");
        }
        std::fs::write(root.path().join(format!("file_{file}.rs")), text).expect("a file");
    }
    root
}

/// The words the finish is handed for the corpus.
fn handed_over(root: &std::path::Path) -> usize {
    handed_by(root, "duplicate-function-bodies").0
}

/// The words the finish is handed by one recorded rule for the corpus, and how many findings the rule gave.
fn handed_by(root: &std::path::Path, name: &str) -> (usize, usize) {
    let (_, rule) = counting::runnable()
        .into_iter()
        .find(|(each, _)| each == name)
        .expect("the recorded rule");
    let counting = Arc::new(Counting {
        store: Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        words: Arc::new(AtomicUsize::new(0)),
    });
    let (rows, _) = base::lint_with(
        root,
        vec![rule],
        Arc::<Counting>::clone(&counting),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    (counting.words.load(Ordering::Relaxed), rows.len())
}

#[test]
fn the_finish_is_handed_a_row_for_each_function_and_none_for_a_token() {
    let _beside = crate::beside();
    let (files, functions) = (4, 5);
    let small = handed_over(corpus(files, functions, 6).path());
    let large = handed_over(corpus(files, functions, 60).path());
    let qualifying = files * functions;
    assert!(
        small > 0,
        "the functions are large enough to compare, so rows are handed over"
    );
    assert_eq!(
        small, large,
        "ten times the tokens hand over the same words"
    );
    assert!(
        small <= qualifying * 64,
        "at most 64 words for each of the {qualifying} functions, not {small}"
    );
}

#[test]
fn no_rule_is_handed_words_in_proportion_to_the_elements_of_a_file() {
    let _beside = crate::beside();
    let (files, functions) = (4, 5);
    let (small, large) = (corpus(files, functions, 6), corpus(files, functions, 60));
    let mut grown = Vec::new();
    let mut handing = 0_usize;
    for (name, _) in counting::runnable() {
        let ((few, few_found), (many, many_found)) = (
            handed_by(small.path(), &name),
            handed_by(large.path(), &name),
        );
        eprintln!("FRONTIER {name} {few} {many} (findings {few_found} and {many_found})");
        handing += usize::from(few > 0);
        if many > few + files * functions * 64 {
            grown.push(format!("{name}: {few} -> {many}"));
        }
    }
    // A run in which no rule handed anything would hold the bound: some rule is held to hand words over.
    assert!(handing > 0, "no recorded rule handed the finish anything");
    assert!(
        grown.is_empty(),
        "ten times the tokens hand over more than a row of 64 words for each function: {grown:?}"
    );
}

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
        counting::Work::Counted,
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
