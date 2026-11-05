//! Request-graph named binding configuration refusals.
/// A configuration error, never an absent successful publication.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileProgramError {
    #[error("too many named roots or input bindings")]
    TooManyOutputs,
    #[error("named-root plan or input reference is invalid")]
    InvalidReference,

    #[error("rule `{consumer}` binds unknown producer `{producer}`")]
    UnknownProducer { consumer: String, producer: String },
    #[error("rule `{rule}` is executable but declared NoQuery")]
    ExecutableNoQuery { rule: String },
    #[error("rule `{rule}` depends on itself through an observation")]
    SelfDependency { rule: String },
    #[error("observation dependencies contain a cycle")]
    Cycle,
    #[error("rule `{producer}` does not export `{output}`")]
    MissingOutput { producer: String, output: String },
    #[error("rule `{consumer}` binds incompatible schema from `{producer}` output `{output}`")]
    IncompatibleObservationColumns {
        consumer: String,
        producer: String,
        output: String,
    },
    #[error("rule `{rule}` repeats bound input `{name}`")]
    DuplicateBinding { rule: String, name: String },
    #[error("producer `{rule}` is not compiled before its consumer")]
    NotCompiled { rule: String },
}
