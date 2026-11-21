//! What the `follow` meaning reads and answers: entities and the pointers between them, of one key group. Plain data, no engine type.

/// An entity a pointer leaves or reaches, as the number the caller knows it by within one key group.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::follow::types::Entity;
/// assert_eq!(Entity(7), Entity(7));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity(pub u32);

/// One edge: from a source to a destination. In an answer, a source and a fixed point it reaches.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::follow::types::{Entity, Pointer};
/// let own = Pointer { source: Entity(3), destination: Entity(3) };
/// assert_eq!(own.source, own.destination);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pointer {
    /// Where the edge leaves from.
    pub source: Entity,
    /// Where it arrives.
    pub destination: Entity,
}

/// One key group's edges, sorted by source.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::follow::types::GroupPointers;
/// assert!(GroupPointers(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupPointers<'a>(pub &'a [Pointer]);

/// How many times the walks back of one key group read an edge.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::follow::types::EdgeVisits;
/// assert_eq!(EdgeVisits(0), EdgeVisits::default());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeVisits(pub usize);

/// The place of an entity among the sources of one key group, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourcePlace(pub(crate) usize);

/// One edge turned round: the place of the entity it arrives at, then of the one it leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Arrival {
    /// Where the edge arrives.
    pub(crate) at: SourcePlace,
    /// Where it leaves from.
    pub(crate) from: SourcePlace,
}
