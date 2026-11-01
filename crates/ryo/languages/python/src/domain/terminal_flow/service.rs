//! One reverse-preorder fold for every function and compound statement.
use crate::domain::{
    extras::types::Extra,
    fields::service::field_of,
    labels::types::Field,
    model::types::Declarations,
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind},
        types::Kind,
    },
    terminal_flow::types::{IndexState, NodeSummary, Outcome, TerminalIndex},
};
use ryo::languages::{self, Position, Tree};
use std::cell::Ref;

/// Reset the retained terminal summaries before processing a fresh tree.
pub(crate) fn clear(index: &mut TerminalIndex) {
    index.nodes.clear();
    index.state = IndexState::Fresh;
}

/// Return syntax children with the requested field and category.
fn children<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    field: Field,
    category_wanted: Category,
) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, at).filter(move |child| {
        field_of(tree, *child) == field && category(tree, *child) == category_wanted
    })
}

/// Return the `elif` and `else` clause wrappers owned by a statement.
fn clauses<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, at)
        .filter(|child| kind(tree, *child) == Kind::Extra(Extra::ElifElseClause))
}

/// Read a previously computed child summary, defaulting when unavailable.
fn node_index(nodes: &[NodeSummary], at: Position) -> NodeSummary {
    usize::try_from(at.0)
        .ok()
        .and_then(|index| nodes.get(index))
        .copied()
        .unwrap_or_default()
}

/// Fold the terminal behavior of statements in one ordered field.
fn sequence(tree: &Tree<'_>, nodes: &[NodeSummary], at: Position, field: Field) -> Outcome {
    let outcome = children(tree, at, field, Category::Statement)
        .fold(Outcome::None, |current, child| {
            and_then(current, node_index(nodes, child).terminal)
        });
    if outcome == Outcome::None {
        Outcome::Implicit
    } else {
        outcome
    }
}

/// Check whether a terminal summary includes a return path.
fn has_return(value: Outcome) -> bool {
    matches!(
        value,
        Outcome::Return | Outcome::RaiseOrReturn | Outcome::ConditionalReturn
    )
}

