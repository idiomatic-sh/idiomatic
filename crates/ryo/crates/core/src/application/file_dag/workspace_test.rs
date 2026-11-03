//! Actual view acquisition consumes one immutable proof and refuses stale/mismatched publications.
use super::*;
use crate::application::file_dag::{derived::owner::ViewContext, relations::RelationOwner};
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operating::{errors::OperatingError, types::OperatorContext},
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
    table_model::types::{ParameterBinding, RowCount, Rows},
};
use ryo_language::domain::{
    relations::types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value},
    symbols::types::Symbols,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::{rc::Rc, sync::Arc};
fn table(values: &[i64], width: usize) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: values
            .iter()
            .map(|value| Value::Number(Integer(*value)))
            .collect(),
        count: RowCount(values.len() / width),
    }
}
fn use_(
    kind: DerivedKind,
    request: ViewRequest,
    schemas: &[&[CellKind]],
    len: u8,
) -> PreparedViewUse {
    let schemas = schemas
        .iter()
        .map(|kinds| InputSchema { kinds, tree: None })
        .collect::<Vec<_>>();
    PreparedViewUse {
        plan: Arc::new(registered(kind).prepare(&request, &schemas).unwrap()),
        inputs: [0, 1],
        relations: [None, None],
        len,
    }
}
fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut crate::domain::table_scratch::types::OperatorScratch,
    memo: &'a mut Vec<crate::domain::operating::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[] as &[ParameterBinding],
        symbols: symbols.into(),
        source: crate::domain::table_texts::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}
