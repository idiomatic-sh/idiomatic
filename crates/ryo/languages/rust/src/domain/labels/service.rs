//! The `labels` service: free functions named by verb that say what an element is and which role the grammar labels it with when its
//! kind alone decides: the tables of `rust.ungram`, with no IO and no ports.

use super::types::{Category, Field};
use crate::domain::nodes::types::RustNode;
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
            RustNode::ArrayExpr
            | RustNode::AsmExpr
            | RustNode::AwaitExpr
            | RustNode::BecomeExpr
            | RustNode::BinExpr
            | RustNode::BlockExpr
            | RustNode::BreakExpr
            | RustNode::CallExpr
            | RustNode::CastExpr
            | RustNode::ClosureExpr
            | RustNode::ContinueExpr
            | RustNode::FieldExpr
            | RustNode::FormatArgsExpr
            | RustNode::ForExpr
            | RustNode::IfExpr
            | RustNode::IncludeBytesExpr
            | RustNode::IndexExpr
            | RustNode::LetExpr
            | RustNode::Literal
            | RustNode::LoopExpr
            | RustNode::MacroExpr
            | RustNode::MatchExpr
            | RustNode::MethodCallExpr
            | RustNode::OffsetOfExpr
            | RustNode::ParenExpr
            | RustNode::PathExpr
            | RustNode::PrefixExpr
            | RustNode::RangeExpr
            | RustNode::RecordExpr
            | RustNode::RefExpr
            | RustNode::ReturnExpr
            | RustNode::TryExpr
            | RustNode::TupleExpr
            | RustNode::UnderscoreExpr
            | RustNode::WhileExpr
            | RustNode::YeetExpr
            | RustNode::YieldExpr,
        ) => Category::Expression,
        Kind::Node(
            RustNode::BoxPat
            | RustNode::ConstBlockPat
            | RustNode::DerefPat
            | RustNode::IdentPat
            | RustNode::LiteralPat
            | RustNode::MacroPat
            | RustNode::OrPat
            | RustNode::ParenPat
            | RustNode::PathPat
            | RustNode::RangePat
            | RustNode::RecordPat
            | RustNode::RefPat
            | RustNode::RestPat
            | RustNode::SlicePat
            | RustNode::TuplePat
            | RustNode::TupleStructPat
            | RustNode::WildcardPat,
        ) => Category::Pattern,
        Kind::Node(
            RustNode::ArrayType
            | RustNode::DynTraitType
            | RustNode::FnPtrType
            | RustNode::ForType
            | RustNode::ImplTraitType
            | RustNode::InferType
            | RustNode::MacroType
            | RustNode::NeverType
            | RustNode::ParenType
            | RustNode::PathType
            | RustNode::PtrType
            | RustNode::RefType
            | RustNode::SliceType
            | RustNode::TupleType,
        ) => Category::Type,
        Kind::Node(
            RustNode::Const
            | RustNode::Enum
            | RustNode::ExternBlock
            | RustNode::ExternCrate
            | RustNode::Fn
            | RustNode::Impl
            | RustNode::MacroCall
            | RustNode::MacroDef
            | RustNode::MacroRules
            | RustNode::Module
            | RustNode::Static
            | RustNode::Struct
            | RustNode::Trait
            | RustNode::TypeAlias
            | RustNode::Union
            | RustNode::Use,
        ) => Category::Item,
        Kind::Node(RustNode::ExprStmt | RustNode::LetStmt) => Category::Statement,
        _ => Category::Other,
    }
}

/// The role a token plays: the operator of a prefix, binary or range construct, or the keyword of a predicate.
pub(crate) fn token_role(parent_kind: Kind, element: Kind) -> Field {
    match (parent_kind, element) {
        (
            Kind::Node(RustNode::BinExpr),
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
        | (Kind::Node(RustNode::PrefixExpr), Kind::Token(Token::Minus | Token::Bang | Token::Star))
        | (
            Kind::Node(RustNode::RangeExpr | RustNode::RangePat),
            Kind::Token(Token::Dot2 | Token::Dot2eq),
        ) => Field::Op,
        (Kind::Node(RustNode::CfgAtom), Kind::Token(Token::Ident)) => Field::Keyword,
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
        Kind::Node(RustNode::Name) => Field::Name,
        Kind::Node(RustNode::NameRef) => Field::NameRef,
        Kind::Node(RustNode::Path) => Field::Path,
        Kind::Node(RustNode::PathSegment) => Field::Segment,
        Kind::Node(RustNode::Visibility) => Field::Visibility,
        Kind::Node(RustNode::Attr) => Field::Attrs,
        Kind::Node(RustNode::Lifetime) => Field::Lifetime,
        Kind::Node(RustNode::Label) => Field::Label,
        Kind::Node(RustNode::Abi) => Field::Abi,
        Kind::Node(RustNode::Rename) => Field::Rename,
        Kind::Node(RustNode::GenericParamList) => Field::GenericParamList,
        Kind::Node(RustNode::GenericArgList) => Field::GenericArgList,
        Kind::Node(RustNode::WhereClause) => Field::WhereClause,
        Kind::Node(RustNode::ParamList) => Field::ParamList,
        Kind::Node(RustNode::RetType) => Field::RetType,
        Kind::Node(RustNode::ArgList) => Field::ArgList,
        Kind::Node(RustNode::ItemList) => Field::ItemList,
        Kind::Node(RustNode::AssocItemList) => Field::AssocItemList,
        Kind::Node(RustNode::RecordFieldList) => Field::RecordFieldList,
        Kind::Node(RustNode::TupleFieldList) => Field::TupleFieldList,
        Kind::Node(RustNode::VariantList) => Field::VariantList,
        Kind::Node(RustNode::MatchArmList) => Field::MatchArmList,
        Kind::Node(RustNode::StmtList) => Field::StmtList,
        Kind::Node(RustNode::UseTreeList) => Field::UseTreeList,
        Kind::Node(RustNode::RecordExprFieldList) => Field::RecordExprFieldList,
        Kind::Node(RustNode::RecordPatFieldList) => Field::RecordPatFieldList,
        Kind::Node(RustNode::UseTree) => Field::UseTree,
        Kind::Node(RustNode::LetElse) => Field::LetElse,
        _ => Field::None,
    }
}
