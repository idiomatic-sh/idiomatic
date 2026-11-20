//! The `terminal_facts` service: free functions over the parser's own statements. How a function's body ends is a fold of how
//! each of its statements ends; a statement's ending is read from its own clauses, and a function or a class defined inside
//! ends nothing of the body around it, so each statement of a file is read once.

use super::types::{AlternativeCoverage, Ending};
use crate::domain::terminal_flow::constants::{
    CONDITIONAL_RETURN, IMPLICIT, RAISE, RETURN_OR_RAISE,
};
use crate::domain::terminal_flow::service::{and_then, branch, has_return};
use crate::domain::terminal_flow::types::{Breaks, TerminalOutcome, WildcardPattern};
use ruff_python_ast::{ExceptHandler, Pattern, Stmt, StmtFunctionDef, StmtTry};
use ryo::languages::NameText;

/// Whether a pattern takes every subject: a capture, a wildcard, or alternatives that each do.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only an `as` pattern and an `or` pattern can take every subject; every other pattern refuses some"
)]
fn takes_all(pattern: &Pattern) -> WildcardPattern {
    WildcardPattern(match pattern {
        Pattern::MatchAs(named) => named
            .pattern
            .as_deref()
            .is_none_or(|inner| takes_all(inner).0),
        Pattern::MatchOr(either) => {
            !either.patterns.is_empty() && either.patterns.iter().all(|each| takes_all(each).0)
        }
        _ => false,
    })
}

/// How a run of statements ends: the sequence of its statements' outcomes, implicit when none ends it, and whether a `break`
/// leaves the loop around it before a `return` or a `raise` stands in its way.
fn run_ending(statements: &[Stmt]) -> Ending {
    let mut terminal = TerminalOutcome::None;
    let mut breaks = None;
    for statement in statements {
        let each = ending(statement);
        terminal = and_then(terminal, each.terminal);
        if breaks.is_none() {
            if matches!(statement, Stmt::Return(_) | Stmt::Raise(_)) {
                breaks = Some(Breaks(false));
            } else if each.breaks.0 {
                breaks = Some(Breaks(true));
            } else {
                // The statement neither breaks nor ends the run: the next one is asked.
            }
        }
    }
    Ending {
        terminal: if terminal == TerminalOutcome::None {
            TerminalOutcome::Implicit
        } else {
            terminal
        },
        breaks: breaks.unwrap_or_default(),
    }
}

/// Whether the body of a loop always leaves it by a `break` of its own, before any `return` or `raise`.
fn always_breaks(statements: &[Stmt]) -> Breaks {
    Breaks(
        statements
            .iter()
            .find(|statement| {
                matches!(statement, Stmt::Break(_) | Stmt::Return(_) | Stmt::Raise(_))
            })
            .is_some_and(|first| matches!(first, Stmt::Break(_))),
    )
}

/// What alternatives end as: their joined outcome when one of them always runs, a conditional return when one of them returns,
/// nothing otherwise.
fn settled(coverage: AlternativeCoverage, joined: TerminalOutcome) -> TerminalOutcome {
    if coverage == AlternativeCoverage::Exhaustive {
        joined
    } else if has_return(joined).0 {
        TerminalOutcome::ConditionalReturn
    } else {
        TerminalOutcome::None
    }
}

/// How a `try` statement ends.
fn try_ending(statement: &StmtTry) -> Ending {
    let body = run_ending(&statement.body);
    let mut handlers = TerminalOutcome::None;
    let mut breaks = body.breaks.0;
    for ExceptHandler::ExceptHandler(handler) in &statement.handlers {
        let each = run_ending(&handler.body);
        handlers = branch(handlers, each.terminal);
        breaks |= each.breaks.0;
    }
    let orelse = run_ending(&statement.orelse);
    let finally = run_ending(&statement.finalbody);
    let mut terminal = TerminalOutcome::None;
    if has_return(body.terminal).0 {
        terminal = and_then(terminal, TerminalOutcome::ConditionalReturn);
    }
    terminal = and_then(terminal, finally.terminal);
    if statement.orelse.is_empty() {
        if has_return(handlers).0 {
            terminal = and_then(terminal, TerminalOutcome::ConditionalReturn);
        }
    } else {
        terminal = and_then(terminal, branch(handlers, orelse.terminal));
    }
    Ending {
        terminal,
        breaks: Breaks(breaks || orelse.breaks.0 || finally.breaks.0),
    }
}

