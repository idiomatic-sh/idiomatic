//! One bottom-up pass for basic expression-type classification.
use super::inclusion::service::union;
use super::operators::service::{binary, binary_operator, unary_operator};
use crate::domain::{
    expression_types::types::{
        BasicType, ExpressionClassification, ExpressionTypeIndex, ExpressionTypeState, Resolution,
    },
    fields::service::field_of,
    labels::types::Field,
    literal_properties::{service::classify as classify_literal, types::LiteralClassification},
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind},
        types::Kind,
    },
};
use ryo::languages::{self, Position, Tree};
use std::cell::{Ref, RefCell};

/// Reset the retained classifications before a fresh tree is processed.
pub(crate) fn clear(index: &mut ExpressionTypeIndex) {
    index.entries.clear();
    index.state = ExpressionTypeState::Fresh;
}

/// Find a direct expression child in the requested field.
fn child(tree: &Tree<'_>, at: Position, field: Field) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| {
        field_of(tree, *child) == field && category(tree, *child) == Category::Expression
    })
}

/// Read a child classification, returning unknown when it is unavailable.
fn child_resolution(index: &ExpressionTypeIndex, at: Option<Position>) -> Resolution {
    at.and_then(|position| {
        let Ok(slot) = usize::try_from(position.0) else {
            // A nonrepresentable position cannot identify an entry in this index.
            return None;
        };
        index.entries.get(slot).cloned().flatten()
    })
    .unwrap_or(Resolution::Unknown)
}

/// Classify one expression using only the bounded supported syntax forms.
fn classify_expression(
    tree: &Tree<'_>,
    index: &ExpressionTypeIndex,
    at: Position,
) -> ExpressionClassification {
    let result = match kind(tree, at) {
        Kind::Node(Node::Dict | Node::DictComp) => Resolution::Atom(BasicType::Dict),
        Kind::Node(Node::Set | Node::SetComp) => Resolution::Atom(BasicType::Set),
        Kind::Node(Node::List | Node::ListComp) => Resolution::Atom(BasicType::List),
        Kind::Node(Node::Tuple) => Resolution::Atom(BasicType::Tuple),
        Kind::Node(Node::GeneratorExp) => Resolution::Atom(BasicType::Generator),
        Kind::Node(Node::JoinedStr) => Resolution::Atom(BasicType::String),
        Kind::Node(Node::Constant) => match classify_literal(tree, at) {
            LiteralClassification::Known(properties) => {
                Resolution::Atom(super::constants::literal_type(properties.kind))
            }
            LiteralClassification::MissingPayload => {
                return ExpressionClassification::MissingPayload;
            }
            LiteralClassification::NotPrimitive => {
                return ExpressionClassification::Complete(Resolution::Unknown);
            }
        },
        Kind::Node(Node::NamedExpr) => child_resolution(index, child(tree, at, Field::Value)),
        Kind::Node(Node::IfExp) => union(
            child_resolution(index, child(tree, at, Field::Body)),
            child_resolution(index, child(tree, at, Field::Orelse)),
        ),
        Kind::Node(Node::BoolOp) => {
            let mut values = languages::children(tree.elements, at)
                .filter(|each| {
                    field_of(tree, *each) == Field::Values
                        && category(tree, *each) == Category::Expression
                })
                .map(|each| child_resolution(index, Some(each)));
            values
                .next()
                .map_or(Resolution::Unknown, |first| values.fold(first, union))
        }
        Kind::Node(Node::UnaryOp) => {
            let operand = child_resolution(index, child(tree, at, Field::Operand));
            let operator = unary_operator(tree, at);
            if operator == Some(Node::Not) {
                Resolution::Atom(BasicType::Boolean)
            } else if operator == Some(Node::Invert) {
                match operand {
                    Resolution::Atom(BasicType::Boolean | BasicType::Integer) => {
                        Resolution::Atom(BasicType::Integer)
                    }
                    Resolution::Atom(_) => Resolution::TypeError,
                    Resolution::Union(_) | Resolution::Unknown | Resolution::TypeError => {
                        Resolution::Unknown
                    }
                }
            } else if operator == Some(Node::UAdd) || operator == Some(Node::USub) {
                match operand {
                    Resolution::Atom(BasicType::Boolean) => Resolution::Atom(BasicType::Integer),
                    Resolution::Atom(
                        resolved_type
                        @ (BasicType::Integer | BasicType::Float | BasicType::Complex),
                    ) => Resolution::Atom(resolved_type),
                    Resolution::Atom(_) => Resolution::TypeError,
                    Resolution::Union(_) | Resolution::Unknown | Resolution::TypeError => {
                        Resolution::Unknown
                    }
                }
            } else {
                Resolution::Unknown
            }
        }
        Kind::Node(Node::BinOp) => {
            let left = child_resolution(index, child(tree, at, Field::Left));
            let right = child_resolution(index, child(tree, at, Field::Right));
            binary_operator(tree, at).map_or(Resolution::Unknown, |operator| {
                binary(operator, &left, &right)
            })
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => Resolution::Unknown,
    };
    ExpressionClassification::Complete(result)
}

/// Populate one expression classification per source position.
fn build(tree: &Tree<'_>, retained: &mut ExpressionTypeIndex) {
    if retained.state != ExpressionTypeState::Fresh {
        return;
    }
    retained.entries.clear();
    let length = usize::try_from(languages::end_of(tree.elements).0).unwrap_or(0);
    if retained.entries.try_reserve(length).is_err() {
        retained.state = ExpressionTypeState::Partial;
        return;
    }
    retained.entries.resize(length, None);
    retained.state = if languages::has_incomplete_syntax(tree.elements)
        == languages::SyntaxCompleteness::Incomplete
    {
        ExpressionTypeState::Partial
    } else {
        ExpressionTypeState::Complete
    };
    for raw in (0..length).rev() {
        let slot = raw;
        let Ok(position) = u32::try_from(slot) else {
            retained.state = ExpressionTypeState::Partial;
            break;
        };
        let at = Position(position);
        if category(tree, at) != Category::Expression {
            continue;
        }
        match classify_expression(tree, retained, at) {
            ExpressionClassification::MissingPayload => {
                retained.state = ExpressionTypeState::Partial
            }
            ExpressionClassification::Complete(value) => {
                let Some(entry) = retained.entries.get_mut(slot) else {
                    retained.state = ExpressionTypeState::Partial;
                    break;
                };
                *entry = Some(value);
            }
        }
    }
}

/// Lazily compute and borrow the retained expression classifications.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    held: &'a RefCell<ExpressionTypeIndex>,
) -> Ref<'a, ExpressionTypeIndex> {
    if held.borrow().state == ExpressionTypeState::Fresh {
        build(tree, &mut held.borrow_mut());
    }
    Ref::map(held.borrow(), |index| index)
}
