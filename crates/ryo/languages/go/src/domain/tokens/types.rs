//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is a
//! token that `go/token` defines, named by its constant in kebab case; one that shares its name with a node ends in `-token`.

/// What a token of a Go syntax tree is. One closed list, each name once, in the order `go/token` lists its constants.
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
    /// `token.ILLEGAL`.
    Illegal,
    /// `token.EOF`.
    Eof,
    /// `token.COMMENT`.
    #[strum(serialize = "comment-token")]
    Comment,
    /// `token.IDENT`.
    #[strum(serialize = "ident-token")]
    Ident,
    /// `token.INT`.
    Int,
    /// `token.FLOAT`.
    Float,
    /// `token.IMAG`.
    Imag,
    /// `token.CHAR`.
    Char,
    /// `token.STRING`.
    String,
    /// `token.ADD`.
    Add,
    /// `token.SUB`.
    Sub,
    /// `token.MUL`.
    Mul,
    /// `token.QUO`.
    Quo,
    /// `token.REM`.
    Rem,
    /// `token.AND`.
    And,
    /// `token.OR`.
    Or,
    /// `token.XOR`.
    Xor,
    /// `token.SHL`.
    Shl,
    /// `token.SHR`.
    Shr,
    /// `token.AND_NOT`.
    AndNot,
    /// `token.ADD_ASSIGN`.
    AddAssign,
    /// `token.SUB_ASSIGN`.
    SubAssign,
    /// `token.MUL_ASSIGN`.
    MulAssign,
    /// `token.QUO_ASSIGN`.
    QuoAssign,
    /// `token.REM_ASSIGN`.
    RemAssign,
    /// `token.AND_ASSIGN`.
    AndAssign,
    /// `token.OR_ASSIGN`.
    OrAssign,
    /// `token.XOR_ASSIGN`.
    XorAssign,
    /// `token.SHL_ASSIGN`.
    ShlAssign,
    /// `token.SHR_ASSIGN`.
    ShrAssign,
    /// `token.AND_NOT_ASSIGN`.
    AndNotAssign,
    /// `token.LAND`.
    Land,
    /// `token.LOR`.
    Lor,
    /// `token.ARROW`.
    Arrow,
    /// `token.INC`.
    Inc,
    /// `token.DEC`.
    Dec,
    /// `token.EQL`.
    Eql,
    /// `token.LSS`.
    Lss,
    /// `token.GTR`.
    Gtr,
    /// `token.ASSIGN`.
    Assign,
    /// `token.NOT`.
    Not,
    /// `token.NEQ`.
    Neq,
    /// `token.LEQ`.
    Leq,
    /// `token.GEQ`.
    Geq,
    /// `token.DEFINE`.
    Define,
    /// `token.ELLIPSIS`.
    #[strum(serialize = "ellipsis-token")]
    Ellipsis,
    /// `token.LPAREN`.
    Lparen,
    /// `token.LBRACK`.
    Lbrack,
    /// `token.LBRACE`.
    Lbrace,
    /// `token.COMMA`.
    Comma,
    /// `token.PERIOD`.
    Period,
    /// `token.RPAREN`.
    Rparen,
    /// `token.RBRACK`.
    Rbrack,
    /// `token.RBRACE`.
    Rbrace,
    /// `token.SEMICOLON`.
    Semicolon,
    /// `token.COLON`.
    Colon,
    /// `token.BREAK`.
    Break,
    /// `token.CASE`.
    Case,
    /// `token.CHAN`.
    Chan,
    /// `token.CONST`.
    Const,
    /// `token.CONTINUE`.
    Continue,
    /// `token.DEFAULT`.
    Default,
    /// `token.DEFER`.
    Defer,
    /// `token.ELSE`.
    Else,
    /// `token.FALLTHROUGH`.
    Fallthrough,
    /// `token.FOR`.
    For,
    /// `token.FUNC`.
    Func,
    /// `token.GO`.
    Go,
    /// `token.GOTO`.
    Goto,
    /// `token.IF`.
    If,
    /// `token.IMPORT`.
    Import,
    /// `token.INTERFACE`.
    Interface,
    /// `token.MAP`.
    Map,
    /// `token.PACKAGE`.
    #[strum(serialize = "package-token")]
    Package,
    /// `token.RANGE`.
    Range,
    /// `token.RETURN`.
    Return,
    /// `token.SELECT`.
    Select,
    /// `token.STRUCT`.
    Struct,
    /// `token.SWITCH`.
    Switch,
    /// `token.TYPE`.
    Type,
    /// `token.VAR`.
    Var,
    /// `token.TILDE`.
    Tilde,
}
