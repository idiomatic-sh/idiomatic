//! Actual view acquisition consumes one immutable proof and refuses stale/mismatched publications.
use super::*;
use crate::application::file_dag::derived::owner::ViewContext;
use crate::domain::{
    derived_views::types::*,
    operating::{errors::OperatingError, types::OperatorContext},
    operation_schema::types::InputShape,
    operator::contract::views::BorrowedRows,
    table_model::types::Rows,
};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::{rc::Rc, sync::Arc};

#[test]
fn admission_requires_a_live_matching_publication_in_test_builds_too() {
    let owner = owner();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let raw = table(&[7], 1);
    assert!(matches!(
        workspace.admit(0, BorrowedRows::Computed(&raw)),
        Err(OperatingError::MissingInput { position: 1 })
    ));
    workspace.publications = vec![None];
    assert!(matches!(
        workspace.admit(0, BorrowedRows::Computed(&raw)),
        Err(OperatingError::NotRun)
    ));
    let publication = owner.computed(raw.clone()).unwrap();
    let rows = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    workspace.capture_admitted(vec![publication.clone()]);
    let admitted = workspace.admit(0, BorrowedRows::from(&rows)).unwrap();
    assert_eq!(admitted.identity(&owner).unwrap().1, None);
    assert_eq!(admitted.rows().kinds(), &[CellKind::Number]);
    assert!(matches!(
        workspace.admit(0, BorrowedRows::Computed(&raw)),
        Err(OperatingError::NotRead)
    ));
    owner.release_population();
    assert!(matches!(
        workspace.admit(0, BorrowedRows::from(&rows)),
        Err(OperatingError::NotRead)
    ));
}
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
        .map(|kinds| InputShape { kinds, tree: None })
        .collect::<Vec<_>>();
    PreparedViewUse {
        plan: {
            let plan = crate::application::file_dag::derived::prepare::view(
                (std::sync::Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                )
                    as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
                &request,
                &schemas,
            )
            .unwrap();
            assert_eq!(plan.kind, kind);
            Arc::new(plan)
        },
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
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[] as &[ParameterBinding],
        symbols: symbols.into(),
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}
#[test]
fn immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof() {
    let owner = owner();
    let published = owner.computed(table(&[3, 1, 3], 1)).unwrap();
    let rows = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
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
        assert_eq!(index.rows().len(), 3);
        let DerivedView::Membership(ryo_derived::domain::contract::types::MembershipView::Values(
            values,
        )) = members.view().unwrap()
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
    let builds = owner.derived().counts().0;
    assert!(matches!(
        scope.acquire_rows(&equality, &mut context),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        owner.derived().counts().0,
        builds,
        "expired proof never reaches the builder"
    );
}
#[test]
fn distinct_published_populations_never_share_slot_only_identities() {
    let owner = owner();
    let published = owner.computed(table(&[1, 0], 2)).unwrap();
    let left = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
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
    let mut retained_views = Vec::new();
    for expected in [9, 4] {
        let right_publication = owner.computed(table(&[expected, 0], 2)).unwrap();
        let right = owner
            .resolve(right_publication.as_ref(), &|_| {
                Err(OperatingError::NotRead)
            })
            .unwrap();
        workspace.capture_admitted(vec![published.clone(), right_publication.clone()]);
        let inputs = [BorrowedRows::from(&left), BorrowedRows::from(&right)];
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
            matches!(rows.right()[0].keys()[0],ryo_derived::domain::contract::types::OwnedOrderedKey::Value(Value::Number(Integer(value)))if value==expected)
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
        first.right()[0].keys()[0],
        ryo_derived::domain::contract::types::OwnedOrderedKey::Value(Value::Number(Integer(9)))
    ));
    assert!(matches!(
        second.right()[0].keys()[0],
        ryo_derived::domain::contract::types::OwnedOrderedKey::Value(Value::Number(Integer(4)))
    ));
    assert_eq!(
        owner.derived().counts().0,
        2,
        "mixed inputs use the same owner with distinct admitted populations"
    );
    workspace.publications[1] = Some(published.clone());
    let wrong = table(&[4, 0], 2);
    let inputs = [BorrowedRows::from(&left), BorrowedRows::Computed(&wrong)];
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
    let owner = owner();
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![3])];
    let raw = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let published = owner.source(Relation::LanguageTarget, &raw).unwrap();
    let normalized = owner.normalize(&published, raw).unwrap();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
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
    let inputs = [BorrowedRows::from(&normalized)];
    let storage = workspace
        .inputs(&inputs)
        .acquire_rows(&use_, &mut context)
        .unwrap();
    let DerivedView::Equality(index) = storage.view().unwrap() else {
        panic!("equality")
    };
    assert_eq!(
        index.rows().len(),
        crate::domain::table_model::service::count_of(&normalized).0
    );
    let transformed = table(&[8, 9], 2);
    let inputs = [BorrowedRows::Computed(&transformed)];
    assert!(matches!(
        workspace.inputs(&inputs).acquire_rows(&use_, &mut context),
        Err(OperatingError::NotRead)
    ));
    workspace.publications[0] = None;
    assert!(matches!(
        workspace.inputs(&inputs).acquire_rows(&use_, &mut context),
        Err(OperatingError::NotRun)
    ));
}

#[test]
fn recapturing_publications_replaces_population_without_replacing_owner() {
    let owner = owner();
    let use_ = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number]],
        1,
    );
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let mut views = Vec::new();
    for value in [9, 4] {
        let publication = owner.computed(table(&[value], 1)).unwrap();
        let rows = owner
            .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
            .unwrap();
        workspace.capture_admitted(vec![publication.clone()]);
        let inputs = [BorrowedRows::from(&rows)];
        views.push(
            workspace
                .inputs(&inputs)
                .acquire_rows(&use_, &mut context)
                .unwrap(),
        );
    }
    assert_ne!(views[0].storage_identity(), views[1].storage_identity());
    assert_eq!(owner.derived().counts().0, 2);
}

use ryo_values::domain::rows::types::RowCount;

use ryo_derived::domain::contract::types::DerivedKind;

fn owner() -> Rc<crate::application::file_dag::relations::RelationOwner> {
    Rc::new(crate::application::file_dag::relations::RelationOwner::new(
        true,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    ))
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
