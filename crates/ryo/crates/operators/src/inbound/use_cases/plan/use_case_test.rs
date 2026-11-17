//! The plan use case: a program is refused for its shape, and for a node whose rows grow as a product of a file's rows with
//! themselves that the query did not declare.
use super::interface::PlanUseCase as _;
use super::use_case::PlanUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::errors::GrowthError;
use crate::domain::growth::types::{Degree, GrowthKind, GrowthNode};
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::band_joins::types::{BandAnswer, BandEdge, BandJoin, BandTest};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, KeyFan, Side, StepId, StepScope,
};
use ryo_values::domain::plans::types::Operation;

#[derive(Clone, Copy)]
enum Scenario {
    ReadsEarlier,
    ReadsLater,
    EmitsUnknown,
    /// Two selects of `function` joined on the file alone with a band (`l.id < r.id`).
    FileOnlyBand,
    /// The FA100 shape: rows joined on the file alone to a set not proven distinct on it.
    FileOnlyJoin,
    /// The same join once the set is proven distinct on the file.
    DistinctFileSet,
}
type Input = Scenario;
type Value = usize;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_program_reading_only_earlier_nodes_is_planned"),
        word: Word::Succeeds,
        input: || Scenario::ReadsEarlier,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_node_reading_a_later_node_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ReadsLater,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
    Case {
        name: CaseName("an_emit_of_no_node_is_refused"),
        word: Word::Refuses,
        input: || Scenario::EmitsUnknown,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
    Case {
        name: CaseName("a_file_only_band_join_is_refused_at_plan"),
        word: Word::Refuses,
        input: || Scenario::FileOnlyBand,
        expect: Expect::Error(PortableOperatorFault::Superlinear(
            GrowthError::Superlinear {
                node: GrowthNode(2),
                kind: GrowthKind("BandJoin"),
                degree: Degree(2),
            },
        )),
    },
    Case {
        name: CaseName("a_file_only_join_against_a_non_distinct_set_is_refused"),
        word: Word::Refuses,
        input: || Scenario::FileOnlyJoin,
        expect: Expect::Error(PortableOperatorFault::Superlinear(
            GrowthError::Superlinear {
                node: GrowthNode(2),
                kind: GrowthKind("Probe"),
                degree: Degree(2),
            },
        )),
    },
    Case {
        name: CaseName("the_same_join_against_a_distinct_file_set_is_accepted"),
        word: Word::Succeeds,
        input: || Scenario::DistinctFileSet,
        expect: Expect::Value(3),
    },
];

/// The file (column 2) and the id (column 0) of `function`.
const FILE: ColumnIndex = ColumnIndex(2);
const ID: ColumnIndex = ColumnIndex(0);

/// Two selects of `function`, then `joined` reading them, emitted.
fn joined_program(joined: &Operation) -> Result<Value, Failure> {
    let operation = Operation::Select {
        from: StepInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let relation = [PlanInput::Relation(Relation::Function)];
    let both = [
        PlanInput::Node(PlanNodeId(0)),
        PlanInput::Node(PlanNodeId(1)),
    ];
    let mut program = LogicalProgram::new();
    for (operation, inputs) in [
        (&operation, &relation[..]),
        (&operation, &relation[..]),
        (joined, &both[..]),
    ] {
        let _id = program.node(LogicalNode {
            operation,
            inputs: PlanInputList(inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(2));
    PlanUseCaseImpl
        .plan(&program)
        .map(|plan| plan.node_count().0)
}

/// The join of the two selects on the file, answering the left id, its keys proven to pair `fan`.
fn file_join(fan: KeyFan) -> Operation {
    Operation::Join {
        left: StepInput::Step(StepId(0)),
        right: StepInput::Step(StepId(1)),
        left_on: vec![FILE],
        right_on: vec![FILE],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ID,
        }],
        filter: Vec::new(),
        view: Vec::new(),
        fan,
        reach: None,
    }
}

/// The nodes planned of a two-node program: a select of a relation, then a select of node `read`, emitting `emit`; or of a
/// join of two selects.
fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::FileOnlyBand => {
            return joined_program(&Operation::BandJoin(BandJoin {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Step(StepId(1)),
                left_on: vec![FILE],
                right_on: vec![FILE],
                band: vec![BandEdge {
                    left: ID,
                    test: BandTest::Below,
                    right: ID,
                }],
                answer: BandAnswer::Every,
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ID,
                }],
                filter: Vec::new(),
                view: Vec::new(),
                fan: KeyFan::FileOnly,
            }));
        }
        Scenario::FileOnlyJoin => return joined_program(&file_join(KeyFan::FileOnly)),
        Scenario::DistinctFileSet => return joined_program(&file_join(KeyFan::Unique)),
        Scenario::ReadsEarlier | Scenario::ReadsLater | Scenario::EmitsUnknown => {}
    }
    let operation = Operation::Select {
        from: StepInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let (read, emit) = match scenario {
        Scenario::ReadsEarlier => (0, 1),
        Scenario::ReadsLater => (1, 1),
        Scenario::EmitsUnknown => (0, 2),
        Scenario::FileOnlyBand | Scenario::FileOnlyJoin | Scenario::DistinctFileSet => (0, 1),
    };
    let relation = [PlanInput::Relation(Relation::Function)];
    let node = [PlanInput::Node(PlanNodeId(read))];
    let mut program = LogicalProgram::new();
    for inputs in [&relation[..], &node[..]] {
        let _id = program.node(LogicalNode {
            operation: &operation,
            inputs: PlanInputList(inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(emit));
    PlanUseCaseImpl
        .plan(&program)
        .map(|plan| plan.node_count().0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
