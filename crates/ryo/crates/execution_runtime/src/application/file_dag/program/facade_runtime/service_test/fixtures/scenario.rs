//! The program shapes of the compiled-parity table.
#[derive(Clone, Copy)]
pub(crate) enum ProgramShape {
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
