//! The `type_facts` service: free functions over the parser's own expressions. The basic type of an expression is read from its
//! own syntax and the types of its operands: a literal's from its payload, a display's and a comprehension's from what it
//! builds, an operation's from Python's numeric tower, a conditional's and a boolean operation's as the union of its branches.
//! The type algebra (inclusion, union, binary operations) is the `expression_types` concept's.

use crate::domain::expression_types::constants::literal_type;
use crate::domain::expression_types::inclusion::service::union;
use crate::domain::expression_types::operators::service::binary;
use crate::domain::expression_types::types::{BasicType, Resolution};
use crate::domain::literal_facts::service::properties;
use crate::domain::node::service::id_of;
use crate::domain::node::types::SyntaxFile;
use crate::domain::nodes::types::Node;
use ruff_python_ast::{AnyNodeRef, Expr, Operator, UnaryOp};

/// The operator of a binary operation, as the standard names it.
const fn operator(written: Operator) -> Node {
    match written {
        Operator::Add => Node::Add,
        Operator::Sub => Node::Sub,
        Operator::Mult => Node::Mult,
        Operator::MatMult => Node::MatMult,
        Operator::Div => Node::Div,
        Operator::Mod => Node::Mod,
        Operator::Pow => Node::Pow,
        Operator::LShift => Node::LShift,
        Operator::RShift => Node::RShift,
        Operator::BitOr => Node::BitOr,
        Operator::BitXor => Node::BitXor,
        Operator::BitAnd => Node::BitAnd,
        Operator::FloorDiv => Node::FloorDiv,
    }
}

/// The type a unary operation gives its operand's type.
fn unary(written: UnaryOp, operand: &Resolution) -> Resolution {
    match (written, operand) {
        (UnaryOp::Not, _) => Resolution::Atom(BasicType::Boolean),
        (UnaryOp::Invert, Resolution::Atom(BasicType::Boolean | BasicType::Integer))
        | (UnaryOp::UAdd | UnaryOp::USub, Resolution::Atom(BasicType::Boolean)) => {
            Resolution::Atom(BasicType::Integer)
        }
        (
            UnaryOp::UAdd | UnaryOp::USub,
            Resolution::Atom(
                numeric @ (BasicType::Integer | BasicType::Float | BasicType::Complex),
            ),
        ) => Resolution::Atom(*numeric),
        (UnaryOp::Invert | UnaryOp::UAdd | UnaryOp::USub, Resolution::Atom(_)) => {
            Resolution::TypeError
        }
        (
            UnaryOp::Invert | UnaryOp::UAdd | UnaryOp::USub,
            Resolution::Union(_) | Resolution::Unknown | Resolution::TypeError,
        ) => Resolution::Unknown,
    }
}

/// The basic type of `node` from its own syntax, its operands' types read from `known` by their ids; unknown for any node the
/// syntax alone gives no type.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the expressions whose syntax gives a type are named; a literal is asked of its payload and every other node is unknown"
)]
#[must_use]
pub(crate) fn resolution(node: AnyNodeRef<'_>, known: &[Resolution]) -> Resolution {
    let of = |operand: &Expr| {
        id_of(operand)
            .and_then(|id| known.get(usize::from(id)))
            .cloned()
            .unwrap_or(Resolution::Unknown)
    };
    match node {
        AnyNodeRef::ExprDict(_) | AnyNodeRef::ExprDictComp(_) => Resolution::Atom(BasicType::Dict),
        AnyNodeRef::ExprSet(_) | AnyNodeRef::ExprSetComp(_) => Resolution::Atom(BasicType::Set),
        AnyNodeRef::ExprList(_) | AnyNodeRef::ExprListComp(_) => Resolution::Atom(BasicType::List),
        AnyNodeRef::ExprTuple(_) => Resolution::Atom(BasicType::Tuple),
        AnyNodeRef::ExprGenerator(_) => Resolution::Atom(BasicType::Generator),
        AnyNodeRef::ExprFString(_) | AnyNodeRef::InterpolatedStringFormatSpec(_) => {
            Resolution::Atom(BasicType::String)
        }
        AnyNodeRef::ExprNamed(named) => of(&named.value),
        AnyNodeRef::ExprIf(conditional) => union(of(&conditional.body), of(&conditional.orelse)),
        AnyNodeRef::ExprBoolOp(operation) => {
            let mut values = operation.values.iter().map(of);
            values
                .next()
                .map_or(Resolution::Unknown, |first| values.fold(first, union))
        }
        AnyNodeRef::ExprUnaryOp(operation) => unary(operation.op, &of(&operation.operand)),
        AnyNodeRef::ExprBinOp(operation) => binary(
            operator(operation.op),
            &of(&operation.left),
            &of(&operation.right),
        ),
        other => properties(other).map_or(Resolution::Unknown, |literal| {
            Resolution::Atom(literal_type(literal.kind))
        }),
    }
}

/// Writes the basic type of every syntax node of `file` into `types`, by id: one pass from the last node to the first, so an
/// operand's type is known before the operation's is asked.
pub(crate) fn resolutions(file: &SyntaxFile<'_>, types: &mut Vec<Resolution>) {
    types.clear();
    types.resize(file.nodes.len(), Resolution::Unknown);
    for (id, node) in file.nodes.iter().enumerate().rev() {
        let found = resolution(*node, types);
        if let Some(slot) = types.get_mut(id) {
            *slot = found;
        }
    }
}
