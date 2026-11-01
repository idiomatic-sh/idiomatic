//! One bottom-up pass for basic expression-type classification.
use crate::domain::{
    expression_types::types::{ExpressionTypeIndex, IndexState, Resolution},
    fields::service::field_of,
    labels::types::Field,
    literal_properties::service::{Classification, classify as classify_literal},
    model::types::Declarations,
    nodes::types::{Category, Node},
    syntax::{
        service::{category, kind, written},
        types::Kind,
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Tree};
use std::cell::Ref;

/// Reset the retained classifications before a fresh tree is processed.
pub(crate) fn clear(index: &mut ExpressionTypeIndex) {
    index.entries.clear();
    index.state = IndexState::Fresh;
}

/// Return a resolved value with one known type label.
fn atom(kind: &'static str) -> Resolution {
    Resolution::Atom(kind)
}
/// Return a numeric value with its inferred type label.
fn numeric(kind: &'static str) -> Resolution {
    atom(kind)
}

/// Check whether the left type is included in the right type.
fn subtype(left: &str, right: &str) -> bool {
    left == right
        || matches!(
            (left, right),
            ("boolean", "integer" | "float" | "complex")
                | ("integer", "float" | "complex")
                | ("float", "complex")
        )
}

/// Combine branch results while preserving known subtype relationships.
fn union(left: Resolution, right: Resolution) -> Resolution {
    match (left, right) {
        (Resolution::TypeError, _) | (_, Resolution::TypeError) => Resolution::TypeError,
        (Resolution::Unknown, _) | (_, Resolution::Unknown) => Resolution::Unknown,
        (Resolution::Atom(left_kind), Resolution::Atom(right_kind)) => {
            if subtype(left_kind, right_kind) {
                atom(right_kind)
            } else if subtype(right_kind, left_kind) {
                atom(left_kind)
            } else {
                Resolution::Union(vec![left_kind, right_kind])
            }
        }
        (Resolution::Atom(left_kind), Resolution::Union(mut right_types)) => {
            if !right_types
                .iter()
                .any(|candidate| subtype(left_kind, candidate))
            {
                right_types.push(left_kind);
            }
            Resolution::Union(right_types)
        }
        (Resolution::Union(mut left_types), Resolution::Atom(right_kind)) => {
            if !left_types
                .iter()
                .any(|candidate| subtype(right_kind, candidate))
            {
                left_types.push(right_kind);
            }
            Resolution::Union(left_types)
        }
        (Resolution::Union(mut left_types), Resolution::Union(right_types)) => {
            for candidate in right_types {
                if !left_types.iter().any(|known| subtype(candidate, known)) {
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
        index
            .entries
            .get(usize::try_from(position.0).ok()?)
            .cloned()
            .flatten()
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
        } else if token == Token::Name && written(tree, each).0 == "not" {
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
            Resolution::Atom("string") => return atom("string"),
            Resolution::Atom("bytes") => return atom("bytes"),
            Resolution::Atom(_)
            | Resolution::Union(_)
            | Resolution::Unknown
            | Resolution::TypeError => {}
        }
    }
    let (Resolution::Atom(left_kind), Resolution::Atom(right_kind)) = (left, right) else {
        return Resolution::Unknown;
    };
    let numeric_a = matches!(*left_kind, "boolean" | "integer" | "float" | "complex");
    let numeric_b = matches!(*right_kind, "boolean" | "integer" | "float" | "complex");
    let coerced = if *left_kind == "complex" || *right_kind == "complex" {
        "complex"
    } else if *left_kind == "float" || *right_kind == "float" {
        "float"
    } else {
        "integer"
    };
    if op == Node::Add {
        return match (*left_kind, *right_kind) {
            ("string", "string") => atom("string"),
            ("bytes", "bytes") => atom("bytes"),
            ("list", "list") => atom("list"),
            ("tuple", "tuple") => atom("tuple"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::Sub {
        return match (*left_kind, *right_kind) {
            ("set", "set") => atom("set"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::Mod {
        return if numeric_a && numeric_b {
            numeric(coerced)
        } else {
            Resolution::Unknown
        };
    }
    if op == Node::Mult {
        return match (*left_kind, *right_kind) {
            ("string", "integer") | ("integer", "string") => atom("string"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        };
    }
    if op == Node::FloorDiv || op == Node::Pow {
        return if numeric_a && numeric_b {
            numeric(coerced)
        } else {
            Resolution::TypeError
        };
    }
    if op == Node::Div {
        return if numeric_a && numeric_b {
            numeric(if coerced == "integer" {
                "float"
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
        return if numeric_a && numeric_b && coerced == "integer" {
            atom("integer")
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
) -> (Resolution, bool) {
    let result = match kind(tree, at) {
        Kind::Node(Node::Dict | Node::DictComp) => atom("dict"),
        Kind::Node(Node::Set | Node::SetComp) => atom("set"),
        Kind::Node(Node::List | Node::ListComp) => atom("list"),
        Kind::Node(Node::Tuple) => atom("tuple"),
        Kind::Node(Node::GeneratorExp) => atom("generator"),
        Kind::Node(Node::JoinedStr) => atom("string"),
        Kind::Node(Node::Constant) => match classify_literal(tree, at) {
            Classification::Known(properties) => atom(match properties.kind {
                "integer" => "integer",
                "float" => "float",
                "complex" => "complex",
                "boolean" => "boolean",
                "string" => "string",
                "bytes" => "bytes",
                "null" => "null",
                "ellipsis" => "ellipsis",
                _ => "unknown",
            }),
            Classification::MissingPayload => return (Resolution::Unknown, true),
            Classification::NotPrimitive => return (Resolution::Unknown, false),
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
                atom("boolean")
            } else if operator == Some(Node::Invert) {
                match operand {
                    Resolution::Atom("boolean" | "integer") => atom("integer"),
                    Resolution::Atom(_) => Resolution::TypeError,
                    Resolution::Union(_) | Resolution::Unknown | Resolution::TypeError => {
                        Resolution::Unknown
                    }
                }
            } else if operator == Some(Node::UAdd) || operator == Some(Node::USub) {
                match operand {
                    Resolution::Atom("boolean") => atom("integer"),
                    Resolution::Atom(resolved_type @ ("integer" | "float" | "complex")) => {
                        atom(resolved_type)
                    }
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
    (result, false)
}

/// Populate one expression classification per source position.
fn build(tree: &Tree<'_>, retained: &mut ExpressionTypeIndex) {
    if retained.state != IndexState::Fresh {
        return;
    }
    retained.entries.clear();
    let length = usize::try_from(languages::end_of(tree.elements).0).unwrap_or(0);
    if retained.entries.try_reserve(length).is_err() {
        retained.state = IndexState::Partial;
        return;
    }
    retained.entries.resize(length, None);
    retained.state = if languages::has_incomplete_syntax(tree.elements) {
        IndexState::Partial
    } else {
        IndexState::Complete
    };
    for raw in (0..length).rev() {
        let slot = raw;
        let Ok(position) = u32::try_from(slot) else {
            retained.state = IndexState::Partial;
            break;
        };
        let at = Position(position);
        if category(tree, at) != Category::Expression {
            continue;
        }
        let (value, missing) = classify_expression(tree, retained, at);
        if missing {
            retained.state = IndexState::Partial;
        } else if let Some(entry) = retained.entries.get_mut(slot) {
            *entry = Some(value);
        } else {
            retained.state = IndexState::Partial;
            break;
        }
    }
}

/// Lazily compute and borrow the retained expression classifications.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
) -> Ref<'a, ExpressionTypeIndex> {
    if model.expression_types.borrow().state == IndexState::Fresh {
        build(tree, &mut model.expression_types.borrow_mut());
    }
    Ref::map(model.expression_types.borrow(), |index| index)
}
