//! The operator of a unary or binary expression, read from its operator token, and the basic type a supported binary operation gives
//! its two operand types.
use crate::domain::{expression_types::types::{BasicType, Resolution}, nodes::types::Node};


/// Infer a result for the supported binary operations.
pub(crate) fn binary(op: Node, left: &Resolution, right: &Resolution) -> Resolution {
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
