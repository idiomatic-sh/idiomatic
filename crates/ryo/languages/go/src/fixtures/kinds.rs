//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::constants::{EXPRESSION, NAME, NEVER_WRITTEN, PART, STATEMENT, TYPE};
use crate::domain::nodes::types::GoNode;
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
const fn node(node: GoNode) -> Sample {
    match node {
        GoNode::File => Sample::Node(SampleText("")),
        GoNode::BasicLit => Sample::Node(SampleText("package p\nvar s = \"a\"\n")),
        GoNode::ImportSpec => Sample::Node(SampleText("package p\nimport \"a\"\n")),
        GoNode::FuncDecl => Sample::Node(SampleText("package p\nfunc f() {}\n")),
        GoNode::FuncLit => Sample::Node(SampleText("package p\nvar f = func() {}\n")),
        GoNode::Field => Sample::Node(SampleText("package p\nfunc f(a int) {}\n")),
        GoNode::CallExpr => Sample::Node(SampleText("package p\nvar x = f()\n")),
        GoNode::DeclStmt
        | GoNode::EmptyStmt
        | GoNode::LabeledStmt
        | GoNode::ExprStmt
        | GoNode::SendStmt
        | GoNode::IncDecStmt
        | GoNode::AssignStmt
        | GoNode::GoStmt
        | GoNode::DeferStmt
        | GoNode::ReturnStmt
        | GoNode::BranchStmt
        | GoNode::BlockStmt
        | GoNode::IfStmt
        | GoNode::CaseClause
        | GoNode::SwitchStmt
        | GoNode::TypeSwitchStmt
        | GoNode::CommClause
        | GoNode::SelectStmt
        | GoNode::ForStmt
        | GoNode::RangeStmt => Sample::Unread(Why(STATEMENT)),
        GoNode::CompositeLit
        | GoNode::ParenExpr
        | GoNode::SelectorExpr
        | GoNode::IndexExpr
        | GoNode::IndexListExpr
        | GoNode::SliceExpr
        | GoNode::TypeAssertExpr
        | GoNode::StarExpr
        | GoNode::UnaryExpr
        | GoNode::BinaryExpr
        | GoNode::KeyValueExpr => Sample::Unread(Why(EXPRESSION)),
        GoNode::ArrayType
        | GoNode::StructType
        | GoNode::FuncType
        | GoNode::InterfaceType
        | GoNode::MapType
        | GoNode::ChanType
        | GoNode::Ellipsis => Sample::Unread(Why(TYPE)),
        GoNode::FieldList | GoNode::ValueSpec | GoNode::TypeSpec | GoNode::GenDecl => {
            Sample::Unread(Why(PART))
        }
        GoNode::Ident => Sample::Unread(Why(NAME)),
        GoNode::BadExpr
        | GoNode::BadStmt
        | GoNode::BadDecl
        | GoNode::Comment
        | GoNode::CommentGroup
        | GoNode::Package => Sample::Unread(Why(NEVER_WRITTEN)),
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
