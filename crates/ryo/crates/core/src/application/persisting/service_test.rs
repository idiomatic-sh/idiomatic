//! The `persisting` service's cases: only a run that answers findings keeps them, once, and a store that cannot keep fails the run.
use super::service::kept;
use super::types::KeepRun;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::domain::render::types::{AnswerColumns, Answering, RowsAnswer};
use crate::domain::storing::types::ToKeep;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::lint_store::interface::LintStorePort as _;
use crate::outbound::lint_store::types::StoreKeeping;
use ryo_values::domain::row_texts::types::{FindingRows, RowTotal};
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Reuse};
use std::sync::atomic::Ordering;

/// What a row's run answers, and whether its store can keep.
#[derive(Clone, Copy, Debug)]
enum Keeping {
    /// A run that answers findings, over a store that keeps.
    Findings,
    /// A run that answers the rules' own rows.
    Rows,
    /// A run that answers findings, over a store that cannot keep.
    Unwritable,
}

type Input = Keeping;
type Value = usize;
type Failure = LintStoreError;

fn subject(keeping: Input) -> Result<Value, Failure> {
    let store = LintStoreFake {
        unwritable: matches!(keeping, Keeping::Unwritable),
        ..LintStoreFake::default()
    };
    let keeps = std::sync::Arc::clone(&store.keeps);
    let read = store.stored()?;
    let stored = ryo_values::domain::stores::types::Stored::default();
    let answering = match keeping {
        Keeping::Rows => Answering::Rows(RowsAnswer {
            limit: RowTotal(0),
            headings: Vec::new(),
            columns: AnswerColumns::One,
        }),
        Keeping::Findings | Keeping::Unwritable => Answering::Findings,
    };
    let later = FindingRows::default();
    kept(
        StoreKeeping::Keeps,
        Some(&read),
        &KeepRun {
            answering: &answering,
            again: &Reuse::Whole,
            read_again: &[],
            stored: &stored,
            metadata: &[],
            first_bucket: &[],
            wanted: &BucketsWanted::All,
            handed: &ReadHanded::default(),
        },
        ToKeep {
            rules: Vec::new(),
            records: Vec::new(),
            later: &later,
            refused: &[],
            unknowns: &[],
            skipped: &[],
            consulted: &[],
            consulted_absent: &[],
            project_observations: &[],
        },
    )?;
    Ok(keeps.load(Ordering::Relaxed))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_run_that_answers_findings_keeps_them_once"),
        word: Word::Succeeds,
        input: || Keeping::Findings,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_run_that_answers_its_rules_own_rows_keeps_nothing"),
        word: Word::Succeeds,
        input: || Keeping::Rows,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_store_that_cannot_keep_fails_the_run"),
        word: Word::Fails,
        input: || Keeping::Unwritable,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintStoreError::Unwritable { .. })))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
