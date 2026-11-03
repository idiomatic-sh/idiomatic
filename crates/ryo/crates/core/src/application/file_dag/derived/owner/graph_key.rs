//! Immutable canonical map keys; live position compatibility is a separate lookup guard.
use super::*;
/// Selected positions are immutable. A Weak guard prevents address reuse while
/// retaining no position payload after the last semantic/consumer handle drops.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag::derived) struct GraphSelection {
    rows: Weak<Vec<u32>>,
    identity: Arc<crate::domain::table_model::types::SelectionIdentity>,
}
impl PartialEq for GraphSelection {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.rows, &other.rows) && self.identity == other.identity
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
    pub(in crate::application::file_dag::derived) fn new(
        rows: &Arc<Vec<u32>>,
        identity: &Arc<crate::domain::table_model::types::SelectionIdentity>,
    ) -> Self {
        Self {
            rows: Arc::downgrade(rows),
            identity: Arc::clone(identity),
        }
    }
}
impl GraphSelection {
    pub(super) fn compatible(&self, other: &Self) -> bool {
        if self.identity != other.identity {
            return false;
        }
        if Weak::ptr_eq(&self.rows, &other.rows) {
            return true;
        }
        let (Some(left), Some(right)) = (self.rows.upgrade(), other.rows.upgrade()) else {
            return false;
        };
        crate::application::file_dag::route_counts::event("graph_positions_compare", "graph");
        left == right
    }
}
fn same_selection(left: &Option<GraphSelection>, right: &Option<GraphSelection>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.compatible(right),
        _ => false,
    }
}
impl GraphRequirement {
    pub(super) fn compatible(&self, other: &Self) -> bool {
        self.endpoints == other.endpoints
            && self.keys == other.keys
            && same_selection(&self.selection, &other.selection)
    }
}
impl GraphViewIdentity {
    pub(super) fn compatible(&self, other: &Self) -> bool {
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
            ) => lc == rc && left.compatible(right) && same_selection(ls, rs),
            _ => self == other,
        }
    }
}
impl RequirementIdentity {
    pub(super) fn compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.compatible(right),
            _ => self == other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived) struct GraphRequirement {
    pub endpoints: [ryo_values::domain::plan_parts::types::ColumnIndex; 2],
    pub keys: Arc<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    pub selection: Option<GraphSelection>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived) enum GraphViewIdentity {
    Adjacency(GraphRequirement),
    Dense {
        graph: GraphRequirement,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
        selection: Option<GraphSelection>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag::derived) enum RequirementIdentity {
    Prepared(usize),
    Graph(Arc<GraphViewIdentity>),
}

impl GraphRequirement {
    fn same_header(&self, other: &Self) -> bool {
        let same = match (&self.selection, &other.selection) {
            (None, None) => true,
            (Some(left), Some(right)) => left.identity == right.identity,
            _ => false,
        };
        self.endpoints == other.endpoints && self.keys == other.keys && same
    }
}
impl GraphViewIdentity {
    fn same_header(&self, other: &Self) -> bool {
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
                lc == rc && left.same_header(right) && same
            }
            _ => false,
        }
    }
}
impl RequirementIdentity {
    pub(super) fn same_header(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Graph(left), Self::Graph(right)) => left.same_header(right),
            _ => self == other,
        }
    }
    pub(super) fn metadata_bytes(&self) -> usize {
        fn graph(value: &GraphRequirement) -> usize {
            value.keys.len() * size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>()
                + 2 * size_of::<usize>()
                + usize::from(value.selection.is_some())
                    * (size_of::<Vec<u32>>() + 2 * size_of::<usize>())
        }
        match self {
            Self::Prepared(_) => 0,
            Self::Graph(value) => {
                size_of::<GraphViewIdentity>()
                    + 2 * size_of::<usize>()
                    + match value.as_ref() {
                        GraphViewIdentity::Adjacency(value) => graph(value),
                        GraphViewIdentity::Dense {
                            graph: value,
                            selection,
                            ..
                        } => {
                            graph(value)
                                + usize::from(selection.is_some())
                                    * (size_of::<Vec<u32>>() + 2 * size_of::<usize>())
                        }
                    }
            }
        }
    }
}
