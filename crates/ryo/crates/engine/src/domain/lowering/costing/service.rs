//! The services of the `costing` part: the planner's arithmetic, the rows of a source, and the access of an atom for the
//! cells that stand bound, all by the fixed priors. What a use of a definition costs is the definition's own plan for the
//! columns bound, which the planning asks (`planning::uses`); here a use is sized only as a read of the definition's table.

use super::constants::{
    CHILDREN, ELEMENT_COLUMN_ROWS, ELEMENTS, ESTIMATE_CAP, FIELD_CHILDREN, GROUP_ROWS, KIND_SHARE,
    LITERAL_SHARE, MADE_ROWS, MANY_ROWS, ONE, PAIRS_AN_ELEMENT, PARTIAL_KEY_ROWS, PLAN_CAP,
    SEVERAL_KEYS_SHARE, TABLE_KEY_KEEPS, VALUE_COLUMN_ROWS,
};
use super::types::{CellState, KeyChoice, PlanEstimate};
use crate::domain::lowered::types::{FillInput, MadeFill, OrderedFill};
use crate::domain::lowering::definitions::service::{draft, kind_bits};
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatBranch, FlatTerm, RowColumn, RowsPerFile,
};
use crate::domain::lowering::planning::uses::service::branches_estimated;
use crate::domain::statistics::service::{element_rows, field_rows, kind_rows, literal_rows};
use crate::domain::statistics::types::SizedRows;
use crate::domain::steps::stages::types::{FillStage, Pipeline};
use crate::domain::steps::types::{ConstantText, CostClass, MadeColumn, MadeTableId, StepRows};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::catalog::service::{fact_form, node_column, relation_size};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{
    CellKind, Column, FactForm, FilledKey, NodeColumn, Relation, RelationSize,
};
use ryo_relations::domain::terms::types::NameText;

/// The product of two estimates, held to the cap.
#[must_use]
pub(crate) fn times(left: PlanEstimate, right: PlanEstimate) -> PlanEstimate {
    let product = u128::from(left.0).saturating_mul(u128::from(right.0)) / u128::from(ONE.0);
    PlanEstimate(u64::try_from(product).unwrap_or(u64::MAX)).min(PLAN_CAP)
}

/// The sum of two estimates, held to the cap.
#[must_use]
pub(crate) fn plus(left: PlanEstimate, right: PlanEstimate) -> PlanEstimate {
    PlanEstimate(left.0.saturating_add(right.0)).min(PLAN_CAP)
}

/// Whole rows as an estimate of the planner.
#[must_use]
pub(crate) fn planned_rows(rows: RowEstimate) -> PlanEstimate {
    PlanEstimate(rows.0.min(ESTIMATE_CAP.0).saturating_mul(ONE.0))
}

/// An estimate of the planner as whole rows: rounded up, and never none.
#[must_use]
pub(crate) fn whole_rows(rows: PlanEstimate) -> RowEstimate {
    RowEstimate(rows.0.div_ceil(ONE.0).max(1))
}

/// One row in `share` of an estimate, and never none.
const fn shared(rows: RowEstimate, share: RowEstimate) -> RowEstimate {
    let part = rows.0 / share.0;
    RowEstimate(if part == 0 { 1 } else { part })
}

/// The rows a filled relation is taken to hold in a file, by the size its catalog item declares: one for a relation of one
/// row a file; a row an element for one that grows with the file's elements (a scan of it is a scan of the file, whatever
/// share of the elements it holds); several an element for a relation of pairs.
fn filled_rows(relation: Relation) -> RowEstimate {
    match relation_size(relation) {
        Some(RelationSize::PerFile) => RowEstimate(1),
        Some(RelationSize::PerElement) | None => ELEMENTS,
        Some(RelationSize::PerPair) => RowEstimate(ELEMENTS.0.saturating_mul(PAIRS_AN_ELEMENT)),
    }
}