/// Compose sequential statement outcomes from left to right.
fn and_then(left: Outcome, right: Outcome) -> Outcome {
    use Outcome::{ConditionalReturn, Implicit, None, Raise, RaiseOrReturn, Return};
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
fn branch(left: Outcome, right: Outcome) -> Outcome {
    use Outcome::{ConditionalReturn, Implicit, None, Raise, RaiseOrReturn, Return};
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

/// Check whether the first terminal statement in a body is `break`.
fn always_breaks(tree: &Tree<'_>, at: Position) -> bool {
    for statement in children(tree, at, Field::Body, Category::Statement) {
        match kind(tree, statement) {
            Kind::Node(Node::Break) => return true,
            Kind::Node(Node::Return | Node::Raise) => return false,
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
    false
}

/// Check whether a body can reach a break statement.
fn body_breaks(tree: &Tree<'_>, nodes: &[NodeSummary], at: Position) -> bool {
    for statement in children(tree, at, Field::Body, Category::Statement) {
        match kind(tree, statement) {
            Kind::Node(Node::Break) => return true,
            Kind::Node(Node::Return | Node::Raise) => return false,
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_)
                if node_index(nodes, statement).sometimes_breaks =>
            {
                return true;
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
    false
}

/// Check whether a match case is an unguarded wildcard pattern.
fn exhaustive_case(tree: &Tree<'_>, nodes: &[NodeSummary], case: Position) -> bool {
    children(tree, case, Field::Guard, Category::Expression)
        .next()
        .is_none()
        && children(tree, case, Field::Pattern, Category::Pattern)
            .next()
            .is_some_and(|pattern| node_index(nodes, pattern).wildcard_pattern)
}

#[expect(
    clippy::too_many_lines,
    reason = "the pinned terminal fold is one exhaustive arm per structured statement kind"
)]
/// Summarize the supported terminal behavior for one syntax node.
fn node_summary(tree: &Tree<'_>, nodes: &[NodeSummary], at: Position) -> NodeSummary {
    use Kind::Node as SyntaxNode;
    use Node::{
        AsyncFor, AsyncFunctionDef, AsyncWith, Break, For, FunctionDef, If, Match, MatchAs,
        MatchOr, Raise, Return, Try, TryStar, While, With,
    };
    match kind(tree, at) {
        SyntaxNode(MatchAs) => NodeSummary {
            wildcard_pattern: children(tree, at, Field::Pattern, Category::Pattern)
                .next()
                .is_none_or(|pattern| node_index(nodes, pattern).wildcard_pattern),
            ..NodeSummary::default()
        },
        SyntaxNode(MatchOr) => {
            let mut patterns = children(tree, at, Field::Patterns, Category::Pattern);
            let wildcard = patterns.next().is_some_and(|first| {
                node_index(nodes, first).wildcard_pattern
                    && patterns.all(|pattern| node_index(nodes, pattern).wildcard_pattern)
            });
            NodeSummary {
                wildcard_pattern: wildcard,
                ..NodeSummary::default()
            }
        }
        SyntaxNode(FunctionDef | AsyncFunctionDef) => NodeSummary {
            function_terminal: sequence(tree, nodes, at, Field::Body),
            ..NodeSummary::default()
        },
        SyntaxNode(Break) => NodeSummary {
            terminal: Outcome::None,
            sometimes_breaks: true,
            ..NodeSummary::default()
        },
        SyntaxNode(Return) => NodeSummary {
            terminal: Outcome::RaiseOrReturn,
            sometimes_breaks: false,
            ..NodeSummary::default()
        },
        SyntaxNode(Raise) => NodeSummary {
            terminal: Outcome::Raise,
            sometimes_breaks: false,
            ..NodeSummary::default()
        },
        SyntaxNode(If) => {
            let mut branches = sequence(tree, nodes, at, Field::Body);
            let mut exhaustive = false;
            let mut breaks = body_breaks(tree, nodes, at);
            for clause in clauses(tree, at) {
                let has_test = children(tree, clause, Field::Test, Category::Expression)
                    .next()
                    .is_some();
                exhaustive |= !has_test;
                branches = branch(branches, sequence(tree, nodes, clause, Field::Body));
                breaks |= body_breaks(tree, nodes, clause);
            }
            let terminal = if exhaustive {
                and_then(Outcome::None, branches)
            } else if has_return(branches) {
                and_then(Outcome::None, Outcome::ConditionalReturn)
            } else {
                Outcome::None
            };
            NodeSummary {
                terminal,
                sometimes_breaks: breaks,
                ..NodeSummary::default()
            }
        }
        SyntaxNode(Match) => {
            let mut terminal = Outcome::None;
            let mut exhaustive = false;
            let mut breaks = false;
            for case in children(tree, at, Field::Cases, Category::Part) {
                terminal = branch(terminal, sequence(tree, nodes, case, Field::Body));
                exhaustive |= exhaustive_case(tree, nodes, case);
                breaks |= body_breaks(tree, nodes, case);
            }
            let terminal = if exhaustive {
                and_then(Outcome::None, terminal)
            } else if has_return(terminal) {
                and_then(Outcome::None, Outcome::ConditionalReturn)
            } else {
                Outcome::None
            };
            NodeSummary {
                terminal,
                sometimes_breaks: breaks,
                ..NodeSummary::default()
            }
        }
        SyntaxNode(For | AsyncFor | While) => {
            let body = sequence(tree, nodes, at, Field::Body);
            let otherwise = sequence(tree, nodes, at, Field::Orelse);
            if always_breaks(tree, at) {
                NodeSummary {
                    terminal: Outcome::None,
                    sometimes_breaks: false,
                    ..NodeSummary::default()
                }
            } else {
                let mut terminal = and_then(Outcome::None, body);
                if !body_breaks(tree, nodes, at) {
                    terminal = and_then(terminal, otherwise);
                }
                NodeSummary {
                    terminal,
                    sometimes_breaks: !has_return(body)
                        && sequence_breaks(tree, nodes, at, Field::Orelse),
                    ..NodeSummary::default()
                }
            }
        }
        SyntaxNode(With | AsyncWith) => NodeSummary {
            terminal: sequence(tree, nodes, at, Field::Body),
            sometimes_breaks: body_breaks(tree, nodes, at),
            ..NodeSummary::default()
        },
        SyntaxNode(Try | TryStar) => {
            let body = sequence(tree, nodes, at, Field::Body);
            let mut branches = Outcome::None;
            let mut handler_breaks = false;
            for handler in children(tree, at, Field::Handlers, Category::Part) {
                branches = branch(branches, sequence(tree, nodes, handler, Field::Body));
                handler_breaks |= body_breaks(tree, nodes, handler);
            }
            let orelse = sequence(tree, nodes, at, Field::Orelse);
            let finally = sequence(tree, nodes, at, Field::Finalbody);
            let mut terminal = Outcome::None;
            if has_return(body) {
                terminal = and_then(terminal, Outcome::ConditionalReturn);
            }
            terminal = and_then(terminal, finally);
            if children(tree, at, Field::Orelse, Category::Statement)
                .next()
                .is_none()
            {
                if has_return(branches) {
                    terminal = and_then(terminal, Outcome::ConditionalReturn);
                }
            } else {
                terminal = and_then(terminal, branch(branches, orelse));
            }
            NodeSummary {
                terminal,
                sometimes_breaks: body_breaks(tree, nodes, at)
                    || handler_breaks
                    || sequence_breaks(tree, nodes, at, Field::Orelse)
                    || sequence_breaks(tree, nodes, at, Field::Finalbody),
                ..NodeSummary::default()
            }
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => NodeSummary::default(),
    }
}

/// Check whether an ordered statement field can reach a break.
fn sequence_breaks(tree: &Tree<'_>, nodes: &[NodeSummary], at: Position, field: Field) -> bool {
    for statement in children(tree, at, field, Category::Statement) {
        match kind(tree, statement) {
            Kind::Node(Node::Break) => return true,
            Kind::Node(Node::Return | Node::Raise) => return false,
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_)
                if node_index(nodes, statement).sometimes_breaks =>
            {
                return true;
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
    false
}

/// Populate terminal summaries in reverse source-tree order.
fn build(tree: &Tree<'_>, index: &mut TerminalIndex) {
    if index.state != IndexState::Fresh {
        return;
    }
    index.nodes.clear();
    let length = usize::try_from(languages::end_of(tree.elements).0).unwrap_or(0);
    if index.nodes.try_reserve(length).is_err() {
        index.state = IndexState::Partial;
        return;
    }
    index.nodes.resize(length, NodeSummary::default());
    index.state = if languages::has_incomplete_syntax(tree.elements) {
        IndexState::Partial
    } else {
        IndexState::Complete
    };
    for slot in (0..length).rev() {
        let Ok(position) = u32::try_from(slot) else {
            index.state = IndexState::Partial;
            break;
        };
        let at = Position(position);
        let summary = node_summary(tree, &index.nodes, at);
        let Some(entry) = index.nodes.get_mut(slot) else {
            index.state = IndexState::Partial;
            break;
        };
        *entry = summary;
    }
}

/// Lazily compute and borrow the retained terminal summaries.
pub(crate) fn prepare<'a>(tree: &Tree<'_>, model: &'a Declarations) -> Ref<'a, TerminalIndex> {
    if model.terminal_flow.borrow().state == IndexState::Fresh {
        build(tree, &mut model.terminal_flow.borrow_mut());
    }
    Ref::map(model.terminal_flow.borrow(), |index| index)
}

/// Return the exposed terminal label for a function node.
pub(crate) fn outcome(index: &TerminalIndex, at: Position) -> Option<&'static str> {
    let value = index
        .nodes
        .get(usize::try_from(at.0).ok()?)?
        .function_terminal;
    Some(match value {
        Outcome::None | Outcome::Implicit => "implicit",
        Outcome::Raise => "raise",
        Outcome::Return | Outcome::RaiseOrReturn => "return-or-raise",
        Outcome::ConditionalReturn => "conditional-return",
    })
}
