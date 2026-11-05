//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is a composite `SyntaxKind` of rust-analyzer's parser, named by the kind in kebab case.

/// What a node of a Rust syntax tree is. One closed list, each name once, in the order rust-analyzer's parser lists them.
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
pub(crate) enum Node {
    /// The ABI string of an `extern "C"` item or function pointer.
    Abi,
    /// The parenthesised arguments of a call, `(a, b)`.
    ArgList,
    /// An array literal, `[a, b]` or `[x; n]`.
    ArrayExpr,
    /// A fixed-length array type, `[T; N]`.
    ArrayType,
    /// A `clobber_abi("C")` operand of inline assembly.
    AsmClobberAbi,
    /// A `const` operand of inline assembly.
    AsmConst,
    /// The direction of an inline-assembly register operand: `in`, `out`, `inout` and their late forms.
    AsmDirSpec,
    /// An `asm!` or `global_asm!` invocation, read as an expression.
    AsmExpr,
    /// A `label { .. }` operand of inline assembly.
    AsmLabel,
    /// The expression, or `in => out` pair, an inline-assembly register operand binds.
    AsmOperandExpr,
    /// An inline-assembly operand with an optional `name =` before it.
    AsmOperandNamed,
    /// One option of an inline-assembly `options(..)` list, such as `nostack`.
    AsmOption,
    /// The `options(..)` list of an inline-assembly invocation.
    AsmOptions,
    /// A register operand of inline assembly: direction, register and expression.
    AsmRegOperand,
    /// The register an inline-assembly operand names: a class or an explicit register string.
    AsmRegSpec,
    /// A `sym path` operand of inline assembly.
    AsmSym,
    /// The braced body of a trait or impl: its associated items.
    AssocItemList,
    /// An associated-type binding or bound in generic arguments, `Item = T` or `Item: Bound`.
    AssocTypeArg,
    /// An outer or inner attribute, `#[..]` or `#![..]`.
    Attr,
    /// A `.await` on a future.
    AwaitExpr,
    /// A tail call, `become f(x)`.
    BecomeExpr,
    /// An expression with an infix operator, `a + b`, `a = b` or `a && b`.
    BinExpr,
    /// A braced block, with its optional `unsafe`, `async`, `const` or label.
    BlockExpr,
    /// A `box` pattern.
    BoxPat,
    /// A `break`, with its optional label and value.
    BreakExpr,
    /// A call of a function or tuple constructor, `f(x)`.
    CallExpr,
    /// An `as` cast, `x as u32`.
    CastExpr,
    /// One predicate inside a `cfg`, such as `test` or `feature = "x"`.
    CfgAtom,
    /// The `cfg_attr(predicate, attrs)` form of an attribute.
    CfgAttrMeta,
    /// An `all(..)`, `any(..)` or `not(..)` combination of `cfg` predicates.
    CfgComposite,
    /// The `cfg(..)` form of an attribute.
    CfgMeta,
    /// A closure, `|x| x + 1`, with its optional `move`, `async` and binder.
    ClosureExpr,
    /// A `const` item.
    Const,
    /// A constant passed as a generic argument, `{ N + 1 }` or a literal.
    ConstArg,
    /// An inline `const { .. }` block used as a pattern.
    ConstBlockPat,
    /// A const generic parameter, `const N: usize`.
    ConstParam,
    /// A `continue`, with its optional label.
    ContinueExpr,
    /// A `deref!(..)` pattern that matches through a smart pointer.
    DerefPat,
    /// A trait-object type, `dyn Trait`.
    DynTraitType,
    /// An `enum` item.
    Enum,
    /// An expression used as a statement, with its optional `;`.
    ExprStmt,
    /// An `extern` block of foreign items.
    ExternBlock,
    /// An `extern crate` item.
    ExternCrate,
    /// The braced foreign items of an `extern` block.
    ExternItemList,
    /// A field access, `value.field` or `tuple.0`.
    FieldExpr,
    /// A function or method item.
    Fn,
    /// A function-pointer type, `fn(u8) -> u8`.
    FnPtrType,
    /// One argument of a `format_args!` call, with its optional `name =`.
    FormatArgsArg,
    /// A `format_args!` invocation, read as an expression.
    FormatArgsExpr,
    /// A higher-ranked binder, `for<'a>`.
    ForBinder,
    /// A `for` loop.
    ForExpr,
    /// A higher-ranked type, `for<'a> fn(&'a u8)`.
    ForType,
    /// The angle-bracketed generic arguments of a path, `::<T, 'a>`.
    GenericArgList,
    /// The angle-bracketed generic parameters of an item, `<T, const N: usize>`.
    GenericParamList,
    /// A binding pattern, `x`, `mut x`, `ref x` or `x @ pat`.
    IdentPat,
    /// An `if` or `if let` expression, with its `else`.
    IfExpr,
    /// An `impl` block, inherent or of a trait.
    Impl,
    /// An `impl(crate)` restriction on who may implement a trait.
    ImplRestriction,
    /// An opaque type, `impl Trait`.
    ImplTraitType,
    /// An `include_bytes!` invocation, read as an expression.
    IncludeBytesExpr,
    /// An index, `value[i]`.
    IndexExpr,
    /// The inferred type `_`.
    InferType,
    /// The braced items of an inline module.
    ItemList,
    /// A `key = value` attribute input.
    KeyValueMeta,
    /// A loop or block label, `'outer:`.
    Label,
    /// The `else` block of a `let .. else` statement.
    LetElse,
    /// A `let` condition inside `if` or `while`.
    LetExpr,
    /// A `let` statement.
    LetStmt,
    /// A lifetime, `'a` or `'static`.
    Lifetime,
    /// A lifetime passed as a generic argument.
    LifetimeArg,
    /// A lifetime generic parameter with its bounds, `'a: 'b`.
    LifetimeParam,
    /// A literal value: number, string, character, byte or boolean.
    Literal,
    /// A literal used as a pattern, optionally negated.
    LiteralPat,
    /// An unconditional `loop`.
    LoopExpr,
    /// A macro invocation, `name!(..)`, before expansion.
    MacroCall,
    /// A declarative macro of the `macro name { .. }` form.
    MacroDef,
    /// A macro invocation in expression position.
    MacroExpr,
    /// The items a macro invocation expands to.
    MacroItems,
    /// A macro invocation in pattern position.
    MacroPat,
    /// A `macro_rules!` definition.
    MacroRules,
    /// The statements a macro invocation expands to.
    MacroStmts,
    /// A macro invocation in type position.
    MacroType,
    /// One arm of a `match`: pattern, guard and body.
    MatchArm,
    /// The braced arms of a `match`.
    MatchArmList,
    /// A `match` expression.
    MatchExpr,
    /// The `if` guard of a match arm.
    MatchGuard,
    /// A method call, `value.method(x)`.
    MethodCallExpr,
    /// A `mod` item, inline or declared.
    Module,
    /// A `mut(crate)` restriction on who may mutate a field.
    MutRestriction,
    /// The name an item, binding or field is declared with.
    Name,
    /// A name used to refer to something declared elsewhere.
    NameRef,
    /// The never type `!`.
    NeverType,
    /// A `!null` refinement in a pattern type.
    NotNull,
    /// An `offset_of!` invocation, read as an expression.
    OffsetOfExpr,
    /// An alternative of patterns, `a | b`.
    OrPat,
    /// One parameter of a function or closure.
    Param,
    /// The parenthesised parameters of a function or the bars of a closure.
    ParamList,
    /// The `(A, B) -> C` arguments of a `Fn`-family trait path.
    ParenthesizedArgList,
    /// An expression in parentheses.
    ParenExpr,
    /// A pattern in parentheses.
    ParenPat,
    /// A type in parentheses.
    ParenType,
    /// A path, `a::b::C`, built from its qualifier and last segment.
    Path,
    /// A path used as an expression.
    PathExpr,
    /// An attribute that is a bare path, `#[test]`.
    PathMeta,
    /// A path used as a pattern, such as a unit variant.
    PathPat,
    /// One segment of a path, with its generic arguments.
    PathSegment,
    /// A path used as a type.
    PathType,
    /// A pattern type, `u32 is 1..`.
    PatternType,
    /// A prefix operator applied to an expression: `-x`, `!x` or `*x`.
    PrefixExpr,
    /// A raw-pointer type, `*const T` or `*mut T`.
    PtrType,
    /// A range expression, `a..b` or `a..=b`.
    RangeExpr,
    /// A range pattern, `0..=9`.
    RangePat,
    /// A struct literal with named fields, `Point { x, y }`.
    RecordExpr,
    /// One `field: value` of a struct literal.
    RecordExprField,
    /// The braced fields of a struct literal, with its `..base`.
    RecordExprFieldList,
    /// One named field of a struct, union or variant declaration.
    RecordField,
    /// The braced named fields of a declaration.
    RecordFieldList,
    /// A struct pattern with named fields, `Point { x, .. }`.
    RecordPat,
    /// One `field: pattern` of a struct pattern.
    RecordPatField,
    /// The braced fields of a struct pattern.
    RecordPatFieldList,
    /// A borrow, `&x`, `&mut x` or `&raw const x`.
    RefExpr,
    /// A reference pattern, `&x`.
    RefPat,
    /// A reference type, `&'a T` or `&mut T`.
    RefType,
    /// The `as name` of an import or `extern crate`.
    Rename,
    /// The rest pattern `..`.
    RestPat,
    /// A `return`, with its optional value.
    ReturnExpr,
    /// The `(..)` return-type notation of an associated function in a bound.
    ReturnTypeSyntax,
    /// The `-> T` of a function or closure.
    RetType,
    /// The `self` parameter of a method, in any of its forms.
    SelfParam,
    /// A slice pattern, `[first, .., last]`.
    SlicePat,
    /// A slice type, `[T]`.
    SliceType,
    /// The root of a file's tree: every item and attribute it holds.
    SourceFile,
    /// A `static` item.
    Static,
    /// The braced statements and tail expression of a block.
    StmtList,
    /// A `struct` item.
    Struct,
    /// A delimited token tree, the unparsed input of a macro or attribute.
    TokenTree,
    /// An attribute whose input is a path and a token tree, `#[derive(..)]`.
    TokenTreeMeta,
    /// A `trait` item.
    Trait,
    /// The `try` that makes a block a try block.
    TryBlockModifier,
    /// The `?` operator applied to an expression.
    TryExpr,
    /// A tuple expression, `(a, b)`.
    TupleExpr,
    /// One positional field of a tuple struct or variant.
    TupleField,
    /// The parenthesised positional fields of a declaration.
    TupleFieldList,
    /// A tuple pattern, `(a, b)`.
    TuplePat,
    /// A tuple-struct pattern, `Some(x)`.
    TupleStructPat,
    /// A tuple type, `(A, B)`, the unit type included.
    TupleType,
    /// A `type` item, free or associated.
    TypeAlias,
    /// The `<T as Trait>` qualifier at the start of a qualified path.
    TypeAnchor,
    /// A type passed as a generic argument.
    TypeArg,
    /// One bound on a type or lifetime: a trait, a lifetime or `use<..>`.
    TypeBound,
    /// The `+`-separated bounds after a `:` or `impl`/`dyn`.
    TypeBoundList,
    /// A type generic parameter with its bounds and default.
    TypeParam,
    /// The `_` placeholder in an assignment target.
    UnderscoreExpr,
    /// A `union` item.
    Union,
    /// An `unsafe(..)` attribute input.
    UnsafeMeta,
    /// A `use` item.
    Use,
    /// The `use<'a, T>` precise-capturing arguments of an `impl Trait`.
    UseBoundGenericArgs,
    /// One tree of a `use`: a path with its rename, glob or nested list.
    UseTree,
    /// The braced nested trees of a `use`.
    UseTreeList,
    /// One variant of an `enum`.
    Variant,
    /// The braced variants of an `enum`.
    VariantList,
    /// A visibility, `pub` or `pub(crate)`.
    Visibility,
    /// The parenthesised scope of a visibility, `(crate)` or `(in path)`.
    VisibilityInner,
    /// A `where` clause of an item.
    WhereClause,
    /// One predicate of a `where` clause.
    WherePred,
    /// A `while` or `while let` loop.
    WhileExpr,
    /// The wildcard pattern `_`.
    WildcardPat,
    /// A `do yeet` expression.
    YeetExpr,
    /// A `yield` in a coroutine.
    YieldExpr,
}