#[test]
fn immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof() {
    let owner = Rc::new(RelationOwner::new(true));
    let published = owner.computed(table(&[3, 1, 3], 1)).unwrap();
    let rows = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::default();
    workspace.relation_owner = Some(Rc::clone(&owner));
    workspace.publications = vec![Some((published).clone())];
    let equality = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number]],
        1,
    );
    let membership = use_(
        DerivedKind::Membership,
        ViewRequest::Membership {
            column: ColumnIndex(0),
        },
        &[&[CellKind::Number]],
        1,
    );
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let inputs = [BorrowedRows::from(&rows)];
    let scope = workspace.inputs(&inputs);
    let before = owner.identity_checks.get();
    let index = scope.acquire_rows(&equality, &mut context).unwrap();
    let members = scope
        .acquire_derived(
            &membership,
            ViewContext::default(),
            ViewInputs::MembershipValues,
        )
        .unwrap();
    for _ in 0..16 {
        let DerivedView::Equality(index) = index.view().unwrap() else {
            panic!("equality")
        };
        assert_eq!(index.rows.len(), 3);
        let DerivedView::Membership(MembershipData::Values(values)) = members.view().unwrap()
        else {
            panic!("membership")
        };
        assert_eq!(values.len(), 2);
    }
    assert_eq!(
        owner.identity_checks.get() - before,
        1,
        "both actual families use the same admitted input"
    );
    owner.release_population();
    let builds = owner.derived.counts().0;
    assert!(matches!(
        scope.acquire_rows(&equality, &mut context),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        owner.derived.counts().0,
        builds,
        "expired proof never reaches the builder"
    );
}
#[test]
fn mixed_published_and_temporary_inputs_never_share_slot_only_identities() {
    let owner = Rc::new(RelationOwner::new(true));
    let published = owner.computed(table(&[1, 0], 2)).unwrap();
    let left = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::default();
    workspace.relation_owner = Some(Rc::clone(&owner));
    workspace.publications = vec![Some((published).clone()), None];
    let use_ = use_(
        DerivedKind::OrderedKeys,
        ViewRequest::OrderedKeys {
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(1),
            right_order: ColumnIndex(1),
        },
        &[&[CellKind::Number; 2], &[CellKind::Number; 2]],
        2,
    );
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let mut right = table(&[9, 0], 2);
    let mut retained_views = Vec::new();
    for expected in [9, 4] {
        right.cells[0] = Value::Number(Integer(expected));
        let inputs = [BorrowedRows::from(&left), BorrowedRows::Computed(&right)];
        let texts = crate::domain::table_texts::types::Texts {
            symbols: context.symbols.as_ref(),
            source: context.source,
            paths: &[],
            arenas: &[],
        };
        let storage = workspace
            .inputs(&inputs)
            .acquire_derived(&use_, ViewContext::default(), ViewInputs::Text(&texts))
            .unwrap();
        let view = storage;
        let DerivedView::OrderedKeys(rows) = view.view().unwrap() else {
            panic!("ordered")
        };
        assert!(
            matches!(rows.right[0].keys[0],crate::domain::operator::contract::graph::OwnedOrderedKey::Value(Value::Number(Integer(value)))if value==expected)
        );
        retained_views.push(view);
    }
    let DerivedView::OrderedKeys(first) = retained_views[0].view().unwrap() else {
        panic!("first ordered")
    };
    let DerivedView::OrderedKeys(second) = retained_views[1].view().unwrap() else {
        panic!("second ordered")
    };
    assert!(
        !std::ptr::eq(first, second),
        "live old view cannot be reused by temporary slot identity"
    );
    assert!(matches!(
        first.right[0].keys[0],
        crate::domain::operator::contract::graph::OwnedOrderedKey::Value(Value::Number(Integer(9)))
    ));
    assert!(matches!(
        second.right[0].keys[0],
        crate::domain::operator::contract::graph::OwnedOrderedKey::Value(Value::Number(Integer(4)))
    ));
    assert_eq!(
        owner.derived.counts().0,
        2,
        "mixed inputs use the same owner with distinct admitted populations"
    );
    workspace.publications[1] = Some((published).clone());
    let inputs = [BorrowedRows::from(&left), BorrowedRows::Computed(&right)];
    let texts = crate::domain::table_texts::types::Texts {
        symbols: context.symbols.as_ref(),
        source: context.source,
        paths: &[],
        arenas: &[],
    };
    assert!(matches!(
        workspace.inputs(&inputs).acquire_derived(
            &use_,
            ViewContext::default(),
            ViewInputs::Text(&texts)
        ),
        Err(OperatingError::NotRead)
    ));
}
#[test]
fn normalized_physical_input_is_admitted_but_unproved_transform_is_not() {
    let owner = Rc::new(RelationOwner::new(true));
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![3])];
    let raw = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let published = owner.source(Relation::LanguageTarget, &raw).unwrap();
    let normalized = owner.normalized(&published, raw).unwrap().unwrap();
    let mut workspace = InvocationWorkspace::default();
    workspace.relation_owner = Some(owner);
    workspace.publications = vec![Some((published).clone())];
    let use_ = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number; 2]],
        1,
    );
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let inputs = [BorrowedRows::Computed(&normalized)];
    assert!(workspace.acquire_rows(&use_, &inputs, &mut context).is_ok());
    let transformed = table(&[8, 9], 2);
    let inputs = [BorrowedRows::Computed(&transformed)];
    assert!(matches!(
        workspace.acquire_rows(&use_, &inputs, &mut context),
        Err(OperatingError::NotRead)
    ));
    workspace.publications[0] = None;
    assert!(
        workspace.acquire_rows(&use_, &inputs, &mut context).is_ok(),
        "explicit unpublished temporary uses invocation ownership"
    );
}

#[test]
fn recapturing_delivery_ends_invocation_local_identity_before_slot_reuse() {
    let use_ = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number]],
        1,
    );
    let mut workspace = InvocationWorkspace::default();
    let first = [table(&[9], 1)];
    let feed = crate::application::file_dag::testing::Held {
        tables: &first,
        visits: vec![0],
    };
    workspace.capture(&feed, 1).unwrap();
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let old = workspace
        .acquire_rows(&use_, &[BorrowedRows::Computed(&first[0])], &mut context)
        .unwrap();
    let second = [table(&[4], 1)];
    let feed = crate::application::file_dag::testing::Held {
        tables: &second,
        visits: vec![0],
    };
    workspace.capture(&feed, 1).unwrap();
    assert_eq!(
        workspace.owner().derived.counts().0,
        0,
        "recapture starts a new invocation owner"
    );
    let fresh = workspace
        .acquire_rows(&use_, &[BorrowedRows::Computed(&second[0])], &mut context)
        .unwrap();
    assert!(!old.same_storage(&fresh));
    let DerivedView::Equality(index) = fresh.view().unwrap() else {
        panic!("equality")
    };
    let view = crate::domain::operator::contract::views::IndexedRows {
        rows: BorrowedRows::Computed(&second[0]),
        columns: &[ColumnIndex(0)],
        index,
    };
    assert_eq!(view.matching(&[Value::Number(Integer(4))]).count(), 1);
    assert_eq!(view.matching(&[Value::Number(Integer(9))]).count(), 0);
}
