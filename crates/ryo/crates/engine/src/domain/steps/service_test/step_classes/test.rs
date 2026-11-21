//! The cost class of each step: one row for each instruction, and one for each way an instruction's operands change its class.

use super::fixtures::{Scenario, step};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::steps::service::step_class;
use crate::domain::steps::types::CostClass;

type Input = Scenario;
type Value = CostClass;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_kind_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Kind,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("each_match_on_a_register_is_a_keys_fanout"),
        word: Word::Succeeds,
        input: || Scenario::MatchRegister,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("each_match_on_constants_alone_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::MatchConstants,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("each_match_over_a_written_table_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::MatchWritten,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("each_range_with_both_bounds_registers_is_a_keys_fanout"),
        word: Word::Succeeds,
        input: || Scenario::RangeRegisters,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("each_range_on_constants_alone_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::RangeConstants,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("each_row_of_a_relation_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::RowRelation,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("each_row_of_a_definitions_table_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::RowDefinition,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("each_row_of_a_written_table_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::RowWritten,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("children_are_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Children,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("the_parent_axis_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::AxisParent,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_descendant_axis_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::AxisDescendant,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("within_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Within,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("nearest_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Nearest,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("nearest_over_edges_that_collect_rows_with_nothing_bound_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::NearestUnbound,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("closure_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Closure,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("read_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Read,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("test_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Test,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_call_of_a_meaning_that_gives_at_most_one_row_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Call,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_call_of_a_meaning_that_gives_several_rows_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::CallSeveral,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("render_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Render,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("let_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Let,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("exists_over_an_inline_body_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::ExistsInline,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("exists_over_a_use_that_binds_nothing_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::ExistsUnbound,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("exists_over_a_use_that_binds_a_column_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::ExistsBound,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("or_over_inline_bodies_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::OrInline,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("or_over_a_use_that_binds_a_column_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::OrBound,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("or_over_a_use_that_binds_nothing_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::OrUnbound,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("fork_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Fork,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("yield_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Yield,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("latest_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Latest,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("emit_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Emit,
        expect: Expect::Value(CostClass::Constant),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(step_class(&step(scenario)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
