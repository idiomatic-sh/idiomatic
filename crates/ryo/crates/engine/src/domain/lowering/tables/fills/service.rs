//! The services of the `fills` part: a made table added to the pack, and the pipeline that gives a fill the rows of a
//! definition. A fill's input is stages over a batch's columns: a scan of the definition's table, then one read a column.

use crate::domain::lowered::types::{Emptiness, MadeFill, MadeTable};
use crate::domain::lowering::definitions::service::{asked, draft, fresh_register};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::nodes::types::NodeHome;
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, MadeColumn, MadeTableId, MatchColumns, ReadColumn, StepOperand,
    StepOperands, StepRows,
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::table_words::types::ColumnCount;

/// What a made table is of: the merged plan node it is the rows of, where that node was lowered from, and what a file must
/// hold for it to have a row.
#[derive(Debug)]
pub(crate) struct MadeOf {
    /// The node.
    pub(crate) node: NodeDigest,
    /// Where it was lowered from.
    pub(crate) home: NodeHome,
    /// Its emptiness.
    pub(crate) emptiness: Emptiness,
}

/// Adds a made table to the pack.
pub(crate) fn made(drafts: &mut Drafts, of: MadeOf, fill: MadeFill, width: usize) -> MadeTableId {
    let table = MadeTableId(u32::try_from(drafts.tables.len()).unwrap_or(u32::MAX));
    drafts.tables.push(MadeTable {
        node: of.node,
        fill,
        width: ColumnCount(width),
        indexes: Vec::new(),
        emptiness: of.emptiness,
    });
    drafts.table_homes.push(of.home);
    table
}

/// The pipeline that gives a fill every row of a definition: a scan of the definition's table, a read of each column that is
/// not the file's, and the definition's row. Reading the table asks the definition's pattern that binds nothing.
pub(crate) fn scanned(drafts: &mut Drafts, definition: DefinitionId) -> Pipeline {
    let nothing = ColumnMask::default();
    asked(drafts, definition, &nothing, &nothing);
    let kinds: Vec<CellKind> =
        draft(drafts, definition).map_or_else(Vec::new, |draft| draft.shape.0.clone());
    let source = StepRows::Definition(definition);
    let handle = fresh_register(drafts);
    let mut stages = vec![FillStage::Scan {
        source,
        columns: MatchColumns::default(),
        values: StepOperands::default(),
        out: handle,
    }];
    let mut row = Vec::with_capacity(kinds.len());
    for (place, kind) in kinds.iter().enumerate() {
        match (kind, u16::try_from(place)) {
            (CellKind::File, _) | (_, Err(_)) => row.push(StepOperand::File),
            (
                CellKind::Node
                | CellKind::Symbol
                | CellKind::Span
                | CellKind::Number
                | CellKind::Flag,
                Ok(column),
            ) => {
                let out = fresh_register(drafts);
                stages.push(FillStage::Read {
                    source,
                    column: ReadColumn::Made(MadeColumn(column)),
                    row: handle,
                    out,
                });
                row.push(StepOperand::Register(out));
            }
        }
    }
    Pipeline {
        stages: stages
            .into_iter()
            .map(|stage| PlacedStage { point: None, stage })
            .collect(),
        row: StepOperands(row.into()),
    }
}
