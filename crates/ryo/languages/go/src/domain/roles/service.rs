//! The `roles` service: the role of an element under its parent, the field of `go/ast` that holds it, read from the kinds of the
//! two and from where the element stands among its siblings; a token's is the `token.Pos` field that says where it is. A function's
//! name and receiver, which `go/ast` holds in `FuncDecl`, are under its `FuncType` (which starts at `func`) and keep those roles.

use ryo::languages::{self, Count, Position, Tree};
use strum::IntoEnumIterator as _;

use super::types::{AstField, Holding};
use crate::domain::nodes::types::Node;
use crate::domain::siblings::service::{
    among, before, first, last, matching, nodes_after, opening, previous, siblings,
};
use crate::domain::siblings::types::{Match, Standing, Wanted};
use crate::domain::syntax::service::{is_root, is_token, kind, parent};
use crate::domain::syntax::types::{Kind, Leaf, Rootness};
use crate::domain::tokens::types::Token;

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: AstField) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: AstField) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = AstField> {
    AstField::iter()
}

/// Which of two roles an element plays: `then` when `holding` holds of it, `other` else.
const fn either(holding: Holding, then: AstField, other: AstField) -> AstField {
    match holding {
        Holding::Holds => then,
        Holding::Fails => other,
    }
}

/// Whether `count` is none.
fn none(count: Count) -> Holding {
    if count == Count(0) {
        Holding::Holds
    } else {
        Holding::Fails
    }
}

/// Whether the element at `at` is a node of kind `wanted`.
fn is(tree: &Tree<'_>, at: Position, wanted: Node) -> Holding {
    if kind(tree, at) == Kind::Node(wanted) {
        Holding::Holds
    } else {
        Holding::Fails
    }
}

/// Whether no node comes before `at` among its siblings.
fn leading(tree: &Tree<'_>, at: Position) -> Holding {
    none(before(tree, at, Wanted::Node))
}

/// Whether no token `wanted` comes before `at` among its siblings.
fn ahead_of(tree: &Tree<'_>, at: Position, wanted: Token) -> Holding {
    none(before(tree, at, Wanted::Token(wanted)))
}

