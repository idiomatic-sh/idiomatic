//! The cases of the `placement` part: the one rule that gives a pattern its form, case by case of scope-engine3 2.6, and what
//! a load's statistics are read as.

use super::service::{placed, sampled};
use super::types::{CallForm, FormChoice, PatternFacts, Placement, SampleStage, TableForm};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::MeasuredForm;
use crate::domain::statistics::types::{LearnStage, ShapeChoice, Statistics};

type Input = (PatternFacts, SampleStage);
type Value = Placement;
type Failure = ();

/// A pattern that binds one column, has a program, and whose definition's table is keyed throughout: a flip candidate.
const CANDIDATE: PatternFacts = PatternFacts {
    binds: 1,
    call: CallForm::Keyed,
    table: TableForm::Keyed,
    call_patterns: 0,
    measured: None,
};

const fn answer(form: FormChoice, shape: ShapeChoice) -> Expect<Value, Failure> {
    Expect::Value(Placement { form, shape })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_that_binds_nothing_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                binds: 0,
                ..CANDIDATE
            };
            (facts, SampleStage::Absent)
        },
        expect: answer(FormChoice::Table, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("a_bucket_under_a_key_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                call: CallForm::Scans,
                ..CANDIDATE
            };
            (facts, SampleStage::Absent)
        },
        expect: answer(FormChoice::Table, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("with_no_statistics_a_keyed_pattern_takes_call_and_is_marked_a_candidate"),
        word: Word::Succeeds,
        input: || (CANDIDATE, SampleStage::Absent),
        expect: answer(FormChoice::Call, ShapeChoice::Open),
    },
    Case {
        name: CaseName("after_a_first_run_every_candidate_takes_table"),
        word: Word::Succeeds,
        input: || (CANDIDATE, SampleStage::First),
        expect: answer(FormChoice::Table, ShapeChoice::Open),
    },
    Case {
        name: CaseName("a_pattern_whose_table_holds_a_product_stage_never_flips"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                table: TableForm::Product,
                ..CANDIDATE
            };
            (facts, SampleStage::First)
        },
        expect: answer(FormChoice::Call, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("a_pattern_past_its_definitions_call_patterns_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                call_patterns: 4,
                ..CANDIDATE
            };
            (facts, SampleStage::Absent)
        },
        expect: answer(FormChoice::Table, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("after_a_second_run_a_candidate_takes_the_table_written_for_it"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                measured: Some(MeasuredForm::Table),
                ..CANDIDATE
            };
            (facts, SampleStage::Second)
        },
        expect: answer(FormChoice::Table, ShapeChoice::Open),
    },
    Case {
        name: CaseName("after_a_second_run_a_candidate_the_statistics_do_not_name_takes_call"),
        word: Word::Succeeds,
        input: || (CANDIDATE, SampleStage::Second),
        expect: answer(FormChoice::Call, ShapeChoice::Open),
    },
    Case {
        name: CaseName("a_definition_whose_table_runs_its_own_program_keeps_that_program"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                table: TableForm::Calls,
                call_patterns: 9,
                ..CANDIDATE
            };
            (facts, SampleStage::First)
        },
        expect: answer(FormChoice::Call, ShapeChoice::Decided),
    },
];

fn subject((facts, stage): Input) -> Result<Value, Failure> {
    Ok(placed(facts, stage))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A load given nothing is read as given no sample; a second run's statistics as a second run's.
#[test]
fn statistics_are_read_as_their_stage() {
    assert_eq!(sampled(&Statistics::default()), SampleStage::Absent);
    let second = Statistics {
        stage: LearnStage::Second,
        ..Statistics::default()
    };
    assert_eq!(sampled(&second), SampleStage::Second);
}
