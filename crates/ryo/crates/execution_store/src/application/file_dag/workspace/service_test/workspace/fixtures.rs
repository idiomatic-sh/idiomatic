//! The support of the `workspace` cases.
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::{
    derived_views::types::contract::value::PreparedViewUse, operator::types::OperatorContext,
};
use ryo_execution_domain::domain::{derived_views::types::*, operation_schema::types::InputShape};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::rows::types::Table;
use std::rc::Rc;
use std::sync::Arc;

pub(super) fn table(values: &[i64], width: usize) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: values
            .iter()
            .map(|value| Value::Number(Integer(*value)))
            .collect(),
        count: RowCount(values.len() / width),
    }
}

pub(super) fn use_(request: ViewRequest, schemas: &[&[CellKind]], len: u8) -> PreparedViewUse {
    let schemas = schemas
        .iter()
        .map(|kinds| InputShape { kinds, tree: None })
        .collect::<Vec<_>>();
    PreparedViewUse {
        plan: {
            let plan = crate::application::file_dag::derived::prepare::service::view(
                Arc::new(crate::application::file_dag::fixtures::derived()),
                &request,
                &schemas,
            )
            .unwrap();
            Arc::new(plan)
        },
        inputs: [InputNumber(0), InputNumber(1)],
        relations: [None, None],
        len: InputsUsed(len),
    }
}

pub(super) fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut ryo_operators::domain::table_scratch::types::OperatorScratch,
    memo: &'a mut Vec<ryo_operators::domain::table_scratch::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        bindings: &[],
        symbols: symbols.into(),
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}

pub(super) fn owner() -> Rc<crate::application::file_dag::relations::service::RelationOwner> {
    Rc::new(
        crate::application::file_dag::relations::service::RelationOwner::new(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            Arc::new(crate::application::file_dag::fixtures::derived()),
        ),
    )
}
use crate::application::file_dag::derived::owner::types::BuildCount;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::InputNumber;

/// What a workspace holds when a one-number input is admitted or its views acquired.
#[derive(Clone, Copy, Debug)]
pub(super) enum Held {
    /// No publications captured; a raw table is admitted.
    Nothing,
    /// One slot with no publication; a raw table is admitted.
    EmptySlot,
    /// The captured publication of the table `7`; its resolved rows are admitted.
    Captured,
    /// The captured publication; the raw table, not its resolved rows, is admitted.
    Unpublished,
    /// The captured publication after its population was released; its rows are admitted.
    Released,
    /// The publication of `3, 1, 3` and its equality and membership views, read 16 times.
    TwoViews(usize),
    /// A left publication and two right publications of `(9, 0)` then `(4, 0)`, each joined by
    /// ordered keys; then an unpublished right table.
    TwoRights([i64; 2]),
    /// A ragged `LanguageTarget` source, normalized; then a transformed table and an empty slot.
    Normalized,
    /// Publications of `9` then `4`, each captured in turn.
    Recaptured([i64; 2]),
    /// The views the cases use: an equality on the first column, a membership of the first column,
    /// and ordered keys on the first column ordered by the second.
    Prepared,
    /// The captured publication of the table `7`, released; its rows are then read at the first place.
    AfterRelease,
    /// Four captured publications of the table `7` and their four inputs; the rows are read at the fourth place.
    BeyondProofSlots,
    /// One captured publication of the table `7` and its one input; the input at this place is read.
    PastInputs(usize),
}

/// What two views of one admitted input observed.
#[derive(Debug, PartialEq)]
pub(super) struct TwoViews {
    pub(super) index_rows: usize,
    pub(super) membership_values: usize,
    pub(super) inspections: u64,
    pub(super) expired: Option<OperatingError>,
    pub(super) built_after_expiry: bool,
}

/// What two right publications observed.
#[derive(Debug, PartialEq)]
pub(super) struct TwoRights {
    pub(super) right_keys: [Option<CellValue>; 2],
    pub(super) distinct_views: bool,
    pub(super) retained_keys: [Option<CellValue>; 2],
    pub(super) builds: BuildCount,
    pub(super) unpublished: Option<OperatingError>,
}

