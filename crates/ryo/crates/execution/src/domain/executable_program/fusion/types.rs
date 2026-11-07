//! The closed classification every compiled instruction receives before regions are formed.

/// An owner-issued identity that fusion must never erase: the producer keeps its own publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Barrier {
    /// The producer, or the instruction itself, acquires a derived view or counted resource.
    Derived,
    /// More than one reader (including a rule root) consumes the producer's publication.
    SharedConsumer,
    /// The producer is a rule's private diagnostic frontier root.
    Frontier,
    /// The producer is an authored observation source of its rule.
    Observation,
    /// The producer is a named observation output captured into the request's observed frontier.
    Capture,
    /// The producer is a rule's emitted root, handed to the caller.
    Handover,
    /// The instruction and its producer run in different partitions or phases.
    Partition,
    /// Another route keeps the producer's publication alive: ordered-existence or column fences.
    Lifetime,
    /// Fault or completeness order requires publication first: absence-sensitive or invalid producers.
    Fault,
}

/// Why an instruction can never continue a producer, whatever the surrounding program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Ineligible {
    /// It reads a relation, named observation, request parameter or host relation: it begins a region.
    Source,
    /// It admits more than one input; demand and fault order across inputs belongs to the engine.
    MultiInput,
    /// It needs its whole input: grouping, closure, cycles or word splitting.
    Whole,
    /// It computes per row and can fault or write derived or source text on a reached row.
    RowFault,
    /// It reshapes rows: kept columns or repeated-row removal.
    Reshapes,
}

/// How one instruction relates to the region of its producer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Fusibility {
    /// A direct continuation: it runs on the producer's borrowed row inside the producer's region.
    Fusable,
    /// A continuation by shape, held apart by an identity the producer must keep.
    Barrier(Barrier),
    /// Never a continuation.
    Excluded(Ineligible),
}

/// Whether equal instructions of different rules may compute once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegionSharing {
    /// The answer depends only on the operation and its inputs.
    Shareable,
    /// A binding, an effect or an identity belongs to one rule.
    RuleScoped,
}
