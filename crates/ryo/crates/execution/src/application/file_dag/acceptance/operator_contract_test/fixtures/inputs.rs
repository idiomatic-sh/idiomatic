//! The request a proof authors over tables: its plan, hosted relations and facts, assembled without running anything.
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
use std::cell::Cell;

/// Assemble explicit step data; the production compiler owns dependencies and schemas.
pub(crate) fn plan(operations: Vec<ryo_values::domain::plans::types::Operation>) -> Plan {
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, StepId, StepScope},
        plans::types::{Emit, PlanStep},
    };
    let emit = Emit {
        step: StepId((operations.len() - 1) as u32),
        subject: ColumnIndex(0),
        key: vec![],
        headings: vec![],
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit,
        steps: operations
            .into_iter()
            .map(|operation| PlanStep {
                scope: StepScope::PerFile,
                operation,
            })
            .collect(),
    }
}
/// Resolve published physical vocabulary; this does not evaluate an operation.
pub(crate) fn text<'a>(
    cell: ryo_relations::domain::relations::types::Value,
    symbols: &'a Symbols,
) -> &'a str {
    use ryo_execution_domain::domain::table_texts::{
        service::datum,
        types::{Datum, Texts},
    };
    let texts = Texts {
        symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    match datum(cell, &texts) {
        Datum::Text(text) => text.0,
        _ => panic!("published text cell required"),
    }
}
pub(crate) struct AuthoredRows {
    pub plan: Plan,
    pub hosted: Vec<HostRelation>,
    pub facts: Vec<(Relation, Table)>,
}
/// Only fixture input and plan assembly; callers execute through the real inbound API.
pub(crate) fn table_inputs(
    mut operation: ryo_values::domain::plans::types::Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
    symbols: &mut Symbols,
) -> AuthoredRows {
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{CellKind, Flag, Offset, Span, Value},
    };
    use ryo_values::domain::{
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRow},
        plan_parts::types::{ColumnIndex, HostedNumber, Input, ParameterName, StepId},
        plans::types::Operation,
        query_bindings::types::{ParameterType, PlanParameter},
        row_texts::types::Cell as HostCell,
    };
    let mut operations = vec![];
    let mut sources = vec![];
    let mut hosted = vec![];
    let mut facts = vec![];
    for (at, table) in inputs.iter().enumerate() {
        if table
            .kinds
            .iter()
            .all(|kind| matches!(kind, CellKind::Number | CellKind::Symbol | CellKind::File))
        {
            let columns = table
                .kinds
                .iter()
                .enumerate()
                .map(|(n, kind)| HostColumn {
                    name: HostColumnName(format!("column-{n}").into()),
                    kind: match kind {
                        CellKind::Number => HostKind::Number,
                        CellKind::Symbol => HostKind::Text,
                        CellKind::File => HostKind::File,
                        _ => unreachable!(),
                    },
                })
                .collect();
            let rows = (0..table.count.0)
                .map(|index| {
                    let width = table.kinds.len();
                    let row = &table.cells[index * width..(index + 1) * width];
                    HostRow(
                        row.iter()
                            .map(|cell| match *cell {
                                Value::Number(n) => HostCell::Number(n),
                                Value::Symbol(_) => HostCell::Text(text(*cell, symbols).into()),
                                Value::File(_) => HostCell::File("operator-contract.py".into()),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                })
                .collect();
            operations.push(Operation::Hosted {
                relation: HostedNumber(hosted.len() as u32),
            });
            hosted.push(HostRelation {
                name: HostName(format!("input-{at}").into()),
                columns,
                rows,
            });
        } else {
            let (relation, columns) = match table.kinds.as_slice() {
                [CellKind::Node] => (
                    if at == 2 {
                        Relation::Identifier
                    } else {
                        Relation::Function
                    },
                    vec![0],
                ),
                [CellKind::Node, CellKind::Node] => (Relation::Resolves, vec![0, 1]),
                [CellKind::Node, CellKind::Node, CellKind::File] => {
                    (Relation::Resolves, vec![0, 1, 4])
                }
                [CellKind::Node, CellKind::Number] => (Relation::SemanticChild, vec![0, 2]),
                [
                    CellKind::Node,
                    CellKind::Node,
                    CellKind::Number,
                    CellKind::Symbol,
                    CellKind::File,
                ] => (Relation::SemanticChild, vec![0, 1, 2, 3, 4]),
                [
                    CellKind::Number,
                    CellKind::Symbol,
                    CellKind::Node,
                    CellKind::Span,
                    CellKind::Flag,
                    CellKind::File,
                ] => {
                    if at == 0 {
                        (Relation::FormatParse, vec![6, 1, 0, 12, 8, 11])
                    } else {
                        (Relation::BraceFormatField, vec![1, 2, 0, 9, 6, 8])
                    }
                }
                [CellKind::Number, CellKind::Flag, CellKind::Symbol] => {
                    (Relation::FormatParse, vec![6, 8, 1])
                }
                [CellKind::Span] => {
                    if at == 0 {
                        (Relation::SourceLocation, vec![2])
                    } else {
                        (Relation::StringLiteral, vec![4])
                    }
                }
                [CellKind::Symbol, CellKind::Span] => (Relation::StringLiteral, vec![2, 4]),
                [CellKind::File, CellKind::Span] => (Relation::SourceLocation, vec![1, 2]),
                [CellKind::Span, CellKind::File] => (Relation::SourceLocation, vec![2, 1]),
                [
                    CellKind::Node,
                    CellKind::Symbol,
                    CellKind::File,
                    CellKind::Span,
                ] => (
                    if at == 0 {
                        Relation::Function
                    } else {
                        Relation::Identifier
                    },
                    vec![0, 1, 2, 3],
                ),

                _ => panic!("fixture must supply an actual schema for its typed data"),
            };
            for (&column, kind) in columns.iter().zip(&table.kinds) {
                assert_eq!(
                    kinds(relation)[column as usize],
                    *kind,
                    "fixture projection must follow its actual catalog"
                );
            }
            assert!(
                !facts.iter().any(|(old, _)| *old == relation),
                "distinct facts cannot masquerade as one provider snapshot"
            );
            let word = ryo_language::domain::symbols::service::intern_derived(
                symbols,
                ByteSlice(b""),
                NameText("fixture"),
            );
            let mut supplied = Table {
                kinds: kinds(relation).to_vec(),
                ..Table::default()
            };
            for row in table.cells.chunks(table.kinds.len()) {
                let mut cells = kinds(relation)
                    .iter()
                    .map(|kind| match kind {
                        CellKind::Node => {
                            Value::Node(ryo_relations::domain::relations::service::node_id(
                                FileId(0),
                                ryo_relations::domain::relations::types::Position(0),
                            ))
                        }
                        CellKind::Symbol => Value::Symbol(word),
                        CellKind::File => Value::File(FileId(0)),
                        CellKind::Number => {
                            Value::Number(ryo_relations::domain::relations::types::Integer(0))
                        }
                        CellKind::Span => Value::Span(Span {
                            start: Offset(0),
                            end: Offset(0),
                        }),
                        CellKind::Flag => Value::Flag(Flag::Lowered),
                    })
                    .collect::<Vec<_>>();
                for (&column, &value) in columns.iter().zip(row) {
                    cells[column as usize] = value;
                }
                supplied.cells.extend(cells);
                supplied.count.0 += 1;
            }
            facts.push((relation, supplied));
            operations.push(Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: columns.into_iter().map(ColumnIndex).collect(),
            });
        }
        sources.push(Input::Step(StepId((operations.len() - 1) as u32)));
    }
    let mut at = 0;
    ryo_values::domain::plans::service::each_input(&mut operation, &mut |input| {
        *input = *sources
            .get(at)
            .expect("authored fixture supplies every required input");
        at += 1;
    });
    operations.push(operation);
    let mut plan = plan(operations);
    plan.parameters = bindings
        .iter()
        .enumerate()
        .map(|(at, binding)| PlanParameter {
            name: ParameterName(format!("parameter-{at}").into()),
            kind: match binding {
                ParameterBinding::Number(_) => ParameterType::Number,
                ParameterBinding::Text(_) => ParameterType::Text,
                ParameterBinding::Numbers(_) => ParameterType::Numbers,
                ParameterBinding::Texts(_) => ParameterType::Texts,
                ParameterBinding::TextSets(_) => ParameterType::TextSets,
                ParameterBinding::Tables(_) => ParameterType::Tables,
                ParameterBinding::Flag(_) => ParameterType::Flag,
            },
        })
        .collect();
    AuthoredRows {
        plan,
        hosted,
        facts,
    }
}