/// What the workspace answered.
#[derive(Debug, PartialEq)]
pub(super) enum Answered {
    /// The admitted input's relation and kinds.
    Admitted {
        relation: Option<Relation>,
        kinds: [CellKind; 1],
    },
    TwoViews(TwoViews),
    TwoRights(TwoRights),
    /// The rows the normalized input's index holds against the normalized count, and the
    /// refusals of a transformed table and of an empty slot.
    Normalized {
        indexes_every_row: bool,
        transformed: Option<OperatingError>,
        empty_slot: Option<OperatingError>,
    },
    /// Whether the two captures' views are distinct storages, and the builds.
    Recaptured {
        distinct: bool,
        builds: BuildCount,
    },
    /// The kind each view the cases use is prepared as.
    Prepared([DerivedKind; 3]),
    /// How many rows the admissions read at the place asked.
    Read {
        count: usize,
    },
}

/// A membership of the first of one number column.
pub(super) fn membership_on_first() -> PreparedViewUse {
    use_(
        ViewRequest::Membership {
            column: ColumnIndex(0),
        },
        &[&[CellKind::Number]],
        1,
    )
}

/// Ordered keys of two two-number inputs, keyed by the first column and ordered by the second.
pub(super) fn ordered_keys() -> PreparedViewUse {
    use_(
        ViewRequest::OrderedKeys {
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(1),
            right_order: ColumnIndex(1),
        },
        &[&[CellKind::Number; 2], &[CellKind::Number; 2]],
        2,
    )
}

pub(super) fn equality_on_first(width: usize) -> PreparedViewUse {
    let kinds = vec![CellKind::Number; width];
    use_(
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&kinds],
        1,
    )
}

pub(super) fn ordered_key(
    view: &crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
) -> Option<CellValue> {
    let DerivedView::OrderedKeys(rows) = view.view().ok()? else {
        return None;
    };
    match rows.right[0].keys[0] {
        ryo_derived::domain::ordered::types::OwnedOrderedKey::Value(value) => Some(value),
        _ => None,
    }
}

/// Captures the publication of the table `7` as `input` holds it and reads the admissions through the workspace and
/// admitted views ports: the rows at the first place after a release, the rows at the fourth place of four, or the input
/// at the place past the one input.
pub(super) fn read_through_the_ports(
    owner: &crate::application::file_dag::relations::service::RelationOwner,
    workspace: &mut crate::application::file_dag::workspace::service::InvocationWorkspace,
    input: Held,
) -> Result<Answered, OperatingError> {
    use crate::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    use crate::application::file_dag::relations::handle::service::Publication;
    use crate::application::file_dag::workspace::interface::WorkspacePort;
    use ryo_execution_domain::domain::table_model::types::RepresentationInput;
    let publication = owner.computed(table(&[7], 1))?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let held = if matches!(input, Held::BeyondProofSlots) {
        4
    } else {
        1
    };
    WorkspacePort::capture_admitted(
        workspace,
        vec![Publication::of(publication.clone()); held].into(),
    );
    if matches!(input, Held::AfterRelease) {
        WorkspacePort::release_inputs(workspace);
    }
    let inputs = vec![RepresentationInput::from(&rows); held];
    let admitted = WorkspacePort::inputs(&*workspace, &inputs);
    let read = match input {
        Held::BeyondProofSlots => DerivedAdmittedViewsPort::rows(&admitted, InputNumber(3))?,
        Held::PastInputs(place) => DerivedAdmittedViewsPort::input(&admitted, InputNumber(place))?,
        Held::Nothing
        | Held::EmptySlot
        | Held::Captured
        | Held::Unpublished
        | Held::Released
        | Held::TwoViews(_)
        | Held::TwoRights(_)
        | Held::Normalized
        | Held::Recaptured(_)
        | Held::Prepared
        | Held::AfterRelease => DerivedAdmittedViewsPort::rows(&admitted, InputNumber(0))?,
    };
    Ok(Answered::Read {
        count: read.count().0,
    })
}
