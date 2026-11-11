//! Canonical provider data and its physical encoding; no evaluator or admission factory.
use ryo_execution_domain::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_execution_domain::domain::table_model::service::materialize;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, NodeId};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Flag, Integer, Offset, Relation, Span, Value as Cell},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::{CompactTable, DecodeMode, WordSlice};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
};
use std::sync::Arc;
/// Canonical provider data and its physical encoding; no evaluator or admission factory.
pub(crate) struct Provider<'a> {
    pub(crate) relation: Relation,
    pub(crate) table: Table,
    pub(crate) columns: Vec<FactColumn>,
    pub(crate) compact: Option<CompactTable>,
    pub(crate) tree: Option<TreeRows<'a>>,
    pub(crate) representation: RepresentationKind,
}
impl Provider<'_> {
    pub(crate) fn rows(&self) -> Rows<'_> {
        if let Some(tree) = self.tree {
            return Rows::Tree(tree);
        }
        match self.representation {
            RepresentationKind::Stored => Rows::Stored(ColumnsView {
                kinds: kinds(self.relation),
                columns: &self.columns,
            }),
            RepresentationKind::Compact => Rows::Compact(self.compact.as_ref().unwrap()),
            _ => Rows::Computed(&self.table),
        }
    }
}
/// A stored provider of `relation` holding `table`, with its columns encoded as the provider's facts.
pub(crate) fn stored(relation: Relation, table: Table) -> Provider<'static> {
    let columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(table.kinds.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    Provider {
        relation,
        table,
        columns,
        compact: None,
        tree: None,
        representation: RepresentationKind::Stored,
    }
}
pub(crate) fn defaults(kind: CellKind, row: usize) -> Cell {
    match kind {
        CellKind::Node => Cell::Node(NodeId(row as u64)),
        CellKind::Symbol => Cell::Symbol(Symbol(0)),
        CellKind::File => Cell::File(FileId(0)),
        CellKind::Span => Cell::Span(Span {
            start: Offset(0),
            end: Offset(0),
        }),
        CellKind::Number => Cell::Number(Integer(0)),
        CellKind::Flag => Cell::Flag(Flag::Lowered),
    }
}
/// Project a truthful catalog schema onto the fixture's logical tuple. Entry and
/// SequenceElement each have multiple numeric/node columns and actual Flag fields.
pub(crate) fn provider<'a>(
    rows: Rows<'a>,
    side: usize,
    steps: &mut Vec<PlanStep>,
) -> (Provider<'a>, PlanInput) {
    if let Rows::Tree(tree) = rows {
        let relation = match tree.form {
            TreeForm::Elements => Relation::SyntaxTree,
            TreeForm::Edges => Relation::SyntaxChild,
        };
        return (
            Provider {
                relation,
                table: Table::default(),
                columns: vec![],
                compact: None,
                tree: Some(tree),
                representation: RepresentationKind::Tree,
            },
            PlanInput::Relation(relation),
        );
    }
    let representation = match &rows {
        Rows::Stored(_) => RepresentationKind::Stored,
        Rows::Compact(_) => RepresentationKind::Compact,
        _ => RepresentationKind::Computed,
    };
    let mut logical = Table::default();
    materialize(&rows, &mut logical, &mut Vec::new());
    let relation = if side == 0 {
        Relation::Entry
    } else {
        Relation::SequenceElement
    };
    let schema = kinds(relation);
    let mut used = Vec::new();
    let projection = logical
        .kinds
        .iter()
        .map(|kind| {
            let at = schema
                .iter()
                .enumerate()
                .find(|(at, candidate)| *candidate == kind && !used.contains(at))
                .map(|(at, _)| at)
                .expect("canonical catalog has the fixture's typed columns");
            used.push(at);
            ColumnIndex(at as u32)
        })
        .collect::<Vec<_>>();
    let mut cells = Vec::new();
    for row in 0..logical.count.0 {
        let mut output = schema
            .iter()
            .copied()
            .map(|kind| defaults(kind, row))
            .collect::<Vec<_>>();
        for (column, value) in projection
            .iter()
            .zip(&logical.cells[row * logical.kinds.len()..(row + 1) * logical.kinds.len()])
        {
            output[column.0 as usize] = *value;
        }
        cells.extend(output);
    }
    let table = Table {
        kinds: schema.to_vec(),
        cells,
        count: logical.count,
    };
    let columns = (0..schema.len())
        .map(|column| {
            FactColumn(
                table
                    .cells
                    .chunks_exact(schema.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect();
    let compact = if representation == RepresentationKind::Compact {
        let mut words = ryo_relations::domain::relations::types::Words::default();
        encode_table(&table, &[], &mut words);
        let ryo_values::domain::table_words::types::Admitted { table, names } = admit_compact(
            ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(pack(
                WordSlice(&words.0),
            )))),
            &mut Vec::new(),
            DecodeMode::Complete,
        )
        .unwrap();
        assert!(names.is_empty());
        Some(table)
    } else {
        None
    };
    let input = PlanInput::Step(StepId(steps.len() as u32));
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(relation),
            comparisons: vec![],
            kept: projection,
        },
    });
    (
        Provider {
            relation,
            table,
            columns,
            compact,
            tree: None,
            representation,
        },
        input,
    )
}
pub(crate) fn operation(
    left: &[u32],
    right: &[u32],
    output: &[(Side, u32)],
    filter: Vec<ryo_values::domain::plan_parts::types::PlanComparison>,
    view: &[u32],
) -> Operation {
    Operation::Join {
        left: PlanInput::Relation(Relation::Entry),
        right: PlanInput::Relation(Relation::SequenceElement),
        left_on: left.iter().copied().map(ColumnIndex).collect(),
        right_on: right.iter().copied().map(ColumnIndex).collect(),
        output: output
            .iter()
            .map(|(side, column)| JoinColumn {
                side: *side,
                column: ColumnIndex(*column),
            })
            .collect(),
        filter,
        view: view.iter().copied().map(ColumnIndex).collect(),
        reach: None,
    }
}
