//! Complete graph answers are borrowed from one engine invocation, never from its cache.
use ryo_relations::domain::relations::types::Value;

/// A single source request for a logical closure join. The request contains
/// partition keys followed by the source; destinations retain typed sort order.
#[derive(Clone, Copy)]
pub(crate) struct ReachResult<'a> {
    pub request: &'a [Value],
    pub destinations: &'a [Value],
}

/// A single axis request after exact domain membership and nearest filtering.
/// Direct neighbors preserve physical multiplicity and encounter order.
#[derive(Clone, Copy)]
pub(crate) struct AxisResult<'a> {
    pub destinations: &'a [Value],
}

/// One finite, read-only result. Mutable generation cache handles stay in the
/// application before a primitive or downstream emitter receives this value.
#[derive(Clone, Copy)]
pub(crate) enum CachedGraphRow<'a> {
    Reach(ReachResult<'a>),
    Axis(AxisResult<'a>),
}
