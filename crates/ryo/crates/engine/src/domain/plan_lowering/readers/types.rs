//! The types of the `readers` part: a plan with who reads each step, and the answers already worked out.

use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::types::FinishStep;

/// A plan's steps, who reads each, and which are emitted.
#[derive(Debug)]
pub(crate) struct PlanReaders<'s> {
    /// The steps.
    pub(crate) steps: &'s [FinishStep],
    /// For each step, the steps that read it, each once, in plan order.
    pub(crate) readers: &'s [Vec<StepNumber>],
    /// For each step, whether a rule emits it.
    pub(crate) emitted: &'s [Emitted],
}

/// Dependencies and emitted steps are unchanged by all three marking passes.
pub(crate) struct Uses {
    pub(crate) readers: Vec<Vec<StepNumber>>,
    pub(crate) emitted: Vec<Emitted>,
}

/// Whether a rule emits a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Emitted {
    /// No rule does.
    No,
    /// A rule does.
    Yes,
}

/// A memo of a yes-or-no about each step: not worked out yet, or the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Worked {
    /// Not yet.
    Open,
    /// Yes.
    Holds,
    /// No.
    Fails,
}

/// An alias an expression reads a column under, as the passes compare them: none, a name, or a constant the prototype's
/// walk read in an alias's place (it compares equal to no alias).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ReadAlias {
    /// No alias.
    Unaliased,
    /// This alias.
    Named(Box<str>),
    /// A constant in an alias's place.
    Other(ryo_values::domain::finish_plans::expressions::types::PlanConstant),
}
