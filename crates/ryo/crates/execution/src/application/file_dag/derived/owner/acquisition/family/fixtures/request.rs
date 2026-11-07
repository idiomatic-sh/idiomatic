//! The request each family is prepared from, and the family a leased view reports.
use super::super::super::admission::service::AcquiredView;
use crate::domain::{
    derived_views::types::ViewRequest, operating::errors::OperatingError,
    operator::contract::resources::DfsResources,
    stored_selection::types::{SelectionIdentity, Selector},
};
use ryo_derived::domain::contract::types::{DerivedKind, DerivedView};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use std::sync::Arc;

pub(super) fn request(kind: DerivedKind) -> ViewRequest {
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
pub(super) fn observed(view: &AcquiredView) -> Result<DerivedKind, OperatingError> {
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
