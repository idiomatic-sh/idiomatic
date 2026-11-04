//! Actual view acquisition consumes one immutable proof and refuses stale/mismatched publications.
use super::*;
use crate::application::file_dag::derived::owner::ViewContext;
use crate::domain::{
    derived_views::types::*,
    operating::{errors::OperatingError, types::OperatorContext},
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
    table_model::types::{ParameterBinding, Rows},
};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::{rc::Rc, sync::Arc};

#[test]
fn streamed_capture_preserves_any_held_publication_and_unheld_rows_remain_scoped() {
    use crate::application::file_dag::feed::{Feed, InputOrigin};
    use crate::domain::operating::types::{Emit, InputPlace, Supply};
    use std::cell::Cell;

    struct Streamed {
        owner: Rc<crate::application::file_dag::relations::RelationOwner>,
        publication_calls: Cell<usize>,
        supply: Supply,
        publication: Option<crate::application::file_dag::relations::PublishedInput>,
    }
    impl<'f> Feed<'f> for Streamed {
        fn relation_owner(&self) -> Rc<crate::application::file_dag::relations::RelationOwner> {
            Rc::clone(&self.owner)
        }
        fn origin(&self, _: InputPlace) -> Result<InputOrigin, OperatingError> {
            Ok(InputOrigin::Internal)
        }
        fn publication(
            &self,
            _: InputPlace,
        ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
        {
            self.publication_calls.set(self.publication_calls.get() + 1);
            Ok(self.publication.clone())
        }
        fn kinds(&self, _: InputPlace) -> Result<&[CellKind], OperatingError> {
            Ok(&[CellKind::Number])
        }
        fn streams(&self, _: InputPlace) -> Supply {
            self.supply
        }
        fn stream(
            &mut self,
            _: InputPlace,
            _: &mut OperatorContext<'_>,
            _: &mut dyn Emit,
        ) -> Result<(), OperatingError> {
            panic!("capture must not consume the stream")
        }
        fn found(&mut self, _: InputPlace, _: RowCount) {}
        fn hold<'t>(
            &mut self,
            _: InputPlace,
            _: &mut OperatorContext<'_>,
            _: &'t mut Table,
        ) -> Result<Rows<'t>, OperatingError>
        where
            'f: 't,
        {
            panic!("capture must not hold the stream")
        }
    }

    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
    let feed = Streamed {
        owner: Rc::clone(&owner),
        publication_calls: Cell::new(0),
        supply: Supply::Streamed,
        publication: None,
    };
    let mut workspace = InvocationWorkspace::new(owner);
    workspace.capture(&feed, 1).unwrap();
    assert_eq!(feed.publication_calls.get(), 1);
    assert!(matches!(workspace.origins[0], InputOrigin::InternalStream));
    assert!(workspace.publications[0].is_none());
    let row = table(&[7], 1);
    assert!(workspace.admit(0, BorrowedRows::Computed(&row)).is_ok());

    let held_row = table(&[9], 1);
    let publication = feed.owner.computed(held_row.clone()).unwrap();
    let held_rows = feed
        .owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let held = Streamed {
        owner: Rc::clone(&feed.owner),
        publication_calls: Cell::new(0),
        supply: Supply::Streamed,
        publication: Some(publication.clone()),
    };
    workspace.capture(&held, 1).unwrap();
    assert_eq!(held.publication_calls.get(), 1);
    assert!(matches!(workspace.origins[0], InputOrigin::Internal));
    assert!(
        workspace.publications[0]
            .as_ref()
            .unwrap()
            .same_record(&publication)
    );
    assert!(workspace.admit(0, BorrowedRows::from(&held_rows)).is_ok());
    held.owner.release_population();
    assert!(matches!(
        workspace.admit(0, BorrowedRows::from(&held_rows)),
        Err(OperatingError::NotRead)
    ));
    let unheld = Streamed {
        owner: Rc::clone(&feed.owner),
        publication_calls: Cell::new(0),
        supply: Supply::AtHand,
        publication: None,
    };
    workspace.capture(&unheld, 1).unwrap();
    assert!(matches!(
        workspace.admit(0, BorrowedRows::from(&held_rows)),
        Err(OperatingError::NotRun)
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
        .map(|kinds| InputSchema { kinds, tree: None })
        .collect::<Vec<_>>();
    PreparedViewUse {
        plan: {
            let plan = crate::application::file_dag::derived::prepare::view(
                crate::outbound::derived::factory::test_port(),
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
        semantic: None,
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
    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
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
    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
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
    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![3])];
    let raw = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let published = owner.source(Relation::LanguageTarget, &raw).unwrap();
    let normalized = owner.normalized(&published, raw).unwrap().unwrap();
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
fn recapturing_delivery_replaces_temporary_population_without_replacing_owner() {
    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
    let use_ = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number]],
        1,
    );
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let first = [table(&[9], 1)];
    let feed = crate::application::file_dag::testing::Held {
        owner: Rc::clone(&owner),
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
        owner: Rc::clone(&owner),
        tables: &second,
        visits: vec![0],
    };
    workspace.capture(&feed, 1).unwrap();
    assert_eq!(
        workspace.owner().derived.counts().0,
        1,
        "recapture preserves the required owner and its build accounting"
    );
    let fresh = workspace
        .acquire_rows(&use_, &[BorrowedRows::Computed(&second[0])], &mut context)
        .unwrap();
    assert!(!old.same_storage(&fresh));
    let DerivedView::Equality(index) = fresh.view().unwrap() else {
        panic!("equality")
    };
    let view = crate::domain::operator::contract::views::IndexedRows {
        #[cfg(feature = "diagnostic-row-progress")]
        progress: |_, _| {},
        rows: BorrowedRows::Computed(&second[0]),
        columns: &[ColumnIndex(0)],
        index,
    };
    assert_eq!(view.matching(&[Value::Number(Integer(4))]).count(), 1);
    assert_eq!(view.matching(&[Value::Number(Integer(9))]).count(), 0);
}

#[test]
fn temporary_admission_cannot_repair_missing_or_mismatched_internal_publication() {
    let scope = file_scope();
    let owner = Rc::clone(scope.scope.owner());
    let values = table(&[7], 1);
    let rows = [BorrowedRows::Computed(&values)];
    let use_ = use_(
        DerivedKind::Equality,
        ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        &[&[CellKind::Number]],
        1,
    );
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    workspace.origins = vec![crate::application::file_dag::feed::InputOrigin::External];
    workspace.publications = vec![None];
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    drop(workspace.acquire_rows(&use_, &rows, &mut context).unwrap());
    let builds = owner.derived.counts().0;
    workspace.origins[0] = crate::application::file_dag::feed::InputOrigin::Internal;
    assert!(matches!(
        workspace.acquire_rows(&use_, &rows, &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        owner.derived.counts().0,
        builds,
        "missing producer never reaches construction"
    );
    workspace.origins[0] = crate::application::file_dag::feed::InputOrigin::External;
    workspace.publications[0] = Some(owner.computed(table(&[9], 1)).unwrap());
    assert!(matches!(
        workspace.acquire_rows(&use_, &rows, &mut context),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        owner.derived.counts().0,
        builds,
        "a supplied publication cannot silently become temporary"
    );
}

use ryo_values::domain::rows::types::RowCount;

use ryo_derived::domain::contract::types::DerivedKind;

fn file_scope() -> crate::application::file_dag::testing::FileScope<'static> {
    use crate::application::file_dag::executor::PrimitiveExecutor as _;
    let executor = crate::application::file_dag::execution::CommonOperatorEngine::fixture_ref();
    let request = crate::domain::operator::contract::types::RequestOperators::prepare(
        &[],
        &[],
        &[],
        executor.derived_port(),
    )
    .unwrap();
    let graph = Arc::new(crate::domain::file_program::service::compile_prepared(
        &[],
        &request,
    ));
    crate::application::file_dag::testing::FileScope::new(graph, executor, |_| true, &[], &[])
        .unwrap()
}
