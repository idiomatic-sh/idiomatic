//! The cases of the `placement` part: the one rule that gives a pattern its form, case by case of scope-engine3 2.6, from
//! the statistics a load was given: none, a first run's, a second run's.

use super::service::{placed, sampled};
use super::types::{
    FormChoice, PatternFacts, PatternPlacement, PatternProgram, SampleStage, TableForm,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{MeasuredForm, ShapeChoice};
use crate::domain::statistics::types::{LearnStage, MeasuredSizes, Statistics};
use ryo_relations::domain::relations::types::Count;

/// A pattern's facts, and the statistics the load was given.
type Input = (PatternFacts, Option<Statistics>);
type Value = PatternPlacement;
type Failure = ();

/// A pattern that binds one column, has a program, and whose definition's table is keyed throughout: a flip candidate.
const CANDIDATE: PatternFacts = PatternFacts {
    binds: Count(1),
    call: PatternProgram::Keyed,
    table: TableForm::Keyed,
    call_patterns: Count(0),
    measured: None,
};

/// The statistics a load of a stage is given: none; a first run's, which names no unit; a second run's.
fn given(stage: SampleStage) -> Option<Statistics> {
    let of = |stage| Statistics {
        stage,
        sizes: MeasuredSizes::default(),
        units: Vec::new(),
    };
    match stage {
        SampleStage::Absent => None,
        SampleStage::First => Some(of(LearnStage::First)),
        SampleStage::Second => Some(of(LearnStage::Second)),
    }
}

const fn answer(form: FormChoice, shape: ShapeChoice) -> Expect<Value, Failure> {
    Expect::Value(PatternPlacement { form, shape })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_that_binds_nothing_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                binds: Count(0),
                ..CANDIDATE
            };
            (facts, given(SampleStage::Absent))
        },
        expect: answer(FormChoice::Table, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("a_bucket_under_a_key_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                call: PatternProgram::Scans,
                ..CANDIDATE
            };
            (facts, given(SampleStage::Absent))
        },
        expect: answer(FormChoice::Table, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("with_no_statistics_a_keyed_pattern_takes_call_and_is_marked_a_candidate"),
        word: Word::Succeeds,
        input: || (CANDIDATE, given(SampleStage::Absent)),
        expect: answer(FormChoice::Call, ShapeChoice::Open),
    },
    Case {
        name: CaseName("after_a_first_run_every_candidate_takes_table"),
        word: Word::Succeeds,
        input: || (CANDIDATE, given(SampleStage::First)),
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
            (facts, given(SampleStage::First))
        },
        expect: answer(FormChoice::Call, ShapeChoice::Decided),
    },
    Case {
        name: CaseName("a_pattern_past_its_definitions_call_patterns_is_a_table"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                call_patterns: Count(4),
                ..CANDIDATE
            };
            (facts, given(SampleStage::Absent))
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
            (facts, given(SampleStage::Second))
        },
        expect: answer(FormChoice::Table, ShapeChoice::Open),
    },
    Case {
        name: CaseName("after_a_second_run_a_candidate_the_statistics_do_not_name_takes_call"),
        word: Word::Succeeds,
        input: || (CANDIDATE, given(SampleStage::Second)),
        expect: answer(FormChoice::Call, ShapeChoice::Open),
    },
    Case {
        name: CaseName("a_definition_whose_table_runs_its_own_program_keeps_that_program"),
        word: Word::Succeeds,
        input: || {
            let facts = PatternFacts {
                table: TableForm::Calls,
                call_patterns: Count(9),
                ..CANDIDATE
            };
            (facts, given(SampleStage::First))
        },
        expect: answer(FormChoice::Call, ShapeChoice::Decided),
    },
];

fn subject((facts, statistics): Input) -> Result<Value, Failure> {
    Ok(placed(facts, sampled(statistics.as_ref())))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
