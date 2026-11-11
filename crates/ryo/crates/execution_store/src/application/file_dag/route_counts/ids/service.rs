//! The identities of the opt-in route accounting: each made from its raw value and read only here.

/// Text label of one route, family, counter, namespace or consumer.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::ids::service::RouteLabel;
/// assert_eq!(RouteLabel::from("owner"), RouteLabel::from("owner"));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteLabel(pub(crate) &'static str);

impl From<&'static str> for RouteLabel {
    fn from(text: &'static str) -> Self {
        Self(text)
    }
}

/// The index of the rule a route is recorded under.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::ids::service::RouteRuleIndex;
/// assert_eq!(RouteRuleIndex::from(3), RouteRuleIndex::from(3));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteRuleIndex(pub(crate) usize);

impl From<usize> for RouteRuleIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

/// The program node a route is recorded under.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::ids::service::RouteNodeIndex;
/// assert!(RouteNodeIndex::from(1) < RouteNodeIndex::from(2));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteNodeIndex(pub(crate) usize);

impl From<usize> for RouteNodeIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

/// The index of the file a profile counts.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::ids::service::RouteFileIndex;
/// assert_eq!(RouteFileIndex::from(0), RouteFileIndex::from(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteFileIndex(pub(crate) usize);

impl From<usize> for RouteFileIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
