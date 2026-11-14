//! The rows the engine holds for a run after the files, through `plan` and `run` only: a withheld input is the fault naming the
//! engine's token, never empty rows; the host's rows are read once, in host order; a table parameter's rows are its tables'
//! texts in key order.
use super::super::super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::{PortableOperatorFault, WithheldInput};
use crate::domain::physical_plan::fixtures::Step;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInput, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::plan::interface::PlanUseCase as _;
use crate::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use crate::inbound::use_cases::run::interface::RunUseCase as _;
use crate::inbound::use_cases::run::use_case::RunUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::{
    HostedNumber, Input as StepInput, InputNumber, LiteralText, ParameterId, StepScope, TableKey,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    /// A select of a frontier entry the engine withholds with token 7.
    WithheldFrontier,
    /// A select of the host's rows of relation 0, the engine holding two rows.
    HostedRows,
    /// A table parameter of two tables, each with keys `b` then `a`, scanned under `a` then `b`.
    TableRows,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// How many times the host's rows were read, and whether the rows came in host order.
    Hosted(usize, bool),
    /// How many rows the scan made, and whether each row holds its table's texts in key order.
    Shape(RowCount, bool),
}
type Input = Scenario;
type Answer = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Answer, Failure>] = &[
    Case {
        name: CaseName("a_withheld_input_faults_with_its_token"),
        word: Word::Refuses,
        input: || Scenario::WithheldFrontier,
        expect: Expect::Error(PortableOperatorFault::Withheld(WithheldInput(7))),
    },
    Case {
        name: CaseName("hosted_rows_are_read_once_in_order"),
        word: Word::Succeeds,
        input: || Scenario::HostedRows,
        expect: Expect::Value(Observation::Hosted(1, true)),
    },
    Case {
        name: CaseName("rows_answer_the_bound_tables_in_key_order"),
        word: Word::Succeeds,
        input: || Scenario::TableRows,
        expect: Expect::Value(Observation::Shape(RowCount(2), true)),
    },
];

/// A select of everything `from` reads.
fn all_of(from: PlanInput) -> Step {
    Step {
        operation: Operation::Select {
            from: StepInput::Step(ryo_values::domain::plan_parts::types::StepId(0)),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        inputs: vec![from],
    }
}

fn subject(scenario: Input) -> Result<Answer, Failure> {
    let mut resources = TextResources::new(&["a", "b"]);
    match scenario {
        Scenario::WithheldFrontier => {
            let inputs = TableInputs::new(&[]).withholding(WithheldInput(7));
            let place = PlanInput::Frontier(FrontierPlace::of(RuleOrdinal(0), InputNumber(0)));
            emitted(
                &[all_of(place)],
                &[PlanNodeId(0)],
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            )
            .map(|_| Observation::Shape(RowCount(0), false))
        }
        Scenario::HostedRows => {
            let mut host = Table {
                kinds: vec![CellKind::Number],
                ..Table::default()
            };
            push_row(&mut host, &[Value::Number(Integer(2))]);
            push_row(&mut host, &[Value::Number(Integer(1))]);
            let inputs = TableInputs::new(&[]).with_hosted(vec![host]);
            let hosted = Step {
                operation: Operation::Hosted {
                    relation: HostedNumber(0),
                },
                inputs: Vec::new(),
            };
            let rows = emitted(
                &[hosted, all_of(PlanInput::Node(PlanNodeId(0)))],
                &[PlanNodeId(1)],
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            )?;
            let first = rows
                .first()
                .map(|table| table.cells.clone())
                .unwrap_or_default();
            Ok(Observation::Hosted(
                inputs.hosted_reads(),
                first == [Value::Number(Integer(2)), Value::Number(Integer(1))],
            ))
        }
        Scenario::TableRows => {
            let table = |b: &str, a: &str| {
                BindingTable(vec![
                    BindingCell {
                        key: LiteralText("b".into()),
                        value: LiteralText(b.into()),
                    },
                    BindingCell {
                        key: LiteralText("a".into()),
                        value: LiteralText(a.into()),
                    },
                ])
            };
            let bindings = [ParameterBinding::Tables(vec![table("b", "a"), table("b", "b")])];
            let operation = Operation::Rows {
                parameter: ParameterId(0),
                keys: vec![TableKey("a".into()), TableKey("b".into())],
            };
            let mut program = LogicalProgram::new();
            let node = program.node(LogicalNode {
                operation: &operation,
                inputs: PlanInputList(&[]),
                bindings: NodeBindings(&bindings),
                scope: StepScope::CrossFile,
            });
            program.emit(RuleOrdinal(0), node);
            let plan = PlanUseCaseImpl.plan(&program)?;
            let mut buffers = FileBuffers::default();
            let inputs = TableInputs::new(&[]);
            RunUseCaseImpl::new(std::sync::Arc::new(crate::outbound::matching::factory::use_case()))
                .run(&plan, node, &mut buffers, &inputs, &mut resources)?;
            match buffers.rows(node) {
                NodeRows::Materialized(table) => {
                    let ordered = [resources.symbol("a"), resources.symbol("b")]
                        .into_iter()
                        .chain([resources.symbol("b"), resources.symbol("b")])
                        .eq(table.cells.iter().copied());
                    Ok(Observation::Shape(table.count, ordered))
                }
                NodeRows::Absent => Err(PortableOperatorFault::NotRun),
            }
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
