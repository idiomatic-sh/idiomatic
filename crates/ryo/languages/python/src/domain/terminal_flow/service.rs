//! One reverse-preorder fold for every function and compound statement.

use crate::domain::{terminal_flow::types::{Returns, TerminalOutcome}};



/// Check whether a terminal summary includes a return path.
pub(crate) fn has_return(value: TerminalOutcome) -> Returns {
    Returns(matches!(
        value,
        TerminalOutcome::Return
            | TerminalOutcome::RaiseOrReturn
            | TerminalOutcome::ConditionalReturn
    ))
}

/// Compose sequential statement outcomes from left to right.
pub(crate) fn and_then(left: TerminalOutcome, right: TerminalOutcome) -> TerminalOutcome {
    use crate::domain::terminal_flow::types::TerminalOutcome::{ConditionalReturn, Implicit, None, Raise, RaiseOrReturn, Return};
    match (left, right) {
        (None | Implicit, other) | (other, None | Implicit) => other,
        (ConditionalReturn, ConditionalReturn) => ConditionalReturn,
        (Raise, ConditionalReturn | Return)
        | (ConditionalReturn | Return, Raise)
        | (RaiseOrReturn, _)
        | (_, RaiseOrReturn) => RaiseOrReturn,
        (Return, ConditionalReturn | Return) | (ConditionalReturn, Return) => Return,
        (Raise, Raise) => Raise,
    }
}

/// Join outcomes from alternate branches.
pub(crate) fn branch(left: TerminalOutcome, right: TerminalOutcome) -> TerminalOutcome {
    use crate::domain::terminal_flow::types::TerminalOutcome::{ConditionalReturn, Implicit, None, Raise, RaiseOrReturn, Return};
    match (left, right) {
        (None, other) | (other, None) => other,
        (Implicit, Implicit | Raise) | (Raise, Implicit) => Implicit,
        (Implicit, Return | RaiseOrReturn | ConditionalReturn)
        | (Return | RaiseOrReturn | ConditionalReturn, Implicit)
        | (ConditionalReturn, ConditionalReturn) => ConditionalReturn,
        (Raise, ConditionalReturn | Return)
        | (ConditionalReturn | Return, Raise)
        | (RaiseOrReturn, _)
        | (_, RaiseOrReturn) => RaiseOrReturn,
        (Return, ConditionalReturn | Return) | (ConditionalReturn, Return) => Return,
        (Raise, Raise) => Raise,
    }
}
