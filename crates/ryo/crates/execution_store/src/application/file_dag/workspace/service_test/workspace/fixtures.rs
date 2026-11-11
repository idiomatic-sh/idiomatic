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

pub(super) fn use_(
    kind: DerivedKind,
    request: ViewRequest,
    schemas: &[&[CellKind]],
    len: u8,
) -> PreparedViewUse {
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
            assert_eq!(plan.kind, kind);
            Arc::new(plan)
        },
        inputs: [InputNumber(0), InputNumber(1)],
        relations: [None, None],
        len: InputsUsed(len),
    }
}

pub(super) fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut ryo_execution_domain::domain::table_scratch::types::OperatorScratch,
    memo: &'a mut Vec<ryo_execution_domain::domain::operating::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        phase: ryo_execution_domain::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: ryo_execution_domain::domain::matching::types::PatternMatcher(
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
}

pub(super) fn equality_on_first(width: usize) -> PreparedViewUse {
    let kinds = vec![CellKind::Number; width];
    use_(
        DerivedKind::Equality,
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
