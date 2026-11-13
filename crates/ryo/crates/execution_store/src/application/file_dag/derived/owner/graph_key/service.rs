//! The `graph_key` application operation.

use super::super::types::IdentityCompatibility;
use crate::application::file_dag::accounting::ledger::types::ByteCount;
use ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract;
use std::sync::Arc;

impl GraphRequirement {
    pub(crate) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        IdentityCompatibility::from(self.endpoints == other.endpoints && self.keys == other.keys)
    }
}

impl GraphViewIdentity {
    pub(crate) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Adjacency(left), Self::Adjacency(right)) => left.compatible(right),
            (
                Self::Dense {
                    graph: left,
                    column: lc,
                },
                Self::Dense {
                    graph: right,
                    column: rc,
                },
            ) => {
                if lc == rc && left.compatible(right) == IdentityCompatibility::Compatible {
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
    pub(crate) fn prepared(plan: &Arc<DerivedContract>) -> Self {
        Self::Prepared(PreparedIdentity::new(plan))
    }
    pub(crate) fn compatible(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.compatible(right),
            _ => IdentityCompatibility::from(self == other),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GraphRequirement {
    pub endpoints: [ryo_values::domain::plan_parts::types::ColumnIndex; 2],
    pub keys: Arc<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GraphViewIdentity {
    Adjacency(GraphRequirement),
    Dense {
        graph: GraphRequirement,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
}

/// A retained immutable descriptor prevents address ABA and allows equivalent cross-rule plans.
#[derive(Clone, Debug)]
pub(crate) struct PreparedIdentity(Arc<DerivedContract>);

impl PartialEq for PreparedIdentity {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = (&self.0, &other.0);
        Arc::ptr_eq(left, right)
            || (left.identity_hash() == right.identity_hash()
                && left.kind == right.kind
                && left.schemas == right.schemas
                && left.metadata == right.metadata
                && super::super::super::request::identity::types::StaticRequest(&left.request)
                    == super::super::super::request::identity::types::StaticRequest(&right.request))
    }
}

impl Eq for PreparedIdentity {}

impl std::hash::Hash for PreparedIdentity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.0.identity_hash(), state);
    }
}

impl PreparedIdentity {
    pub(crate) fn new(plan: &Arc<DerivedContract>) -> Self {
        Self(Arc::clone(plan))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RequirementIdentity {
    Prepared(PreparedIdentity),
    Graph(Arc<GraphViewIdentity>),
}

impl GraphRequirement {
    fn same_header(&self, other: &Self) -> IdentityCompatibility {
        IdentityCompatibility::from(self.endpoints == other.endpoints && self.keys == other.keys)
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
                },
                Self::Dense {
                    graph: right,
                    column: rc,
                },
            ) => {
                if lc == rc && left.same_header(right) == IdentityCompatibility::Compatible {
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
    pub(crate) fn same_header(&self, other: &Self) -> IdentityCompatibility {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.same_header(right),
            _ => IdentityCompatibility::from(self == other),
        }
    }
    pub(crate) fn metadata_bytes(&self) -> ByteCount {
        fn graph(value: &GraphRequirement) -> ByteCount {
            ByteCount(
                value.keys.len() * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                    + 2 * size_of::<usize>(),
            )
        }
        match self {
            Self::Prepared(_) => ByteCount(0),
            Self::Graph(value) => ByteCount(
                size_of::<GraphViewIdentity>()
                    + 2 * size_of::<usize>()
                    + match value.as_ref() {
                        GraphViewIdentity::Adjacency(value) => graph(value).0,
                        GraphViewIdentity::Dense { graph: value, .. } => graph(value).0,
                    },
            ),
        }
    }
}
