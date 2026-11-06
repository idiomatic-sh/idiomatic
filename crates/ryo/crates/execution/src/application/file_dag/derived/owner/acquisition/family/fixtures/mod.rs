//! Actual publications enter the production owner; no registry, payload factory or executor is copied.
mod demand;
mod request;
use self::demand::demand;
use self::request::{observed, request};
use crate::application::file_dag::{
    closure_partitions::PartitionedClosure, relations::RelationOwner, testing::program_derived_port,
};
use crate::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    operator::contract::views::BorrowedRows,
    positions::service_test::{SMALL_SOURCE, SmallTree, small_tree},
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::Rows,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
use ryo_relations::domain::{
    relations::types::{CellKind, FileId, Integer, NodeId, Relation, Value},
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::{rc::Rc, sync::Arc};

/// How the last consumer of one family leaves: each scenario ends with the owner generation
/// retired, and `exercise` asserts that no required, optional or unique byte remains charged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Scenario {
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
pub(in crate::application::file_dag) fn exercise(
    kind: DerivedKind,
    scenario: Scenario,
) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(true, program_derived_port()));
    let actual = acquire(kind, scenario, Rc::clone(&owner));
    owner.release_population();
    assert_eq!(
        owner.derived.publication_ownership_bytes(),
        (0, 0, 0),
        "{kind:?} {scenario:?}: every lease retires with the owner generation"
    );
    actual
}
fn acquire(
    kind: DerivedKind,
    scenario: Scenario,
    owner: Rc<RelationOwner>,
) -> Result<DerivedKind, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut table = numbers();
    if kind == DerivedKind::CandidatePostings {
        table.kinds[0] = CellKind::Symbol;
        table.cells[0] = Value::Symbol(intern_derived(
            &mut symbols,
            ByteSlice(SMALL_SOURCE),
            NameText("alpha"),
        ));
        table.cells[4] = Value::Symbol(intern_derived(
            &mut symbols,
            ByteSlice(SMALL_SOURCE),
            NameText("beta"),
        ));
    }
    if kind == DerivedKind::NativeLookup {
        table.kinds[0] = CellKind::Node;
        table.cells[0] = Value::Node(NodeId(0));
        table.cells[4] = Value::Node(NodeId(1));
    }
    let publication = owner.computed(table)?;
    let rows = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let first = owner.admit(publication.as_ref(), BorrowedRows::from(&rows))?;
    if kind == DerivedKind::ClosureAdjacency {
        let keys = [ColumnIndex(2)];
        let (source, destination) = (ColumnIndex(0), ColumnIndex(1));
        if matches!(
            scenario,
            Scenario::Succeeds | Scenario::Cancelled | Scenario::Held
        ) {
            let mut cursor = PartitionedClosure::prepare(
                &publication,
                first.rows(),
                &keys,
                source,
                destination,
                Rc::clone(&owner),
            )?;
            assert!(cursor.advance()?);
            assert_eq!(cursor.current()?.key(), &[Value::Number(Integer(0))]);
            match scenario {
                Scenario::Cancelled => {}
                Scenario::Held => {
                    owner.release_population();
                    assert_eq!(
                        cursor.current()?.key(),
                        &[Value::Number(Integer(0))],
                        "the held partition outlives the retired generation"
                    );
                    assert!(
                        matches!(cursor.advance(), Err(OperatingError::NotRead)),
                        "the next partition is a new demand on a retired generation"
                    );
                }
                _ => {
                    assert!(
                        cursor.advance()?,
                        "the second actual partition rebuilds the scoped acquired view"
                    );
                    assert_eq!(cursor.current()?.key(), &[Value::Number(Integer(1))]);
                    assert!(!cursor.advance()?);
                }
            }
            return Ok(DerivedKind::ClosureAdjacency);
        }
        let schema = [InputShape {
            kinds: first.rows().kinds(),
            tree: None,
        }];
        let partition_plan = Arc::new(owner.derived.prepare_view(
            &ViewRequest::PartitionOrder {
                keys: keys.to_vec(),
            },
            &schema,
        )?);
        let closure_plan = Arc::new(owner.derived.prepare_view(
            &ViewRequest::ClosureAdjacency {
                source,
                destination,
            },
            &schema,
        )?);
        let admitted = owner
            .derived
            .admit_view(&partition_plan, &owner, &[&first])?;
        let ordered_plan = if scenario == Scenario::RetargetedDescriptor {
            Arc::new((*partition_plan).clone())
        } else {
            Arc::clone(&partition_plan)
        };
        let ordered = owner.derived.acquire_stored(
            &ordered_plan,
            &admitted,
            Default::default(),
            DerivedInputs::Rows(&[first.rows()]),
            &mut DerivedScratch::default(),
            None,
        )?;
        let other = owner.computed(numbers())?;
        let resolved = owner.resolve(other.as_ref(), &|_| Err(OperatingError::NotRead))?;
        let second = owner.admit(other.as_ref(), BorrowedRows::from(&resolved))?;
        if scenario == Scenario::Stale {
            owner.release_population();
        }
        let (plan, proof) = match scenario {
            Scenario::OtherPlan => (&partition_plan, &first),
            Scenario::OtherPopulation => (&closure_plan, &second),
            _ => (&closure_plan, &first),
        };
        return owner
            .derived
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
        owner.admit(other.as_ref(), BorrowedRows::Tree(tree))?
    } else {
        resolved = owner.resolve(other.as_ref(), &|_| Err(OperatingError::NotRead))?;
        owner.admit(other.as_ref(), BorrowedRows::from(&resolved))?
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
    let plan = Arc::new(owner.derived.prepare_view(&request(kind), shapes)?);
    let proofs = [&first, &second];
    let admitted = owner
        .derived
        .admit_view(&plan, &owner, &proofs[..shapes.len()])?;
    let presented = match scenario {
        Scenario::OtherPlan => Arc::new(owner.derived.prepare_view(&request(kind), shapes)?),
        Scenario::RetargetedDescriptor => Arc::new((*plan).clone()),
        _ => Arc::clone(&plan),
    };
    let presented_rows = if scenario == Scenario::OtherPopulation {
        [rows[1], rows[0]]
    } else {
        rows
    };
    match scenario {
        Scenario::Cancelled => return Ok(kind),
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
    let view = demand(
        kind,
        &owner,
        &presented,
        &admitted,
        presented_rows,
        &mut symbols,
    )?;
    observed(&view)
}
