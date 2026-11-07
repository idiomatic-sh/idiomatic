//! Immutable canonical map keys; live position compatibility is a separate lookup guard.
use super::types::IdentityCompatibility;
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::domain::derived_views::types::contract::value::DerivedContract;
use std::sync::{Arc, Weak};
/// The original row positions a selection kept, shared with its population: the bare positions inside are named here.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag) struct SelectionRows(pub Arc<Vec<u32>>);

/// A weak hold on selected row positions, which retains nothing: the bare positions inside are named here.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag::derived::owner) struct WeakSelectionRows(pub Weak<Vec<u32>>);

/// Selected positions are immutable. A Weak guard prevents address reuse while
/// retaining no position payload after the last semantic/consumer handle drops.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag::derived::owner) struct GraphSelection {
    rows: WeakSelectionRows,
    identity: Arc<crate::domain::stored_selection::types::SelectionIdentity>,
}
impl PartialEq for GraphSelection {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.rows.0, &other.rows.0) && self.identity == other.identity
    }
}
impl Eq for GraphSelection {}
impl std::hash::Hash for GraphSelection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Buffer content and liveness never participate in immutable map hashing.
        std::hash::Hash::hash(&0_u8, state);
    }
}
impl GraphSelection {
    pub(in crate::application::file_dag::derived::owner) fn new(
        rows: &SelectionRows,
        identity: &Arc<crate::domain::stored_selection::types::SelectionIdentity>,
    ) -> Self {
        Self {
            rows: WeakSelectionRows(Arc::downgrade(&rows.0)),
            identity: Arc::clone(identity),
        }
    }
}
impl GraphSelection {
    pub(super) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        if self.identity != other.identity {
            return IdentityCompatibility::Incompatible;
        }
        if Weak::ptr_eq(&self.rows.0, &other.rows.0) {
            return IdentityCompatibility::Compatible;
        }
        let (Some(left), Some(right)) = (self.rows.0.upgrade(), other.rows.0.upgrade()) else {
            return IdentityCompatibility::Incompatible;
        };
        crate::application::file_dag::route_counts::event(
            constants::GRAPH_POSITIONS_COMPARE,
            constants::ROUTE_GRAPH,
        );
        IdentityCompatibility::from(left == right)
    }
}
fn same_selection(
    left: &Option<GraphSelection>,
    right: &Option<GraphSelection>,
) -> IdentityCompatibility {
    match (left, right) {
        (None, None) => IdentityCompatibility::Compatible,
        (Some(left), Some(right)) => left.compatible(right),
        _ => IdentityCompatibility::Incompatible,
    }
}
impl GraphRequirement {
    pub(super) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        if self.endpoints == other.endpoints && self.keys == other.keys {
            same_selection(&self.selection, &other.selection)
        } else {
            IdentityCompatibility::Incompatible
        }
    }
}
impl GraphViewIdentity {
    pub(super) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Adjacency(left), Self::Adjacency(right)) => left.compatible(right),
            (
                Self::Dense {
                    graph: left,
                    column: lc,
                    selection: ls,
                },
                Self::Dense {
                    graph: right,
                    column: rc,
                    selection: rs,
                },
            ) => {
                if lc == rc
                    && left.compatible(right) == IdentityCompatibility::Compatible
                    && same_selection(ls, rs) == IdentityCompatibility::Compatible
                {
                    IdentityCompatibility::Compatible
                } else {
                    IdentityCompatibility::Incompatible
                }
            }
            _ => IdentityCompatibility::from(self == other),
        }
    }
}
impl RequirementIdentity {
    pub(super) fn prepared(plan: &Arc<DerivedContract>) -> Self {
        Self::Prepared(PreparedIdentity::new(plan))
    }
    pub(super) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.compatible(right),
            _ => IdentityCompatibility::from(self == other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived::owner) struct GraphRequirement {
    pub endpoints: [ryo_values::domain::plan_parts::types::ColumnIndex; 2],
    pub keys: Arc<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    pub selection: Option<GraphSelection>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived::owner) enum GraphViewIdentity {
    Adjacency(GraphRequirement),
    Dense {
        graph: GraphRequirement,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
        selection: Option<GraphSelection>,
    },
}
/// A retained immutable descriptor prevents address ABA and allows equivalent cross-rule plans.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag) struct PreparedIdentity(Arc<DerivedContract>);
impl PartialEq for PreparedIdentity {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = (&self.0, &other.0);
        Arc::ptr_eq(left, right)
            || (left.identity_hash() == right.identity_hash()
                && left.kind == right.kind
                && left.schemas == right.schemas
                && left.metadata == right.metadata
                && super::super::request::identity::types::StaticRequest(&left.request)
                    == super::super::request::identity::types::StaticRequest(&right.request))
    }
}
impl Eq for PreparedIdentity {}
impl std::hash::Hash for PreparedIdentity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0.identity_hash(), state);
    }
}
impl PreparedIdentity {
    pub(super) fn new(plan: &Arc<DerivedContract>) -> Self {
        Self(Arc::clone(plan))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived::owner) enum RequirementIdentity {
    Prepared(PreparedIdentity),
    Graph(Arc<GraphViewIdentity>),
}

impl GraphRequirement {
    fn same_header(&self, other: &Self) -> IdentityCompatibility {
        let same = match (&self.selection, &other.selection) {
            (None, None) => true,
            (Some(left), Some(right)) => left.identity == right.identity,
            _ => false,
        };
        IdentityCompatibility::from(
            self.endpoints == other.endpoints && self.keys == other.keys && same,
        )
    }
}
impl GraphViewIdentity {
    fn same_header(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Adjacency(left), Self::Adjacency(right)) => left.same_header(right),
            (
                Self::Dense {
                    graph: left,
                    column: lc,
                    selection: ls,
                },
                Self::Dense {
                    graph: right,
                    column: rc,
                    selection: rs,
                },
            ) => {
                let same = match (ls, rs) {
                    (None, None) => true,
                    (Some(left), Some(right)) => left.identity == right.identity,
                    _ => false,
                };
                if lc == rc && left.same_header(right) == IdentityCompatibility::Compatible && same
                {
                    IdentityCompatibility::Compatible
                } else {
                    IdentityCompatibility::Incompatible
                }
            }
            _ => IdentityCompatibility::Incompatible,
        }
    }
}
impl RequirementIdentity {
    pub(super) fn same_header(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.same_header(right),
            _ => IdentityCompatibility::from(self == other),
        }
    }
    pub(super) fn metadata_bytes(&self) -> ByteCount {
        fn graph(value: &GraphRequirement) -> ByteCount {
            ByteCount(
                value.keys.len() * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                    + 2 * size_of::<usize>()
                    + usize::from(value.selection.is_some())
                        * (size_of::<Vec<u32>>() + 2 * size_of::<usize>()),
            )
        }
        match self {
            Self::Prepared(_) => ByteCount(0),
            Self::Graph(value) => ByteCount(
                size_of::<GraphViewIdentity>()
                    + 2 * size_of::<usize>()
                    + match value.as_ref() {
                        GraphViewIdentity::Adjacency(value) => graph(value).0,
                        GraphViewIdentity::Dense {
                            graph: value,
                            selection,
                            ..
                        } => {
                            graph(value).0
                                + usize::from(selection.is_some())
                                    * (size_of::<Vec<u32>>() + 2 * size_of::<usize>())
                        }
                    },
            ),
        }
    }
}
