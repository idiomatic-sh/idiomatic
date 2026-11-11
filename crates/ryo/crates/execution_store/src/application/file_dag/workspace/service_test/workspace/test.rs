//! Actual view acquisition consumes one immutable proof and refuses stale/mismatched publications.
use super::super::super::*;
use super::fixtures::{context, owner, table, use_};
use crate::application::file_dag::derived::owner::ViewContext;
use crate::application::file_dag::relations::handle::service::Publication;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::{
    derived_views::types::*, operating::errors::OperatingError, table_model::types::Rows,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::rc::Rc;

fn admission_requires_a_live_matching_publication_in_test_builds_too() {
    let owner = owner();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let raw = table(&[7], 1);
    assert!(matches!(
        workspace.admit(InputNumber(0), RepresentationInput::Computed(&raw)),
        Err(OperatingError::MissingInput { position: 1 })
    ));
    workspace.publications = vec![None];
    assert!(matches!(
        workspace.admit(InputNumber(0), RepresentationInput::Computed(&raw)),
        Err(OperatingError::NotRun)
    ));
    let publication = owner.computed(raw.clone()).unwrap();
    let rows = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    workspace.capture_admitted(vec![Publication::of(publication.clone())]);
    let admitted = workspace
        .admit(InputNumber(0), RepresentationInput::from(&rows))
        .unwrap();
    assert_eq!(admitted.identity(&owner).unwrap().relation, None);
    assert_eq!(admitted.rows().kinds(), &[CellKind::Number]);
    assert!(matches!(
        workspace.admit(InputNumber(0), RepresentationInput::Computed(&raw)),
        Err(OperatingError::NotRead)
    ));
    owner.release_population();
    assert!(matches!(
        workspace.admit(InputNumber(0), RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
}
fn immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof() {
    let owner = owner();
    let published = owner.computed(table(&[3, 1, 3], 1)).unwrap();
    let rows = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    workspace.publications = vec![Some(Publication::of(published.clone()))];
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
    let inputs = [RepresentationInput::from(&rows)];
    let scope = workspace.inputs(&inputs);
    let before = owner.admitted_inspections();
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
        let DerivedView::Membership(
            ryo_derived::domain::membership::types::MembershipView::Values(values),
        ) = members.view().unwrap()
        else {
            panic!("membership")
        };
        assert_eq!(values.len(), 2);
    }
    assert_eq!(
        owner.admitted_inspections().0 - before.0,
        1,
        "both actual families inspect the same admitted input once"
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
fn distinct_published_populations_never_share_slot_only_identities() {
    let owner = owner();
    let published = owner.computed(table(&[1, 0], 2)).unwrap();
    let left = owner
        .resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    workspace.publications = vec![Some(Publication::of(published.clone())), None];
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
        workspace.capture_admitted(vec![
            Publication::of(published.clone()),
            Publication::of(right_publication.clone()),
        ]);
        let inputs = [
            RepresentationInput::from(&left),
            RepresentationInput::from(&right),
        ];
        let texts = ryo_execution_domain::domain::table_texts::types::Texts {
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
            matches!(rows.right[0].keys[0],ryo_derived::domain::ordered::types::OwnedOrderedKey::Value(CellValue::Number(Integer(value)))if value==expected)
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
        ryo_derived::domain::ordered::types::OwnedOrderedKey::Value(CellValue::Number(Integer(9)))
    ));
    assert!(matches!(
        second.right[0].keys[0],
        ryo_derived::domain::ordered::types::OwnedOrderedKey::Value(CellValue::Number(Integer(4)))
    ));
    assert_eq!(
        owner.derived().counts().0,
        2,
        "mixed inputs use the same owner with distinct admitted populations"
    );
    workspace.publications[1] = Some(Publication::of(published.clone()));
    let wrong = table(&[4, 0], 2);
    let inputs = [
        RepresentationInput::from(&left),
        RepresentationInput::Computed(&wrong),
    ];
    let texts = ryo_execution_domain::domain::table_texts::types::Texts {
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
    workspace.publications = vec![Some(Publication::of(published.clone()))];
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
    let inputs = [RepresentationInput::from(&normalized)];
    let storage = workspace
        .inputs(&inputs)
        .acquire_rows(&use_, &mut context)
        .unwrap();
    let DerivedView::Equality(index) = storage.view().unwrap() else {
        panic!("equality")
    };
    assert_eq!(
        index.rows.len(),
        ryo_execution_domain::domain::table_model::service::count_of(&normalized).0
    );
    let transformed = table(&[8, 9], 2);
    let inputs = [RepresentationInput::Computed(&transformed)];
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
        workspace.capture_admitted(vec![Publication::of(publication.clone())]);
        let inputs = [RepresentationInput::from(&rows)];
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

use ryo_derived::domain::request::types::DerivedKind;

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    AdmissionRequiresALiveMatchingPublicationInTestBuildsToo,
    ImmutableDemandChecksIdentityOnceAcrossViewsAndRefusesExpiredProof,
    DistinctPublishedPopulationsNeverShareSlotOnlyIdentities,
    NormalizedPhysicalInputIsAdmittedButUnprovedTransformIsNot,
    RecapturingPublicationsReplacesPopulationWithoutReplacingOwner,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("admission_requires_a_live_matching_publication_in_test_builds_too"),
        word: Word::Succeeds,
        input: || Scenario::AdmissionRequiresALiveMatchingPublicationInTestBuildsToo,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof",
        ),
        word: Word::Succeeds,
        input: || Scenario::ImmutableDemandChecksIdentityOnceAcrossViewsAndRefusesExpiredProof,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("distinct_published_populations_never_share_slot_only_identities"),
        word: Word::Succeeds,
        input: || Scenario::DistinctPublishedPopulationsNeverShareSlotOnlyIdentities,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("normalized_physical_input_is_admitted_but_unproved_transform_is_not"),
        word: Word::Succeeds,
        input: || Scenario::NormalizedPhysicalInputIsAdmittedButUnprovedTransformIsNot,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("recapturing_publications_replaces_population_without_replacing_owner"),
        word: Word::Succeeds,
        input: || Scenario::RecapturingPublicationsReplacesPopulationWithoutReplacingOwner,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::AdmissionRequiresALiveMatchingPublicationInTestBuildsToo => {
            admission_requires_a_live_matching_publication_in_test_builds_too();
            Ok(())
        }
        Scenario::ImmutableDemandChecksIdentityOnceAcrossViewsAndRefusesExpiredProof => {
            immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof();
            Ok(())
        }
        Scenario::DistinctPublishedPopulationsNeverShareSlotOnlyIdentities => {
            distinct_published_populations_never_share_slot_only_identities();
            Ok(())
        }
        Scenario::NormalizedPhysicalInputIsAdmittedButUnprovedTransformIsNot => {
            normalized_physical_input_is_admitted_but_unproved_transform_is_not();
            Ok(())
        }
        Scenario::RecapturingPublicationsReplacesPopulationWithoutReplacingOwner => {
            recapturing_publications_replaces_population_without_replacing_owner();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
