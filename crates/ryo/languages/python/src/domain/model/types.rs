//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language works out of a file once, before its readers run.








use ryo::languages::{Position};














/// One language operation or structured-control delimiter in a static evaluation schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingAction {
    /// Read an existing name.
    Read,
    /// Introduce a real declaration identity.
    Bind,
    /// Declare a type without initializing a value.
    Annotation,
    /// Explicitly delete a name.
    Delete,
    /// Remove an exception target at handler exit.
    Cleanup,
    /// Declare module namespace routing.
    Global,
    /// Declare enclosing function namespace routing.
    Nonlocal,
    /// Begin an immediate or deferred evaluation boundary.
    BeginPhase,
    /// Finish a phase before dependent phases inherit it.
    EndPhase,
    /// Fork alternatives from the current environment.
    BeginAlternatives,
    /// Finish one arm and restart from the same entry environment.
    NextAlternative,
    /// Join all completed arms with explicit possible absence.
    EndAlternatives,
}
/// A real syntax occurrence scheduled inside one execution boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingStep {
    /// The actual occurrence, also used for implicit handler cleanup.
    pub(crate) occurrence: Position,
    /// Real module, callable, generator or deferred annotation identity.
    pub phase: Position,
    /// Language action, not a diagnostic or runtime initialization claim.
    pub(crate) action: BindingAction,
}
/// Deferred regions have grammar-specific entry points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingPhaseKind {
    /// Callable body evaluated after its enclosing definition phase.
    Callable,
    /// Generator target/filter/result evaluated separately from its first iterable.
    Generator,
    /// A type expression not evaluated where it stands: a deferred native or embedded one, or one Python never evaluates (the
    /// annotation of a function's variable). Its names are read over the finished environment of the phase around it.
    Type,
}
/// A deferred region waits for its enclosing execution phase to complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingPhase {
    /// Real syntax root and public phase identity.
    pub root: Position,
    /// The execution boundary whose completed environment it inherits.
    pub(crate) parent: Position,
    /// Which grammar fields execute on entry.
    pub kind: BindingPhaseKind,
}
/// Whether native annotations use deferred source-name evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BindingAnnotationTiming {
    /// Native annotation expressions execute in the enclosing phase.
    #[default]
    Immediate,
    /// Future annotations defer native annotation name lookup.
    Deferred,
}
/// What a declaration `x: T` without a value does to the name `x`, by the kind of the source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AnnotatedDeclarations {
    /// A module that runs: the declaration gives the name no value, so a read of the name that reaches only it reads nothing.
    #[default]
    Unbound,
    /// A stub, which is never run: the typing specification reads its `x: T` as the declaration of `x` with the type `T`, so a
    /// read of `x` that reaches it uses it.
    Declared,
}
/// Reusable event and deferred-region buffers; constructed only on demand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingSchedule {
    /// Native annotation timing; quoted expressions are always separate deferred regions.
    pub(crate) annotations: BindingAnnotationTiming,
    /// Static event order, including structured alternatives rather than last-arm wins.
    pub(crate) steps: Vec<BindingStep>,
    /// Deferred regions in dependency order.
    pub(crate) phases: Vec<BindingPhase>,
    /// Spike (ryo-fmx3.1, 6f probe): per walked position, the phase it is evaluated in (`Position(u32::MAX)` where the walk
    /// did not reach it); written by the same walk that writes the steps.
    pub(crate) region: Vec<Position>,
}



