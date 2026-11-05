//! The `fields` service: free functions named by verb that say the role of an element under its parent. A role is read from the
//! kinds of the parent and the element and, where one kind plays two roles, from where the element stands among its siblings, as the
//! labels of `rust.ungram` and the accessors of its typed syntax tree distinguish them.

use crate::domain::nodes::types::RustNode;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};
use strum::IntoEnumIterator as _;

use crate::domain::labels::service::{category, kind_role, token_role};
use crate::domain::labels::types::{Category, Closing, Field, Following, Precedence};
use crate::domain::syntax::service::{child, is_root, is_token, kind, parent};
use crate::domain::syntax::types::{Kind, Leaf, Rootness};

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: Field) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: Field) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = Field> {
    Field::iter()
}

/// Whether `at` comes after the first token of kind `marker` among the children of its parent.
fn after(tree: &Tree<'_>, at: Position, marker: Kind) -> Following {
    for each in languages::children(tree.elements, parent(tree, at)) {
        if each == at {
            return Following::Before;
        }
        if is_token(tree, each) == Leaf::Token && kind(tree, each) == marker {
            return Following::After;
        }
    }
    Following::Before
}

/// Whether the parent of `at` has no node after it: the tokens that close it (a brace, a semicolon) do not count.
fn closing(tree: &Tree<'_>, at: Position) -> Closing {
    let followed = languages::children(tree.elements, parent(tree, at))
        .skip_while(|each| *each != at)
        .skip(1)
        .any(|each| is_token(tree, each) == Leaf::Node);
    if followed {
        Closing::NotLast
    } else {
        Closing::Last
    }
}

/// Whether a node other than an attribute stands before `at` under its parent.
fn nodes_before(tree: &Tree<'_>, at: Position) -> Precedence {
    let before = languages::children(tree.elements, parent(tree, at))
        .take_while(|each| *each != at)
        .filter(|each| {
            is_token(tree, *each) == Leaf::Node && kind(tree, *each) != Kind::Node(RustNode::Attr)
        })
        .count();
    if before == 0 {
        Precedence::Alone
    } else {
        Precedence::Preceded
    }
}

/// The role of a node that a parent of this kind distinguishes by where the node stands or what it is: the labels of the grammar.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parent kind the grammar gives no positional label to leaves its children with no role here, and `kind_role` then decides by the child's own kind, so the default is right for every kind not named; `fields/service_test.rs` holds a row for each role this function gives, and the walk and oracle rails compare the role of every element of the Rust suite, so a kind that needs a role fails there"
)]
fn positional_role(tree: &Tree<'_>, at: Position, parent_kind: Kind, element: Kind) -> Field {
    let what = category(element);
    match parent_kind {
        Kind::Node(
            RustNode::SourceFile | RustNode::ItemList | RustNode::AssocItemList | RustNode::ExternItemList,
        ) => match what {
            Category::Item => Field::Items,
            _ => Field::None,
        },
        Kind::Node(RustNode::StmtList) => match what {
            Category::Expression if closing(tree, at) == Closing::Last => Field::TailExpr,
            Category::Statement | Category::Item | Category::Expression => Field::Statements,
            _ => Field::None,
        },
        Kind::Node(RustNode::BinExpr) if what == Category::Expression => match nodes_before(tree, at) {
            Precedence::Alone => Field::Lhs,
            Precedence::Preceded => Field::Rhs,
        },
        Kind::Node(RustNode::IndexExpr) if what == Category::Expression => match nodes_before(tree, at)
        {
            Precedence::Alone => Field::Base,
            Precedence::Preceded => Field::Index,
        },
        Kind::Node(RustNode::MethodCallExpr) if what == Category::Expression => Field::Receiver,
        Kind::Node(RustNode::LetStmt) => match what {
            Category::Pattern => Field::Pat,
            Category::Type => Field::Ty,
            Category::Expression => Field::Initializer,
            _ => Field::None,
        },
        Kind::Node(RustNode::RangeExpr | RustNode::RangePat) if what != Category::Other => {
            let marker = if parent_kind == Kind::Node(RustNode::RangeExpr)
                && after(tree, at, Kind::Token(Token::Dot2)) == Following::After
            {
                Following::After
            } else {
                after(tree, at, Kind::Token(Token::Dot2eq))
            };
            if marker == Following::After {
                Field::End
            } else {
                Field::Start
            }
        }
        _ => branch_role(tree, at, parent_kind, element),
    }
}

