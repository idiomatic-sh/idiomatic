//! Actual view acquisition consumes one immutable proof and refuses stale/mismatched publications.
use super::super::super::service::*;
use super::super::super::types::*;
use super::fixtures::{
    Answered, Held, TwoRights, TwoViews, context, equality_on_first, membership_on_first,
    ordered_key, ordered_keys, owner, read_through_the_ports, table,
};
use crate::application::file_dag::derived::owner::service::ViewContext;
use crate::application::file_dag::derived::owner::types::BuildCount;
use crate::application::file_dag::relations::handle::service::Publication;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as CellValue,
};
use ryo_values::domain::plan_parts::types::InputNumber;
use std::rc::Rc;

type Input = Held;

type Value = Answered;

type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("admission_requires_a_live_matching_publication_in_test_builds_too"),
        word: Word::Succeeds,
        input: || Held::Captured,
        expect: Expect::Value(Answered::Admitted {
            relation: None,
            kinds: [CellKind::Number],
        }),
    },
    Case {
        name: CaseName("admission_without_captured_publications_names_the_missing_input"),
        word: Word::Refuses,
        input: || Held::Nothing,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("admission_into_an_empty_slot_is_not_run"),
        word: Word::Refuses,
        input: || Held::EmptySlot,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("admission_after_release_reports_missing_input"),
        word: Word::Refuses,
        input: || Held::AfterRelease,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("admissions_beyond_the_three_proof_slots_are_wrong_operation"),
        word: Word::Refuses,
        input: || Held::BeyondProofSlots,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("admissions_past_the_inputs_name_the_missing_input"),
        word: Word::Refuses,
        input: || Held::PastInputs(1),
        expect: Expect::Error(OperatingError::MissingInput { position: 2 }),
    },
    Case {
        name: CaseName("admission_of_rows_the_publication_did_not_resolve_is_not_read"),
        word: Word::Refuses,
        input: || Held::Unpublished,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("admission_after_the_population_is_released_is_not_read"),
        word: Word::Refuses,
        input: || Held::Released,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName(
            "immutable_demand_checks_identity_once_across_views_and_refuses_expired_proof",
        ),
        word: Word::Succeeds,
        input: || Held::TwoViews(16),
        expect: Expect::Value(Answered::TwoViews(TwoViews {
            index_rows: 3 * 16,
            membership_values: 2 * 16,
            // Both actual families inspect the same admitted input once.
            inspections: 1,
            expired: Some(OperatingError::NotRead),
            // An expired proof never reaches the builder.
            built_after_expiry: false,
        })),
    },
    Case {
        name: CaseName("distinct_published_populations_never_share_slot_only_identities"),
        word: Word::Succeeds,
        input: || Held::TwoRights([9, 4]),
        expect: Expect::Value(Answered::TwoRights(TwoRights {
            right_keys: [
                Some(CellValue::Number(Integer(9))),
                Some(CellValue::Number(Integer(4))),
            ],
            distinct_views: true,
            retained_keys: [
                Some(CellValue::Number(Integer(9))),
                Some(CellValue::Number(Integer(4))),
            ],
            builds: BuildCount(2),
            unpublished: Some(OperatingError::NotRead),
        })),
    },
    Case {
        name: CaseName("normalized_physical_input_is_admitted_but_unproved_transform_is_not"),
        word: Word::Succeeds,
        input: || Held::Normalized,
        expect: Expect::Value(Answered::Normalized {
            indexes_every_row: true,
            transformed: Some(OperatingError::NotRead),
            empty_slot: Some(OperatingError::NotRun),
        }),
    },
    Case {
        name: CaseName("recapturing_publications_replaces_population_without_replacing_owner"),
        word: Word::Succeeds,
        input: || Held::Recaptured([9, 4]),
        expect: Expect::Value(Answered::Recaptured {
            distinct: true,
            builds: BuildCount(2),
        }),
    },
    Case {
        name: CaseName("each_view_request_prepares_a_view_of_its_own_kind"),
        word: Word::Succeeds,
        input: || Held::Prepared,
        expect: Expect::Value(Answered::Prepared([
            DerivedKind::Equality,
            DerivedKind::Membership,
            DerivedKind::OrderedKeys,
        ])),
    },
];

