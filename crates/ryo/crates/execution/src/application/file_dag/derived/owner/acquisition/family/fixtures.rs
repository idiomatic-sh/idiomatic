//! Actual publications enter the production owner; no registry, payload factory or executor is copied.
use super::super::admission::service::AcquiredView;
use crate::application::file_dag::{
    closure_partitions::PartitionedClosure, predicates::BoundPredicate, relations::RelationOwner,
    testing::program_derived_port,
};
use crate::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch, MembershipSpace, ViewRequest},
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    operator::contract::{resources::DfsResources, types::PreparedPredicates, views::BorrowedRows},
    positions::service_test::{SMALL_SOURCE, SmallTree, small_tree},
    stored_selection::types::Selector,
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::{Rows, SelectionIdentity},
    table_texts::types::Texts,
};
use ryo_derived::domain::contract::types::{DerivedKind, DerivedView};
use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
use ryo_relations::domain::{
    relations::types::{CellKind, FileId, Integer, NodeId, Relation, Value},
    terms::types::NameText,
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, LiteralText},
    rows::types::{RowCount, Table},
};
use std::{rc::Rc, sync::Arc};

fn request(kind: DerivedKind) -> ViewRequest {
    let zero = ColumnIndex(0);
    match kind {
        DerivedKind::Equality => ViewRequest::Equality {
            columns: vec![zero],
        },
        DerivedKind::PredicateMatches => ViewRequest::PredicateMatches(Arc::new(Selector {
            relation: Relation::Function,
            identity: Arc::new(SelectionIdentity {
                comparisons: vec![],
                bindings: vec![],
            }),
            columns: vec![zero],
            distinct: false,
        })),
        DerivedKind::CandidatePostings => ViewRequest::CandidatePostings {
            column: zero,
            literals: vec![LiteralText("alpha".into()), LiteralText("beta".into())],
        },
        DerivedKind::DistinctRepresentatives => ViewRequest::DistinctRepresentatives {
            columns: vec![zero],
        },
        DerivedKind::Adjacency => ViewRequest::Adjacency {
            endpoints: [zero, ColumnIndex(1)],
            keys: vec![],
        },
        DerivedKind::Membership => ViewRequest::Membership { column: zero },
        DerivedKind::PartitionOrder => ViewRequest::PartitionOrder {
            keys: vec![ColumnIndex(2)],
        },
        DerivedKind::ClosureAdjacency => ViewRequest::ClosureAdjacency {
            source: zero,
            destination: ColumnIndex(1),
        },
        DerivedKind::CycleAdjacency => ViewRequest::CycleAdjacency {
            source: zero,
            destination: ColumnIndex(1),
        },
        DerivedKind::OrderedKeys => ViewRequest::OrderedKeys {
            left_on: vec![zero],
            right_on: vec![zero],
            left_order: ColumnIndex(2),
            right_order: ColumnIndex(2),
        },
        DerivedKind::OrderedDfs => ViewRequest::OrderedDfs(DfsResources {
            root: zero,
            source: zero,
            destination: ColumnIndex(1),
            order: ColumnIndex(2),
            edge: ColumnIndex(3),
            root_keys: vec![],
            edge_keys: vec![],
        }),
        DerivedKind::NativeLookup => ViewRequest::NativeLookup {
            left_on: vec![zero],
            right_on: vec![zero],
            view: vec![zero],
        },
        DerivedKind::RawKeyOrder => ViewRequest::RawKeyOrder { stride: 1 },
    }
}
fn observed(view: &AcquiredView) -> Result<DerivedKind, OperatingError> {
    Ok(match view.view()? {
        DerivedView::Equality(_) => DerivedKind::Equality,
        DerivedView::RowIds(_) => match view.contract().kind {
            DerivedKind::PredicateMatches => DerivedKind::PredicateMatches,
            DerivedKind::DistinctRepresentatives => DerivedKind::DistinctRepresentatives,
            _ => return Err(OperatingError::WrongOperation),
        },
        DerivedView::Postings(_) => DerivedKind::CandidatePostings,
        DerivedView::Adjacency(_) => DerivedKind::Adjacency,
        DerivedView::Membership(_) => DerivedKind::Membership,
        DerivedView::PartitionOrder(_) => DerivedKind::PartitionOrder,
        DerivedView::ClosureAdjacency(_) => DerivedKind::ClosureAdjacency,
        DerivedView::CycleAdjacency(_) => DerivedKind::CycleAdjacency,
        DerivedView::OrderedKeys(_) => DerivedKind::OrderedKeys,
        DerivedView::OrderedDfs(_) => DerivedKind::OrderedDfs,
        DerivedView::NativeLookup(_) => DerivedKind::NativeLookup,
        DerivedView::RawKeyOrder(_) => DerivedKind::RawKeyOrder,
    })
}
pub(in crate::application::file_dag) fn exercise(
    kind: DerivedKind,
) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(true, program_derived_port()));
    let actual = acquire(kind, Rc::clone(&owner))?;
    owner.release_population();
    assert_eq!(
        owner.derived.publication_ownership_bytes(),
        (0, 0, 0),
        "all actual leases retire with the owner generation"
    );
    Ok(actual)
}
fn acquire(kind: DerivedKind, owner: Rc<RelationOwner>) -> Result<DerivedKind, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut table = Table {
        kinds: vec![CellKind::Number; 4],
        cells: [1, 2, 0, 10, 2, 3, 1, 11]
            .map(|n| Value::Number(Integer(n)))
            .into(),
        count: RowCount(2),
    };
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
        let mut cursor = PartitionedClosure::prepare(
            &publication,
            first.rows(),
            &[ColumnIndex(2)],
            ColumnIndex(0),
            ColumnIndex(1),
            Rc::clone(&owner),
        )?;
        assert!(cursor.advance()?);
        assert_eq!(cursor.current()?.key(), &[Value::Number(Integer(0))]);
        assert!(
            cursor.advance()?,
            "the second actual partition rebuilds the scoped acquired view"
        );
        assert_eq!(cursor.current()?.key(), &[Value::Number(Integer(1))]);
        assert!(!cursor.advance()?);
        return Ok(DerivedKind::ClosureAdjacency);
    }
    if kind == DerivedKind::RawKeyOrder {
        return Err(OperatingError::WrongOperation);
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
        owner.computed(Table {
            kinds: vec![CellKind::Number; 4],
            cells: [1, 2, 0, 10, 2, 3, 1, 11]
                .map(|n| Value::Number(Integer(n)))
                .into(),
            count: RowCount(2),
        })?
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
    let plan = Arc::new(
        owner
            .derived
            .prepare_view(&request(kind), &shapes[..if pair { 2 } else { 1 }])?,
    );
    let proofs = [&first, &second];
    let admitted = owner
        .derived
        .admit_view(&plan, &owner, &proofs[..if pair { 2 } else { 1 }])?;
    let mut scratch = DerivedScratch::default();
    let predicates = PreparedPredicates {
        comparisons: vec![],
        kinds: rows[0].kinds().to_vec(),
    };
    let mut state = Default::default();
    let mut memo = Vec::new();
    let mut names = None;
    let mut predicate = BoundPredicate {
        program: &predicates,
        bindings: &[],
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        prepared: None,
        state: &mut state,
        memo: &mut memo,
        names: &mut names,
    };
    if kind == DerivedKind::OrderedDfs {
        let mut query = (&mut symbols).into();
        let view = owner.derived.acquire_stored(
            &plan,
            &admitted,
            Default::default(),
            DerivedInputs::Dfs {
                roots: rows[0],
                edges: rows[1],
                symbols: &mut query,
                source: SourceView(SMALL_SOURCE),
                paths: &[],
                arenas: &[],
            },
            &mut scratch,
            None,
        )?;
        return observed(&view);
    }
    let active = [LiteralText("alpha".into()), LiteralText("beta".into())];
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
    };
    let inputs = match kind {
        DerivedKind::PredicateMatches => DerivedInputs::Predicate {
            rows: rows[0],
            candidates: None,
            program: &mut predicate,
            symbols: texts.symbols,
        },
        DerivedKind::CandidatePostings => DerivedInputs::Postings {
            rows: rows[0],
            texts: &texts,
            active: &active,
        },
        DerivedKind::DistinctRepresentatives => DerivedInputs::Distinct {
            rows: rows[0],
            positions: &[0, 1],
        },
        DerivedKind::Membership => DerivedInputs::Membership {
            rows: rows[0],
            space: MembershipSpace::Values,
        },
        DerivedKind::OrderedKeys => DerivedInputs::Text {
            rows: &rows,
            texts: &texts,
        },

        _ => DerivedInputs::Rows(&rows[..if pair { 2 } else { 1 }]),
    };
    let view = owner.derived.acquire_stored(
        &plan,
        &admitted,
        Default::default(),
        inputs,
        &mut scratch,
        None,
    )?;
    observed(&view)
}
