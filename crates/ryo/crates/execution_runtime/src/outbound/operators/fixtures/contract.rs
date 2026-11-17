//! Behavioral contract shared by the injected operator adapter and test fake, driven through the port's `plan` and `run`.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Integer, Position, Relation, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Source, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

/// The ordinals of the edge rows, in row order, and the column they are counted by.
type Input = (Vec<i64>, ColumnIndex);
/// Each group's key and how many rows it counted, in first-occurrence order.
type Value = Cow<'static, [(i64, i64)]>;
type Failure = PortableOperatorFault;

/// Observable group counts and key-column refusal across the port.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_are_counted_into_their_groups_in_first_row_order"),
        word: Word::Succeeds,
        input: || (vec![7, 8, 7, 7], ColumnIndex(2)),
        expect: Expect::Value(Cow::Borrowed(&[(7, 3), (8, 1)])),
    },
    Case {
        name: CaseName("an_absent_key_column_is_refused"),
        word: Word::Refuses,
        input: || (vec![7], ColumnIndex(9)),
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(9),
        }),
    },
];

/// The rows the one node of `operation` answers through `adapter` over `inputs`, or the fault its `plan` or `run` answers.
fn answered(
    adapter: &impl OperatorsPort,
    operation: &Operation,
    read: Relation,
    inputs: &TableInputs,
    resources: &mut TextResources,
) -> Result<Table, PortableOperatorFault> {
    let reads = [PlanInput::Relation(read)];
    let mut program = LogicalProgram::new();
    let node = program.node(LogicalNode {
        operation,
        inputs: PlanInputList(&reads),
        bindings: NodeBindings(&[]),
        scope: StepScope::PerFile,
    });
    program.emit(RuleOrdinal(0), node);
    let plan = adapter.plan(&program)?;
    let mut buffers = FileBuffers::default();
    adapter.run(&plan, PlanNodeId(0), &mut buffers, inputs, resources)?;
    Ok(match buffers.rows(node) {
        NodeRows::Materialized(rows) => rows.clone(),
        NodeRows::Absent => Table::default(),
    })
}

/// Counts the edge rows of `ordinals` by `key` through `adapter`: each group's key and count.
///
/// # Errors
///
/// The fault the port answers.
pub fn count(
    adapter: &impl OperatorsPort,
    ordinals: &[i64],
    key: ColumnIndex,
) -> Result<Vec<(i64, i64)>, PortableOperatorFault> {
    // The edge rows the cases count (parent, child, ordinal, field, file): each from node 0 to node 1, of field `f` in file 0.
    let edges = Relation::SemanticChild;
    let mut resources = TextResources::new(&["f"]);
    let mut cells = Vec::with_capacity(ordinals.len() * kinds(edges).len());
    for ordinal in ordinals {
        cells.extend([
            Cell::Node(node_id(FileId(0), Position(0))),
            Cell::Node(node_id(FileId(0), Position(1))),
            Cell::Number(Integer(*ordinal)),
            resources.symbol("f"),
            Cell::File(FileId(0)),
        ]);
    }
    let table = Table {
        kinds: kinds(edges).to_vec(),
        cells,
        count: RowCount(ordinals.len()),
    };
    let inputs = TableInputs::new(&[(edges, &table)]);
    let operation = Operation::GroupCount {
        from: Source::Relation(edges),
        keys: vec![key],
        distinct: None,
    };
    let rows = answered(adapter, &operation, edges, &inputs, &mut resources)?;
    Ok(rows
        .cells
        .chunks(rows.kinds.len().max(1))
        .filter_map(|row| match row {
            [Cell::Number(Integer(key)), Cell::Number(Integer(count))] => Some((*key, *count)),
            _ => None,
        })
        .collect())
}

/// Run the same observable cases for every implementation of the engine port.
///
/// # Panics
///
/// When the implementation changes the group counts or their refusal, or answers a refused relation as empty rows.
pub fn check(adapter: &impl OperatorsPort) {
    let subject = |(ordinals, key): Input| count(adapter, &ordinals, key).map(Cow::Owned);
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    let select = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    assert_eq!(
        answered(
            adapter,
            &select,
            Relation::Function,
            &TableInputs::new(&[]),
            &mut TextResources::new(&[]),
        )
        .map(|rows| rows.count.0),
        Err(PortableOperatorFault::NotRun),
        "a relation the facts refuse is a fault of the run, never empty rows"
    );
}