/// The role of an element under a branch or a loop: the condition, the branches and the body of an `if`, a `while`, a `loop`, a
/// `for` and a `match`. The parents `positional_role` leaves to it, read in the same order.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parent kind the grammar gives no positional label to leaves its children with no role here, and `kind_role` then decides by the child's own kind, so the default is right for every kind not named; `fields/service_test.rs` holds a row for each role this function gives, and the walk and oracle rails compare the role of every element of the Rust suite, so a kind that needs a role fails there"
)]
fn branch_role(tree: &Tree<'_>, at: Position, parent_kind: Kind, element: Kind) -> Field {
    let what = category(element);
    match parent_kind {
        Kind::Node(RustNode::IfExpr) => match what {
            Category::Expression
                if after(tree, at, Kind::Token(Token::ElseKw)) == Following::After =>
            {
                Field::ElseBranch
            }
            Category::Expression if nodes_before(tree, at) == Precedence::Alone => Field::Condition,
            Category::Expression => Field::ThenBranch,
            _ => Field::None,
        },
        Kind::Node(RustNode::WhileExpr) if what == Category::Expression => {
            if closing(tree, at) == Closing::Last {
                Field::LoopBody
            } else {
                Field::Condition
            }
        }
        Kind::Node(RustNode::LoopExpr) if element == Kind::Node(RustNode::BlockExpr) => Field::LoopBody,
        Kind::Node(RustNode::ForExpr) => match what {
            Category::Pattern => Field::Pat,
            Category::Expression if closing(tree, at) == Closing::Last => Field::LoopBody,
            Category::Expression => Field::Iterable,
            _ => Field::None,
        },
        Kind::Node(RustNode::MatchExpr) if what == Category::Expression => Field::Expr,
        Kind::Node(RustNode::MatchArm) => match what {
            Category::Pattern => Field::Pat,
            Category::Expression => Field::Expr,
            _ if element == Kind::Node(RustNode::MatchGuard) => Field::Guard,
            _ => Field::None,
        },
        Kind::Node(RustNode::MatchArmList) if element == Kind::Node(RustNode::MatchArm) => Field::Arms,
        Kind::Node(RustNode::MatchGuard) if what == Category::Expression => Field::Condition,
        _ => member_role(tree, at, parent_kind, element),
    }
}

/// The role of an element under a parent that lists members: the fields of a struct literal, a declaration or a pattern, the
/// variants of an enum, the elements of a tuple or an array. The parents `branch_role` leaves to it, read in the same order.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parent kind the grammar gives no positional label to leaves its children with no role here, and `kind_role` then decides by the child's own kind, so the default is right for every kind not named; `fields/service_test.rs` holds a row for each role this function gives, and the walk and oracle rails compare the role of every element of the Rust suite, so a kind that needs a role fails there"
)]
fn member_role(tree: &Tree<'_>, at: Position, parent_kind: Kind, element: Kind) -> Field {
    let what = category(element);
    match parent_kind {
        Kind::Node(RustNode::RecordExprFieldList) => match element {
            Kind::Node(RustNode::RecordExprField) => Field::Fields,
            _ if what == Category::Expression => Field::Spread,
            _ => Field::None,
        },
        Kind::Node(RustNode::RecordFieldList) if element == Kind::Node(RustNode::RecordField) => {
            Field::Fields
        }
        Kind::Node(RustNode::TupleFieldList) if element == Kind::Node(RustNode::TupleField) => {
            Field::Fields
        }
        Kind::Node(RustNode::RecordPatFieldList) if element == Kind::Node(RustNode::RecordPatField) => {
            Field::Fields
        }
        Kind::Node(RustNode::VariantList) if element == Kind::Node(RustNode::Variant) => Field::Variants,
        Kind::Node(
            RustNode::TupleExpr
            | RustNode::ArrayExpr
            | RustNode::TuplePat
            | RustNode::TupleType
            | RustNode::TupleStructPat,
        ) if what != Category::Other => Field::Fields,
        _ => declaration_role(tree, at, parent_kind, element),
    }
}