/// Builds the workspace the input holds and admits or acquires through it.
fn subject(input: Held) -> Result<Answered, OperatingError> {
    let owner = owner();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    match input {
        Held::Nothing | Held::EmptySlot | Held::Captured | Held::Unpublished | Held::Released => {
            let raw = table(&[7], 1);
            if !matches!(input, Held::Nothing) {
                workspace.publications = vec![None];
            }
            let publication = owner.computed(raw.clone())?;
            let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
            if !matches!(input, Held::Nothing | Held::EmptySlot) {
                workspace.capture_admitted(vec![Publication::of(publication.clone())]);
            }
            if matches!(input, Held::Released) {
                owner.release_population();
            }
            let admitted = match input {
                Held::Captured | Held::Released => {
                    workspace.admit(InputNumber(0), RepresentationInput::from(&rows))?
                }
                Held::Nothing
                | Held::EmptySlot
                | Held::Unpublished
                | Held::TwoViews(_)
                | Held::TwoRights(_)
                | Held::Normalized
                | Held::Recaptured(_)
                | Held::Prepared
                | Held::AfterRelease
                | Held::BeyondProofSlots
                | Held::PastInputs(_) => {
                    workspace.admit(InputNumber(0), RepresentationInput::Computed(&raw))?
                }
            };
            Ok(Answered::Admitted {
                relation: admitted.identity(&owner)?.relation,
                kinds: admitted
                    .rows()
                    .kinds()
                    .try_into()
                    .map_err(|_| OperatingError::WrongOperation)?,
            })
        }
        Held::AfterRelease | Held::BeyondProofSlots | Held::PastInputs(_) => {
            read_through_the_ports(&owner, &mut workspace, input)
        }
        Held::TwoViews(reads) => {
            let published = owner.computed(table(&[3, 1, 3], 1))?;
            let rows = owner.resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))?;
            workspace.publications = vec![Some(Publication::of(published.clone()))];
            let equality = equality_on_first(1);
            let membership = membership_on_first();
            let inputs = [RepresentationInput::from(&rows)];
            let scope = workspace.inputs(&inputs);
            let before = owner.admitted_inspections();
            let index = scope.acquire_rows(&equality, &mut context)?;
            let members = scope.acquire_derived(
                &membership,
                ViewContext::default(),
                ViewInputs::MembershipValues,
            )?;
            let (mut index_rows, mut membership_values) = (0, 0);
            for _ in 0..reads {
                if let Some(DerivedView::Equality(index)) = index.view().ok() {
                    index_rows += index.rows.len();
                }
                if let Some(DerivedView::Membership(
                    ryo_derived::domain::membership::types::MembershipView::Values(values),
                )) = members.view().ok()
                {
                    membership_values += values.len();
                }
            }
            let inspections = owner.admitted_inspections().0 - before.0;
            owner.release_population();
            let builds = owner.derived().counts().0;
            let expired = scope.acquire_rows(&equality, &mut context).err();
            Ok(Answered::TwoViews(TwoViews {
                index_rows,
                membership_values,
                inspections,
                expired,
                built_after_expiry: owner.derived().counts().0 != builds,
            }))
        }
        Held::TwoRights(rights) => {
            let published = owner.computed(table(&[1, 0], 2))?;
            let left = owner.resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))?;
            workspace.publications = vec![Some(Publication::of(published.clone())), None];
            let ordered = ordered_keys();
            let mut retained_views = Vec::new();
            let mut right_keys = [None; 2];
            for (key, right_value) in right_keys.iter_mut().zip(rights) {
                let right_publication = owner.computed(table(&[right_value, 0], 2))?;
                let right = owner.resolve(right_publication.as_ref(), &|_| {
                    Err(OperatingError::NotRead)
                })?;
                workspace.capture_admitted(vec![
                    Publication::of(published.clone()),
                    Publication::of(right_publication.clone()),
                ]);
                let inputs = [
                    RepresentationInput::from(&left),
                    RepresentationInput::from(&right),
                ];
                let texts = ryo_operators::domain::table_texts::types::Texts {
                    symbols: context.symbols.as_ref(),
                    source: context.source,
                    paths: &[],
                    arenas: &[],
                };
                let view = workspace.inputs(&inputs).acquire_derived(
                    &ordered,
                    ViewContext::default(),
                    ViewInputs::Text(&texts),
                )?;
                *key = ordered_key(&view);
                retained_views.push(view);
            }
            let distinct_views = match (retained_views[0].view(), retained_views[1].view()) {
                (Ok(DerivedView::OrderedKeys(first)), Ok(DerivedView::OrderedKeys(second))) => {
                    // A live old view cannot be reused by temporary slot identity.
                    !std::ptr::eq(first, second)
                }
                _ => false,
            };
            let retained_keys = [
                ordered_key(&retained_views[0]),
                ordered_key(&retained_views[1]),
            ];
            // Mixed inputs use the same owner with distinct admitted populations.
            let builds = owner.derived().counts().0;
            workspace.publications[1] = Some(Publication::of(published.clone()));
            let wrong = table(&[4, 0], 2);
            let inputs = [
                RepresentationInput::from(&left),
                RepresentationInput::Computed(&wrong),
            ];
            let texts = ryo_operators::domain::table_texts::types::Texts {
                symbols: context.symbols.as_ref(),
                source: context.source,
                paths: &[],
                arenas: &[],
            };
            let unpublished = workspace
                .inputs(&inputs)
                .acquire_derived(&ordered, ViewContext::default(), ViewInputs::Text(&texts))
                .err();
            Ok(Answered::TwoRights(TwoRights {
                right_keys,
                distinct_views,
                retained_keys,
                builds,
                unpublished,
            }))
        }
        Held::Normalized => {
            let columns = [FactColumn(vec![1, 2]), FactColumn(vec![3])];
            let raw = Rows::Stored(ColumnsView {
                kinds: &[CellKind::Number; 2],
                columns: &columns,
            });
            let published = owner.source(Relation::LanguageTarget, &raw)?;
            let normalized = owner.normalize(&published, raw)?;
            workspace.publications = vec![Some(Publication::of(published.clone()))];
            let equality = equality_on_first(2);
            let inputs = [RepresentationInput::from(&normalized)];
            let storage = workspace
                .inputs(&inputs)
                .acquire_rows(&equality, &mut context)?;
            let DerivedView::Equality(index) = storage.view()? else {
                return Err(OperatingError::WrongOperation);
            };
            let indexes_every_row = index.rows.len()
                == ryo_execution_domain::domain::table_model::service::count_of(&normalized).0;
            let transformed = table(&[8, 9], 2);
            let inputs = [RepresentationInput::Computed(&transformed)];
            let transformed_refused = workspace
                .inputs(&inputs)
                .acquire_rows(&equality, &mut context)
                .err();
            workspace.publications[0] = None;
            let empty_slot = workspace
                .inputs(&inputs)
                .acquire_rows(&equality, &mut context)
                .err();
            Ok(Answered::Normalized {
                indexes_every_row,
                transformed: transformed_refused,
                empty_slot,
            })
        }
        Held::Prepared => Ok(Answered::Prepared(
            [equality_on_first(1), membership_on_first(), ordered_keys()]
                .map(|view| view.plan.kind),
        )),
        Held::Recaptured(values) => {
            let equality = equality_on_first(1);
            let mut views = Vec::new();
            for value in values {
                let publication = owner.computed(table(&[value], 1))?;
                let rows =
                    owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
                workspace.capture_admitted(vec![Publication::of(publication.clone())]);
                let inputs = [RepresentationInput::from(&rows)];
                views.push(
                    workspace
                        .inputs(&inputs)
                        .acquire_rows(&equality, &mut context)?,
                );
            }
            Ok(Answered::Recaptured {
                distinct: views[0].storage_identity() != views[1].storage_identity(),
                builds: owner.derived().counts().0,
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