/// The rows the first stage of a pipeline reads: those of its source, a constant among its columns naming a bucket.
fn scanned_rows(drafts: &Drafts, pipeline: &Pipeline) -> RowEstimate {
    let Some(FillStage::Scan {
        source, columns, ..
    }) = pipeline.stages.first().map(|placed| &placed.stage)
    else {
        return RowEstimate(1);
    };
    let whole = match source {
        StepRows::Node(_) => ELEMENTS,
        StepRows::Filled(relation) => filled_rows(*relation),
        StepRows::Made(table) | StepRows::Written(table) => made_rows(drafts, *table),
        StepRows::Definition(definition) => {
            draft(drafts, *definition).map_or(RowEstimate(1), |draft| draft.rows)
        }
    };
    if columns.0.is_empty() {
        whole
    } else {
        shared(whole, LITERAL_SHARE)
    }
}

/// The rows a made table is taken to hold in a file: those its input's pipelines scan, between the least and the greatest
/// a made table is sized at. A table of written rows holds what is written.
#[must_use]
pub(crate) fn made_rows(drafts: &Drafts, table: MadeTableId) -> RowEstimate {
    let place = usize::try_from(table.0).unwrap_or(usize::MAX);
    let Some(made) = drafts.tables.get(place) else {
        return GROUP_ROWS;
    };
    let scanned = |pipelines: &[Pipeline]| {
        pipelines.iter().fold(0_u64, |sum, pipeline| {
            sum.saturating_add(scanned_rows(drafts, pipeline).0)
        })
    };
    let input = |input: &FillInput| scanned(&input.pipelines);
    let rows = match &made.fill {
        MadeFill::Rows(pipelines, _) => scanned(pipelines),
        MadeFill::Group(_, from) | MadeFill::Ordered(OrderedFill::Sorted(from)) => input(from),
        MadeFill::Ordered(OrderedFill::Match(left, right)) => input(left).min(input(right)),
        MadeFill::Graph(_, inputs) => inputs
            .iter()
            .fold(0_u64, |sum, each| sum.saturating_add(input(each))),
        MadeFill::Constant(rows) => u64::try_from(rows.0.len()).unwrap_or(u64::MAX),
    };
    let (least, greatest) = (whole_rows(MADE_ROWS.0), whole_rows(MADE_ROWS.1));
    RowEstimate(rows).clamp(least, greatest)
}

/// The key columns of a made table: those its fill groups by. None for a table that is no group of one input.
fn made_keys(drafts: &Drafts, table: MadeTableId) -> Option<&[MadeColumn]> {
    let place = usize::try_from(table.0).unwrap_or(usize::MAX);
    match &drafts.tables.get(place)?.fill {
        MadeFill::Group(_, input) | MadeFill::Ordered(OrderedFill::Sorted(input)) => {
            Some(&input.keys.0)
        }
        MadeFill::Rows(..)
        | MadeFill::Ordered(OrderedFill::Match(..))
        | MadeFill::Graph(..)
        | MadeFill::Constant(_) => None,
    }
}

/// The rows a source is taken to hold in a file.
#[must_use]
pub(crate) fn source_rows(drafts: &Drafts, rows: AtomRows) -> RowEstimate {
    match rows {
        AtomRows::Relation(relation) => match fact_form(relation) {
            FactForm::NodeTable => ELEMENTS,
            FactForm::Filled(_) => filled_rows(relation),
        },
        AtomRows::Definition(definition) => {
            draft(drafts, definition).map_or(RowEstimate(1), |draft| draft.rows)
        }
        AtomRows::Made(table) => made_rows(drafts, table),
    }
}

/// The places of the cells in a state.
fn standing(states: &[CellState], state: CellState) -> Vec<RowColumn> {
    let places = states.iter().enumerate();
    places
        .filter(|(_, each)| **each == state)
        .map(|(place, _)| RowColumn(place))
        .collect()
}