/// The role of the token `spelled` at `at` under a node of kind `holder`: the field of the node that says where it is.
fn token_role(tree: &Tree<'_>, at: Position, holder: Node, spelled: Token) -> AstField {
    let assigns = matching(Wanted::Assigning, Kind::Token(spelled)) == Match::Wanted;
    match (holder, spelled) {
        (Node::Ident, _) => AstField::Name,
        (Node::BasicLit, _) => AstField::Value,
        (Node::BinaryExpr | Node::UnaryExpr, _) => AstField::Op,
        (Node::AssignStmt, _) if assigns => AstField::Tok,
        (Node::BranchStmt, Token::Break | Token::Continue | Token::Goto | Token::Fallthrough)
        | (Node::GenDecl, Token::Import | Token::Const | Token::Type | Token::Var)
        | (Node::IncDecStmt, Token::Inc | Token::Dec)
        | (Node::RangeStmt, Token::Define | Token::Assign) => AstField::Tok,
        (Node::BlockStmt | Node::CompositeLit, Token::Lbrace) => AstField::Lbrace,
        (Node::BlockStmt | Node::CompositeLit, Token::Rbrace) => AstField::Rbrace,
        (
            Node::CallExpr | Node::GenDecl | Node::ParenExpr | Node::TypeAssertExpr,
            Token::Lparen,
        ) => AstField::Lparen,
        (
            Node::CallExpr | Node::GenDecl | Node::ParenExpr | Node::TypeAssertExpr,
            Token::Rparen,
        ) => AstField::Rparen,
        (
            Node::ArrayType | Node::IndexExpr | Node::IndexListExpr | Node::SliceExpr,
            Token::Lbrack,
        ) => AstField::Lbrack,
        (Node::IndexExpr | Node::IndexListExpr | Node::SliceExpr, Token::Rbrack) => {
            AstField::Rbrack
        }
        (Node::CallExpr | Node::Ellipsis, Token::Ellipsis) => AstField::Ellipsis,
        (Node::CaseClause | Node::CommClause, Token::Case | Token::Default) => AstField::Case,
        (
            Node::CaseClause | Node::CommClause | Node::KeyValueExpr | Node::LabeledStmt,
            Token::Colon,
        ) => AstField::Colon,
        // `chan T` and `chan<- T` begin at `chan`, `<-chan T` at its arrow, which `Arrow` names too.
        (Node::ChanType, Token::Chan | Token::Arrow) if first(tree, at) == Standing::Edge => {
            AstField::Begin
        }
        (Node::ChanType | Node::SendStmt, Token::Arrow) => AstField::Arrow,
        (Node::FieldList, Token::Lparen | Token::Lbrace | Token::Lbrack)
            if first(tree, at) == Standing::Edge =>
        {
            AstField::Opening
        }
        (Node::FieldList, Token::Rparen | Token::Rbrace | Token::Rbrack)
            if last(tree, at) == Standing::Edge =>
        {
            AstField::Closing
        }
        (Node::DeferStmt, Token::Defer) => AstField::Defer,
        (Node::GoStmt, Token::Go) => AstField::Go,
        (Node::File, Token::Package) => AstField::Package,
        (Node::ForStmt | Node::RangeStmt, Token::For) => AstField::For,
        (Node::RangeStmt, Token::Range) => AstField::Range,
        (Node::FuncType, Token::Func) => AstField::Func,
        (Node::IfStmt, Token::If) => AstField::If,
        (Node::InterfaceType, Token::Interface) => AstField::Interface,
        (Node::StructType, Token::Struct) => AstField::Struct,
        (Node::MapType, Token::Map) => AstField::Map,
        (Node::ReturnStmt, Token::Return) => AstField::Return,
        (Node::SelectStmt, Token::Select) => AstField::Select,
        (Node::SwitchStmt | Node::TypeSwitchStmt, Token::Switch) => AstField::Switch,
        (Node::StarExpr, Token::Mul) => AstField::Star,
        (Node::TypeSpec, Token::Assign) => AstField::Assign,
        (Node::EmptyStmt, Token::Semicolon) => AstField::Semicolon,
        _ => AstField::None,
    }
}

/// The role of a node of a function type: `FuncDecl`'s receiver and name, then type parameters, parameters and results.
fn signature_role(tree: &Tree<'_>, at: Position) -> AstField {
    let named = among(tree, at, Wanted::NodeOf(Node::Ident)) > Count(0);
    if is(tree, at, Node::Ident) == Holding::Holds {
        return AstField::Name;
    }
    if named && before(tree, at, Wanted::NodeOf(Node::Ident)) == Count(0) {
        return AstField::Recv;
    }
    if opening(tree, at) == Some(Token::Lbrack) {
        return AstField::TypeParams;
    }
    let unbracketed = |each: &Position| {
        kind(tree, *each) == Kind::Node(Node::FieldList)
            && opening(tree, *each) != Some(Token::Lbrack)
    };
    let lists = siblings(tree, at)
        .take_while(|each| *each != at)
        .filter(unbracketed)
        .count();
    // A function with no name, a literal or a method of an interface, has no receiver.
    let receivers = siblings(tree, at)
        .take_while(|each| named && kind(tree, *each) != Kind::Node(Node::Ident))
        .filter(unbracketed)
        .count();
    if lists <= receivers {
        AstField::Params
    } else {
        AstField::Results
    }
}

/// The role of a node of a field: its names, then its type, then a tag, the literal a type never is.
fn field_role(tree: &Tree<'_>, at: Position) -> AstField {
    let tagged = siblings(tree, at)
        .filter(|each| is_token(tree, *each) == Leaf::Node)
        .last()
        .is_some_and(|each| kind(tree, each) == Kind::Node(Node::BasicLit))
        && among(tree, at, Wanted::Node) > Count(1);
    match (nodes_after(tree, at), tagged) {
        (Count(0), true) => AstField::Tag,
        (Count(0), false) | (Count(1), true) => AstField::Type,
        (Count(_), _) => AstField::Names,
    }
}

