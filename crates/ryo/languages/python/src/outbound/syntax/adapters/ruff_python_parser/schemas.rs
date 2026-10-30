//! The library's kinds mapped onto the language's, the standards' names: every node kind of `ruff_python_ast` onto the `CPython` `ast`
//! kind it stands for, or onto the closed list of the extras the standard lacks; every token kind of its lexer onto the name of
//! Python's `token` module, a keyword being a `NAME`. Both matches have no wildcard arm, so a kind the library gains is a compile
//! error until it is mapped. Where ruff's tree is not the standard's:
//!
//! - one kind for two of the standard's: `async def`, `async for`, `async with` and `except*` are `FunctionDef`, `For`, `With` and
//!   `Try` with their keyword token, never `AsyncFunctionDef`, `AsyncFor`, `AsyncWith` or `TryStar`;
//! - six literal kinds of ruff (string, bytes, number, boolean, `None`, `...`) are the standard's one `Constant`, its token saying which;
//! - an interpolation is a `FormattedValue` in an f-string and an `Interpolation` in a t-string: the adapter says which;
//! - a literal part of an f-string or a t-string is a `Constant`, and its format spec a `JoinedStr`, as the standard has them;
//! - ruff's nodes the standard lacks are extras: identifiers, decorators, `elif`/`else` clauses, argument lists,
//!   a parameter with its default, type parameter brackets, class pattern arguments and keywords, and the parts of an implicit
//!   concatenation.

use ruff_python_ast::NodeKind as RuffNode;
use ruff_python_ast::{StmtImportFrom, StmtTry};

use super::constants::{
    BRACES, FUTURE, FUTURE_FEATURES, NOT_A_CHANCE, NOT_DEFINED, UNKNOWN_FEATURE,
};
use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_text_size::{Ranged, TextRange};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

use super::types::{ParserRefusal, Said, Strings};

/// A kind of node of the library's tree, and the kind of string an interpolation inside it is in.
pub(super) struct NodeSchema {
    /// The library's kind.
    kind: RuffNode,
    /// The innermost string open.
    strings: Strings,
}

/// A kind of token of the library's lexer.
pub(super) struct TokenSchema(TokenKind);