/// The access of the node table through a relation: it is keyed by one column. A register in the element's own column, its
/// parent's or its span's is a key; with none, a constant kind, field or text names a bucket of the file. A parent's
/// children in a field that is given are fewer than its children.
fn node_access(relation: Relation, states: &[CellState]) -> KeyChoice {
    let listed = columns(relation);
    let cell = |wanted: NodeColumn, state: CellState| {
        listed
            .iter()
            .zip(states)
            .position(|(column, each)| *each == state && node_column(*column) == Some(wanted))
    };
    let children = if cell(NodeColumn::Field, CellState::Constant).is_some() {
        FIELD_CHILDREN
    } else {
        CHILDREN
    };
    let bucket = |share| planned_rows(shared(ELEMENTS, share));
    let keys = [
        (NodeColumn::Id, CellState::Register, CostClass::Fanout, ONE),
        (
            NodeColumn::Parent,
            CellState::Register,
            CostClass::Fanout,
            children,
        ),
        (
            NodeColumn::Span,
            CellState::Register,
            CostClass::Fanout,
            ONE,
        ),
        (
            NodeColumn::Kind,
            CellState::Constant,
            CostClass::File,
            bucket(KIND_SHARE),
        ),
        (
            NodeColumn::Field,
            CellState::Constant,
            CostClass::File,
            bucket(KIND_SHARE),
        ),
        (
            NodeColumn::Text,
            CellState::Constant,
            CostClass::File,
            bucket(LITERAL_SHARE),
        ),
    ];
    for (column, state, class, rows) in keys {
        if let Some(place) = cell(column, state) {
            return KeyChoice {
                cells: vec![RowColumn(place)],
                class,
                rows,
            };
        }
    }
    KeyChoice {
        cells: Vec::new(),
        class: CostClass::File,
        rows: planned_rows(ELEMENTS),
    }
}

/// The access of a source that takes every bound cell as one key: keyed by a register it is a fan-out, by constants alone a
/// bucket of the file, by nothing the file's rows. `reached` is the rows the registers' most selective column reaches; a
/// key of several cells reaches just under it, and no key more than the source holds.
fn composite_access(states: &[CellState], whole: RowEstimate, reached: PlanEstimate) -> KeyChoice {
    let registers = standing(states, CellState::Register);
    let constants = standing(states, CellState::Constant);
    let (class, rows) = match (registers.len(), constants.len()) {
        (0, 0) => (CostClass::File, planned_rows(whole)),
        (0, _) => (CostClass::File, planned_rows(shared(whole, LITERAL_SHARE))),
        (1, 0) => (CostClass::Fanout, reached),
        (_, _) => (CostClass::Fanout, times(reached, SEVERAL_KEYS_SHARE)),
    };
    let mut cells: Vec<RowColumn> = registers.into_iter().chain(constants).collect();
    cells.sort_unstable();
    KeyChoice {
        cells,
        class,
        rows: rows.min(planned_rows(whole)),
    }
}

/// The rows a filled relation holds for one value of a column: one for its unique key, a few for its key of many rows; for
/// any other column, about a row when the column holds an element or a span, and several when it holds a value.
fn column_rows(key: FilledKey, column: Column) -> PlanEstimate {
    match key {
        FilledKey::Unique(own) if own == column => ONE,
        FilledKey::Many(own) if own == column => MANY_ROWS,
        FilledKey::Unique(_) | FilledKey::Many(_) | FilledKey::Whole => match kind(column) {
            CellKind::Node | CellKind::Span => ELEMENT_COLUMN_ROWS,
            CellKind::Symbol | CellKind::Number | CellKind::Flag | CellKind::File => {
                VALUE_COLUMN_ROWS
            }
        },
    }
}

/// The access of an atom's rows for the cells that stand bound. A source that holds one row a file is keyed by the file. A
/// use of a definition by a register is sized here as a read of its table, each bound column keeping a share of its rows:
/// what the use costs by the definition's own plan is the planning's to ask.
#[must_use]
pub(crate) fn keyed(
    drafts: &Drafts,
    rows: AtomRows,
    per_file: RowsPerFile,
    states: &[CellState],
) -> KeyChoice {
    if per_file == RowsPerFile::One {
        return KeyChoice {
            cells: Vec::new(),
            class: CostClass::Fanout,
            rows: ONE,
        };
    }
    let whole = source_rows(drafts, rows);
    let registers = standing(states, CellState::Register);
    match rows {
        AtomRows::Relation(relation) => match fact_form(relation) {
            FactForm::NodeTable => node_access(relation, states),
            FactForm::Filled(key) => {
                let listed = columns(relation);
                let reached = registers
                    .iter()
                    .filter_map(|place| listed.get(place.0))
                    .map(|column| column_rows(key, *column))
                    .min();
                composite_access(states, whole, reached.unwrap_or(ONE))
            }
        },
        // A use keyed by constants alone names a bucket of the definition's rows: file-sized, as any bucket is.
        AtomRows::Definition(_) => {
            let bound = registers.len() + standing(states, CellState::Constant).len();
            let kept = (0..bound).fold(planned_rows(whole), |rows, _| times(rows, TABLE_KEY_KEEPS));
            let mut choice = composite_access(states, whole, kept);
            if choice.class == CostClass::Fanout {
                choice.rows = kept;
            }
            choice
        }
        AtomRows::Made(table) => {
            let whole_key = made_keys(drafts, table).is_some_and(|keys| {
                keys.iter().all(|key| {
                    let state = states.get(usize::from(key.0));
                    matches!(
                        state,
                        Some(CellState::Register | CellState::Constant | CellState::File)
                    )
                })
            });
            let reached = if whole_key { ONE } else { PARTIAL_KEY_ROWS };
            composite_access(states, whole, reached)
        }
    }
}

