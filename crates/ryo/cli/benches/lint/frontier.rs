//! The words each recorded rule hands the cross-file finish over a corpus whose functions hold sixty statements of eight tokens each:
//! four files of five functions. A count is exact, so one run is the whole sample. The measure of a rule is `<rule>-words` of the
//! benchmark `frontier`.

use crate::bmf::Cost;
use crate::frontier::{corpus, handed_by};

/// The files of the corpus.
const FILES: usize = 4;

/// The functions of each file.
const FUNCTIONS: usize = 5;

/// The statements of each function: ten times what the first test of the engine used.
const STATEMENTS: usize = 60;

/// The words each rule is handed, one measure for each.
pub(crate) fn of_each_rule() -> Vec<Cost> {
    let root = corpus(FILES, FUNCTIONS, STATEMENTS);
    crate::counting::runnable()
        .into_iter()
        .map(|(name, _)| {
            let words = handed_by(root.path(), &name);
            eprintln!("frontier {name}: {words} words");
            Cost {
                target: "frontier",
                // A rule's name is known when the bench runs: the document names it once, for the life of the process.
                metric: Box::leak(format!("{name}-words").into_boxed_str()),
                rounds: vec![u128::try_from(words).expect("a count that fits 128 bits")],
            }
        })
        .collect()
}
