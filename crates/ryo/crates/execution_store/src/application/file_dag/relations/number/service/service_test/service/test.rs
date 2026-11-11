//! Numeric source identity and release contracts use the actual owner.
use crate::application::file_dag::relations::publication::types::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::sync::Arc;
use crate::application::file_dag::relations::handle::service::PublishedInput;
use crate::application::file_dag::relations::service::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::self_join_recogniser::types::{
    NumberRequirement, SchemaWidth, StoredNumberOrigin,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
/// The producers, populations and sources the owner's numbers and releases are asked about.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A stored `SourceLocation` source bound to producer 7: its number asked by producer 7 and
    /// 8, then after the population is released, by the next source and by the old one.
    Witness,
    /// A computed source bound to a project scope, rebound to it, then to a foreign scope.
    Rebinding,
    /// Two computed sources bound to producer 7 in two project scopes, whose views are released
    /// by scope.
    QualifiedRelease,
    /// A computed `SourceLocation` source of one row with this many cells short of its width.
    Computed { short: usize },
}

/// What rebinding a producer observed.
#[derive(Debug, PartialEq)]
struct Rebinding {
    foreign: Option<OperatingError>,
    scope_kept: bool,
    population_kept: bool,
    foreign_registered: bool,
    independent_own_population: bool,
}

/// What releasing by producer scope observed.
#[derive(Debug, PartialEq)]
struct Released {
    distinct_populations: bool,
    alive_after_drop: [bool; 2],
    alive_after_first_release: [bool; 2],
    second_alive_after_its_release: bool,
    working_bytes: usize,
}

/// What the owner answered.
#[derive(Debug, PartialEq)]
enum Observed {
    /// Whether each of the four number asks was admitted.
    Witness([bool; 4]),
    Rebinding(Rebinding),
    Released(Released),
    /// Whether the number was admitted.
    Admitted(bool),
}

type Input = Asked;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("number_witness_rejects_wrong_producer_and_recycled_generation"),
        word: Word::Succeeds,
        input: || Asked::Witness,
        expect: Expect::Value(Observed::Witness([true, false, true, false])),
    },
    Case {
        name: CaseName("producer_scope_rebinding_refuses_before_registering_an_alias"),
        word: Word::Succeeds,
        input: || Asked::Rebinding,
        expect: Expect::Value(Observed::Rebinding(Rebinding {
            foreign: Some(OperatingError::WrongBinding),
            scope_kept: true,
            population_kept: true,
            foreign_registered: false,
            independent_own_population: true,
        })),
    },
    Case {
        name: CaseName("qualified_producers_release_only_their_actual_population"),
        word: Word::Succeeds,
        input: || Asked::QualifiedRelease,
        expect: Expect::Value(Observed::Released(Released {
            distinct_populations: true,
            alive_after_drop: [true, true],
            alive_after_first_release: [false, true],
            second_alive_after_its_release: false,
            working_bytes: 0,
        })),
    },
    Case {
        name: CaseName(
            "number_witness_admits_a_rectangular_computed_source_and_refuses_a_ragged_one",
        ),
        word: Word::Succeeds,
        input: || Asked::Computed { short: 0 },
        expect: Expect::Value(Observed::Admitted(true)),
    },
    Case {
        name: CaseName("number_witness_refuses_a_ragged_computed_source"),
        word: Word::Succeeds,
        input: || Asked::Computed { short: 1 },
        expect: Expect::Value(Observed::Admitted(false)),
    },
];

const PRODUCER: crate::application::file_dag::relations::types::ProducerId =
    crate::application::file_dag::relations::types::ProducerId(7);

fn requirement(relation: Relation) -> NumberRequirement {
    let schema = kinds(relation);
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .map(|at| u32::try_from(at).expect("a relation's column number fits in u32"))
            .expect("the relation has a number column"),
    );
    NumberRequirement {
        source: StepId(0),
        column: number,
        width: SchemaWidth(schema.len()),
        origins: vec![StoredNumberOrigin {
            relation,
            column: number,
        }],
    }
}

fn request_owner() -> RelationOwner {
    RelationOwner::for_request(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::relations::types::PinnedViewCount(64),
        Arc::new(crate::application::file_dag::fixtures::derived()),
    )
}

/// Asks the owner as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Asked::Witness => Observed::Witness(witness()?),
        Asked::Rebinding => Observed::Rebinding(rebinding()?),
        Asked::QualifiedRelease => Observed::Released(released()?),
        Asked::Computed { short } => {
            let relation = Relation::SourceLocation;
            let schema = kinds(relation);
            let required = requirement(relation);
            let table = ryo_values::domain::rows::types::Table {
                kinds: schema.to_vec(),
                count: ryo_values::domain::rows::types::RowCount(1),
                cells: vec![
                    ryo_relations::domain::relations::types::Value::Number(
                        ryo_relations::domain::relations::types::Integer(0),
                    );
                    schema.len() - short
                ],
            };
            let rows = RepresentationInput::Computed(&table);
            let owner = request_owner();
            let publication = owner.source_borrowed(relation, rows)?;
            owner.bind_producer(ProducerScope::File, PRODUCER, publication.as_ref())?;
            Observed::Admitted(
                owner
                    .admit_number(&publication, PRODUCER, &required, rows)?
                    .is_some(),
            )
        }
    })
}

