//! The services of the `fills` part: a made table added to the pack, and the pipeline that gives a fill the rows of a
//! definition. A fill's input is stages over a batch's columns: a scan of the definition's table, then one read a column.

use crate::domain::lowered::types::{
    Emptiness, FillInput, GroupFill, MadeFill, MadeTable, OrderedFill,
};
use crate::domain::lowering::definitions::service::{asked, draft, fresh_cells, fresh_register};
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::emptiness::service::derived;
use crate::domain::lowering::flattening::service::picked;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, RowColumn, RowsPerFile,
};
use crate::domain::lowering::nodes::service::{digest_of, input_shape};
use crate::domain::lowering::nodes::types::{NodeHome, StepShape};
use crate::domain::lowering::tables::inputs::service::{input, rule};
use crate::domain::lowering::tables::joins::service::{join_row, key_pairs};
use crate::domain::lowering::tables::types::{InputWant, RuleFlattening};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::types::{
    ColumnList, ColumnMask, DefinitionId, MadeColumn, MadeTableId, MatchColumns, ReadColumn,
    StepOperand, StepOperands, StepRows,
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
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

/// The made table of a fill over a definition's rows, and what it is of.
pub(crate) fn fill_of(
    state: &RuleFlattening<'_>,
    step: StepId,
    from: DefinitionId,
) -> Option<MadeOf> {
    let node = digest_of(rule(state)?, step);
    let home = state.store.0.get(&node).map_or(
        NodeHome {
            rule: state.place,
            step,
        },
        |stored| stored.home,
    );
    Some(MadeOf {
        node,
        home,
        emptiness: Emptiness::Definition(from),
    })
}

/// The made table of a step: made the first time the step's node is flattened, and found after, in whatever rule.
pub(crate) fn made_once(
    state: &RuleFlattening<'_>,
    drafts: &mut Drafts,
    of: (StepId, DefinitionId),
    fill: MadeFill,
    width: usize,
) -> Option<MadeTableId> {
    let node = digest_of(rule(state)?, of.0);
    if let Some(table) = drafts.made.get(&node) {
        return Some(*table);
    }
    let table = made(drafts, fill_of(state, of.0, of.1)?, fill, width);
    let _new = drafts.made.insert(node, table);
    Some(table)
}

/// A list of a row's columns, as a fill names them.
pub(crate) fn column_list(columns: impl IntoIterator<Item = usize>) -> ColumnList {
    let made = columns
        .into_iter()
        .filter_map(|column| u16::try_from(column).ok().map(MadeColumn));
    ColumnList(made.collect())
}

/// The branch of a group-count: one row of its table, the keys then the count. Keyed on the file alone it is one row a file.
pub(crate) fn group_count_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::GroupCount { from, keys, .. } = operation else {
        return None;
    };
    let shape = rule(state)?.shapes.get(usize::from(step))?.clone()?;
    let read = input(state, drafts, *from, InputWant::Defined)?;
    let Emptiness::Definition(definition) = read.emptiness else {
        return None;
    };
    let input = FillInput {
        pipelines: [scanned(drafts, definition)].into(),
        keys: column_list(keys.iter().map(|key| usize::from(*key))),
        order: None,
    };
    let fill = MadeFill::Group(GroupFill::Count { distinct: None }, input);
    let table = made_once(state, drafts, (step, definition), fill, shape.0.len())?;
    let key_kinds = shape.0.iter().take(keys.len());
    let per_file = if !keys.is_empty() && key_kinds.clone().all(|kind| *kind == CellKind::File) {
        RowsPerFile::One
    } else {
        RowsPerFile::Many
    };
    let cells = fresh_cells(drafts, &shape);
    Some(Flattened {
        branches: vec![FlatBranch {
            atoms: vec![FlatAtom::Rows {
                rows: AtomRows::Made(table),
                cells: cells.clone(),
                per_file,
            }],
            row: cells,
        }],
        emptiness: derived(operation, vec![Emptiness::Definition(definition)]),
        keyless: Vec::new(),
    })
}

/// The branches of an as-of: each left branch with the rows of the right side's ordered table nearest its order.
pub(crate) fn as_of_arm(
    state: &mut RuleFlattening<'_>,
    drafts: &mut Drafts,
    step: StepId,
    operation: &Operation,
) -> Option<Flattened> {
    let Operation::AsOf {
        left,
        right,
        left_on,
        right_on,
        left_order,
        right_order,
        bound,
        output,
    } = operation
    else {
        return None;
    };
    let right_shape: StepShape = input_shape(&rule(state)?.shapes, *right)?;
    let mut lefts = input(state, drafts, *left, InputWant::Inline)?;
    let rights = input(state, drafts, *right, InputWant::Defined)?;
    let Emptiness::Definition(definition) = rights.emptiness else {
        return None;
    };
    // The file's own column is no key within the file: the group is the other keys.
    let within = |right_key: usize| right_shape.0.get(right_key) != Some(&CellKind::File);
    let keys: Vec<(usize, usize)> = key_pairs(left_on, right_on)
        .filter(|(_, right_key)| within(*right_key))
        .collect();
    let sorted = FillInput {
        pipelines: [scanned(drafts, definition)].into(),
        keys: column_list(keys.iter().map(|(_, right_key)| *right_key)),
        order: u16::try_from(usize::from(*right_order))
            .ok()
            .map(MadeColumn),
    };
    let fill = MadeFill::Ordered(OrderedFill::Sorted(sorted));
    let table = made_once(state, drafts, (step, definition), fill, right_shape.0.len())?;
    for branch in &mut lefts.branches {
        let key = picked(
            &branch.row,
            keys.iter().map(|(left_key, _)| RowColumn(*left_key)),
        )?;
        let cells = fresh_cells(drafts, &right_shape);
        let width = branch.row.len();
        let mut row = branch.row.clone();
        row.extend(cells.iter().copied());
        branch.atoms.push(FlatAtom::Latest {
            table,
            key,
            bound: *branch.row.get(usize::from(*left_order))?,
            side: *bound,
            cells,
        });
        branch.row = join_row(&row, width, output)?;
    }
    let emptiness = derived(operation, vec![lefts.emptiness, rights.emptiness]);
    Some(Flattened { emptiness, ..lefts })
}
