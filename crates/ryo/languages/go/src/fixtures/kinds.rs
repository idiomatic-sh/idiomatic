//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::constants::{EXPRESSION, NAME, NEVER_WRITTEN, PART, STATEMENT, TYPE};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{TRIVIA, UNREAD_TEXT};
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::constants::{LITERAL, OPERATOR, PUNCTUATION};
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(kind_name(kind).0.into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a node: the source whose tree holds it, or why no relation reads it.
const fn node(node: Node) -> Sample {
    match node {
        Node::File => Sample::Node(SampleText("")),
        Node::BasicLit => Sample::Node(SampleText("package p\nvar s = \"a\"\n")),
        Node::ImportSpec => Sample::Node(SampleText("package p\nimport \"a\"\n")),
        Node::FuncDecl => Sample::Node(SampleText("package p\nfunc f() {}\n")),
        Node::FuncLit => Sample::Node(SampleText("package p\nvar f = func() {}\n")),
        Node::Field => Sample::Node(SampleText("package p\nfunc f(a int) {}\n")),
        Node::CallExpr => Sample::Node(SampleText("package p\nvar x = f()\n")),
        Node::DeclStmt
        | Node::EmptyStmt
        | Node::LabeledStmt
        | Node::ExprStmt
        | Node::SendStmt
        | Node::IncDecStmt
        | Node::AssignStmt
        | Node::GoStmt
        | Node::DeferStmt
        | Node::ReturnStmt
        | Node::BranchStmt
        | Node::BlockStmt
        | Node::IfStmt
        | Node::CaseClause
        | Node::SwitchStmt
        | Node::TypeSwitchStmt
        | Node::CommClause
        | Node::SelectStmt
        | Node::ForStmt
        | Node::RangeStmt => Sample::Unread(Why(STATEMENT)),
        Node::CompositeLit
        | Node::ParenExpr
        | Node::SelectorExpr
        | Node::IndexExpr
        | Node::IndexListExpr
        | Node::SliceExpr
        | Node::TypeAssertExpr
        | Node::StarExpr
        | Node::UnaryExpr
        | Node::BinaryExpr
        | Node::KeyValueExpr => Sample::Unread(Why(EXPRESSION)),
        Node::ArrayType
        | Node::StructType
        | Node::FuncType
        | Node::InterfaceType
        | Node::MapType
        | Node::ChanType
        | Node::Ellipsis => Sample::Unread(Why(TYPE)),
        Node::FieldList | Node::ValueSpec | Node::TypeSpec | Node::GenDecl => {
            Sample::Unread(Why(PART))
        }
        Node::Ident => Sample::Unread(Why(NAME)),
        Node::BadExpr
        | Node::BadStmt
        | Node::BadDecl
        | Node::Comment
        | Node::CommentGroup
        | Node::Package => Sample::Unread(Why(NEVER_WRITTEN)),
    }
}

/// How the language shows a token: the source whose tree holds it, or why no relation reads it.
const fn token(token: Token) -> Sample {
    match token {
        Token::Comment => Sample::Token(SampleText("// a\n")),
        Token::Illegal => Sample::Unread(Why(UNREAD_TEXT)),
        Token::Eof => Sample::Unread(Why(NEVER_WRITTEN)),
        Token::Add
        | Token::Sub
        | Token::Mul
        | Token::Quo
        | Token::Rem
        | Token::And
        | Token::Or
        | Token::Xor
        | Token::Shl
        | Token::Shr
        | Token::AndNot
        | Token::AddAssign
        | Token::SubAssign
        | Token::MulAssign
        | Token::QuoAssign
        | Token::RemAssign
        | Token::AndAssign
        | Token::OrAssign
        | Token::XorAssign
        | Token::ShlAssign
        | Token::ShrAssign
        | Token::AndNotAssign
        | Token::Land
        | Token::Lor
        | Token::Arrow
        | Token::Inc
        | Token::Dec
        | Token::Eql
        | Token::Lss
        | Token::Gtr
        | Token::Assign
        | Token::Not
        | Token::Neq
        | Token::Leq
        | Token::Geq
        | Token::Define
        | Token::Tilde => Sample::Unread(Why(OPERATOR)),
        Token::Ident | Token::Int | Token::Float | Token::Imag | Token::Char | Token::String => {
            Sample::Unread(Why(LITERAL))
        }
        Token::Ellipsis
        | Token::Lparen
        | Token::Lbrack
        | Token::Lbrace
        | Token::Comma
        | Token::Period
        | Token::Rparen
        | Token::Rbrack
        | Token::Rbrace
        | Token::Semicolon
        | Token::Colon
        | Token::Break
        | Token::Case
        | Token::Chan
        | Token::Const
        | Token::Continue
        | Token::Default
        | Token::Defer
        | Token::Else
        | Token::Fallthrough
        | Token::For
        | Token::Func
        | Token::Go
        | Token::Goto
        | Token::If
        | Token::Import
        | Token::Interface
        | Token::Map
        | Token::Package
        | Token::Range
        | Token::Return
        | Token::Select
        | Token::Struct
        | Token::Switch
        | Token::Type
        | Token::Var => Sample::Unread(Why(PUNCTUATION)),
    }
}

/// Why a kind's sample shows no part of a module's `go.mod`: a sample is read as a go source, so it cannot hold one; the `entry`
/// item's cases read `go.mod` files and show the lines it reads.
const MODULE_PART: &str = "a part of a module's go.mod, which a sample read as a go source cannot hold; the entry item's cases show its lines";

/// How the language shows an extra: why no relation reads it.
const fn extra(extra: Extra) -> Sample {
    match extra {
        Extra::Error => Sample::Unread(Why(UNREAD_TEXT)),
        Extra::Whitespace => Sample::Unread(Why(TRIVIA)),
        Extra::ModFile
        | Extra::ModLine
        | Extra::ModLineBlock
        | Extra::ModWord
        | Extra::ModLParen
        | Extra::ModRParen
        | Extra::ModComment => Sample::Unread(Why(MODULE_PART)),
    }
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. Each match has no wildcard arm, so a kind the standard gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(each) => node(each),
        Kind::Token(each) => token(each),
        Kind::Extra(each) => extra(each),
    }
}
