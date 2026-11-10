//! The library's kinds mapped onto the language's, the standards' names: every node kind of `ruff_python_ast` onto the `CPython` `ast`
//! kind it stands for, or onto the closed list of the extras the standard lacks; every token kind of its lexer onto the name of
//! Python's `token` module, a keyword being a `NAME`. Both matches have no wildcard arm, so a kind the library gains is a compile
//! error until it is mapped. Where ruff's tree is not the standard's:
//!
//! - async statements share Ruff node types with their sync counterparts, with an `is_async` field; the adapter preserves that
//!   distinction as `AsyncFunctionDef`, `AsyncFor` and `AsyncWith`. `except*` remains a `Try` with its keyword token;
//! - six literal kinds of ruff (string, bytes, number, boolean, `None`, `...`) are the standard's one `Constant`, its token saying which;
//! - an interpolation is a `FormattedValue` in an f-string and an `Interpolation` in a t-string: the adapter says which;
//! - a literal part of an f-string or a t-string is a `Constant`, and its format spec a `JoinedStr`, as the standard has them;
//! - ruff's nodes the standard lacks are extras: identifiers, decorators, `elif`/`else` clauses, argument lists,
//!   a parameter with its default, type parameter brackets, class pattern arguments and keywords, and the parts of an implicit
//!   concatenation.

use super::types::Strings;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, NodeKind as RuffNode};

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

/// The language's kind of a concrete node, retaining async statement variants Ruff stores in a flag.
#[expect(
    clippy::too_many_lines,
    reason = "the exhaustive upstream node mapping prevents new AST variants from falling through"
)]
#[must_use]
pub(super) fn concrete_node_kind(node: AnyNodeRef<'_>, strings: Strings) -> Kind {
    match node {
        AnyNodeRef::StmtFunctionDef(statement) if statement.is_async => {
            Kind::Node(Node::AsyncFunctionDef)
        }
        AnyNodeRef::StmtFor(statement) if statement.is_async => Kind::Node(Node::AsyncFor),
        AnyNodeRef::StmtWith(statement) if statement.is_async => Kind::Node(Node::AsyncWith),
        AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtFor(_) | AnyNodeRef::StmtWith(_) => {
            node_kind(node.kind(), strings)
        }
        AnyNodeRef::ModModule(_)
        | AnyNodeRef::ModExpression(_)
        | AnyNodeRef::StmtClassDef(_)
        | AnyNodeRef::StmtReturn(_)
        | AnyNodeRef::StmtDelete(_)
        | AnyNodeRef::StmtTypeAlias(_)
        | AnyNodeRef::StmtAssign(_)
        | AnyNodeRef::StmtAugAssign(_)
        | AnyNodeRef::StmtAnnAssign(_)
        | AnyNodeRef::StmtWhile(_)
        | AnyNodeRef::StmtIf(_)
        | AnyNodeRef::StmtMatch(_)
        | AnyNodeRef::StmtRaise(_)
        | AnyNodeRef::StmtTry(_)
        | AnyNodeRef::StmtAssert(_)
        | AnyNodeRef::StmtImport(_)
        | AnyNodeRef::StmtImportFrom(_)
        | AnyNodeRef::StmtGlobal(_)
        | AnyNodeRef::StmtNonlocal(_)
        | AnyNodeRef::StmtExpr(_)
        | AnyNodeRef::StmtPass(_)
        | AnyNodeRef::StmtBreak(_)
        | AnyNodeRef::StmtContinue(_)
        | AnyNodeRef::StmtIpyEscapeCommand(_)
        | AnyNodeRef::ExprBoolOp(_)
        | AnyNodeRef::ExprNamed(_)
        | AnyNodeRef::ExprBinOp(_)
        | AnyNodeRef::ExprUnaryOp(_)
        | AnyNodeRef::ExprLambda(_)
        | AnyNodeRef::ExprIf(_)
        | AnyNodeRef::ExprDict(_)
        | AnyNodeRef::ExprSet(_)
        | AnyNodeRef::ExprListComp(_)
        | AnyNodeRef::ExprSetComp(_)
        | AnyNodeRef::ExprDictComp(_)
        | AnyNodeRef::ExprGenerator(_)
        | AnyNodeRef::ExprAwait(_)
        | AnyNodeRef::ExprYield(_)
        | AnyNodeRef::ExprYieldFrom(_)
        | AnyNodeRef::ExprCompare(_)
        | AnyNodeRef::ExprCall(_)
        | AnyNodeRef::ExprFString(_)
        | AnyNodeRef::ExprTString(_)
        | AnyNodeRef::ExprStringLiteral(_)
        | AnyNodeRef::ExprBytesLiteral(_)
        | AnyNodeRef::ExprNumberLiteral(_)
        | AnyNodeRef::ExprBooleanLiteral(_)
        | AnyNodeRef::ExprNoneLiteral(_)
        | AnyNodeRef::ExprEllipsisLiteral(_)
        | AnyNodeRef::ExprAttribute(_)
        | AnyNodeRef::ExprSubscript(_)
        | AnyNodeRef::ExprStarred(_)
        | AnyNodeRef::ExprName(_)
        | AnyNodeRef::ExprList(_)
        | AnyNodeRef::ExprTuple(_)
        | AnyNodeRef::ExprSlice(_)
        | AnyNodeRef::ExprIpyEscapeCommand(_)
        | AnyNodeRef::ExceptHandlerExceptHandler(_)
        | AnyNodeRef::InterpolatedElement(_)
        | AnyNodeRef::InterpolatedStringLiteralElement(_)
        | AnyNodeRef::PatternMatchValue(_)
        | AnyNodeRef::PatternMatchSingleton(_)
        | AnyNodeRef::PatternMatchSequence(_)
        | AnyNodeRef::PatternMatchMapping(_)
        | AnyNodeRef::PatternMatchClass(_)
        | AnyNodeRef::PatternMatchStar(_)
        | AnyNodeRef::PatternMatchAs(_)
        | AnyNodeRef::PatternMatchOr(_)
        | AnyNodeRef::TypeParamTypeVar(_)
        | AnyNodeRef::TypeParamTypeVarTuple(_)
        | AnyNodeRef::TypeParamParamSpec(_)
        | AnyNodeRef::InterpolatedStringFormatSpec(_)
        | AnyNodeRef::PatternArguments(_)
        | AnyNodeRef::PatternKeyword(_)
        | AnyNodeRef::Comprehension(_)
        | AnyNodeRef::Arguments(_)
        | AnyNodeRef::Parameters(_)
        | AnyNodeRef::Parameter(_)
        | AnyNodeRef::ParameterWithDefault(_)
        | AnyNodeRef::Keyword(_)
        | AnyNodeRef::Alias(_)
        | AnyNodeRef::WithItem(_)
        | AnyNodeRef::MatchCase(_)
        | AnyNodeRef::Decorator(_)
        | AnyNodeRef::ElifElseClause(_)
        | AnyNodeRef::TypeParams(_)
        | AnyNodeRef::FString(_)
        | AnyNodeRef::TString(_)
        | AnyNodeRef::StringLiteral(_)
        | AnyNodeRef::BytesLiteral(_)
        | AnyNodeRef::Identifier(_) => node_kind(node.kind(), strings),
    }
}

/// The language's kind of a token of the library.
#[must_use]
pub(super) fn token_kind(kind: TokenKind) -> Kind {
    Kind::from(TokenSchema(kind))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
