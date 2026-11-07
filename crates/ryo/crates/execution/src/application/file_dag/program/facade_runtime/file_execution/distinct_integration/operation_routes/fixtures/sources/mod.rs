//! The independent source rows of every operation route.
use super::super::super::*;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures;
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostedCell;
/// Data-only fixture assembly: scalar snapshots and canonical provider rows.
/// Every operation, schema, publication and dispatch below belongs to production.
#[derive(Default)]
pub(crate) struct Sources {
    pub(crate) steps: Vec<PlanStep>,
    pub(crate) hosts: Vec<HostRelation>,
    pub(crate) providers: Vec<(Relation, Table)>,
}
impl Sources {
    pub(crate) fn push(&mut self, operation: Operation) -> PlanInput {
        let at = self.steps.len();
        self.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation,
        });
        PlanInput::Step(StepId(at as u32))
    }
    pub(crate) fn snapshot(&mut self, table: &Table, symbols: &Symbols) -> HostedNumber {
        let columns = table
            .kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: match kind {
                    CellKind::Number => HostKind::Number,
                    CellKind::Symbol => HostKind::Text,
                    other => panic!("scalar fixture snapshot cannot invent {other:?}"),
                },
            })
            .collect();
        let rows = table
            .cells
            .chunks_exact(table.kinds.len())
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|cell| match cell {
                            Cell::Number(number) => HostedCell::Number(*number),
                            Cell::Symbol(symbol) => HostedCell::Text(
                                text_of(symbols, ByteSlice(fixtures::expectations::SOURCE), *symbol)
                                    .expect("fixture symbol text")
                                    .0
                                    .into(),
                            ),
                            other => panic!("scalar fixture cell {other:?}"),
                        })
                        .collect(),
                )
            })
            .collect();
        let number = HostedNumber(self.hosts.len() as u32);
        self.hosts.push(HostRelation {
            name: HostName(format!("route-input-{}", number.0).into()),
            columns,
            rows,
        });
        number
    }
    pub(crate) fn free_tree_schema(&self) -> Relation {
        if self
            .providers
            .iter()
            .all(|(relation, _)| *relation != Relation::SemanticSyntax)
        {
            Relation::SemanticSyntax
        } else {
            assert!(
                self.providers
                    .iter()
                    .all(|(relation, _)| *relation != Relation::SourceLocation)
            );
            Relation::SourceLocation
        }
    }
    pub(crate) fn provider(&mut self, relation: Relation, rows: Vec<Vec<Cell>>) -> PlanInput {
        let count = rows.len();
        self.providers.push((
            relation,
            Table {
                kinds: kinds(relation).to_vec(),
                cells: rows.into_iter().flatten().collect(),
                count: RowCount(count),
            },
        ));
        PlanInput::Relation(relation)
    }
    pub(crate) fn base(relation: Relation, at: usize) -> Vec<Cell> {
        kinds(relation)
            .iter()
            .copied()
            .map(|kind| match kind {
                CellKind::Node => {
                    Cell::Node(ryo_relations::domain::relations::types::NodeId(at as u64))
                }
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            })
            .collect()
    }
    pub(crate) fn projection(&mut self, from: PlanInput, columns: Vec<ColumnIndex>) -> PlanInput {
        self.push(Operation::Select {
            from,
            comparisons: vec![],
            kept: columns,
        })
    }
    pub(crate) fn input(&mut self, table: &Table, symbols: &Symbols) -> PlanInput {
        if table
            .kinds
            .iter()
            .all(|kind| matches!(kind, CellKind::Number | CellKind::Symbol))
        {
            let relation = self.snapshot(table, symbols);
            return self.push(Operation::Hosted { relation });
        }
        if table.kinds == [CellKind::Node; 2] {
            let relation = Relation::SemanticChild;
            let rows = table
                .cells
                .chunks_exact(2)
                .enumerate()
                .map(|(at, row)| {
                    let mut cells = Self::base(relation, at);
                    cells[..2].copy_from_slice(row);
                    cells
                })
                .collect();
            let from = self.provider(relation, rows);
            return self.projection(from, vec![ColumnIndex(0), ColumnIndex(1)]);
        }
        if table.kinds == [CellKind::Span; 2] {
            let mut sides = Vec::new();
            for side in 0..2 {
                let relation = self.free_tree_schema();
                let schema = kinds(relation);
                let span = schema
                    .iter()
                    .position(|kind| *kind == CellKind::Span)
                    .unwrap();
                let rows = table
                    .cells
                    .chunks_exact(2)
                    .enumerate()
                    .map(|(at, row)| {
                        let mut cells = Self::base(relation, at);
                        cells[span] = row[side];
                        cells
                    })
                    .collect();
                let from = self.provider(relation, rows);
                sides.push(self.projection(from, vec![ColumnIndex(0), ColumnIndex(span as u32)]));
            }
            return self.push(Operation::Join {
                left: sides[0],
                right: sides[1],
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            });
        }
        let relation = self.free_tree_schema();
        let schema = kinds(relation);
        let projected = table
            .kinds
            .iter()
            .map(|kind| {
                ColumnIndex(
                    schema
                        .iter()
                        .position(|column| column == kind)
                        .expect("canonical fixture schema") as u32,
                )
            })
            .collect::<Vec<_>>();
        let rows = table
            .cells
            .chunks_exact(table.kinds.len())
            .enumerate()
            .map(|(at, row)| {
                let mut cells = Self::base(relation, at);
                for (column, value) in projected.iter().zip(row) {
                    cells[column.0 as usize] = *value;
                }
                cells
            })
            .collect();
        let from = self.provider(relation, rows);
        self.projection(from, projected)
    }
}
