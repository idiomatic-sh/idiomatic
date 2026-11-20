//! What every implementation of the file scan must pass, whatever engines it runs and whatever languages it links: the rule
//! sets of no rule run no rule and keep a pack each, and a file no linked language reads is answered as not walked, with no row.
use super::super::interface::FileScanUseCase;
use super::super::types::{RuleSetNumber, ScanLoadRequest, Scanned};
use super::file;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// A question the contract asks of an implementation.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// How many rules the rule sets of no selected rule run, together.
    RulesOfNoRule,
    /// How many of the rule sets of no selected rule have no pack to keep.
    RuleSetsWithoutAPack,
    /// How many rows the scan of a file no language reads hands, when it answers the file was not walked.
    RowsOfAnUnreadFile,
}

type Input = Question;
type Value = usize;
type Failure = OperatingError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rule_sets_of_no_rule_run_no_rule"),
        word: Word::Succeeds,
        input: || Question::RulesOfNoRule,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("every_rule_set_has_a_pack_to_keep"),
        word: Word::Succeeds,
        input: || Question::RuleSetsWithoutAPack,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_no_linked_language_reads_is_not_walked_and_has_no_row"),
        word: Word::Succeeds,
        input: || Question::RowsOfAnUnreadFile,
        expect: Expect::Value(0),
    },
];

/// Asks one question of the implementation.
fn answer(scan: &dyn FileScanUseCase, question: Question) -> Result<Value, Failure> {
    let session = scan.load(ScanLoadRequest {
        plans: &[],
        bindings: &[],
        given: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        stored: &[],
    })?;
    let sets = 0..session.rule_sets.len();
    Ok(match question {
        Question::RulesOfNoRule => sets
            .map(|at| scan.accepted(&session, RuleSetNumber(at)).len())
            .sum(),
        Question::RuleSetsWithoutAPack => sets.len().abs_diff(scan.packs(&session).len()),
        Question::RowsOfAnUnreadFile => {
            let (mut worker, mut rows) = (scan.worker(&session), Vec::new());
            let unread = file(0, "contract.not-a-language", b"anything");
            match scan.scan(&session, &mut worker, unread, &mut rows) {
                Scanned::NotWalked(_) => rows.len(),
                Scanned::Whole(_) | Scanned::Faulted(..) => usize::MAX,
            }
        }
    })
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(scan: &dyn FileScanUseCase) {
    run(CASES, Word::Succeeds, |question| answer(scan, question));
}