/// The language's kind of a node of the library; an interpolation is the kind of the string it is in.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each kind the language maps the library's nodes onto, with no wildcard: the length is the library's"
)]
impl From<NodeSchema> for Kind {
    fn from(schema: NodeSchema) -> Self {
        let NodeSchema { kind, strings } = schema;
        match kind {
            RuffNode::ModModule => Kind::Node(Node::Module),
            RuffNode::ModExpression => Kind::Node(Node::Expression),
            RuffNode::StmtFunctionDef => Kind::Node(Node::FunctionDef),
            RuffNode::StmtClassDef => Kind::Node(Node::ClassDef),
            RuffNode::StmtReturn => Kind::Node(Node::Return),
            RuffNode::StmtDelete => Kind::Node(Node::Delete),
            RuffNode::StmtTypeAlias => Kind::Node(Node::TypeAlias),
            RuffNode::StmtAssign => Kind::Node(Node::Assign),
            RuffNode::StmtAugAssign => Kind::Node(Node::AugAssign),
            RuffNode::StmtAnnAssign => Kind::Node(Node::AnnAssign),
            RuffNode::StmtFor => Kind::Node(Node::For),
            RuffNode::StmtWhile => Kind::Node(Node::While),
            RuffNode::StmtIf => Kind::Node(Node::If),
            RuffNode::StmtWith => Kind::Node(Node::With),
            RuffNode::StmtMatch => Kind::Node(Node::Match),
            RuffNode::StmtRaise => Kind::Node(Node::Raise),
            RuffNode::StmtTry => Kind::Node(Node::Try),
            RuffNode::StmtAssert => Kind::Node(Node::Assert),
            RuffNode::StmtImport => Kind::Node(Node::Import),
            RuffNode::StmtImportFrom => Kind::Node(Node::ImportFrom),
            RuffNode::StmtGlobal => Kind::Node(Node::Global),
            RuffNode::StmtNonlocal => Kind::Node(Node::Nonlocal),
            RuffNode::StmtExpr => Kind::Node(Node::Expr),
            RuffNode::StmtPass => Kind::Node(Node::Pass),
            RuffNode::StmtBreak => Kind::Node(Node::Break),
            RuffNode::StmtContinue => Kind::Node(Node::Continue),
            RuffNode::StmtIpyEscapeCommand | RuffNode::ExprIpyEscapeCommand => {
                Kind::Extra(Extra::IpyEscapeCommand)
            }
            RuffNode::ExprBoolOp => Kind::Node(Node::BoolOp),
            RuffNode::ExprNamed => Kind::Node(Node::NamedExpr),
            RuffNode::ExprBinOp => Kind::Node(Node::BinOp),
            RuffNode::ExprUnaryOp => Kind::Node(Node::UnaryOp),
            RuffNode::ExprLambda => Kind::Node(Node::Lambda),
            RuffNode::ExprIf => Kind::Node(Node::IfExp),
            RuffNode::ExprDict => Kind::Node(Node::Dict),
            RuffNode::ExprSet => Kind::Node(Node::Set),
            RuffNode::ExprListComp => Kind::Node(Node::ListComp),
            RuffNode::ExprSetComp => Kind::Node(Node::SetComp),
            RuffNode::ExprDictComp => Kind::Node(Node::DictComp),
            RuffNode::ExprGenerator => Kind::Node(Node::GeneratorExp),
            RuffNode::ExprAwait => Kind::Node(Node::Await),
            RuffNode::ExprYield => Kind::Node(Node::Yield),
            RuffNode::ExprYieldFrom => Kind::Node(Node::YieldFrom),
            RuffNode::ExprCompare => Kind::Node(Node::Compare),
            RuffNode::ExprCall => Kind::Node(Node::Call),
            RuffNode::ExprFString | RuffNode::InterpolatedStringFormatSpec => {
                Kind::Node(Node::JoinedStr)
            }
            RuffNode::ExprTString => Kind::Node(Node::TemplateStr),
            RuffNode::ExprStringLiteral
            | RuffNode::ExprBytesLiteral
            | RuffNode::ExprNumberLiteral
            | RuffNode::ExprBooleanLiteral
            | RuffNode::ExprNoneLiteral
            | RuffNode::ExprEllipsisLiteral
            | RuffNode::InterpolatedStringLiteralElement => Kind::Node(Node::Constant),
            RuffNode::ExprAttribute => Kind::Node(Node::Attribute),
            RuffNode::ExprSubscript => Kind::Node(Node::Subscript),
            RuffNode::ExprStarred => Kind::Node(Node::Starred),
            RuffNode::ExprName => Kind::Node(Node::Name),
            RuffNode::ExprList => Kind::Node(Node::List),
            RuffNode::ExprTuple => Kind::Node(Node::Tuple),
            RuffNode::ExprSlice => Kind::Node(Node::Slice),
            RuffNode::ExceptHandlerExceptHandler => Kind::Node(Node::ExceptHandler),
            RuffNode::PatternMatchValue => Kind::Node(Node::MatchValue),
            RuffNode::PatternMatchSingleton => Kind::Node(Node::MatchSingleton),
            RuffNode::PatternMatchSequence => Kind::Node(Node::MatchSequence),
            RuffNode::PatternMatchMapping => Kind::Node(Node::MatchMapping),
            RuffNode::PatternMatchClass => Kind::Node(Node::MatchClass),
            RuffNode::PatternMatchStar => Kind::Node(Node::MatchStar),
            RuffNode::PatternMatchAs => Kind::Node(Node::MatchAs),
            RuffNode::PatternMatchOr => Kind::Node(Node::MatchOr),
            RuffNode::TypeParamTypeVar => Kind::Node(Node::TypeVar),
            RuffNode::TypeParamTypeVarTuple => Kind::Node(Node::TypeVarTuple),
            RuffNode::TypeParamParamSpec => Kind::Node(Node::ParamSpec),
            RuffNode::PatternArguments => Kind::Extra(Extra::PatternArguments),
            RuffNode::PatternKeyword => Kind::Extra(Extra::PatternKeyword),
            RuffNode::Comprehension => Kind::Node(Node::Comprehension),
            RuffNode::Arguments => Kind::Extra(Extra::CallArguments),
            RuffNode::Parameters => Kind::Node(Node::Arguments),
            RuffNode::Parameter => Kind::Node(Node::Arg),
            RuffNode::ParameterWithDefault => Kind::Extra(Extra::ParameterWithDefault),
            RuffNode::Keyword => Kind::Node(Node::Keyword),
            RuffNode::Alias => Kind::Node(Node::Alias),
            RuffNode::WithItem => Kind::Node(Node::Withitem),
            RuffNode::MatchCase => Kind::Node(Node::MatchCase),
            RuffNode::Decorator => Kind::Extra(Extra::Decorator),
            RuffNode::ElifElseClause => Kind::Extra(Extra::ElifElseClause),
            RuffNode::TypeParams => Kind::Extra(Extra::TypeParams),
            RuffNode::FString => Kind::Extra(Extra::FStringPart),
            RuffNode::TString => Kind::Extra(Extra::TStringPart),
            RuffNode::StringLiteral => Kind::Extra(Extra::StringPart),
            RuffNode::BytesLiteral => Kind::Extra(Extra::BytesPart),
            RuffNode::Identifier => Kind::Extra(Extra::Identifier),
            RuffNode::InterpolatedElement => match strings {
                Strings::Template => Kind::Node(Node::Interpolation),
                Strings::Formatted => Kind::Node(Node::FormattedValue),
            },
        }
    }
}

