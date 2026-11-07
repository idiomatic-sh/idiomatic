//! The `nodes` service: free functions named by verb over the domain's types, with no IO and no ports. It says a node's name and
//! its category, the ASDL type it is a constructor of.

#[cfg(any(test, feature = "fixtures"))]
use super::constants::STANDARD;
use super::types::{Category, Node};
#[cfg(any(test, feature = "fixtures"))]
use ryo::languages::NameText;

/// The standard the node names come from, and its version.
#[cfg(any(test, feature = "fixtures"))]
#[must_use]
pub(crate) fn standard() -> NameText<'static> {
    NameText(STANDARD)
}

/// The standard's category of a node. The match has no wildcard arm, so a node the standard gains is placed before it compiles.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each category, naming every node of the standard once: the length is the standard's"
)]
pub(crate) const fn category(node: Node) -> Category {
    match node {
        Node::Module | Node::Interactive | Node::Expression | Node::FunctionType => {
            Category::Module
        }
        Node::FunctionDef
        | Node::AsyncFunctionDef
        | Node::ClassDef
        | Node::Return
        | Node::Delete
        | Node::Assign
        | Node::TypeAlias
        | Node::AugAssign
        | Node::AnnAssign
        | Node::For
        | Node::AsyncFor
        | Node::While
        | Node::If
        | Node::With
        | Node::AsyncWith
        | Node::Match
        | Node::Raise
        | Node::Try
        | Node::TryStar
        | Node::Assert
        | Node::Import
        | Node::ImportFrom
        | Node::Global
        | Node::Nonlocal
        | Node::Expr
        | Node::Pass
        | Node::Break
        | Node::Continue => Category::Statement,
        Node::BoolOp
        | Node::NamedExpr
        | Node::BinOp
        | Node::UnaryOp
        | Node::Lambda
        | Node::IfExp
        | Node::Dict
        | Node::Set
        | Node::ListComp
        | Node::SetComp
        | Node::DictComp
        | Node::GeneratorExp
        | Node::Await
        | Node::Yield
        | Node::YieldFrom
        | Node::Compare
        | Node::Call
        | Node::FormattedValue
        | Node::Interpolation
        | Node::JoinedStr
        | Node::TemplateStr
        | Node::Constant
        | Node::Attribute
        | Node::Subscript
        | Node::Starred
        | Node::Name
        | Node::List
        | Node::Tuple
        | Node::Slice => Category::Expression,
        Node::MatchValue
        | Node::MatchSingleton
        | Node::MatchSequence
        | Node::MatchMapping
        | Node::MatchClass
        | Node::MatchStar
        | Node::MatchAs
        | Node::MatchOr => Category::Pattern,
        Node::Comprehension
        | Node::ExceptHandler
        | Node::Arguments
        | Node::Arg
        | Node::Keyword
        | Node::Alias
        | Node::Withitem
        | Node::MatchCase
        | Node::TypeVar
        | Node::ParamSpec
        | Node::TypeVarTuple
        | Node::TypeIgnore => Category::Part,
        Node::Load
        | Node::Store
        | Node::Del
        | Node::And
        | Node::Or
        | Node::Add
        | Node::Sub
        | Node::Mult
        | Node::MatMult
        | Node::Div
        | Node::Mod
        | Node::Pow
        | Node::LShift
        | Node::RShift
        | Node::BitOr
        | Node::BitXor
        | Node::BitAnd
        | Node::FloorDiv
        | Node::Invert
        | Node::Not
        | Node::UAdd
        | Node::USub
        | Node::Eq
        | Node::NotEq
        | Node::Lt
        | Node::LtE
        | Node::Gt
        | Node::GtE
        | Node::Is
        | Node::IsNot
        | Node::In
        | Node::NotIn => Category::Operator,
    }
}
