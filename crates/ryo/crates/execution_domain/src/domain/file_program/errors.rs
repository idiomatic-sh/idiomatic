//! Request-graph named binding configuration refusals.
/// A configuration error, never an absent successful publication.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileProgramError {
    /// There are too many named roots or input bindings.
    #[error("too many named roots or input bindings")]
    TooManyOutputs,
    /// A named-root plan or an input reference is invalid.
    #[error("named-root plan or input reference is invalid")]
    InvalidReference,

    /// A rule binds a producer the request does not hold.
    #[error("rule `{consumer}` binds unknown producer `{producer}`")]
    UnknownProducer {
        /// The consuming rule.
        consumer: String,
        /// The producer it names.
        producer: String,
    },
    /// A rule declared without a query is executable.
    #[error("rule `{rule}` is executable but declared NoQuery")]
    ExecutableNoQuery {
        /// The rule.
        rule: String,
    },
    /// A rule depends on itself through an observation.
    #[error("rule `{rule}` depends on itself through an observation")]
    SelfDependency {
        /// The rule.
        rule: String,
    },
    /// The observation dependencies contain a cycle.
    #[error("observation dependencies contain a cycle")]
    Cycle,
    /// A producer does not export the output a rule binds.
    #[error("rule `{producer}` does not export `{output}`")]
    MissingOutput {
        /// The producing rule.
        producer: String,
        /// The output it does not export.
        output: String,
    },
    /// A rule binds a producer's output whose schema it cannot read.
    #[error("rule `{consumer}` binds incompatible schema from `{producer}` output `{output}`")]
    IncompatibleObservationColumns {
        /// The consuming rule.
        consumer: String,
        /// The producing rule.
        producer: String,
        /// The producer's output.
        output: String,
    },
    /// A rule repeats a bound input.
    #[error("rule `{rule}` repeats bound input `{name}`")]
    DuplicateBinding {
        /// The rule.
        rule: String,
        /// The repeated input.
        name: String,
    },
    /// A producer is not compiled before its consumer.
    #[error("producer `{rule}` is not compiled before its consumer")]
    NotCompiled {
        /// The producing rule.
        rule: String,
    },
}
