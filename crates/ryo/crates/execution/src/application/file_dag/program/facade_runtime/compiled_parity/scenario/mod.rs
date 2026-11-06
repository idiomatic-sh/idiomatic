//! The scenarios of the compiled-parity table.
#[derive(Clone, Copy)]
pub(crate) enum Scenario {
    Words,
    Counted,
    MissingRight,
    OrdinaryFunctionJoin,
    RowsOfWords,
    DistinctKinds,
    KeptKinds,
    NativeAntijoin,
    NativeUnion,
    NativeJoinSteps,
    NativeJoinTree,
}
