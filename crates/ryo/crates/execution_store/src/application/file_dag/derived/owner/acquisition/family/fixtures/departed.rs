//! What a family's route answers: the family read or the refusal, what it saw on the way, the lease
//! its exit left charged, and the bytes the owner still charges once its generation retired.
use super::Leaked;
use crate::application::file_dag::closure_partitions::types::PartitionAdvance;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Value;
use std::borrow::Cow;

/// What a family's route observed on the way, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seen {
    /// The closure cursor built its next partition.
    Advanced,
    /// The closure cursor had no partition left.
    Exhausted,
    /// The key of the closure cursor's current partition.
    Key(Cow<'static, [Value]>),
    /// A leased view was read as this family.
    Read(DerivedKind),
    /// A step was refused with this error.
    Refused(OperatingError),
}

/// A family's route that ended in its view: the family read, what the route saw on the way, the
/// lease left charged at the exit, and the `(required, optional, unique)` bytes the owner still
/// charges once its generation retired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departed {
    /// The family the view was read as.
    pub kind: DerivedKind,
    /// What the route saw on the way.
    pub along: Cow<'static, [Seen]>,
    /// The lease the exit left charged; none when the exit released it or was not reached.
    pub leaked: Option<Leaked>,
    /// The bytes still charged after the generation retired.
    pub retired: (usize, usize, usize),
}

/// A family's route that the owner refused: the refusal, and what [`Departed`] reports besides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    /// The refusal the route ended with.
    pub error: OperatingError,
    /// What the route saw on the way.
    pub along: Cow<'static, [Seen]>,
    /// The lease the exit left charged; none when the exit released it or was not reached.
    pub leaked: Option<Leaked>,
    /// The bytes still charged after the generation retired.
    pub retired: (usize, usize, usize),
}

/// A route's result with what it observed.
pub(crate) fn departed(
    result: Result<DerivedKind, OperatingError>,
    along: Vec<Seen>,
    leaked: Option<Leaked>,
    retired: (usize, usize, usize),
) -> Result<Departed, Refused> {
    let along: Cow<'static, [Seen]> = Cow::Owned(along);
    result
        .map(|kind| Departed {
            kind,
            along: along.clone(),
            leaked,
            retired,
        })
        .map_err(|error| Refused {
            error,
            along,
            leaked,
            retired,
        })
}

/// What a cursor advance answered.
pub(crate) fn advanced(advance: Result<PartitionAdvance, OperatingError>) -> Seen {
    match advance {
        Ok(PartitionAdvance::Advanced) => Seen::Advanced,
        Ok(PartitionAdvance::Exhausted) => Seen::Exhausted,
        Err(error) => Seen::Refused(error),
    }
}

/// What reading a demanded view answered.
pub(crate) fn read(view: Result<DerivedKind, OperatingError>) -> Seen {
    view.map_or_else(Seen::Refused, Seen::Read)
}
