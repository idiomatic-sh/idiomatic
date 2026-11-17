//! Rows: a node can have a row in a file only as its operator's presence mask declares over its inputs' rows, a root
//! select only where the file holds one of its kinds.
use super::super::super::fixtures::{
    CLASS, FACT, Program, activity_of, antijoin, join, node, project, root, union,
};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::StepScope;
use std::borrow::Cow;

type Input = Program;
/// Each node's activity in the file, asked in node order (`A` active, `D` dormant), then the relations read, in order.
type Value = Cow<'static, str>;
type Failure = OperatingError;

fn subject(program: Input) -> Result<Value, Failure> {
    activity_of(&program)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_is_dormant_when_its_left_kind_is_absent"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
            ],
            roots: vec![(CLASS, vec![0])],
            held: Vec::new(),
            empty: Vec::new(),
            refused: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("DD; reads [Function]")),
    },
    Case {
        name: CaseName("a_root_of_two_kinds_runs_where_the_file_holds_either"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
            ],
            roots: vec![(CLASS, vec![0]), ("function_definition", vec![0])],
            held: vec!["function_definition"],
            empty: Vec::new(),
            refused: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("AA; reads [Function]")),
    },
    Case {
        name: CaseName("an_antijoin_runs_where_its_left_has_rows_and_its_right_none"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                antijoin(node(0), PlanInput::Relation(FACT)),
            ],
            roots: vec![(CLASS, vec![0])],
            held: vec![CLASS],
            empty: vec![FACT],
            refused: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("AA; reads [Function]")),
    },
    Case {
        name: CaseName("a_union_runs_where_one_side_has_rows"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![root(Vec::new()), union(node(0), PlanInput::Relation(FACT))],
            roots: vec![(CLASS, vec![0])],
            held: Vec::new(),
            empty: Vec::new(),
            refused: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("DA; reads [Function]")),
    },
    Case {
        name: CaseName("every_trigger_present_makes_every_node_active"),
        word: Word::Succeeds,
        input: || Program {
            nodes: vec![
                root(Vec::new()),
                join(node(0), PlanInput::Relation(FACT), Vec::new()),
                antijoin(node(1), PlanInput::Relation(FACT)),
                project(node(2), StepScope::PerFile),
            ],
            roots: vec![(CLASS, vec![0])],
            held: vec![CLASS],
            empty: Vec::new(),
            refused: Vec::new(),
        },
        expect: Expect::Value(Cow::Borrowed("AAAA; reads [Function, Function]")),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
