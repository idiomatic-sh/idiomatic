//! Numeric source identity and release contracts use the actual owner.
use crate::application::file_dag::relations::publication::types::*;
use crate::application::file_dag::relations::*;
use crate::cases::{Case, CaseName, Expect, Word, run};

use crate::application::file_dag::relations::handle::service::PublishedInput;
use ryo_execution_domain::domain::self_join_recogniser::types::{
    NumberRequirement, SchemaWidth, StoredNumberOrigin,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
fn number_witness_rejects_wrong_producer_and_recycled_generation() {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .map(|at| u32::try_from(at).expect("a relation's column number fits in u32"))
            .unwrap(),
    );
    let columns = schema
        .iter()
        .map(|_| FactColumn(vec![0]))
        .collect::<Vec<_>>();
    let rows = RepresentationInput::Stored(ColumnsView {
        kinds: schema,
        columns: &columns,
    });
    let required = NumberRequirement {
        source: StepId(0),
        column: number,
        width: SchemaWidth(schema.len()),
        origins: vec![StoredNumberOrigin {
            relation,
            column: number,
        }],
    };
    let owner = RelationOwner::for_request(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::relations::types::PinnedViewCount(64),
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let publication = owner.source_borrowed(relation, rows).unwrap();
    owner
        .bind_producer(
            crate::application::file_dag::relations::publication::types::ProducerScope::File,
            crate::application::file_dag::relations::types::ProducerId(7),
            publication.as_ref(),
        )
        .unwrap();
    assert!(
        owner
            .admit_number(
                &publication,
                crate::application::file_dag::relations::types::ProducerId(7),
                &required,
                rows
            )
            .unwrap()
            .is_some()
    );
    assert!(
        owner
            .admit_number(
                &publication,
                crate::application::file_dag::relations::types::ProducerId(8),
                &required,
                rows
            )
            .unwrap()
            .is_none()
    );
    owner.release_population();
    let next = owner.source_borrowed(relation, rows).unwrap();
    owner
        .bind_producer(
            crate::application::file_dag::relations::publication::types::ProducerScope::File,
            crate::application::file_dag::relations::types::ProducerId(7),
            next.as_ref(),
        )
        .unwrap();
    assert!(
        owner
            .admit_number(
                &next,
                crate::application::file_dag::relations::types::ProducerId(7),
                &required,
                rows
            )
            .unwrap()
            .is_some()
    );
    assert!(
        owner
            .admit_number(
                &publication,
                crate::application::file_dag::relations::types::ProducerId(7),
                &required,
                rows
            )
            .unwrap()
            .is_none()
    );
}
fn producer_scope_rebinding_refuses_before_registering_an_alias() {
    use crate::application::file_dag::derived::owner::fixtures::resources::ResourceFixture;
    use ryo_values::domain::rows::types::Table;

    let fixture = ResourceFixture::new(&[], &[], 0).unwrap();
    let owner = fixture.owner();
    let first = crate::application::file_dag::relations::publication::types::ProducerScope::Project(
        owner.project_identity().unwrap(),
    );
    let foreign =
        crate::application::file_dag::relations::publication::types::ProducerScope::Project(
            owner.project_identity().unwrap(),
        );
    let publication = owner.computed(Table::default()).unwrap();
    owner
        .bind_producer(
            first,
            crate::application::file_dag::relations::types::ProducerId(7),
            publication.as_ref(),
        )
        .unwrap();
    let identity = owner.result_population(publication.as_ref());
    owner
        .bind_producer(
            first,
            crate::application::file_dag::relations::types::ProducerId(7),
            publication.as_ref(),
        )
        .unwrap();
    assert_eq!(
        owner.bind_producer(
            foreign,
            crate::application::file_dag::relations::types::ProducerId(8),
            publication.as_ref()
        ),
        Err(OperatingError::WrongBinding),
    );
    assert_eq!(publication.as_ref().producer_scope.get(), Some(&first));
    assert_eq!(owner.result_population(publication.as_ref()), identity);
    assert!(!owner.producer_populations.borrow().contains_key(
        &crate::application::file_dag::relations::publication::types::ProducerKey {
            scope: foreign,
            producer: crate::application::file_dag::relations::types::ProducerId(8),
        }
    ));
    let independent = owner.computed(Table::default()).unwrap();
    owner
        .bind_producer(
            foreign,
            crate::application::file_dag::relations::types::ProducerId(8),
            independent.as_ref(),
        )
        .unwrap();
    assert_ne!(owner.result_population(independent.as_ref()), identity);
}

fn qualified_producers_release_only_their_actual_population() {
    use crate::application::file_dag::derived::owner::{
        ViewContext,
        fixtures::resources::{ResourceFixture, weak_storage},
    };
    use ryo_execution_domain::domain::derived_views::types::{
        DerivedInputs, DerivedScratch, ViewRequest,
    };
    use ryo_relations::domain::relations::types::{Integer, Value};
    use ryo_values::domain::rows::types::{RowCount, Table};
    // This is a generic owned-row index contract, not a catalog-provider fixture.
    let fixture = ResourceFixture::new(&[Relation::Function], &[], 0).unwrap();
    let owner = fixture.owner();
    let first_scope =
        crate::application::file_dag::relations::publication::types::ProducerScope::Project(
            owner.project_identity().unwrap(),
        );
    let second_scope =
        crate::application::file_dag::relations::publication::types::ProducerScope::Project(
            owner.project_identity().unwrap(),
        );
    let table = |number| Table {
        kinds: vec![CellKind::Number],
        count: RowCount(1),
        cells: vec![Value::Number(Integer(number))],
    };
    let first = owner.computed(table(1)).unwrap();
    let second = owner.computed(table(2)).unwrap();
    owner
        .bind_producer(
            first_scope,
            crate::application::file_dag::relations::types::ProducerId(7),
            first.as_ref(),
        )
        .unwrap();
    owner
        .bind_producer(
            second_scope,
            crate::application::file_dag::relations::types::ProducerId(7),
            second.as_ref(),
        )
        .unwrap();
    assert_ne!(
        owner.result_population(first.as_ref()),
        owner.result_population(second.as_ref())
    );
    let acquire = |publication: &PublishedInput| {
        let rows = owner
            .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
            .unwrap();
        let proof = owner
            .admit(publication.as_ref(), RepresentationInput::from(&rows))
            .unwrap();
        let population = proof.identity(owner).unwrap().population;
        let plan = Arc::new(
            owner
                .derived
                .prepare_view(
                    &ViewRequest::Equality {
                        columns: vec![ColumnIndex(0)],
                    },
                    &[
                        ryo_execution_domain::domain::operation_schema::types::InputShape {
                            kinds: ryo_execution_domain::domain::table_model::service::kinds_of(
                                &rows,
                            ),
                            tree: None,
                        },
                    ],
                )
                .unwrap(),
        );
        owner
            .derived
            .acquire_stored(
                &plan,
                &owner.derived.admit_view(&plan, &owner, &[&proof]).unwrap(),
                ViewContext::default(),
                DerivedInputs::Rows(&[proof.rows()]),
                &mut DerivedScratch::default(),
                None,
            )
            .unwrap()
    };
    owner.pin_derived(first.as_ref());
    owner.pin_derived(second.as_ref());
    let a = acquire(&first);
    let b = acquire(&second);
    let weak_a = weak_storage(&a);
    let weak_b = weak_storage(&b);
    drop((a, b));
    assert!(weak_a.upgrade().is_some());
    assert!(weak_b.upgrade().is_some());
    // Bind/reuse B before A's delayed retirement: release must carry A's key.
    let b = acquire(&second);
    owner.release_derived_producer(
        first_scope,
        crate::application::file_dag::relations::types::ProducerId(7),
    );
    assert!(weak_a.upgrade().is_none());
    assert!(weak_b.upgrade().is_some());
    drop(b);
    owner.release_derived_producer(
        second_scope,
        crate::application::file_dag::relations::types::ProducerId(7),
    );
    assert!(weak_b.upgrade().is_none());
    drop((first, second));
    owner.release_population();
    assert_eq!(owner.derived.working_bytes(), 0);
}

/// A computed numeric source is admitted when its cells fill its rows, and refused when they do not.
fn number_witness_admits_a_rectangular_computed_source_and_refuses_a_ragged_one() {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .map(|at| u32::try_from(at).expect("a relation's column number fits in u32"))
            .unwrap(),
    );
    let required = NumberRequirement {
        source: StepId(0),
        column: number,
        width: SchemaWidth(schema.len()),
        origins: vec![StoredNumberOrigin {
            relation,
            column: number,
        }],
    };
    let admitted = |cells: usize| {
        let table = ryo_values::domain::rows::types::Table {
            kinds: schema.to_vec(),
            count: ryo_values::domain::rows::types::RowCount(1),
            cells: vec![
                ryo_relations::domain::relations::types::Value::Number(
                    ryo_relations::domain::relations::types::Integer(0),
                );
                cells
            ],
        };
        let rows = RepresentationInput::Computed(&table);
        let owner = RelationOwner::for_request(
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            crate::application::file_dag::relations::types::PinnedViewCount(64),
            std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
        );
        let publication = owner.source_borrowed(relation, rows).unwrap();
        owner
            .bind_producer(
                ProducerScope::File,
                crate::application::file_dag::relations::types::ProducerId(7),
                publication.as_ref(),
            )
            .unwrap();
        owner
            .admit_number(
                &publication,
                crate::application::file_dag::relations::types::ProducerId(7),
                &required,
                rows,
            )
            .map(|number| number.is_some())
    };
    assert_eq!(admitted(schema.len()), Ok(true), "the cells fill the row");
    assert_eq!(
        admitted(schema.len() - 1),
        Ok(false),
        "a ragged table is refused"
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    NumberWitnessRejectsWrongProducerAndRecycledGeneration,
    ProducerScopeRebindingRefusesBeforeRegisteringAnAlias,
    QualifiedProducersReleaseOnlyTheirActualPopulation,
    NumberWitnessAdmitsARectangularComputedSourceAndRefusesARaggedOne,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("number_witness_rejects_wrong_producer_and_recycled_generation"),
        word: Word::Succeeds,
        input: || Scenario::NumberWitnessRejectsWrongProducerAndRecycledGeneration,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("producer_scope_rebinding_refuses_before_registering_an_alias"),
        word: Word::Succeeds,
        input: || Scenario::ProducerScopeRebindingRefusesBeforeRegisteringAnAlias,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("qualified_producers_release_only_their_actual_population"),
        word: Word::Succeeds,
        input: || Scenario::QualifiedProducersReleaseOnlyTheirActualPopulation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "number_witness_admits_a_rectangular_computed_source_and_refuses_a_ragged_one",
        ),
        word: Word::Succeeds,
        input: || Scenario::NumberWitnessAdmitsARectangularComputedSourceAndRefusesARaggedOne,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::NumberWitnessRejectsWrongProducerAndRecycledGeneration => {
            number_witness_rejects_wrong_producer_and_recycled_generation();
            Ok(())
        }
        Scenario::ProducerScopeRebindingRefusesBeforeRegisteringAnAlias => {
            producer_scope_rebinding_refuses_before_registering_an_alias();
            Ok(())
        }
        Scenario::QualifiedProducersReleaseOnlyTheirActualPopulation => {
            qualified_producers_release_only_their_actual_population();
            Ok(())
        }
        Scenario::NumberWitnessAdmitsARectangularComputedSourceAndRefusesARaggedOne => {
            number_witness_admits_a_rectangular_computed_source_and_refuses_a_ragged_one();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