/// The language's kind of a token of the library: a keyword is a `NAME`, as Python's tokenizer gives it.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each token kind the library has, with no wildcard: the length is the lexer's"
)]
impl From<TokenSchema> for Kind {
    fn from(schema: TokenSchema) -> Self {
        let kind = schema.0;
        Kind::Token(match kind {
            TokenKind::Identifier
            | TokenKind::And
            | TokenKind::As
            | TokenKind::Assert
            | TokenKind::Async
            | TokenKind::Await
            | TokenKind::Break
            | TokenKind::Class
            | TokenKind::Continue
            | TokenKind::Def
            | TokenKind::Del
            | TokenKind::Elif
            | TokenKind::Else
            | TokenKind::Except
            | TokenKind::False
            | TokenKind::Finally
            | TokenKind::For
            | TokenKind::From
            | TokenKind::Global
            | TokenKind::If
            | TokenKind::Import
            | TokenKind::In
            | TokenKind::Is
            | TokenKind::Lambda
            | TokenKind::None
            | TokenKind::Nonlocal
            | TokenKind::Not
            | TokenKind::Or
            | TokenKind::Pass
            | TokenKind::Raise
            | TokenKind::Return
            | TokenKind::True
            | TokenKind::Try
            | TokenKind::While
            | TokenKind::With
            | TokenKind::Yield
            | TokenKind::Case
            | TokenKind::Lazy
            | TokenKind::Match
            | TokenKind::Type => Token::Name,
            TokenKind::Int | TokenKind::Float | TokenKind::Complex => Token::Number,
            TokenKind::String => Token::String,
            TokenKind::FStringStart => Token::FstringStart,
            TokenKind::FStringMiddle => Token::FstringMiddle,
            TokenKind::FStringEnd => Token::FstringEnd,
            TokenKind::TStringStart => Token::TstringStart,
            TokenKind::TStringMiddle => Token::TstringMiddle,
            TokenKind::TStringEnd => Token::TstringEnd,
            TokenKind::IpyEscapeCommand | TokenKind::Question | TokenKind::Unknown => {
                Token::Errortoken
            }
            TokenKind::Comment => Token::Comment,
            TokenKind::Newline => Token::Newline,
            TokenKind::NonLogicalNewline => Token::Nl,
            TokenKind::Indent => Token::Indent,
            TokenKind::Dedent => Token::Dedent,
            TokenKind::EndOfFile => Token::Endmarker,
            TokenKind::Exclamation => Token::Exclamation,
            TokenKind::Lpar => Token::Lpar,
            TokenKind::Rpar => Token::Rpar,
            TokenKind::Lsqb => Token::Lsqb,
            TokenKind::Rsqb => Token::Rsqb,
            TokenKind::Colon => Token::Colon,
            TokenKind::Comma => Token::Comma,
            TokenKind::Semi => Token::Semi,
            TokenKind::Plus => Token::Plus,
            TokenKind::Minus => Token::Minus,
            TokenKind::Star => Token::Star,
            TokenKind::Slash => Token::Slash,
            TokenKind::Vbar => Token::Vbar,
            TokenKind::Amper => Token::Amper,
            TokenKind::Less => Token::Less,
            TokenKind::Greater => Token::Greater,
            TokenKind::Equal => Token::Equal,
            TokenKind::Dot => Token::Dot,
            TokenKind::Percent => Token::Percent,
            TokenKind::Lbrace => Token::Lbrace,
            TokenKind::Rbrace => Token::Rbrace,
            TokenKind::EqEqual => Token::Eqequal,
            TokenKind::NotEqual => Token::Notequal,
            TokenKind::LessEqual => Token::Lessequal,
            TokenKind::GreaterEqual => Token::Greaterequal,
            TokenKind::Tilde => Token::Tilde,
            TokenKind::CircumFlex => Token::Circumflex,
            TokenKind::LeftShift => Token::Leftshift,
            TokenKind::RightShift => Token::Rightshift,
            TokenKind::DoubleStar => Token::Doublestar,
            TokenKind::DoubleStarEqual => Token::Doublestarequal,
            TokenKind::PlusEqual => Token::Plusequal,
            TokenKind::MinusEqual => Token::Minequal,
            TokenKind::StarEqual => Token::Starequal,
            TokenKind::SlashEqual => Token::Slashequal,
            TokenKind::PercentEqual => Token::Percentequal,
            TokenKind::AmperEqual => Token::Amperequal,
            TokenKind::VbarEqual => Token::Vbarequal,
            TokenKind::CircumflexEqual => Token::Circumflexequal,
            TokenKind::LeftShiftEqual => Token::Leftshiftequal,
            TokenKind::RightShiftEqual => Token::Rightshiftequal,
            TokenKind::DoubleSlash => Token::Doubleslash,
            TokenKind::DoubleSlashEqual => Token::Doubleslashequal,
            TokenKind::ColonEqual => Token::Colonequal,
            TokenKind::At => Token::At,
            TokenKind::AtEqual => Token::Atequal,
            TokenKind::Rarrow => Token::Rarrow,
            TokenKind::Ellipsis => Token::Ellipsis,
        })
    }
}

