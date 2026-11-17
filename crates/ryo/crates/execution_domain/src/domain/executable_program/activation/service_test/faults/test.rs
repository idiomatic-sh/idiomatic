//! Row errors: a node with no row still runs where it can raise or carry one: a fallible input with rows, a fault carried
//! from an input, a literal pattern compiled at its first row, a root select's fallible suffix, a live node.
use super::super::super::fixtures::{
    CLASS, FACT, Program, activity_of, fact_select, join, matches, node, project, root,
};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    HostedNumber, InputNumber, ObservationBindingId, StepScope,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

type Input = Program;
/// Each node's activity in the file: `A` active, `D` dormant, in node order.
type Value = Cow<'static, str>;
type Failure = OperatingError;

fn subject(program: Input) -> Result<Value, Failure> {
    activity_of(&program)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fallible_input_with_rows_keeps_a_rowless_node_running"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                join(node(0), PlanInput::Relation(FACT), vec![matches(false)]),
            ],
            roots: vec![(CLASS, vec![0])],
            held: Vec::new(),
            empty: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("DA")),
    },
    Case {
        name: CaseName("a_fault_carried_from_an_input_keeps_its_reader_running"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                fact_select(vec![matches(false)]),
                root(Vec::new()),
                join(node(1), node(0), Vec::new()),
                project(node(2), StepScope::PerFile),
            ],
            roots: vec![(CLASS, vec![1])],
            held: Vec::new(),
            empty: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("ADAA")),
    },
    Case {
        name: CaseName("a_live_node_runs_in_every_file"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                project(node(0), StepScope::CrossFile),
                project(
                    PlanInput::Frontier(FrontierPlace::of(RuleOrdinal(0), InputNumber(0))),
                    StepScope::PerFile,
                ),
                join(
                    node(0),
                    PlanInput::Observation(BoundObservationPlace::of(
                        RuleOrdinal(0),
                        ObservationBindingId(0),
                    )),
                    Vec::new(),
                ),
            ],
            roots: vec![(CLASS, vec![0])],
            held: Vec::new(),
            empty: vec![FACT],
        },
        expect: Expect::Value(Cow::Borrowed("DAAA")),
    },
    Case {
        name: CaseName("a_literal_pattern_select_with_rows_keeps_its_reader_running"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                fact_select(vec![matches(true)]),
                root(Vec::new()),
                join(node(1), node(0), Vec::new()),
            ],
            roots: vec![(CLASS, vec![1])],
            held: Vec::new(),
            empty: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("ADA")),
    },
    Case {
        name: CaseName("a_root_select_with_a_fallible_suffix_keeps_its_reader_running"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(vec![matches(true)]),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
            ],
            roots: vec![(CLASS, vec![0])],
            held: vec![CLASS],
            empty: vec![FACT],
        },
        expect: Expect::Value(Cow::Borrowed("AA")),
    },
    Case {
        name: CaseName("a_root_select_with_a_fallible_suffix_is_dormant_where_its_kind_is_absent"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(vec![matches(true)]),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
            ],
            roots: vec![(CLASS, vec![0])],
            held: Vec::new(),
            empty: vec![FACT],
        },
        expect: Expect::Value(Cow::Borrowed("DD")),
    },
    Case {
        name: CaseName("a_hosted_node_is_live"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                (
                    Operation::Hosted {
                        relation: HostedNumber(0),
                    },
                    Vec::new(),
                    StepScope::PerFile,
                ),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
            ],
            roots: Vec::new(),
            held: Vec::new(),
            empty: vec![FACT],
        },
        expect: Expect::Value(Cow::Borrowed("AA")),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
