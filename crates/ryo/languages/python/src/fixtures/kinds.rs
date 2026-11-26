//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it. The match
//! has no wildcard arm, so a kind the standards gain is a compile error until it is shown.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{
    EXPRESSION, EXTRA, LITERAL_TOKEN, NEVER_GIVEN, NO_TYPE_COMMENTS, NOT_A_FILE, NOT_IN_A_MODULE,
    NOT_SPLIT, OPERATOR, PART, PATTERN, PUNCTUATION, STATEMENT, TRIVIA, UNREAD_TEXT,
};
use crate::domain::syntax::service::{kind_bits, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(format!("{kind:?}").into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a kind of its trees.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "every kind of both standards and every extra is shown once, with no wildcard: the length is the standards'"
)]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(Node::Module) => Sample::Node(SampleText("")),
        Kind::Node(Node::FunctionDef) => Sample::Node(SampleText("def f(): pass")),
        Kind::Node(Node::ClassDef) => Sample::Node(SampleText("class C: pass")),
        Kind::Node(Node::Assign) => Sample::Node(SampleText("a = \"x\"")),
        Kind::Node(Node::AnnAssign) => Sample::Node(SampleText("a: int = 1")),
        Kind::Node(Node::NamedExpr) => Sample::Node(SampleText("(a := 1)")),
        Kind::Node(Node::Lambda) => Sample::Node(SampleText("lambda: 0")),
        Kind::Node(Node::ListComp) => Sample::Node(SampleText("[a for a in b]")),
        Kind::Node(Node::SetComp) => Sample::Node(SampleText("{a for a in b}")),
        Kind::Node(Node::DictComp) => Sample::Node(SampleText("{a: a for a in b}")),
        Kind::Node(Node::GeneratorExp) => Sample::Node(SampleText("(a for a in b)")),
        Kind::Node(Node::Call) => Sample::Node(SampleText("f()")),
        Kind::Node(Node::JoinedStr) => Sample::Node(SampleText("f'a'")),
        Kind::Node(Node::TemplateStr) => Sample::Node(SampleText("t'a'")),
        Kind::Node(Node::Constant) => Sample::Node(SampleText("'a'")),
        Kind::Node(Node::Attribute) => Sample::Node(SampleText("a.b")),
        Kind::Node(Node::Name) => Sample::Node(SampleText("a")),
        Kind::Node(Node::MatchMapping) => {
            Sample::Node(SampleText("match x:\n    case {**y}:\n        pass\n"))
        }
        Kind::Node(Node::MatchStar) => {
            Sample::Node(SampleText("match x:\n    case [*y]:\n        pass\n"))
        }
        Kind::Node(Node::MatchAs) => {
            Sample::Node(SampleText("match x:\n    case y:\n        pass\n"))
        }
        Kind::Node(Node::ExceptHandler) => {
            Sample::Node(SampleText("try:\n    pass\nexcept E as e:\n    pass\n"))
        }
        Kind::Node(Node::Arg) => Sample::Node(SampleText("def f(a): pass")),
        Kind::Node(Node::Alias) => Sample::Node(SampleText("import a")),
        Kind::Node(Node::TypeVar) => Sample::Node(SampleText("type A[T] = T")),
        Kind::Node(Node::ParamSpec) => Sample::Node(SampleText("type A[**P] = P")),
        Kind::Node(Node::TypeVarTuple) => Sample::Node(SampleText("type A[*T] = T")),
        Kind::Token(Token::Comment) => Sample::Token(SampleText("# a")),
        Kind::Extra(Extra::Decorator) => Sample::Node(SampleText("@d\ndef f(): pass")),
        Kind::Node(Node::Interactive | Node::Expression | Node::FunctionType) => {
            Sample::Unread(Why(NOT_A_FILE))
        }
        Kind::Node(Node::AsyncFunctionDef | Node::AsyncFor | Node::AsyncWith | Node::TryStar) => {
            Sample::Unread(Why(NOT_SPLIT))
        }
        Kind::Node(
            Node::Return
            | Node::Delete
            | Node::TypeAlias
            | Node::AugAssign
            | Node::For
            | Node::While
            | Node::If
            | Node::With
            | Node::Match
            | Node::Raise
            | Node::Try
            | Node::Assert
            | Node::Import
            | Node::ImportFrom
            | Node::Global
            | Node::Nonlocal
            | Node::Expr
            | Node::Pass
            | Node::Break
            | Node::Continue,
        ) => Sample::Unread(Why(STATEMENT)),
        Kind::Node(
            Node::BoolOp
            | Node::BinOp
            | Node::UnaryOp
            | Node::IfExp
            | Node::Dict
            | Node::Set
            | Node::Await
            | Node::Yield
            | Node::YieldFrom
            | Node::Compare
            | Node::FormattedValue
            | Node::Interpolation
            | Node::Subscript
            | Node::Starred
            | Node::List
            | Node::Tuple
            | Node::Slice,
        ) => Sample::Unread(Why(EXPRESSION)),
        Kind::Node(
            Node::MatchValue
            | Node::MatchSingleton
            | Node::MatchSequence
            | Node::MatchClass
            | Node::MatchOr,
        ) => Sample::Unread(Why(PATTERN)),
        Kind::Node(
            Node::Comprehension
            | Node::Arguments
            | Node::Keyword
            | Node::Withitem
            | Node::MatchCase,
        ) => Sample::Unread(Why(PART)),
        Kind::Node(Node::TypeIgnore) => Sample::Unread(Why(NO_TYPE_COMMENTS)),
        Kind::Node(
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
            | Node::NotIn,
        ) => Sample::Unread(Why(OPERATOR)),
        Kind::Token(
            Token::Name
            | Token::Number
            | Token::String
            | Token::FstringStart
            | Token::FstringMiddle
            | Token::FstringEnd
            | Token::TstringStart
            | Token::TstringMiddle
            | Token::TstringEnd,
        ) => Sample::Unread(Why(LITERAL_TOKEN)),
        Kind::Token(
            Token::Newline | Token::Nl | Token::Indent | Token::Dedent | Token::Endmarker,
        )
        | Kind::Extra(Extra::Whitespace) => Sample::Unread(Why(TRIVIA)),
        Kind::Token(Token::Errortoken) => Sample::Unread(Why(UNREAD_TEXT)),
        Kind::Token(Token::Op) => Sample::Unread(Why(NEVER_GIVEN)),
        Kind::Token(
            Token::Exclamation
            | Token::Lpar
            | Token::Rpar
            | Token::Lsqb
            | Token::Rsqb
            | Token::Colon
            | Token::Comma
            | Token::Semi
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Vbar
            | Token::Amper
            | Token::Less
            | Token::Greater
            | Token::Equal
            | Token::Dot
            | Token::Percent
            | Token::Lbrace
            | Token::Rbrace
            | Token::Eqequal
            | Token::Notequal
            | Token::Lessequal
            | Token::Greaterequal
            | Token::Tilde
            | Token::Circumflex
            | Token::Leftshift
            | Token::Rightshift
            | Token::Doublestar
            | Token::Plusequal
            | Token::Minequal
            | Token::Starequal
            | Token::Slashequal
            | Token::Percentequal
            | Token::Amperequal
            | Token::Vbarequal
            | Token::Circumflexequal
            | Token::Leftshiftequal
            | Token::Rightshiftequal
            | Token::Doublestarequal
            | Token::Doubleslash
            | Token::Doubleslashequal
            | Token::At
            | Token::Atequal
            | Token::Rarrow
            | Token::Ellipsis
            | Token::Colonequal,
        ) => Sample::Unread(Why(PUNCTUATION)),
        Kind::Extra(
            Extra::Identifier
            | Extra::ElifElseClause
            | Extra::CallArguments
            | Extra::ParameterWithDefault
            | Extra::TypeParams
            | Extra::PatternArguments
            | Extra::PatternKeyword
            | Extra::StringPart
            | Extra::BytesPart
            | Extra::FStringPart
            | Extra::TStringPart,
        ) => Sample::Unread(Why(EXTRA)),
        Kind::Extra(Extra::IpyEscapeCommand) => Sample::Unread(Why(NOT_IN_A_MODULE)),
    }
}
