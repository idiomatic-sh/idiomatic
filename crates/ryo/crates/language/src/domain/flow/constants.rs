//! The words of the `flow` concept: the closed vocabularies of `flow-edge.edge-kind` and `flow-step.step-kind`, as rows carry them.

/// `flow-edge.edge-kind` of a fall-through or a test's false branch.
pub(crate) const NORMAL: &str = "normal";
/// `flow-edge.edge-kind` of a test's true branch.
pub(crate) const JUMP: &str = "jump";
/// `flow-edge.edge-kind` of a loop's way back.
pub(crate) const BACKEDGE: &str = "backedge";
/// `flow-edge.edge-kind` into a `finally`'s join.
pub(crate) const JOIN: &str = "join";
/// `flow-edge.edge-kind` into a `finally`.
pub(crate) const FINALIZE: &str = "finalize";
/// `flow-edge.edge-kind` into a function's first block.
pub(crate) const NEW_FUNCTION: &str = "new-function";
/// `flow-edge.edge-kind` into the block after an exit.
pub(crate) const UNREACHABLE: &str = "unreachable";
/// `flow-edge.edge-kind` into a `catch`.
pub(crate) const ERROR_EXPLICIT: &str = "error-explicit";
/// `flow-edge.edge-kind` out of the function on a throw outside any `try`.
pub(crate) const ERROR_IMPLICIT: &str = "error-implicit";
/// `flow-step.step-kind` of a statement.
pub(crate) const STATEMENT: &str = "statement";
/// `flow-step.step-kind` of a return with a value.
pub(crate) const RETURN: &str = "return";
/// `flow-step.step-kind` of a return with no value.
pub(crate) const RETURN_UNDEFINED: &str = "return-undefined";
/// `flow-step.step-kind` of an unlabeled break.
pub(crate) const BREAK: &str = "break";
/// `flow-step.step-kind` of a labeled break.
pub(crate) const BREAK_LABELED: &str = "break-labeled";
/// `flow-step.step-kind` of an unlabeled continue.
pub(crate) const CONTINUE: &str = "continue";
/// `flow-step.step-kind` of a labeled continue.
pub(crate) const CONTINUE_LABELED: &str = "continue-labeled";
/// `flow-step.step-kind` of a throw.
pub(crate) const THROW: &str = "throw";
/// `flow-step.step-kind` of a test.
pub(crate) const CONDITION: &str = "condition";
/// `flow-step.step-kind` of a `for … of` step.
pub(crate) const ITERATION_OF: &str = "iteration-of";
/// `flow-step.step-kind` of a `for … in` step.
pub(crate) const ITERATION_IN: &str = "iteration-in";
