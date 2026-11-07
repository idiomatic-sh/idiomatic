//! The schemas' cases: each of the library's node and token kinds is the language's kind of the same meaning, the standards' names:
//! a keyword is a `NAME`, an interpolation is the kind of the string it is in, and the library's nodes the standard lacks are extras.

use super::{node_kind, token_kind};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::adapters::ruff_python_parser::types::Strings;
use ruff_python_ast::NodeKind as RuffNode;
use ruff_python_ast::token::TokenKind;

/// A kind the library gives.
#[derive(Clone, Copy, Debug)]
enum Piece {
    /// A kind of node of its tree, inside the kind of string the second says.
    Node(RuffNode, Strings),
    /// A kind of token of its lexer.
    Token(TokenKind),
}

/// What the library gave: one kind of it, or all of them.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One kind of the library.
    One(Piece),
    /// Every kind the library names.
    Every,
}

type Input = Given;
/// The kind of the language it is, or whether every kind of the library is accounted for.
type Value = Seen;
type Failure = ();

/// What a case looks at.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// The kind of the language.
    Kind(Kind),
    /// Whether every kind of the library is mapped to a kind of the language or dropped with a reason.
    Accounted(bool),
}

/// The library's own enumeration of its kinds, written once: `RuffNode` and `TokenKind` have no iterator, so the list is the one
/// source of both the kinds the test walks and a `match` that names each of them with no wildcard, so a kind the library adds is
/// a compile error here until it is listed.
macro_rules! every {
    ($kinds:ty, $listed:ident, $all:ident; $($kind:ident),+ $(,)?) => {
        /// Every kind of the library, once.
        const $all: &[$kinds] = &[$(<$kinds>::$kind),+];

        /// Whether the library names `kind`: a `match` of every kind it has.
        const fn $listed(kind: $kinds) -> bool {
            match kind {
                $(<$kinds>::$kind)|+ => true,
            }
        }
    };
}

every! { RuffNode, node_listed, NODES;
        ModModule, ModExpression, StmtFunctionDef, StmtClassDef, StmtReturn, StmtDelete, StmtTypeAlias, StmtAssign,
        StmtAugAssign, StmtAnnAssign, StmtFor, StmtWhile, StmtIf, StmtWith, StmtMatch, StmtRaise, StmtTry, StmtAssert,
        StmtImport, StmtImportFrom, StmtGlobal, StmtNonlocal, StmtExpr, StmtPass, StmtBreak, StmtContinue, StmtIpyEscapeCommand,
        ExprBoolOp, ExprNamed, ExprBinOp, ExprUnaryOp, ExprLambda, ExprIf, ExprDict, ExprSet, ExprListComp, ExprSetComp,
        ExprDictComp, ExprGenerator, ExprAwait, ExprYield, ExprYieldFrom, ExprCompare, ExprCall, ExprFString, ExprTString,
        ExprStringLiteral, ExprBytesLiteral, ExprNumberLiteral, ExprBooleanLiteral, ExprNoneLiteral, ExprEllipsisLiteral,
        ExprAttribute, ExprSubscript, ExprStarred, ExprName, ExprList, ExprTuple, ExprSlice, ExprIpyEscapeCommand,
        ExceptHandlerExceptHandler, InterpolatedElement, InterpolatedStringLiteralElement, PatternMatchValue,
        PatternMatchSingleton, PatternMatchSequence, PatternMatchMapping, PatternMatchClass, PatternMatchStar, PatternMatchAs,
        PatternMatchOr, TypeParamTypeVar, TypeParamTypeVarTuple, TypeParamParamSpec, InterpolatedStringFormatSpec,
        PatternArguments, PatternKeyword, Comprehension, Arguments, Parameters, Parameter, ParameterWithDefault, Keyword, Alias,
        WithItem, MatchCase, Decorator, ElifElseClause, TypeParams, FString, TString, StringLiteral, BytesLiteral, Identifier,
}

