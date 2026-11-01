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

pub(crate) fn clear(index: &mut ExpressionTypeIndex) {
    index.entries.clear();
    index.state = IndexState::Fresh;
}

fn atom(kind: &'static str) -> Resolution {
    Resolution::Atom(kind)
}
fn numeric(kind: &'static str) -> Resolution {
    atom(kind)
}

fn subtype(left: &str, right: &str) -> bool {
    left == right
        || matches!(
            (left, right),
            ("boolean", "integer" | "float" | "complex")
                | ("integer", "float" | "complex")
                | ("float", "complex")
        )
}

fn union(left: Resolution, right: Resolution) -> Resolution {
    match (left, right) {
        (Resolution::TypeError, _) | (_, Resolution::TypeError) => Resolution::TypeError,
        (Resolution::Unknown, _) | (_, Resolution::Unknown) => Resolution::Unknown,
        (Resolution::Atom(a), Resolution::Atom(b)) => {
            if subtype(a, b) {
                atom(b)
            } else if subtype(b, a) {
                atom(a)
            } else {
                Resolution::Union(vec![a, b])
            }
        }
        (Resolution::Atom(a), Resolution::Union(mut b)) => {
            if !b.iter().any(|candidate| subtype(a, candidate)) {
                b.push(a);
            }
            Resolution::Union(b)
        }
        (Resolution::Union(mut a), Resolution::Atom(b)) => {
            if !a.iter().any(|candidate| subtype(b, candidate)) {
                a.push(b);
            }
            Resolution::Union(a)
        }
        (Resolution::Union(mut a), Resolution::Union(b)) => {
            for candidate in b {
                if !a.iter().any(|known| subtype(candidate, known)) {
                    a.push(candidate);
                }
            }
            Resolution::Union(a)
        }
    }
}

fn child(tree: &Tree<'_>, at: Position, field: Field) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| {
        field_of(tree, *child) == field && category(tree, *child) == Category::Expression
    })
}

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

fn binary_operator(tree: &Tree<'_>, at: Position) -> Option<Node> {
    languages::children(tree.elements, at).find_map(|each| {
        if field_of(tree, each) != Field::Op {
            return None;
        }
        match kind(tree, each) {
            Kind::Token(Token::Plus) => Some(Node::Add),
            Kind::Token(Token::Minus) => Some(Node::Sub),
            Kind::Token(Token::Star) => Some(Node::Mult),
            Kind::Token(Token::At) => Some(Node::MatMult),
            Kind::Token(Token::Slash) => Some(Node::Div),
            Kind::Token(Token::Percent) => Some(Node::Mod),
            Kind::Token(Token::Doublestar) => Some(Node::Pow),
            Kind::Token(Token::Doubleslash) => Some(Node::FloorDiv),
            Kind::Token(Token::Amper) => Some(Node::BitAnd),
            Kind::Token(Token::Vbar) => Some(Node::BitOr),
            Kind::Token(Token::Circumflex) => Some(Node::BitXor),
            Kind::Token(Token::Leftshift) => Some(Node::LShift),
            Kind::Token(Token::Rightshift) => Some(Node::RShift),
            _ => None,
        }
    })
}

fn unary_operator(tree: &Tree<'_>, at: Position) -> Option<Node> {
    languages::children(tree.elements, at).find_map(|each| {
        if field_of(tree, each) != Field::Op {
            return None;
        }
        match kind(tree, each) {
            Kind::Token(Token::Plus) => Some(Node::UAdd),
            Kind::Token(Token::Minus) => Some(Node::USub),
            Kind::Token(Token::Tilde) => Some(Node::Invert),
            Kind::Token(Token::Name) if written(tree, each).0 == "not" => Some(Node::Not),
            _ => None,
        }
    })
}

fn binary(op: Node, left: Resolution, right: Resolution) -> Resolution {
    if op == Node::Mod {
        match left {
            Resolution::Atom("string") => return atom("string"),
            Resolution::Atom("bytes") => return atom("bytes"),
            _ => {}
        }
    }
    let (Resolution::Atom(a), Resolution::Atom(b)) = (&left, &right) else {
        return Resolution::Unknown;
    };
    let numeric_a = matches!(*a, "boolean" | "integer" | "float" | "complex");
    let numeric_b = matches!(*b, "boolean" | "integer" | "float" | "complex");
    let coerced = if *a == "complex" || *b == "complex" {
        "complex"
    } else if *a == "float" || *b == "float" {
        "float"
    } else {
        "integer"
    };
    match op {
        Node::Add => match (*a, *b) {
            ("string", "string") => atom("string"),
            ("bytes", "bytes") => atom("bytes"),
            ("list", "list") => atom("list"),
            ("tuple", "tuple") => atom("tuple"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        },
        Node::Sub => match (*a, *b) {
            ("set", "set") => atom("set"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        },
        Node::Mod if numeric_a && numeric_b => numeric(coerced),
        Node::Mod => Resolution::Unknown,
        Node::Mult => match (*a, *b) {
            ("string", "integer") | ("integer", "string") => atom("string"),
            _ if numeric_a && numeric_b => numeric(coerced),
            _ => Resolution::TypeError,
        },
        Node::FloorDiv | Node::Pow => {
            if numeric_a && numeric_b {
                numeric(coerced)
            } else {
                Resolution::TypeError
            }
        }
        Node::Div => {
            if numeric_a && numeric_b {
                numeric(if coerced == "integer" {
                    "float"
                } else {
                    coerced
                })
            } else {
                Resolution::TypeError
            }
        }
        Node::BitAnd | Node::BitOr | Node::BitXor | Node::LShift | Node::RShift => {
            if numeric_a && numeric_b && coerced == "integer" {
                atom("integer")
            } else {
                Resolution::TypeError
            }
        }
        _ => Resolution::Unknown,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the bounded pinned inference is one exhaustive arm per supported expression form"
)]
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
        Kind::Node(Node::TemplateStr) => Resolution::Unknown,
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
            Classification::NotPrimitive => Resolution::Unknown,
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
            match unary_operator(tree, at) {
                Some(Node::Not) => atom("boolean"),
                Some(Node::Invert) => match operand {
                    Resolution::Atom("boolean" | "integer") => atom("integer"),
                    Resolution::Atom(_) => Resolution::TypeError,
                    _ => Resolution::Unknown,
                },
                Some(Node::UAdd | Node::USub) => match operand {
                    Resolution::Atom("boolean") => atom("integer"),
                    Resolution::Atom(t @ ("integer" | "float" | "complex")) => atom(t),
                    Resolution::Atom(_) => Resolution::TypeError,
                    _ => Resolution::Unknown,
                },
                _ => Resolution::Unknown,
            }
        }
        Kind::Node(Node::BinOp) => {
            let left = child_resolution(index, child(tree, at, Field::Left));
            let right = child_resolution(index, child(tree, at, Field::Right));
            binary_operator(tree, at)
                .map(|operator| binary(operator, left, right))
                .unwrap_or(Resolution::Unknown)
        }
        _ => Resolution::Unknown,
    };
    (result, false)
}

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
        } else {
            retained.entries[slot] = Some(value);
        }
    }
}

pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
) -> Ref<'a, ExpressionTypeIndex> {
    if model.expression_types.borrow().state == IndexState::Fresh {
        build(tree, &mut model.expression_types.borrow_mut());
    }
    Ref::map(model.expression_types.borrow(), |index| index)
}
