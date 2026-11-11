//! Select, count and refusal cases through the prepared file program.
mod evaluation;
#[cfg(test)]
mod fixtures;
use fixtures::{ProgramShape, plan, rule};

use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use ryo_execution_domain::domain::table_cells::fixtures::{
    SMALL_SOURCE, SmallTree, rows_over, small_tree,
};
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    operating::errors::OperatingError,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_model::types::Rows,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Integer, Relation},
};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plan_parts::types::{Input as PlanInput, LiteralText, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;
type Input = ProgramShape;
type Value = (usize, &'static str);
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_elements_of_the_tree_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || ProgramShape::Words,
        expect: Expect::Value((2, "1 _ 3 _")),
    },
    Case {
        name: CaseName("the_rows_of_a_step_whose_element_is_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || ProgramShape::RowsOfWords,
        expect: Expect::Value((2, "1 3")),
    },
    Case {
        name: CaseName("a_projection_that_drops_repeats_keeps_the_first_row_of_each_kind"),
        word: Word::Succeeds,
        input: || ProgramShape::DistinctKinds,
        expect: Expect::Value((2, "word other")),
    },
    Case {
        name: CaseName("a_projection_that_keeps_repeats_keeps_every_row"),
        word: Word::Succeeds,
        input: || ProgramShape::KeptKinds,
        expect: Expect::Value((3, "word other word")),
    },
    Case {
        name: CaseName("a_count_of_lowered_rows_is_passed_on_with_the_keys"),
        word: Word::Succeeds,
        input: || ProgramShape::Counted,
        expect: Expect::Value((1, "_ 2")),
    },
    Case {
        name: CaseName("native_antijoin_preserves_missing_key_rows"),
        word: Word::Succeeds,
        input: || ProgramShape::NativeAntijoin,
        expect: Expect::Value((1, "other")),
    },
    Case {
        name: CaseName("native_union_preserves_first_distinct_keys"),
        word: Word::Succeeds,
        input: || ProgramShape::NativeUnion,
        expect: Expect::Value((2, "word other")),
    },
    Case {
        name: CaseName("native_join_pairs_each_matching_step_row"),
        word: Word::Succeeds,
        input: || ProgramShape::NativeJoinSteps,
        expect: Expect::Value((2, "1 1 3 1")),
    },
    Case {
        name: CaseName("native_join_follows_tree_node"),
        word: Word::Succeeds,
        input: || ProgramShape::NativeJoinTree,
        expect: Expect::Value((1, "2 2")),
    },
    Case {
        name: CaseName("non_tree_join_uses_registered_operator_after_native_decline"),
        word: Word::Succeeds,
        input: || ProgramShape::OrdinaryFunctionJoin,
        expect: Expect::Value((1, "2")),
    },
    Case {
        name: CaseName("missing_demanded_right_source_refuses_instead_of_empty_success"),
        word: Word::Refuses,
        input: || ProgramShape::MissingRight,
        expect: Expect::Error(OperatingError::NotRead),
    },
];

/// Runs one scenario through the prepared file program the evaluation group's fixtures build.
fn subject(input: Input) -> Result<Value, Failure> {
    evaluation::fixtures::subject(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod errored_rule;

#[cfg(test)]
mod source_partition;