fn witness() -> Result<[bool; 4], OperatingError> {
    let relation = Relation::SourceLocation;
    let schema = kinds(relation);
    let columns = schema
        .iter()
        .map(|_| FactColumn(vec![0]))
        .collect::<Vec<_>>();
    let rows = RepresentationInput::Stored(ColumnsView {
        kinds: schema,
        columns: &columns,
    });
    let required = requirement(relation);
    let owner = request_owner();
    let publication = owner.source_borrowed(relation, rows)?;
    owner.bind_producer(ProducerScope::File, PRODUCER, publication.as_ref())?;
    let by_producer = owner
        .admit_number(&publication, PRODUCER, &required, rows)?
        .is_some();
    let by_another = owner
        .admit_number(
            &publication,
            crate::application::file_dag::relations::types::ProducerId(8),
            &required,
            rows,
        )?
        .is_some();
    owner.release_population();
    let next = owner.source_borrowed(relation, rows)?;
    owner.bind_producer(ProducerScope::File, PRODUCER, next.as_ref())?;
    let by_next = owner.admit_number(&next, PRODUCER, &required, rows)?.is_some();
    let by_recycled = owner
        .admit_number(&publication, PRODUCER, &required, rows)?
        .is_some();
    Ok([by_producer, by_another, by_next, by_recycled])
}

fn rebinding() -> Result<Rebinding, OperatingError> {
    use crate::application::file_dag::derived::owner::fixtures::resources::ResourceFixture;
    use ryo_values::domain::rows::types::Table;

    let fixture = ResourceFixture::new(&[], &[], 0).expect("a resource fixture");
    let owner = fixture.owner();
    let first = ProducerScope::Project(owner.project_identity()?);
    let foreign = ProducerScope::Project(owner.project_identity()?);
    let publication = owner.computed(Table::default())?;
    owner.bind_producer(first, PRODUCER, publication.as_ref())?;
    let identity = owner.result_population(publication.as_ref());
    owner.bind_producer(first, PRODUCER, publication.as_ref())?;
    let another = crate::application::file_dag::relations::types::ProducerId(8);
    let refused = owner
        .bind_producer(foreign, another, publication.as_ref())
        .err();
    let scope_kept = publication.as_ref().producer_scope.get() == Some(&first);
    let population_kept = owner.result_population(publication.as_ref()) == identity;
    let foreign_registered = owner
        .producer_populations
        .borrow()
        .contains_key(&ProducerKey {
            scope: foreign,
            producer: another,
        });
    let independent = owner.computed(Table::default())?;
    owner.bind_producer(foreign, another, independent.as_ref())?;
    Ok(Rebinding {
        foreign: refused,
        scope_kept,
        population_kept,
        foreign_registered,
        independent_own_population: owner.result_population(independent.as_ref()) != identity,
    })
}

fn released() -> Result<Released, OperatingError> {
    use crate::application::file_dag::derived::owner::{
        fixtures::resources::{ResourceFixture, weak_storage},
        service::ViewContext,
    };
    use ryo_execution_domain::domain::derived_views::types::{
        DerivedInputs, DerivedScratch, ViewRequest,
    };
    use ryo_relations::domain::relations::types::{Integer, Value};
    use ryo_values::domain::rows::types::{RowCount, Table};
    // This is a generic owned-row index contract, not a catalog-provider fixture.
    let fixture = ResourceFixture::new(&[Relation::Function], &[], 0).expect("a resource fixture");
    let owner = fixture.owner();
    let first_scope = ProducerScope::Project(owner.project_identity()?);
    let second_scope = ProducerScope::Project(owner.project_identity()?);
    let table = |number| Table {
        kinds: vec![CellKind::Number],
        count: RowCount(1),
        cells: vec![Value::Number(Integer(number))],
    };
    let first = owner.computed(table(1))?;
    let second = owner.computed(table(2))?;
    owner.bind_producer(first_scope, PRODUCER, first.as_ref())?;
    owner.bind_producer(second_scope, PRODUCER, second.as_ref())?;
    let distinct_populations =
        owner.result_population(first.as_ref()) != owner.result_population(second.as_ref());
    let acquire = |publication: &PublishedInput| {
        let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
        let proof = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
        let plan = Arc::new(owner.derived.prepare_view(
            &ViewRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            &[
                ryo_execution_domain::domain::operation_schema::types::InputShape {
                    kinds: ryo_execution_domain::domain::table_model::service::kinds_of(&rows),
                    tree: None,
                },
            ],
        )?);
        owner.derived.acquire_stored(
            &plan,
            &owner.derived.admit_view(&plan, &owner, &[&proof])?,
            ViewContext::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            None,
        )
    };
    owner.pin_derived(first.as_ref());
    owner.pin_derived(second.as_ref());
    let a = acquire(&first)?;
    let b = acquire(&second)?;
    let weak_a = weak_storage(&a);
    let weak_b = weak_storage(&b);
    drop((a, b));
    let alive_after_drop = [weak_a.upgrade().is_some(), weak_b.upgrade().is_some()];
    // Bind/reuse B before A's delayed retirement: release must carry A's key.
    let b = acquire(&second)?;
    owner.release_derived_producer(first_scope, PRODUCER);
    let alive_after_first_release = [weak_a.upgrade().is_some(), weak_b.upgrade().is_some()];
    drop(b);
    owner.release_derived_producer(second_scope, PRODUCER);
    let second_alive_after_its_release = weak_b.upgrade().is_some();
    drop((first, second));
    owner.release_population();
    Ok(Released {
        distinct_populations,
        alive_after_drop,
        alive_after_first_release,
        second_alive_after_its_release,
        working_bytes: owner.derived.working_bytes().0,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
