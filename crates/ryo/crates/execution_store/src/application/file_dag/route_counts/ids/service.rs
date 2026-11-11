//! The identities of the opt-in route accounting: each made from its raw value and read only here.

/// Text label of one route, family, counter, namespace or consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteLabel(pub(crate) &'static str);

impl From<&'static str> for RouteLabel {
    fn from(text: &'static str) -> Self {
        Self(text)
    }
}

/// The index of the rule a route is recorded under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteRuleIndex(pub(crate) usize);

impl From<usize> for RouteRuleIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

/// The program node a route is recorded under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteNodeIndex(pub(crate) usize);

impl From<usize> for RouteNodeIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

/// The index of the file a profile counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteFileIndex(pub(crate) usize);

impl From<usize> for RouteFileIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