/// The rows of a step's branches together in a file: what each branch's own plan gives with nothing bound.
#[must_use]
pub(crate) fn branches_rows(drafts: &Drafts, branches: &[FlatBranch]) -> RowEstimate {
    branches_estimated(drafts, branches)
}

/// The mean rows a file of a size a sample measured, in the planner's 1,024ths, and never none.
fn mean_rows(sized: SizedRows) -> PlanEstimate {
    let whole = |count: usize| u128::try_from(count).unwrap_or(u128::MAX);
    let mean = whole(sized.rows.0).saturating_mul(u128::from(ONE.0)) / whole(sized.files.0).max(1);
    PlanEstimate(u64::try_from(mean).unwrap_or(u64::MAX).max(1)).min(PLAN_CAP)
}

/// The text a constant is written as: a text as the plan writes it, a number in decimal.
fn written_as(drafts: &Drafts, constant: FlatTerm) -> Option<String> {
    match constant {
        FlatTerm::Text(number) => drafts.written.get(&number).map(|text| text.0.to_string()),
        FlatTerm::Number(number) => Some(number.0.to_string()),
        FlatTerm::Variable(_) | FlatTerm::File => None,
    }
}

/// The rows a sample measured for the bucket an atom's rows are read by when no register keys them: the kind's or the
/// field's elements for the node table (its elements, read whole), the rows holding the value for a filled relation.
/// `held` is the constant each cell is held to. None when the load was given no sizes, when a register keys the read, or
/// when the sample did not size that bucket: the fixed prior stands.
#[must_use]
pub(crate) fn measured(
    drafts: &Drafts,
    rows: AtomRows,
    states: &[CellState],
    held: &[Option<FlatTerm>],
) -> Option<PlanEstimate> {
    let sized = drafts.sized.as_ref()?;
    let AtomRows::Relation(relation) = rows else {
        return None;
    };
    if states.contains(&CellState::Register) {
        return None;
    }
    let listed = columns(relation);
    let constant = |place: usize| held.get(place).copied().flatten();
    let found = match fact_form(relation) {
        FactForm::NodeTable => {
            let at = |wanted: NodeColumn| {
                let mut places = listed.iter().zip(states).enumerate();
                places.find_map(|(place, (column, state))| {
                    let named =
                        *state == CellState::Constant && node_column(*column) == Some(wanted);
                    named.then(|| constant(place)).flatten()
                })
            };
            if let Some(kind) = at(NodeColumn::Kind) {
                let text = written_as(drafts, kind)?;
                let bits = kind_bits(&sized.names, &ConstantText(text.into()))?;
                kind_rows(&sized.statistics, bits)
            } else if let Some(field) = at(NodeColumn::Field) {
                let text = written_as(drafts, field)?;
                let place = sized
                    .names
                    .fields
                    .iter()
                    .position(|name| *name.0 == *text)?;
                field_rows(&sized.statistics, FieldCode(u16::try_from(place).ok()?))
            } else if states.contains(&CellState::Constant) {
                None
            } else {
                element_rows(&sized.statistics)
            }
        }
        FactForm::Filled(_) => {
            let sizes = listed.iter().enumerate().filter_map(|(place, column)| {
                let text = written_as(drafts, constant(place)?)?;
                literal_rows(&sized.statistics, relation, *column, NameText(&text))
            });
            sizes.min_by_key(|sized| sized.rows)
        }
    };
    found.map(mean_rows)
}