/// The role of an element under a parent of an argument, parameter or generic list, a declaration, a binding or a path: the
/// parents `member_role` leaves to it, read in the same order.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parent kind the grammar gives no positional label to leaves its children with no role here, and `kind_role` then decides by the child's own kind, so the default is right for every kind not named; `fields/service_test.rs` holds a row for each role this function gives, and the walk and oracle rails compare the role of every element of the Rust suite, so a kind that needs a role fails there"
)]
fn declaration_role(tree: &Tree<'_>, at: Position, parent_kind: Kind, element: Kind) -> Field {
    let what = category(element);
    match parent_kind {
        Kind::Node(RustNode::ArgList) if what == Category::Expression => Field::Args,
        Kind::Node(RustNode::ParamList)
            if matches!(element, Kind::Node(RustNode::Param | RustNode::SelfParam)) =>
        {
            Field::Params
        }
        Kind::Node(RustNode::GenericParamList)
            if matches!(
                element,
                Kind::Node(RustNode::TypeParam | RustNode::ConstParam | RustNode::LifetimeParam)
            ) =>
        {
            Field::Params
        }
        Kind::Node(RustNode::GenericArgList)
            if matches!(
                element,
                Kind::Node(RustNode::TypeArg | RustNode::ConstArg | RustNode::LifetimeArg | RustNode::AssocTypeArg)
            ) =>
        {
            Field::Args
        }
        Kind::Node(RustNode::WhereClause) if element == Kind::Node(RustNode::WherePred) => {
            Field::Predicates
        }
        Kind::Node(RustNode::TypeBoundList) if element == Kind::Node(RustNode::TypeBound) => Field::Bounds,
        Kind::Node(RustNode::Fn) if element == Kind::Node(RustNode::BlockExpr) => Field::Body,
        Kind::Node(RustNode::Const | RustNode::Static) => match what {
            Category::Type => Field::Ty,
            Category::Expression => Field::Body,
            _ => Field::None,
        },
        Kind::Node(RustNode::ClosureExpr) if what == Category::Expression => Field::Body,
        Kind::Node(RustNode::MacroDef) if element == Kind::Node(RustNode::TokenTree) => {
            if nodes_before_of_kind(tree, at, Kind::Node(RustNode::TokenTree)) == Precedence::Alone {
                Field::Args
            } else {
                Field::Body
            }
        }
        Kind::Node(RustNode::MacroRules) if element == Kind::Node(RustNode::TokenTree) => Field::Body,
        Kind::Node(RustNode::Impl) if what == Category::Type => {
            if after(tree, at, Kind::Token(Token::ForKw)) == Following::After
                || child(tree, parent(tree, at), Kind::Token(Token::ForKw)).is_none()
            {
                Field::SelfTy
            } else {
                Field::Trait
            }
        }
        Kind::Node(RustNode::TypeParam) if what == Category::Type => Field::DefaultType,
        Kind::Node(RustNode::ConstParam) => match element {
            Kind::Node(RustNode::ConstArg) => Field::DefaultVal,
            _ if what == Category::Type => Field::Ty,
            _ => Field::None,
        },
        _ => binding_role(parent_kind, element),
    }
}

/// The role of an element under a parameter, a field, a binding pattern, an expression that holds one more, or a path: the
/// parents `declaration_role` leaves to it, read in the same order.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a parent kind the grammar gives no positional label to leaves its children with no role here, and `kind_role` then decides by the child's own kind, so the default is right for every kind not named; `fields/service_test.rs` holds a row for each role this function gives, and the walk and oracle rails compare the role of every element of the Rust suite, so a kind that needs a role fails there"
)]
fn binding_role(parent_kind: Kind, element: Kind) -> Field {
    let what = category(element);
    match parent_kind {
        Kind::Node(
            RustNode::Param
            | RustNode::SelfParam
            | RustNode::RecordField
            | RustNode::TupleField
            | RustNode::TypeAlias
            | RustNode::CastExpr
            | RustNode::RefType
            | RustNode::PtrType
            | RustNode::SliceType
            | RustNode::ArrayType
            | RustNode::ParenType,
        ) if what == Category::Type => Field::Ty,
        Kind::Node(
            RustNode::Param | RustNode::RecordPatField | RustNode::RefPat | RustNode::BoxPat | RustNode::ParenPat,
        ) if what == Category::Pattern => Field::Pat,
        Kind::Node(
            RustNode::Variant
            | RustNode::RecordExprField
            | RustNode::FieldExpr
            | RustNode::CallExpr
            | RustNode::PrefixExpr
            | RustNode::RefExpr
            | RustNode::TryExpr
            | RustNode::AwaitExpr
            | RustNode::CastExpr
            | RustNode::ParenExpr
            | RustNode::ReturnExpr
            | RustNode::BreakExpr
            | RustNode::ExprStmt,
        ) if what == Category::Expression => Field::Expr,
        Kind::Node(
            RustNode::PathExpr
            | RustNode::PathType
            | RustNode::PathPat
            | RustNode::TupleStructPat
            | RustNode::RecordPat
            | RustNode::RecordExpr
            | RustNode::MacroCall
            | RustNode::PathMeta
            | RustNode::UseTree,
        ) if element == Kind::Node(RustNode::Path) => Field::Path,
        Kind::Node(RustNode::Path) if element == Kind::Node(RustNode::Path) => Field::Qualifier,
        _ => Field::None,
    }
}

/// Whether a node of kind `wanted` stands before `at` under its parent.
fn nodes_before_of_kind(tree: &Tree<'_>, at: Position, wanted: Kind) -> Precedence {
    let before = languages::children(tree.elements, parent(tree, at))
        .take_while(|each| *each != at)
        .filter(|each| kind(tree, *each) == wanted)
        .count();
    if before == 0 {
        Precedence::Alone
    } else {
        Precedence::Preceded
    }
}

/// The role of the element at `at` under its parent; `None` for the root and for an edge the grammar gives no role.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: Position) -> Field {
    if is_root(tree, at) == Rootness::Root {
        return Field::None;
    }
    let (parent_kind, element) = (kind(tree, parent(tree, at)), kind(tree, at));
    if is_token(tree, at) == Leaf::Token {
        return token_role(parent_kind, element);
    }
    let role = positional_role(tree, at, parent_kind, element);
    if matches!(role, Field::None) {
        kind_role(element)
    } else {
        role
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
