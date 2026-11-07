//! The types of the `tokens` concept: the kinds of the tokens of a Python tree, by the names of Python's `token` module, in kebab
//! case: `FSTRING_START` is `fstring-start`, and `NAME` is `name-token`, the one word the node standard has too (`Name`). A keyword is a `NAME`, as Python's tokenizer gives it.

/// A token of the standard.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Token {
    /// `NAME`: a name or a keyword, `def` and `async` included. Its word is `name-token`: `name` is the node `Name` of the node standard,
    /// and a kind's word is one kind's (a token whose word is a node's takes `-token`).
    #[strum(serialize = "name-token")]
    Name,
    /// `NUMBER`: a number.
    Number,
    /// `STRING`: a string or bytes literal, its prefix and quotes included.
    String,
    /// `FSTRING_START`: an f-string's prefix and opening quote.
    FstringStart,
    /// `FSTRING_MIDDLE`: the literal text of an f-string.
    FstringMiddle,
    /// `FSTRING_END`: an f-string's closing quote.
    FstringEnd,
    /// `TSTRING_START`: a t-string's prefix and opening quote.
    TstringStart,
    /// `TSTRING_MIDDLE`: the literal text of a t-string.
    TstringMiddle,
    /// `TSTRING_END`: a t-string's closing quote.
    TstringEnd,
    /// `COMMENT`: a comment, from `#` to the end of the line.
    Comment,
    /// `NEWLINE`: the end of a logical line.
    Newline,
    /// `NL`: a line break inside a logical line, or a blank line.
    Nl,
    /// `INDENT`: an indentation.
    Indent,
    /// `DEDENT`: the end of an indented block, of no length.
    Dedent,
    /// `ENDMARKER`: the end of the file, of no length.
    Endmarker,
    /// `ERRORTOKEN`: text the lexer could not read as any token.
    Errortoken,
    /// `OP`: an operator or a delimiter of no exact type of its own.
    Op,
    /// `EXCLAMATION`: `!`.
    Exclamation,
    /// `LPAR`: `(`.
    Lpar,
    /// `RPAR`: `)`.
    Rpar,
    /// `LSQB`: `[`.
    Lsqb,
    /// `RSQB`: `]`.
    Rsqb,
    /// `COLON`: `:`.
    Colon,
    /// `COMMA`: `,`.
    Comma,
    /// `SEMI`: `;`.
    Semi,
    /// `PLUS`: `+`.
    Plus,
    /// `MINUS`: `-`.
    Minus,
    /// `STAR`: `*`.
    Star,
    /// `SLASH`: `/`.
    Slash,
    /// `VBAR`: `|`.
    Vbar,
    /// `AMPER`: `&`.
    Amper,
    /// `LESS`: `<`.
    Less,
    /// `GREATER`: `>`.
    Greater,
    /// `EQUAL`: `=`.
    Equal,
    /// `DOT`: `.`.
    Dot,
    /// `PERCENT`: `%`.
    Percent,
    /// `LBRACE`: `{`.
    Lbrace,
    /// `RBRACE`: `}`.
    Rbrace,
    /// `EQEQUAL`: `==`.
    Eqequal,
    /// `NOTEQUAL`: `!=`.
    Notequal,
    /// `LESSEQUAL`: `<=`.
    Lessequal,
    /// `GREATEREQUAL`: `>=`.
    Greaterequal,
    /// `TILDE`: `~`.
    Tilde,
    /// `CIRCUMFLEX`: `^`.
    Circumflex,
    /// `LEFTSHIFT`: `<<`.
    Leftshift,
    /// `RIGHTSHIFT`: `>>`.
    Rightshift,
    /// `DOUBLESTAR`: `**`.
    Doublestar,
    /// `PLUSEQUAL`: `+=`.
    Plusequal,
    /// `MINEQUAL`: `-=`.
    Minequal,
    /// `STAREQUAL`: `*=`.
    Starequal,
    /// `SLASHEQUAL`: `/=`.
    Slashequal,
    /// `PERCENTEQUAL`: `%=`.
    Percentequal,
    /// `AMPEREQUAL`: `&=`.
    Amperequal,
    /// `VBAREQUAL`: `|=`.
    Vbarequal,
    /// `CIRCUMFLEXEQUAL`: `^=`.
    Circumflexequal,
    /// `LEFTSHIFTEQUAL`: `<<=`.
    Leftshiftequal,
    /// `RIGHTSHIFTEQUAL`: `>>=`.
    Rightshiftequal,
    /// `DOUBLESTAREQUAL`: `**=`.
    Doublestarequal,
    /// `DOUBLESLASH`: `//`.
    Doubleslash,
    /// `DOUBLESLASHEQUAL`: `//=`.
    Doubleslashequal,
    /// `AT`: `@`.
    At,
    /// `ATEQUAL`: `@=`.
    Atequal,
    /// `RARROW`: `->`.
    Rarrow,
    /// `ELLIPSIS`: `...`.
    Ellipsis,
    /// `COLONEQUAL`: `:=`.
    Colonequal,
}
