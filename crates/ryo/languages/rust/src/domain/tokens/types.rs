//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is a punctuation, keyword, literal or trivia `SyntaxKind` of rust-analyzer's parser, named by the kind in kebab case.

/// What a token of a Rust syntax tree is. One closed list, each name once, in the order rust-analyzer's parser lists them.
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
    /// `$`, which starts a macro metavariable.
    Dollar,
    /// `;`, which ends a statement or item.
    Semicolon,
    /// `,`, which separates list elements.
    Comma,
    /// `(`, which opens a parenthesised group.
    LParen,
    /// `)`, which closes a parenthesised group.
    RParen,
    /// `{`, which opens a braced group.
    LCurly,
    /// `}`, which closes a braced group.
    RCurly,
    /// `[`, which opens a bracketed group.
    LBrack,
    /// `]`, which closes a bracketed group.
    RBrack,
    /// `<`, which opens generics or compares.
    LAngle,
    /// `>`, which closes generics or compares.
    RAngle,
    /// `@`, which binds a pattern to a name.
    At,
    /// `#`, which starts an attribute.
    Pound,
    /// `~`, left over from old syntax and reserved.
    Tilde,
    /// `?`, which propagates an error or relaxes a bound.
    Question,
    /// `&`, which borrows or is a bitwise and.
    Amp,
    /// `|`, which delimits closure parameters, joins patterns or is a bitwise or.
    Pipe,
    /// `+`, which adds or joins bounds.
    Plus,
    /// `*`, which multiplies, dereferences or globs.
    Star,
    /// `/`, which divides.
    Slash,
    /// `^`, which is a bitwise exclusive or.
    Caret,
    /// `%`, which takes a remainder.
    Percent,
    /// `_`, which is the wildcard or the inferred type.
    Underscore,
    /// `.`, which accesses a field or calls a method.
    Dot,
    /// `..`, which makes a range or a rest.
    Dot2,
    /// `...`, which marks C variadics.
    Dot3,
    /// `..=`, which makes an inclusive range.
    Dot2eq,
    /// `:`, which introduces a type or a bound.
    Colon,
    /// `::`, which separates path segments.
    Colon2,
    /// `=`, which assigns or binds a default.
    Eq,
    /// `==`, which compares for equality.
    Eq2,
    /// `=>`, which separates a match arm from its body.
    FatArrow,
    /// `!`, which negates or invokes a macro.
    Bang,
    /// `!=`, which compares for inequality.
    Neq,
    /// `-`, which subtracts or negates.
    Minus,
    /// `->`, which introduces a return type.
    ThinArrow,
    /// `<=`, which compares less or equal.
    Lteq,
    /// `>=`, which compares greater or equal.
    Gteq,
    /// `+=`, which adds in place.
    Pluseq,
    /// `-=`, which subtracts in place.
    Minuseq,
    /// `|=`, which is a bitwise or in place.
    Pipeeq,
    /// `&=`, which is a bitwise and in place.
    Ampeq,
    /// `^=`, which is a bitwise exclusive or in place.
    Careteq,
    /// `/=`, which divides in place.
    Slasheq,
    /// `*=`, which multiplies in place.
    Stareq,
    /// `%=`, which takes a remainder in place.
    Percenteq,
    /// `&&`, which is a lazy boolean and.
    Amp2,
    /// `||`, which is a lazy boolean or.
    Pipe2,
    /// `<<`, which shifts left.
    Shl,
    /// `>>`, which shifts right.
    Shr,
    /// `<<=`, which shifts left in place.
    Shleq,
    /// `>>=`, which shifts right in place.
    Shreq,
    /// The keyword `Self`, the implementing type.
    SelfTypeKw,
    /// The keyword `abstract`, reserved for future use.
    AbstractKw,
    /// The keyword `as`, of casts and renames.
    AsKw,
    /// The keyword `become`, of tail calls.
    BecomeKw,
    /// The keyword `box`, of box patterns.
    BoxKw,
    /// The keyword `break`, which leaves a loop or block.
    BreakKw,
    /// The keyword `const`, of constants and const contexts.
    ConstKw,
    /// The keyword `continue`, which starts the next loop iteration.
    ContinueKw,
    /// The keyword `crate`, naming the current crate's root.
    CrateKw,
    /// The keyword `do`, reserved, and part of `do yeet`.
    DoKw,
    /// The keyword `else`, of the other branch of `if` and `let .. else`.
    ElseKw,
    /// The keyword `enum`, of enum items.
    EnumKw,
    /// The keyword `extern`, of foreign items, ABIs and `extern crate`.
    ExternKw,
    /// The keyword `false`, the boolean literal.
    FalseKw,
    /// The keyword `final`, reserved for future use.
    FinalKw,
    /// The keyword `fn`, of functions and function-pointer types.
    FnKw,
    /// The keyword `for`, of loops, trait impls and higher-ranked binders.
    ForKw,
    /// The keyword `if`, of conditionals and match guards.
    IfKw,
    /// The keyword `impl`, of impl blocks and opaque types.
    ImplKw,
    /// The keyword `in`, of `for` loops and `pub(in path)`.
    InKw,
    /// The keyword `let`, of bindings and let conditions.
    LetKw,
    /// The keyword `loop`, of unconditional loops.
    LoopKw,
    /// The keyword `macro`, of declarative macros 2.0.
    MacroKw,
    /// The keyword `match`, of match expressions.
    MatchKw,
    /// The keyword `mod`, of modules.
    ModKw,
    /// The keyword `move`, of closures that take ownership.
    MoveKw,
    /// The keyword `mut`, of mutable bindings, references and pointers.
    MutKw,
    /// The keyword `override`, reserved for future use.
    OverrideKw,
    /// The keyword `priv`, reserved for future use.
    PrivKw,
    /// The keyword `pub`, of visibilities.
    PubKw,
    /// The keyword `ref`, of by-reference bindings.
    RefKw,
    /// The keyword `return`, which leaves a function.
    ReturnKw,
    /// The keyword `self`, the method receiver or the current module.
    SelfKw,
    /// The keyword `static`, of statics and the `'static` bound.
    StaticKw,
    /// The keyword `struct`, of struct items.
    StructKw,
    /// The keyword `super`, naming the parent module.
    SuperKw,
    /// The keyword `trait`, of trait items.
    TraitKw,
    /// The keyword `true`, the boolean literal.
    TrueKw,
    /// The keyword `type`, of type aliases and associated types.
    TypeKw,
    /// The keyword `typeof`, reserved for future use.
    TypeofKw,
    /// The keyword `unsafe`, of unsafe blocks, functions, traits and impls.
    UnsafeKw,
    /// The keyword `unsized`, reserved for future use.
    UnsizedKw,
    /// The keyword `use`, of imports and precise capturing.
    UseKw,
    /// The keyword `virtual`, reserved for future use.
    VirtualKw,
    /// The keyword `where`, of where clauses.
    WhereKw,
    /// The keyword `while`, of conditional loops.
    WhileKw,
    /// The keyword `yield`, of coroutine yields.
    YieldKw,
    /// The keyword `asm`, of inline assembly.
    AsmKw,
    /// The keyword `async`, of async functions, blocks and closures.
    AsyncKw,
    /// The keyword `att_syntax`, an inline-assembly option.
    AttSyntaxKw,
    /// The keyword `auto`, of auto traits.
    AutoKw,
    /// The keyword `await`, of `.await`.
    AwaitKw,
    /// The keyword `bikeshed`, a placeholder keyword of unstable syntax.
    BikeshedKw,
    /// The keyword `builtin`, of `builtin #` syntax.
    BuiltinKw,
    /// The keyword `cfg_attr`, of conditional attributes.
    CfgAttrKw,
    /// The keyword `cfg`, of conditional compilation.
    CfgKw,
    /// The keyword `clobber_abi`, an inline-assembly operand.
    ClobberAbiKw,
    /// The keyword `default`, of specialised impl items.
    DefaultKw,
    /// The keyword `deref`, of deref patterns.
    DerefKw,
    /// The keyword `dyn`, of trait objects.
    DynKw,
    /// The keyword `format_args`, of the built-in formatting macro.
    FormatArgsKw,
    /// The keyword `gen`, of generator blocks.
    GenKw,
    /// The keyword `global_asm`, of module-level assembly.
    GlobalAsmKw,
    /// The keyword `include_bytes`, of the built-in byte-inclusion macro.
    IncludeBytesKw,
    /// The keyword `inlateout`, an inline-assembly operand direction.
    InlateoutKw,
    /// The keyword `inout`, an inline-assembly operand direction.
    InoutKw,
    /// The keyword `is`, of pattern types.
    IsKw,
    /// The keyword `label`, an inline-assembly operand.
    LabelKw,
    /// The keyword `lateout`, an inline-assembly operand direction.
    LateoutKw,
    /// The keyword `macro_rules`, of declarative macros.
    MacroRulesKw,
    /// The keyword `may_unwind`, an inline-assembly option.
    MayUnwindKw,
    /// The keyword `naked_asm`, of naked-function assembly.
    NakedAsmKw,
    /// The keyword `nomem`, an inline-assembly option.
    NomemKw,
    /// The keyword `noreturn`, an inline-assembly option.
    NoreturnKw,
    /// The keyword `nostack`, an inline-assembly option.
    NostackKw,
    /// The keyword `null`, of `!null` pattern-type refinements.
    NullKw,
    /// The keyword `offset_of`, of the built-in field-offset macro.
    OffsetOfKw,
    /// The keyword `options`, of inline-assembly option lists.
    OptionsKw,
    /// The keyword `out`, an inline-assembly operand direction.
    OutKw,
    /// The keyword `pattern_type`, of the built-in pattern-type macro.
    PatternTypeKw,
    /// The keyword `preserves_flags`, an inline-assembly option.
    PreservesFlagsKw,
    /// The keyword `pure`, an inline-assembly option.
    PureKw,
    /// The keyword `raw`, of raw borrows, `&raw const`.
    RawKw,
    /// The keyword `readonly`, an inline-assembly option.
    ReadonlyKw,
    /// The keyword `safe`, of safe items in `unsafe extern` blocks.
    SafeKw,
    /// The keyword `sym`, of inline-assembly symbol operands.
    SymKw,
    /// The keyword `try`, of try blocks.
    TryKw,
    /// The keyword `union`, of union items.
    UnionKw,
    /// The keyword `yeet`, of `do yeet`.
    YeetKw,
    /// A byte literal, `b'a'`.
    Byte,
    /// A byte-string literal, `b"ab"`.
    ByteString,
    /// A character literal, `'a'`.
    Char,
    /// A C-string literal, `c"ab"`.
    CString,
    /// A floating-point literal, `1.5`.
    FloatNumber,
    /// An integer literal, `42` or `0xff_u8`.
    IntNumber,
    /// A string literal, `"ab"` or `r"ab"`.
    String,
    /// A line or block comment, doc comments included.
    Comment,
    /// Text the parser could not read as any token.
    Error,
    /// The `---` frontmatter block at the top of a script file.
    Frontmatter,
    /// An identifier, raw ones included.
    Ident,
    /// A lifetime or label name, `'a`.
    LifetimeIdent,
    /// A line break the lexer keeps apart from other whitespace.
    Newline,
    /// The `#!` line at the top of a script file.
    Shebang,
    /// Whitespace between tokens.
    Whitespace,
}
