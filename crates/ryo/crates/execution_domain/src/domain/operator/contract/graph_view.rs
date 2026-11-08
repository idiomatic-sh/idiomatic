//! Complete graph answers are borrowed from one engine invocation, never from its cache.
use ryo_relations::domain::relations::types::Value;

/// A single source request for a logical closure join. The request contains
/// partition keys followed by the source; destinations retain typed sort order.
#[derive(Clone, Copy, Debug)]
pub struct ReachResult<'a> {
    /// The partition keys followed by the source the reach was asked for.
    pub request: &'a [Value],
    /// The destinations reached, in typed sort order.
    pub destinations: &'a [Value],
}

/// A single axis request after exact domain membership and nearest filtering.
/// Direct neighbors preserve physical multiplicity and encounter order.
#[derive(Clone, Copy, Debug)]
pub struct AxisResult<'a> {
    /// The direct neighbors, with their physical multiplicity and in encounter order.
    pub destinations: &'a [Value],
}

/// One finite, read-only result. Mutable generation cache handles stay in the
/// application before a primitive or downstream emitter receives this value.
#[derive(Clone, Copy, Debug)]
pub enum CachedGraphRow<'a> {
    /// A reach answer.
    Reach(ReachResult<'a>),
    /// An axis answer.
    Axis(AxisResult<'a>),
}
