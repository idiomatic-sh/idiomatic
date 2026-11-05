//! The `labels` service: free functions named by verb that say what an element is and which role the grammar labels it with when its
//! kind alone decides: the tables of `rust.ungram`, with no IO and no ports.

use super::types::{Category, Field};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What an element is, as far as the roles it can play are concerned.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "an element that is no expression, pattern, type, item or statement (a token, a list, a name) plays no role by its category, so `Other` is right for every kind not named; `labels/service_test.rs` holds a row for each named category and the oracle rail compares the kinds of the Rust suite"
)]
pub(crate) fn category(element: Kind) -> Category {
    match element {
        Kind::Node(
            Node::ArrayExpr
            | Node::AsmExpr
            | Node::AwaitExpr
            | Node::BecomeExpr
            | Node::BinExpr
            | Node::BlockExpr
            | Node::BreakExpr
            | Node::CallExpr
            | Node::CastExpr
            | Node::ClosureExpr
            | Node::ContinueExpr
            | Node::FieldExpr
            | Node::FormatArgsExpr
            | Node::ForExpr
            | Node::IfExpr
            | Node::IncludeBytesExpr
            | Node::IndexExpr
            | Node::LetExpr
            | Node::Literal
            | Node::LoopExpr
            | Node::MacroExpr
            | Node::MatchExpr
            | Node::MethodCallExpr
            | Node::OffsetOfExpr
            | Node::ParenExpr
            | Node::PathExpr
            | Node::PrefixExpr
            | Node::RangeExpr
            | Node::RecordExpr
            | Node::RefExpr
            | Node::ReturnExpr
            | Node::TryExpr
            | Node::TupleExpr
            | Node::UnderscoreExpr
            | Node::WhileExpr
            | Node::YeetExpr
            | Node::YieldExpr,
        ) => Category::Expression,
        Kind::Node(
            Node::BoxPat
            | Node::ConstBlockPat
            | Node::DerefPat
            | Node::IdentPat
            | Node::LiteralPat
            | Node::MacroPat
            | Node::OrPat
            | Node::ParenPat
            | Node::PathPat
            | Node::RangePat
            | Node::RecordPat
            | Node::RefPat
            | Node::RestPat
            | Node::SlicePat
            | Node::TuplePat
            | Node::TupleStructPat
            | Node::WildcardPat,
        ) => Category::Pattern,
        Kind::Node(
            Node::ArrayType
            | Node::DynTraitType
            | Node::FnPtrType
            | Node::ForType
            | Node::ImplTraitType
            | Node::InferType
            | Node::MacroType
            | Node::NeverType
            | Node::ParenType
            | Node::PathType
            | Node::PtrType
            | Node::RefType
            | Node::SliceType
            | Node::TupleType,
        ) => Category::Type,
        Kind::Node(
            Node::Const
            | Node::Enum
            | Node::ExternBlock
            | Node::ExternCrate
            | Node::Fn
            | Node::Impl
            | Node::MacroCall
            | Node::MacroDef
            | Node::MacroRules
            | Node::Module
            | Node::Static
            | Node::Struct
            | Node::Trait
            | Node::TypeAlias
            | Node::Union
            | Node::Use,
        ) => Category::Item,
        Kind::Node(Node::ExprStmt | Node::LetStmt) => Category::Statement,
        _ => Category::Other,
    }
}

/// The role a token plays: the operator of a prefix, binary or range construct, or the keyword of a predicate.
pub(crate) fn token_role(parent_kind: Kind, element: Kind) -> Field {
    match (parent_kind, element) {
        (
            Kind::Node(Node::BinExpr),
            Kind::Token(
                Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::Shl
                | Token::Shr
                | Token::Caret
                | Token::Pipe
                | Token::Amp
                | Token::Pipe2
                | Token::Amp2
                | Token::Eq2
                | Token::Neq
                | Token::Lteq
                | Token::Gteq
                | Token::LAngle
                | Token::RAngle
                | Token::Eq
                | Token::Pluseq
                | Token::Minuseq
                | Token::Stareq
                | Token::Slasheq
                | Token::Percenteq
                | Token::Shleq
                | Token::Shreq
                | Token::Careteq
                | Token::Pipeeq
                | Token::Ampeq,
            ),
        )
        | (Kind::Node(Node::PrefixExpr), Kind::Token(Token::Minus | Token::Bang | Token::Star))
        | (
            Kind::Node(Node::RangeExpr | Node::RangePat),
            Kind::Token(Token::Dot2 | Token::Dot2eq),
        ) => Field::Op,
        (Kind::Node(Node::CfgAtom), Kind::Token(Token::Ident)) => Field::Keyword,
        _ => Field::None,
    }
}

/// The role of a node that the grammar names by the kind of the node itself: the accessor of its parent has that name.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the kinds the grammar names by an accessor of the parent have a role by their kind, so `None` is right for every kind not named; `labels/service_test.rs` holds a row for a kind with a role and one without, and the oracle rail compares the roles of every element of the Rust suite"
)]
pub(crate) fn kind_role(element: Kind) -> Field {
    match element {
        Kind::Node(Node::Name) => Field::Name,
        Kind::Node(Node::NameRef) => Field::NameRef,
        Kind::Node(Node::Path) => Field::Path,
        Kind::Node(Node::PathSegment) => Field::Segment,
        Kind::Node(Node::Visibility) => Field::Visibility,
        Kind::Node(Node::Attr) => Field::Attrs,
        Kind::Node(Node::Lifetime) => Field::Lifetime,
        Kind::Node(Node::Label) => Field::Label,
        Kind::Node(Node::Abi) => Field::Abi,
        Kind::Node(Node::Rename) => Field::Rename,
        Kind::Node(Node::GenericParamList) => Field::GenericParamList,
        Kind::Node(Node::GenericArgList) => Field::GenericArgList,
        Kind::Node(Node::WhereClause) => Field::WhereClause,
        Kind::Node(Node::ParamList) => Field::ParamList,
        Kind::Node(Node::RetType) => Field::RetType,
        Kind::Node(Node::ArgList) => Field::ArgList,
        Kind::Node(Node::ItemList) => Field::ItemList,
        Kind::Node(Node::AssocItemList) => Field::AssocItemList,
        Kind::Node(Node::RecordFieldList) => Field::RecordFieldList,
        Kind::Node(Node::TupleFieldList) => Field::TupleFieldList,
        Kind::Node(Node::VariantList) => Field::VariantList,
        Kind::Node(Node::MatchArmList) => Field::MatchArmList,
        Kind::Node(Node::StmtList) => Field::StmtList,
        Kind::Node(Node::UseTreeList) => Field::UseTreeList,
        Kind::Node(Node::RecordExprFieldList) => Field::RecordExprFieldList,
        Kind::Node(Node::RecordPatFieldList) => Field::RecordPatFieldList,
        Kind::Node(Node::UseTree) => Field::UseTree,
        Kind::Node(Node::LetElse) => Field::LetElse,
        _ => Field::None,
    }
}
