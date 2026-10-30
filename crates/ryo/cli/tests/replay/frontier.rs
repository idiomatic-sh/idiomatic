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
        store: Arc::new(ryo_core::outbound::facts::factory::memory()),
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
    for (name, _) in counting::runnable() {
        let ((few, few_found), (many, many_found)) = (
            handed_by(small.path(), &name),
            handed_by(large.path(), &name),
        );
        eprintln!("FRONTIER {name} {few} {many} (findings {few_found} and {many_found})");
        if many > few + files * functions * 64 {
            grown.push(format!("{name}: {few} -> {many}"));
        }
    }
    assert!(
        grown.is_empty(),
        "ten times the tokens hand over more than a row of 64 words for each function: {grown:?}"
    );
}
