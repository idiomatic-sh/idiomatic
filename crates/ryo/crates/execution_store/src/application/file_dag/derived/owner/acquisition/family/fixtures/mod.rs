//! Actual publications enter the production owner; no registry, payload factory or executor is copied.
mod demand;
mod refusing;
mod request;
pub mod route;
use self::demand::demand;
use self::request::{observed, request};
use crate::application::file_dag::relations::publication::types::ProducerScope;
use crate::application::file_dag::{fixtures::program_derived_port, relations::RelationOwner};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    table_cells::fixtures::{SmallTree, small_tree},
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, NodeId, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::{rc::Rc, sync::Arc};

/// How the last consumer of one family leaves: each scenario ends with the owner generation
/// retired, and `exercise` asserts that no required, optional or unique byte remains charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// The view is acquired, read and dropped.
    Succeeds,
    /// The demand is withdrawn after admission and before any acquisition.
    Cancelled,
    /// A lease taken before the generation retires stays readable; a new demand refuses.
    Held,
    /// The owner generation retired between admission and acquisition.
    Stale,
    /// A separately prepared descriptor for the same request.
    OtherPlan,
    /// Rows of another published population than the one admitted.
    OtherPopulation,
    /// A copy of the admitted descriptor, equal in value but not the issued one.
    RetargetedDescriptor,
    /// The semantic port refuses to build: no lease may stay charged.
    BuildFails,
}
fn numbers() -> Table {
    Table {
        kinds: vec![CellKind::Number; 4],
        cells: [1, 2, 0, 10, 2, 3, 1, 11]
            .map(|n| Value::Number(Integer(n)))
            .into(),
        count: RowCount(2),
    }
}
/// A production exit that returned with its lease still charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leaked {
    /// Required bytes before the view was acquired.
    pub before: usize,
    /// Required bytes after the exit ran, before any fixture-side release.
    pub after: usize,
}
/// Who releases the pins at the last consumer's exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Release {
    /// The production exit: `RelationOwner::release_derived_producer`, as cancellation runs it.
    Production,
    /// A fake exit that releases nothing; the control that proves a leak is detected.
    Forgetful,
}
/// Runs one acquisition family's scenario and answers the kind it acquired.
pub fn exercise(kind: DerivedKind, scenario: Scenario) -> Result<DerivedKind, OperatingError> {
    let (actual, leaked) = drive(kind, scenario, Release::Production);
    assert_eq!(
        leaked, None,
        "{kind:?} {scenario:?}: the production exit itself releases the lease, before the generation retires"
    );
    actual
}
/// The negative control: the same exit with a release that never happens must be observed as a
/// leak by the ledger before the fixture retires the generation. `None` means the exit was not
/// reached (a scenario that retires the generation first) or nothing stayed charged.
pub fn exercise_forgetful(kind: DerivedKind, scenario: Scenario) -> Option<Leaked> {
    drive(kind, scenario, Release::Forgetful).1
}
fn drive(
    kind: DerivedKind,
    scenario: Scenario,
    release: Release,
) -> (Result<DerivedKind, OperatingError>, Option<Leaked>) {
    let port = if scenario == Scenario::BuildFails {
        Arc::new(refusing::RefusingBuild)
            as Arc<dyn crate::outbound::derived::interface::DerivedPort>
    } else {
        program_derived_port()
    };
    let owner = Rc::new(RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        port,
    ));
    let mut leaked = None;
    let actual = acquire(kind, scenario, Rc::clone(&owner), release, &mut leaked);
    owner.release_population();
    assert_eq!(
        owner.derived().publication_ownership_bytes(),
        (0, 0, 0),
        "{kind:?} {scenario:?}: every lease retires with the owner generation"
    );
    (actual, leaked)
}
fn acquire(
    kind: DerivedKind,
    scenario: Scenario,
    owner: Rc<RelationOwner>,
    release: Release,
    leaked: &mut Option<Leaked>,
) -> Result<DerivedKind, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut table = numbers();
    if kind == DerivedKind::NativeLookup {
        table.kinds[0] = CellKind::Node;
        table.cells[0] = Value::Node(NodeId(0));
        table.cells[4] = Value::Node(NodeId(1));
    }
    let publication = owner.computed(table)?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let first = owner.admit(publication.as_ref(), RepresentationInput::from(&rows))?;
    if kind == DerivedKind::ClosureAdjacency {
        let keys = [ColumnIndex(2)];
        let (source, destination) = (ColumnIndex(0), ColumnIndex(1));
        if matches!(
            scenario,
            Scenario::Succeeds | Scenario::Cancelled | Scenario::Held
        ) {
            let mut cursor = crate::application::file_dag::closure_partitions::schemas::prepare(
                &crate::application::file_dag::relations::handle::service::Publication::of(publication.clone()),
                first.rows(),
                &keys,
                source,
                destination,
                Rc::clone(&owner),
            )?;
            assert_eq!(
                cursor.advance()?,
                crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced
            );
            assert_eq!(cursor.current()?.key, &[Value::Number(Integer(0))]);
            match scenario {
                Scenario::Cancelled => {}
                Scenario::Held => {
                    owner.release_population();
                    assert_eq!(
                        cursor.current()?.key,
                        &[Value::Number(Integer(0))],
                        "the held partition outlives the retired generation"
                    );
                    assert!(
                        matches!(cursor.advance(), Err(OperatingError::NotRead)),
                        "the next partition is a new demand on a retired generation"
                    );
                }
                _ => {
                    assert_eq!(cursor.advance()?, crate::application::file_dag::closure_partitions::types::PartitionAdvance::Advanced, "the second actual partition rebuilds the scoped acquired view");
                    assert_eq!(cursor.current()?.key, &[Value::Number(Integer(1))]);
                    assert_eq!(cursor.advance()?, crate::application::file_dag::closure_partitions::types::PartitionAdvance::Exhausted);
                }
            }
            return Ok(DerivedKind::ClosureAdjacency);
        }
        let schema = [InputShape {
            kinds: first.rows().kinds(),
            tree: None,
        }];
        let partition_plan = Arc::new(owner.derived().prepare_view(
            &ViewRequest::PartitionOrder {
                keys: keys.to_vec(),
            },
            &schema,
        )?);
        let closure_plan = Arc::new(owner.derived().prepare_view(
            &ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            &schema,
        )?);
        let admitted = owner
            .derived()
            .admit_view(&partition_plan, &owner, &[&first])?;
        let ordered_plan = if scenario == Scenario::RetargetedDescriptor {
            Arc::new((*partition_plan).clone())
        } else {
            Arc::clone(&partition_plan)
        };
        let ordered = owner.derived().acquire_stored(
            &ordered_plan,
            &admitted,
            Default::default(),
            DerivedInputs::Rows(&[first.rows()]),
            &mut DerivedScratch::default(),
            None,
        )?;
        let other = owner.computed(numbers())?;
        let resolved = owner.resolve(other.as_ref(), &|_| Err(OperatingError::NotRead))?;
        let second = owner.admit(other.as_ref(), RepresentationInput::from(&resolved))?;
        if scenario == Scenario::Stale {
            owner.release_population();
        }
        let (plan, proof) = match scenario {
            Scenario::OtherPlan => (&partition_plan, &first),
            Scenario::OtherPopulation => (&closure_plan, &second),
            _ => (&closure_plan, &first),
        };
        return owner
            .derived()
            .admit_partition(plan, &owner, proof, &ordered)
            .map(|_| DerivedKind::ClosureAdjacency);
    }
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let tree_rows = Rows::Tree(tree);
    let other = if kind == DerivedKind::NativeLookup {
        owner.source(Relation::SyntaxTree, &tree_rows)?
    } else {
        owner.computed(numbers())?
    };
    let resolved;
    let second = if kind == DerivedKind::NativeLookup {
        owner.admit(other.as_ref(), RepresentationInput::Tree(tree))?
    } else {
        resolved = owner.resolve(other.as_ref(), &|_| Err(OperatingError::NotRead))?;
        owner.admit(other.as_ref(), RepresentationInput::from(&resolved))?
    };
    let pair = matches!(
        kind,
        DerivedKind::OrderedKeys | DerivedKind::OrderedDfs | DerivedKind::NativeLookup
    );
    let rows = [first.rows(), second.rows()];
    let shapes = [
        InputShape {
            kinds: rows[0].kinds(),
            tree: None,
        },
        InputShape {
            kinds: rows[1].kinds(),
            tree: (kind == DerivedKind::NativeLookup).then_some(TreeForm::Elements),
        },
    ];
    let shapes = &shapes[..if pair { 2 } else { 1 }];
    let plan = Arc::new(owner.derived().prepare_view(&request(kind), shapes)?);
    let proofs = [&first, &second];
    let admitted = owner
        .derived()
        .admit_view(&plan, &owner, &proofs[..shapes.len()])?;
    let presented = match scenario {
        Scenario::OtherPlan => Arc::new(owner.derived().prepare_view(&request(kind), shapes)?),
        Scenario::RetargetedDescriptor => Arc::new((*plan).clone()),
        _ => Arc::clone(&plan),
    };
    let presented_rows = if scenario == Scenario::OtherPopulation {
        [rows[1], rows[0]]
    } else {
        rows
    };
    let producers = [
        (ProducerScope::File, RegionId(0)),
        (ProducerScope::File, RegionId(1)),
    ];
    owner.bind_producer(producers[0].0, producers[0].1, publication.as_ref())?;
    owner.bind_producer(producers[1].0, producers[1].1, other.as_ref())?;
    let baseline = owner.derived().publication_ownership_bytes().0;
    owner.pin_derived(publication.as_ref());
    owner.pin_derived(other.as_ref());
    // A real lease exists before every exit that is not a success.
    let held = if matches!(
        scenario,
        Scenario::Cancelled
            | Scenario::Stale
            | Scenario::OtherPlan
            | Scenario::OtherPopulation
            | Scenario::RetargetedDescriptor
    ) {
        Some(demand(kind, &owner, &plan, &admitted, rows, &mut symbols)?)
    } else {
        None
    };
    match scenario {
        Scenario::Cancelled => {
            drop(held);
            *leaked = leave(&owner, producers, baseline, release).err();
            return Ok(kind);
        }
        Scenario::Stale => owner.release_population(),
        Scenario::Held => {
            let held = demand(kind, &owner, &plan, &admitted, rows, &mut symbols)?;
            owner.release_population();
            assert_eq!(
                observed(&held)?,
                kind,
                "{kind:?}: a held lease stays readable after retirement"
            );
            let again = demand(kind, &owner, &plan, &admitted, rows, &mut symbols);
            assert!(
                matches!(again, Err(OperatingError::NotRead)),
                "{kind:?}: a new demand on the retired generation refuses, got {again:?}"
            );
            return Ok(kind);
        }
        _ => {}
    }
    let result = demand(
        kind,
        &owner,
        &presented,
        &admitted,
        presented_rows,
        &mut symbols,
    )
    .and_then(|view| observed(&view));
    drop(held);
    if scenario != Scenario::Stale {
        *leaked = leave(&owner, producers, baseline, release).err();
    }
    result
}
/// The last consumer's exit: the producer release of the relation owner runs while the generation is still
/// live, and nothing here releases a population. The required bytes must be what they were
/// before the view was acquired; otherwise the exit leaked its lease.
fn leave(
    owner: &RelationOwner,
    producers: [(ProducerScope, RegionId); 2],
    baseline: usize,
    release: Release,
) -> Result<(), Leaked> {
    if release == Release::Production {
        for (scope, node) in producers {
            owner.release_derived_producer(scope, node);
        }
    }
    let after = owner.derived().publication_ownership_bytes().0;
    if after == baseline {
        Ok(())
    } else {
        Err(Leaked {
            before: baseline,
            after,
        })
    }
}