every! { TokenKind, token_listed, TOKENS;
        Identifier, Int, Float, Complex, String, FStringStart, FStringMiddle, FStringEnd, TStringStart, TStringMiddle,
        TStringEnd, IpyEscapeCommand, Comment, Newline, NonLogicalNewline, Indent, Dedent, EndOfFile, Question, Exclamation,
        Lpar, Rpar, Lsqb, Rsqb, Colon, Comma, Semi, Plus, Minus, Star, Slash, Vbar, Amper, Less, Greater, Equal, Dot, Percent,
        Lbrace, Rbrace, EqEqual, NotEqual, LessEqual, GreaterEqual, Tilde, CircumFlex, LeftShift, RightShift, DoubleStar,
        DoubleStarEqual, PlusEqual, MinusEqual, StarEqual, SlashEqual, PercentEqual, AmperEqual, VbarEqual, CircumflexEqual,
        LeftShiftEqual, RightShiftEqual, DoubleSlash, DoubleSlashEqual, ColonEqual, At, AtEqual, Rarrow, Ellipsis, And, As,
        Assert, Async, Await, Break, Class, Continue, Def, Del, Elif, Else, Except, False, Finally, For, From, Global, If,
        Import, In, Is, Lambda, None, Nonlocal, Not, Or, Pass, Raise, Return, True, Try, While, With, Yield, Case, Lazy, Match,
        Type, Unknown,
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_module_is_a_module"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Node(RuffNode::ModModule, Strings::Formatted)),
        expect: Expect::Value(Seen::Kind(Kind::Node(Node::Module))),
    },
    Case {
        name: CaseName("a_function_definition_is_one"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Node(RuffNode::StmtFunctionDef, Strings::Formatted)),
        expect: Expect::Value(Seen::Kind(Kind::Node(Node::FunctionDef))),
    },
    Case {
        name: CaseName("a_parameter_is_an_arg"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Node(RuffNode::Parameter, Strings::Formatted)),
        expect: Expect::Value(Seen::Kind(Kind::Node(Node::Arg))),
    },
    Case {
        name: CaseName("the_nodes_of_the_library_the_standard_lacks_are_extras"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Node(RuffNode::Identifier, Strings::Formatted)),
        expect: Expect::Value(Seen::Kind(Kind::Extra(Extra::Identifier))),
    },
    Case {
        name: CaseName("an_interpolation_in_an_f_string_is_a_formatted_value"),
        word: Word::Succeeds,
        input: || {
            Given::One(Piece::Node(
                RuffNode::InterpolatedElement,
                Strings::Formatted,
            ))
        },
        expect: Expect::Value(Seen::Kind(Kind::Node(Node::FormattedValue))),
    },
    Case {
        name: CaseName("an_interpolation_in_a_t_string_is_an_interpolation"),
        word: Word::Succeeds,
        input: || {
            Given::One(Piece::Node(
                RuffNode::InterpolatedElement,
                Strings::Template,
            ))
        },
        expect: Expect::Value(Seen::Kind(Kind::Node(Node::Interpolation))),
    },
    Case {
        name: CaseName("the_token_identifier_is_a_name"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Token(TokenKind::Identifier)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::Name))),
    },
    Case {
        name: CaseName("a_keyword_token_is_a_name_as_pythons_tokenizer_gives_it"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Token(TokenKind::Def)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::Name))),
    },
    Case {
        name: CaseName("an_operator_token_is_its_operator"),
        word: Word::Succeeds,
        input: || Given::One(Piece::Token(TokenKind::DoubleStarEqual)),
        expect: Expect::Value(Seen::Kind(Kind::Token(Token::Doublestarequal))),
    },
    Case {
        name: CaseName("every_kind_of_the_library_is_mapped_or_dropped"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
];

/// Why the language has no kind for a kind of the library, or none when it has one. The language drops no kind of the library: the
/// matches of `schemas.rs` name every one and map it, and a kind it could not map would be named here with the reason.
const fn dropped(piece: Piece) -> Option<&'static str> {
    match piece {
        Piece::Node(..) | Piece::Token(..) => None,
    }
}

/// The language's kind of a kind of the library.
fn kind_of_piece(piece: Piece) -> Kind {
    match piece {
        Piece::Node(kind, strings) => node_kind(kind, strings),
        Piece::Token(kind) => token_kind(kind),
    }
}

/// Whether the language answers for `piece`: it is dropped with a reason, or it is mapped to a kind the language has.
fn accounted(piece: Piece) -> bool {
    if let Some(reason) = dropped(piece) {
        return !reason.is_empty();
    }
    let kind = kind_of_piece(piece);
    kind_of(kind_bits(kind)) == kind
}

/// Every kind the library names, from the library's own enumeration: one entry for each kind, so that a kind the library adds is
/// tested the day it is added and not when a file first meets it. An interpolation is met in an f-string and in a t-string.
fn library_kinds() -> Vec<Piece> {
    let nodes = NODES
        .iter()
        .filter(|kind| node_listed(**kind))
        .flat_map(|kind| {
            [Strings::Formatted, Strings::Template]
                .into_iter()
                .map(|strings| Piece::Node(*kind, strings))
        });
    let tokens = TOKENS
        .iter()
        .filter(|kind| token_listed(**kind))
        .map(|kind| Piece::Token(*kind));
    nodes.chain(tokens).collect()
}

fn subject(given: Input) -> Value {
    match given {
        Given::One(piece) => Seen::Kind(kind_of_piece(piece)),
        Given::Every => Seen::Accounted(library_kinds().into_iter().all(accounted)),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
