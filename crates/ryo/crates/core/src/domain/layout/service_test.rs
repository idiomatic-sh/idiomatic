//! The `layout` service's cases, on the plans of the `pipeline`'s cases.

use ryo_language::domain::relations::types::Relation;
use ryo_language::domain::symbols::types::Symbols;

use super::service::{layout_of, reads_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::{PlanIdentity, StepPlace};
use crate::domain::pipeline::service_test::{KINDS, SOURCE, counting_plan, functions, plan};
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::ColumnsView;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lays out a plan whose last step is emitted.
    LaysOutAnEmittingPlan,
    /// Lays out a plan whose first step is handed to the cross-file steps.
    LaysOutAHandingPlan,
    /// Counts the steps that read each step of a plan.
    CountsReads,
    /// Lays out a plan over a relation nothing provides.
    LaysOutWithoutTheRelation,
}

/// What a row of the cases does.
type Input = Scenario;
/// The steps that keep their rows, times ten, plus the columns of the widest step, or the reads counted.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let columns = functions(&mut symbols);
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let provided = |relation: Relation| {
        if relation == Relation::Function {
            Ok(Rows::Stored(ColumnsView {
                kinds: KINDS,
                columns: &columns,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let nothing = |_: Relation| Err(OperatingError::NotRead);
    match scenario {
        Scenario::LaysOutAnEmittingPlan | Scenario::LaysOutAHandingPlan => {
            let plan = if matches!(scenario, Scenario::LaysOutAnEmittingPlan) {
                plan()
            } else {
                counting_plan()
            };
            let layout = layout_of(&plan, PlanIdentity(0), &provided, &mut context)?;
            let kept = layout.kept.0.iter().filter(|kept| **kept).count();
            Ok(kept * 10 + layout.kinds.iter().map(Vec::len).max().unwrap_or(0))
        }
        Scenario::CountsReads => {
            let plan = plan();
            Ok(reads_of(&plan, StepPlace(0)).0 * 10 + reads_of(&plan, StepPlace(1)).0)
        }
        Scenario::LaysOutWithoutTheRelation => {
            layout_of(&plan(), PlanIdentity(0), &nothing, &mut context)
                .map(|layout| layout.kinds.len())
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only_the_emitted_step_keeps_its_rows_when_each_is_read_once"),
        word: Word::Succeeds,
        input: || Scenario::LaysOutAnEmittingPlan,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("the_step_handed_to_the_cross_file_steps_keeps_its_rows"),
        word: Word::Succeeds,
        input: || Scenario::LaysOutAHandingPlan,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_step_read_by_a_step_and_emitted_is_read_once_each_way"),
        word: Word::Succeeds,
        input: || Scenario::CountsReads,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_plan_over_a_relation_nobody_read_is_refused"),
        word: Word::Refuses,
        input: || Scenario::LaysOutWithoutTheRelation,
        expect: Expect::Error(OperatingError::NotRead),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
