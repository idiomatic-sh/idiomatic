//! One bottom-up pass for basic expression-type classification.
use crate::domain::{
    expression_types::types::{
        BasicType, ExpressionClassification, ExpressionTypeIndex, ExpressionTypeState, Resolution,
        TypeInclusion,
    },
    fields::service::field_of,
    labels::types::Field,
    literal_properties::{service::classify as classify_literal, types::LiteralClassification},
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind, written},
        types::Kind,
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Tree};
use std::cell::{Ref, RefCell};

/// Reset the retained classifications before a fresh tree is processed.
pub(crate) fn clear(index: &mut ExpressionTypeIndex) {
    index.entries.clear();
    index.state = ExpressionTypeState::Fresh;
}

/// Check whether the left type is included in the right type.
fn subtype(left: BasicType, right: BasicType) -> TypeInclusion {
    if left == right
        || matches!(
            (left, right),
            (
                BasicType::Boolean,
                BasicType::Integer | BasicType::Float | BasicType::Complex
            ) | (BasicType::Integer, BasicType::Float | BasicType::Complex)
                | (BasicType::Float, BasicType::Complex)
        )
    {
        TypeInclusion::Included
    } else {
        TypeInclusion::Separate
    }
}

/// Combine branch results while preserving known subtype relationships.
fn union(left: Resolution, right: Resolution) -> Resolution {
    match (left, right) {
        (Resolution::TypeError, _) | (_, Resolution::TypeError) => Resolution::TypeError,
        (Resolution::Unknown, _) | (_, Resolution::Unknown) => Resolution::Unknown,
        (Resolution::Atom(left_kind), Resolution::Atom(right_kind)) => {
            if subtype(left_kind, right_kind) == TypeInclusion::Included {
                Resolution::Atom(right_kind)
            } else if subtype(right_kind, left_kind) == TypeInclusion::Included {
                Resolution::Atom(left_kind)
            } else {
                Resolution::Union(vec![left_kind, right_kind])
            }
        }
        (Resolution::Atom(left_kind), Resolution::Union(mut right_types)) => {
            if !right_types
                .iter()
                .any(|candidate| subtype(left_kind, *candidate) == TypeInclusion::Included)
            {
                right_types.push(left_kind);
            }
            Resolution::Union(right_types)
        }
        (Resolution::Union(mut left_types), Resolution::Atom(right_kind)) => {
            if !left_types
                .iter()
                .any(|candidate| subtype(right_kind, *candidate) == TypeInclusion::Included)
            {
                left_types.push(right_kind);
            }
            Resolution::Union(left_types)
        }
        (Resolution::Union(mut left_types), Resolution::Union(right_types)) => {
            for candidate in right_types {
                if !left_types
                    .iter()
                    .any(|known| subtype(candidate, *known) == TypeInclusion::Included)
                {
                    left_types.push(candidate);
                }
            }
            Resolution::Union(left_types)
        }
    }
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

/// Map the binary operator token for an expression to its syntax node.
fn binary_operator(tree: &Tree<'_>, at: Position) -> Option<Node> {
    languages::children(tree.elements, at).find_map(|each| {
        if field_of(tree, each) != Field::Op {
            return None;
        }
        let Kind::Token(token) = kind(tree, each) else {
            return None;
        };
        if token == Token::Plus {
            Some(Node::Add)
        } else if token == Token::Minus {
            Some(Node::Sub)
        } else if token == Token::Star {
            Some(Node::Mult)
        } else if token == Token::At {
            Some(Node::MatMult)
        } else if token == Token::Slash {
            Some(Node::Div)
        } else if token == Token::Percent {
            Some(Node::Mod)
        } else if token == Token::Doublestar {
            Some(Node::Pow)
        } else if token == Token::Doubleslash {
            Some(Node::FloorDiv)
        } else if token == Token::Amper {
            Some(Node::BitAnd)
        } else if token == Token::Vbar {
            Some(Node::BitOr)
        } else if token == Token::Circumflex {
            Some(Node::BitXor)
        } else if token == Token::Leftshift {
            Some(Node::LShift)
        } else if token == Token::Rightshift {
            Some(Node::RShift)
        } else {
            None
        }
    })
}

/// Map the unary operator token for an expression to its syntax node.
fn unary_operator(tree: &Tree<'_>, at: Position) -> Option<Node> {
    languages::children(tree.elements, at).find_map(|each| {
        if field_of(tree, each) != Field::Op {
            return None;
        }
        let Kind::Token(token) = kind(tree, each) else {
            return None;
        };
        if token == Token::Plus {
            Some(Node::UAdd)
        } else if token == Token::Minus {
            Some(Node::USub)
        } else if token == Token::Tilde {
            Some(Node::Invert)
        } else if token == Token::Name && written(tree, each).0 == super::constants::NOT {
            Some(Node::Not)
        } else {
            None
        }
    })
}

/// Infer a result for the supported binary operations.
fn binary(op: Node, left: &Resolution, right: &Resolution) -> Resolution {
    if op == Node::Mod {
        match left {
            Resolution::Atom(BasicType::String) => return Resolution::Atom(BasicType::String),
            Resolution::Atom(BasicType::Bytes) => return Resolution::Atom(BasicType::Bytes),
            Resolution::Atom(_)
            | Resolution::Union(_)
            | Resolution::Unknown
            | Resolution::TypeError => {}
        }
    }
    let (Resolution::Atom(left_kind), Resolution::Atom(right_kind)) = (left, right) else {
        return Resolution::Unknown;
    };
    let numeric_a = matches!(
        *left_kind,
        BasicType::Boolean | BasicType::Integer | BasicType::Float | BasicType::Complex
    );
    let numeric_b = matches!(
        *right_kind,
        BasicType::Boolean | BasicType::Integer | BasicType::Float | BasicType::Complex
    );
    let coerced = if *left_kind == BasicType::Complex || *right_kind == BasicType::Complex {
        BasicType::Complex
    } else if *left_kind == BasicType::Float || *right_kind == BasicType::Float {
        BasicType::Float
    } else {
        BasicType::Integer
    };
    if op == Node::Add {
        return match (*left_kind, *right_kind) {
            (BasicType::String, BasicType::String) => Resolution::Atom(BasicType::String),
            (BasicType::Bytes, BasicType::Bytes) => Resolution::Atom(BasicType::Bytes),
            (BasicType::List, BasicType::List) => Resolution::Atom(BasicType::List),
            (BasicType::Tuple, BasicType::Tuple) => Resolution::Atom(BasicType::Tuple),
            _ if numeric_a && numeric_b => Resolution::Atom(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::Sub {
        return match (*left_kind, *right_kind) {
            (BasicType::Set, BasicType::Set) => Resolution::Atom(BasicType::Set),
            _ if numeric_a && numeric_b => Resolution::Atom(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::Mod {
        return if numeric_a && numeric_b {
            Resolution::Atom(coerced)
        } else {
            Resolution::Unknown
        };
    }
    if op == Node::Mult {
        return match (*left_kind, *right_kind) {
            (BasicType::String, BasicType::Integer) | (BasicType::Integer, BasicType::String) => {
                Resolution::Atom(BasicType::String)
            }
            _ if numeric_a && numeric_b => Resolution::Atom(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::FloorDiv || op == Node::Pow {
        return if numeric_a && numeric_b {
            Resolution::Atom(coerced)
        } else {
            Resolution::TypeError
        };
    }
    if op == Node::Div {
        return if numeric_a && numeric_b {
            Resolution::Atom(if coerced == BasicType::Integer {
                BasicType::Float
            } else {
                coerced
            })
        } else {
            Resolution::TypeError
        };
    }
    if matches!(
        op,
        Node::BitAnd | Node::BitOr | Node::BitXor | Node::LShift | Node::RShift
    ) {
        return if numeric_a && numeric_b && coerced == BasicType::Integer {
            Resolution::Atom(BasicType::Integer)
        } else {
            Resolution::TypeError
        };
    }
    Resolution::Unknown
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
            LiteralClassification::Known(properties) => Resolution::Atom(super::constants::literal_type(
                properties.kind,
            )),
            LiteralClassification::MissingPayload => return ExpressionClassification::MissingPayload,
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
    retained.state = if languages::has_incomplete_syntax(tree.elements) {
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