/// How a loop with this body and this `else` ends.
fn loop_ending(body: &[Stmt], orelse: &[Stmt]) -> Ending {
    if always_breaks(body).0 {
        return Ending::default();
    }
    let inside = run_ending(body);
    let after = run_ending(orelse);
    let terminal = if inside.breaks.0 {
        inside.terminal
    } else {
        and_then(inside.terminal, after.terminal)
    };
    Ending {
        terminal,
        breaks: Breaks(!has_return(inside.terminal).0 && after.breaks.0),
    }
}

/// How one statement ends. A definition ends nothing of the body it stands in.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the statements that end a body or hold bodies are named; every other statement ends nothing"
)]
fn ending(statement: &Stmt) -> Ending {
    match statement {
        Stmt::Break(_) => Ending {
            terminal: TerminalOutcome::None,
            breaks: Breaks(true),
        },
        Stmt::Return(_) => Ending {
            terminal: TerminalOutcome::RaiseOrReturn,
            breaks: Breaks(false),
        },
        Stmt::Raise(_) => Ending {
            terminal: TerminalOutcome::Raise,
            breaks: Breaks(false),
        },
        Stmt::If(conditional) => {
            let first = run_ending(&conditional.body);
            let (mut joined, mut breaks) = (first.terminal, first.breaks.0);
            let mut coverage = AlternativeCoverage::Partial;
            for clause in &conditional.elif_else_clauses {
                let each = run_ending(&clause.body);
                if clause.test.is_none() {
                    coverage = AlternativeCoverage::Exhaustive;
                }
                joined = branch(joined, each.terminal);
                breaks |= each.breaks.0;
            }
            Ending {
                terminal: settled(coverage, joined),
                breaks: Breaks(breaks),
            }
        }
        Stmt::Match(matching) => {
            let (mut joined, mut breaks) = (TerminalOutcome::None, false);
            let mut coverage = AlternativeCoverage::Partial;
            for case in &matching.cases {
                let each = run_ending(&case.body);
                joined = branch(joined, each.terminal);
                if case.guard.is_none() && takes_all(&case.pattern).0 {
                    coverage = AlternativeCoverage::Exhaustive;
                }
                breaks |= each.breaks.0;
            }
            Ending {
                terminal: settled(coverage, joined),
                breaks: Breaks(breaks),
            }
        }
        Stmt::For(looping) => loop_ending(&looping.body, &looping.orelse),
        Stmt::While(looping) => loop_ending(&looping.body, &looping.orelse),
        Stmt::With(with) => run_ending(&with.body),
        Stmt::Try(attempt) => try_ending(attempt),
        _ => Ending::default(),
    }
}

/// How the body of a function ends, as the relation names it: `implicit` when it falls off its end, `raise`,
/// `return-or-raise`, or `conditional-return` when one path returns and another falls off.
#[must_use]
pub(crate) fn outcome(function: &StmtFunctionDef) -> NameText<'static> {
    NameText(match run_ending(&function.body).terminal {
        TerminalOutcome::None | TerminalOutcome::Implicit => IMPLICIT,
        TerminalOutcome::Raise => RAISE,
        TerminalOutcome::Return | TerminalOutcome::RaiseOrReturn => RETURN_OR_RAISE,
        TerminalOutcome::ConditionalReturn => CONDITIONAL_RETURN,
    })
}
