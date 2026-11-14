//! Actual publications enter the production owner; no registry, payload factory or executor is copied.
mod demand;
pub mod departed;
mod refusing;
mod request;
pub mod route;
use self::demand::demand;
use self::departed::{Departed, Leaked, Refused, Seen, advanced, departed, read};
use self::request::{observed, request};
use crate::application::file_dag::relations::publication::types::ProducerScope;
use crate::application::file_dag::relations::types::ProducerId;
use crate::application::file_dag::{
    fixtures::program_derived_port, relations::service::RelationOwner,
};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    table_cells::fixtures::{SmallTree, small_tree},
    table_model::types::Rows,
};
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, NodeId, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};

/// How the last consumer of one family leaves: each departure ends with the owner generation
/// retired, and `exercise` answers what the owner still charges then.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Departure {
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
/// Who releases the pins at the last consumer's exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Release {
    /// The production exit: `RelationOwner::release_derived_producer`, as cancellation runs it.
    Production,
}
/// Runs one acquisition family's departure through the production exit and answers what it observed.
///
/// # Errors
///
/// The production refusal the departure drives the family into, or the first refusal of its setup,
/// with what the route observed.
pub fn exercise(kind: DerivedKind, departure: Departure) -> Result<Departed, Refused> {
    drive(kind, departure, Release::Production)
}
fn drive(kind: DerivedKind, departure: Departure, release: Release) -> Result<Departed, Refused> {
    let port: Arc<dyn crate::outbound::derived::interface::DerivedPort> =
        if departure == Departure::BuildFails {
            Arc::new(refusing::RefusingBuild)
        } else {
            program_derived_port()
        };
    let owner = Rc::new(RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        port,
    ));
    let mut leaked = None;
    let mut along = Vec::new();
    let actual = acquire(
        kind,
        departure,
        Rc::clone(&owner),
        release,
        (&mut leaked, &mut along),
    );
    owner.release_population();
    departed(
        actual,
        along,
        leaked,
        owner.derived().publication_ownership_bytes(),
    )
}
fn acquire(
    kind: DerivedKind,
    departure: Departure,
    owner: Rc<RelationOwner>,
    release: Release,
    (leaked, along): (&mut Option<Leaked>, &mut Vec<Seen>),
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
            departure,
            Departure::Succeeds | Departure::Cancelled | Departure::Held
        ) {
            let mut cursor = crate::application::file_dag::closure_partitions::service::prepare(
                &crate::application::file_dag::relations::handle::service::Publication::of(
                    publication.clone(),
                ),
                first.rows(),
                &keys,
                source,
                destination,
                Rc::clone(&owner),
            )?;
            along.push(advanced(cursor.advance()));
            along.push(Seen::Key(Cow::Owned(cursor.current()?.key.to_vec())));
            match departure {
                Departure::Cancelled => {}
                Departure::Held => {
                    // The held partition outlives the retired generation; the next partition is a
                    // new demand on it.
                    owner.release_population();
                    along.push(Seen::Key(Cow::Owned(cursor.current()?.key.to_vec())));
                    along.push(advanced(cursor.advance()));
                }
                Departure::Succeeds
                | Departure::Stale
                | Departure::OtherPlan
                | Departure::OtherPopulation
                | Departure::RetargetedDescriptor
                | Departure::BuildFails => {
                    // The second actual partition rebuilds the scoped acquired view.
                    along.push(advanced(cursor.advance()));
                    along.push(Seen::Key(Cow::Owned(cursor.current()?.key.to_vec())));
                    along.push(advanced(cursor.advance()));
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
        let ordered_plan = if departure == Departure::RetargetedDescriptor {
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
        if departure == Departure::Stale {
            owner.release_population();
        }
        let (plan, proof) = match departure {
            Departure::OtherPlan => (&partition_plan, &first),
            Departure::OtherPopulation => (&closure_plan, &second),
            Departure::Succeeds
            | Departure::Cancelled
            | Departure::Held
            | Departure::Stale
            | Departure::RetargetedDescriptor
            | Departure::BuildFails => (&closure_plan, &first),
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
    let presented = match departure {
        Departure::OtherPlan => Arc::new(owner.derived().prepare_view(&request(kind), shapes)?),
        Departure::RetargetedDescriptor => Arc::new((*plan).clone()),
        Departure::Succeeds
        | Departure::Cancelled
        | Departure::Held
        | Departure::Stale
        | Departure::OtherPopulation
        | Departure::BuildFails => Arc::clone(&plan),
    };
    let presented_rows = if departure == Departure::OtherPopulation {
        [rows[1], rows[0]]
    } else {
        rows
    };
    let producers = [
        (ProducerScope::File, ProducerId(0)),
        (ProducerScope::File, ProducerId(1)),
    ];
    owner.bind_producer(producers[0].0, producers[0].1, publication.as_ref())?;
    owner.bind_producer(producers[1].0, producers[1].1, other.as_ref())?;
    let baseline = owner.derived().publication_ownership_bytes().0;
    owner.pin_derived(publication.as_ref());
    owner.pin_derived(other.as_ref());
    // A real lease exists before every exit that is not a success.
    let held = if matches!(
        departure,
        Departure::Cancelled
            | Departure::Stale
            | Departure::OtherPlan
            | Departure::OtherPopulation
            | Departure::RetargetedDescriptor
    ) {
        Some(demand(kind, &owner, &plan, &admitted, rows, &mut symbols)?)
    } else {
        None
    };
    match departure {
        Departure::Cancelled => {
            drop(held);
            *leaked = leave(&owner, producers, baseline, release).err();
            return Ok(kind);
        }
        Departure::Stale => owner.release_population(),
        Departure::Held => {
            // A held lease stays readable after retirement; a new demand on the retired
            // generation refuses.
            let held = demand(kind, &owner, &plan, &admitted, rows, &mut symbols)?;
            owner.release_population();
            along.push(read(observed(&held)));
            let again = demand(kind, &owner, &plan, &admitted, rows, &mut symbols);
            along.push(read(again.and_then(|view| observed(&view))));
            return Ok(kind);
        }
        Departure::Succeeds
        | Departure::OtherPlan
        | Departure::OtherPopulation
        | Departure::RetargetedDescriptor
        | Departure::BuildFails => {}
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
    if departure != Departure::Stale {
        *leaked = leave(&owner, producers, baseline, release).err();
    }
    result
}
/// The last consumer's exit: the producer release of the relation owner runs while the generation is still
/// live, and nothing here releases a population. The required bytes must be what they were
/// before the view was acquired; otherwise the exit leaked its lease.
fn leave(
    owner: &RelationOwner,
    producers: [(ProducerScope, ProducerId); 2],
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