/// The role of a part of the header of an `if` or a switch: `Init` before its `;`, `then` after it or with no `;`.
fn header_role(tree: &Tree<'_>, at: Position, then: AstField) -> AstField {
    let parted = among(tree, at, Wanted::Token(Token::Semicolon)) > Count(0);
    if parted && ahead_of(tree, at, Token::Semicolon) == Holding::Holds {
        AstField::Init
    } else {
        then
    }
}

/// The role of a node of a `for`: its body last, before it the init, condition and post its `;` part, or the condition alone.
fn loop_role(tree: &Tree<'_>, at: Position) -> AstField {
    if none(nodes_after(tree, at)) == Holding::Holds {
        return AstField::Body;
    }
    if among(tree, at, Wanted::Token(Token::Semicolon)) == Count(0) {
        return AstField::Cond;
    }
    match before(tree, at, Wanted::Token(Token::Semicolon)) {
        Count(0) => AstField::Init,
        Count(1) => AstField::Cond,
        Count(_) => AstField::Post,
    }
}

/// The role of a node of an `if`: what follows its `else`, its body the last node before it, and the header before the body.
fn if_role(tree: &Tree<'_>, at: Position) -> AstField {
    if ahead_of(tree, at, Token::Else) == Holding::Fails {
        return AstField::Else;
    }
    let body = siblings(tree, at)
        .skip_while(|each| *each != at)
        .skip(1)
        .find(|each| {
            is_token(tree, *each) == Leaf::Node || kind(tree, *each) == Kind::Token(Token::Else)
        })
        .is_none_or(|each| kind(tree, each) == Kind::Token(Token::Else));
    if body {
        AstField::Body
    } else {
        header_role(tree, at, AstField::Cond)
    }
}

/// The role of a node of a slice expression: the operand, then the indexes its colons part.
fn slice_role(tree: &Tree<'_>, at: Position) -> AstField {
    if leading(tree, at) == Holding::Holds {
        return AstField::FieldX;
    }
    match before(tree, at, Wanted::Token(Token::Colon)) {
        Count(0) => AstField::Low,
        Count(1) => AstField::High,
        Count(_) => AstField::Max,
    }
}

/// The role of a node of a `range` statement: the key and the value before `range`, then the operand and the body.
fn range_role(tree: &Tree<'_>, at: Position) -> AstField {
    if ahead_of(tree, at, Token::Range) == Holding::Holds {
        either(leading(tree, at), AstField::Key, AstField::Value)
    } else {
        either(
            none(nodes_after(tree, at)),
            AstField::Body,
            AstField::FieldX,
        )
    }
}

/// The role of a node of a switch or a type switch: its body last, the header before it.
fn switch_role(tree: &Tree<'_>, at: Position, then: AstField) -> AstField {
    either(
        none(nodes_after(tree, at)),
        AstField::Body,
        header_role(tree, at, then),
    )
}

/// The role of a node of a value spec: the names it declares, its type, and the values after its `=`.
fn value_spec_role(tree: &Tree<'_>, at: Position) -> AstField {
    if ahead_of(tree, at, Token::Assign) == Holding::Fails {
        return AstField::Values;
    }
    let named = leading(tree, at) == Holding::Holds
        || previous(tree, at) == Some(Kind::Token(Token::Comma));
    if named {
        AstField::Names
    } else {
        AstField::Type
    }
}

/// The role of a node of a type spec: its name, its type parameters in brackets, and its type.
fn type_spec_role(tree: &Tree<'_>, at: Position) -> AstField {
    match (leading(tree, at), is(tree, at, Node::FieldList)) {
        (Holding::Holds, _) => AstField::Name,
        (Holding::Fails, Holding::Holds) => AstField::TypeParams,
        (Holding::Fails, Holding::Fails) => AstField::Type,
    }
}