/// The language's kind of a node of the library; an interpolation is the kind of the string it is in.
#[must_use]
pub(super) fn node_kind(kind: RuffNode, strings: Strings) -> Kind {
    Kind::from(NodeSchema { kind, strings })
}

/// The language's kind of a token of the library.
#[must_use]
pub(super) fn token_kind(kind: TokenKind) -> Kind {
    Kind::from(TokenSchema(kind))
}

/// The kind of string a node of the library opens, for the interpolations inside it: an f-string or a format spec, a t-string, or
/// none. The standard's format spec is a `JoinedStr` in a t-string too: what it interpolates is a `FormattedValue`.
#[must_use]
pub(super) const fn opens(kind: RuffNode) -> Option<Strings> {
    if matches!(kind, RuffNode::ExprTString) {
        Some(Strings::Template)
    } else if matches!(
        kind,
        RuffNode::ExprFString | RuffNode::InterpolatedStringFormatSpec
    ) {
        Some(Strings::Formatted)
    } else {
        None
    }
}

/// Where `CPython`'s parser refuses a `try` the library reads without an error: an `else` with no `except` before it (`CPython`: "expected
/// 'except' or 'finally' block", at the `else`). The library refuses a `try` with neither `except` nor `finally`, and nothing else
/// of this.
#[must_use]
pub(super) fn else_without_except(statement: &StmtTry, tokens: &Tokens) -> Option<TextRange> {
    if !statement.handlers.is_empty() {
        return None;
    }
    let from = statement.body.last().map_or(statement.start(), Ranged::end);
    let to = statement.orelse.first()?.start();
    tokens
        .in_range(TextRange::new(from, to))
        .iter()
        .find(|token| token.kind() == TokenKind::Else)
        .map(Ranged::range)
}

/// Where `CPython`'s parser refuses a feature of `from __future__ import ...` it does not know, and what it says: each alias whose
/// name is none of `__future__`'s features. The library reads any name there without an error.
pub(super) fn unknown_futures(
    statement: &StmtImportFrom,
) -> impl Iterator<Item = ParserRefusal> + '_ {
    let future = statement.level == 0
        && statement
            .module
            .as_ref()
            .is_some_and(|module| module.as_str() == FUTURE);
    statement
        .names
        .iter()
        .filter(move |alias| future && !FUTURE_FEATURES.contains(&alias.name.as_str()))
        .map(|alias| {
            let said = if alias.name.as_str() == BRACES {
                NOT_A_CHANCE.to_owned()
            } else {
                format!("{UNKNOWN_FEATURE}{}{NOT_DEFINED}", alias.name.as_str())
            };
            ParserRefusal {
                at: alias.name.range(),
                said: Said(said),
            }
        })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