/// The role of the node at `at` under a node of kind `holder`.
fn node_role(tree: &Tree<'_>, at: Position, holder: Node) -> AstField {
    match holder {
        Node::ArrayType | Node::Ellipsis => {
            either(none(nodes_after(tree, at)), AstField::Elt, AstField::Len)
        }
        Node::BinaryExpr => either(leading(tree, at), AstField::FieldX, AstField::FieldY),
        Node::CallExpr => either(leading(tree, at), AstField::Fun, AstField::Args),
        Node::ChanType => AstField::Value,
        Node::CompositeLit => either(
            ahead_of(tree, at, Token::Lbrace),
            AstField::Type,
            AstField::Elts,
        ),
        Node::IndexExpr => either(leading(tree, at), AstField::FieldX, AstField::Index),
        Node::IndexListExpr => either(leading(tree, at), AstField::FieldX, AstField::Indices),
        Node::SelectorExpr => either(leading(tree, at), AstField::FieldX, AstField::Sel),
        Node::SliceExpr => slice_role(tree, at),
        Node::InterfaceType => AstField::Methods,
        Node::StructType => AstField::Fields,
        Node::KeyValueExpr => either(
            ahead_of(tree, at, Token::Colon),
            AstField::Key,
            AstField::Value,
        ),
        Node::MapType => either(leading(tree, at), AstField::Key, AstField::Value),
        Node::TypeAssertExpr => either(
            ahead_of(tree, at, Token::Lparen),
            AstField::FieldX,
            AstField::Type,
        ),
        Node::FuncType => signature_role(tree, at),
        Node::FuncDecl | Node::FuncLit => {
            either(is(tree, at, Node::FuncType), AstField::Type, AstField::Body)
        }
        Node::ExprStmt | Node::IncDecStmt | Node::ParenExpr | Node::StarExpr | Node::UnaryExpr => {
            AstField::FieldX
        }
        Node::AssignStmt => either(
            none(before(tree, at, Wanted::Assigning)),
            AstField::Lhs,
            AstField::Rhs,
        ),
        Node::BlockStmt | Node::FieldList => AstField::List,
        Node::BranchStmt => AstField::Label,
        Node::LabeledStmt => either(leading(tree, at), AstField::Label, AstField::Stmt),
        Node::CaseClause => either(
            ahead_of(tree, at, Token::Colon),
            AstField::List,
            AstField::Body,
        ),
        Node::CommClause => either(
            ahead_of(tree, at, Token::Colon),
            AstField::Comm,
            AstField::Body,
        ),
        Node::SelectStmt => AstField::Body,
        Node::DeclStmt => AstField::Decl,
        Node::DeferStmt | Node::GoStmt => AstField::Call,
        Node::ForStmt => loop_role(tree, at),
        Node::IfStmt => if_role(tree, at),
        Node::SwitchStmt => switch_role(tree, at, AstField::Tag),
        Node::TypeSwitchStmt => switch_role(tree, at, AstField::Assign),
        Node::RangeStmt => range_role(tree, at),
        Node::ReturnStmt => AstField::Results,
        Node::SendStmt => either(leading(tree, at), AstField::Chan, AstField::Value),
        Node::Field => field_role(tree, at),
        Node::File => either(is(tree, at, Node::Ident), AstField::Name, AstField::Decls),
        Node::ImportSpec => either(is(tree, at, Node::Ident), AstField::Name, AstField::Path),
        Node::GenDecl => AstField::Specs,
        Node::TypeSpec => type_spec_role(tree, at),
        Node::ValueSpec => value_spec_role(tree, at),
        Node::BadDecl
        | Node::BadExpr
        | Node::BadStmt
        | Node::BasicLit
        | Node::Comment
        | Node::CommentGroup
        | Node::EmptyStmt
        | Node::Ident
        | Node::Package => AstField::None,
    }
}

/// The role of the element at `at` under its parent; `None` for the root, for trivia and for an edge `go/ast` gives no field.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: Position) -> AstField {
    if is_root(tree, at) == Rootness::Root {
        return AstField::None;
    }
    let Kind::Node(holder) = kind(tree, parent(tree, at)) else {
        return AstField::None;
    };
    match kind(tree, at) {
        Kind::Token(Token::Comment) | Kind::Extra(_) => AstField::None,
        Kind::Token(spelled) => token_role(tree, at, holder, spelled),
        Kind::Node(_) => node_role(tree, at, holder),
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
