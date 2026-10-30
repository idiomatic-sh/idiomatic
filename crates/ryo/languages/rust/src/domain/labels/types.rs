//! The types of the `labels` concept: plain data, public fields and derives, with no constructor and no inherent impl. A field is the role
//! of an element under its parent, in the Rust grammar's own words: the labels of `rust.ungram` and the names of the accessors of its
//! typed syntax tree. One closed list, each name once, `None` first: the number `0` is the role of an edge that has none.

/// The role of an element under its parent.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
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
pub(crate) enum Field {
    /// The edge has no role.
    #[default]
    None,
    /// The name a construct declares.
    Name,
    /// The name a construct refers to.
    NameRef,
    /// The path of a construct.
    Path,
    /// The last segment of a path.
    Segment,
    /// The path before the last segment of a path.
    Qualifier,
    /// The visibility of an item.
    Visibility,
    /// An attribute of a construct.
    Attrs,
    /// A lifetime a construct names.
    Lifetime,
    /// The label of a loop or block.
    Label,
    /// The ABI of a function or block.
    Abi,
    /// The `as` name of an import.
    Rename,
    /// The generic parameters of a construct.
    GenericParamList,
    /// The generic arguments of a path segment.
    GenericArgList,
    /// The `where` clause of a construct.
    WhereClause,
    /// The parameters of a function or closure.
    ParamList,
    /// The `->` type of a function or closure.
    RetType,
    /// The arguments of a call.
    ArgList,
    /// The items of a module.
    ItemList,
    /// The associated items of an impl or trait.
    AssocItemList,
    /// The fields of a struct or variant written with names.
    RecordFieldList,
    /// The fields of a struct or variant written by position.
    TupleFieldList,
    /// The variants of an enum.
    VariantList,
    /// The arms of a match.
    MatchArmList,
    /// The statements of a block.
    StmtList,
    /// The trees of a braced import.
    UseTreeList,
    /// The fields of a struct expression.
    RecordExprFieldList,
    /// The fields of a struct pattern.
    RecordPatFieldList,
    /// The tree of an import.
    UseTree,
    /// The `else` of a `let`.
    LetElse,
    /// An item of a file, module or impl.
    Items,
    /// A statement of a block.
    Statements,
    /// The expression that ends a block and gives its value.
    TailExpr,
    /// The expression a construct holds.
    Expr,
    /// The pattern a construct holds.
    Pat,
    /// The type a construct holds.
    Ty,
    /// A field, element or operand of a construct.
    Fields,
    /// A variant of an enum.
    Variants,
    /// A parameter of a list.
    Params,
    /// An argument of a call or a list.
    Args,
    /// An arm of a match.
    Arms,
    /// A bound of a bound list.
    Bounds,
    /// A predicate of a `where` clause.
    Predicates,
    /// The condition of an `if`, a `while` or a guard.
    Condition,
    /// The block an `if` runs when its condition holds.
    ThenBranch,
    /// The block or `if` an `if` runs otherwise.
    ElseBranch,
    /// The block a loop repeats.
    LoopBody,
    /// What a `for` goes over.
    Iterable,
    /// What a method is called on.
    Receiver,
    /// The left operand of a binary expression.
    Lhs,
    /// The right operand of a binary expression.
    Rhs,
    /// The operator of a prefix, binary or range construct.
    Op,
    /// What an index expression indexes.
    Base,
    /// The index of an index expression.
    Index,
    /// The start of a range.
    Start,
    /// The end of a range.
    End,
    /// The base a struct expression is completed from.
    Spread,
    /// What a `let` binds its pattern to.
    Initializer,
    /// The guard of a match arm.
    Guard,
    /// The body of a function, closure, constant or macro.
    Body,
    /// The type an impl is for.
    SelfTy,
    /// The trait an impl implements.
    Trait,
    /// The default of a type parameter.
    DefaultType,
    /// The default of a const parameter.
    DefaultVal,
    /// The keyword of a configuration predicate.
    Keyword,
}

/// What a kind of element is, as far as the roles it can play are concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    /// An expression.
    Expression,
    /// A pattern.
    Pattern,
    /// A type.
    Type,
    /// An item.
    Item,
    /// A statement.
    Statement,
    /// Anything else.
    Other,
}

/// Whether an element comes after a given token among its parent's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Following {
    /// It does.
    After,
    /// It comes before it, or the parent has no such token.
    Before,
}

/// Whether a node has another node after it under its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Closing {
    /// It is the last node.
    Last,
    /// Another node follows.
    NotLast,
}

/// Whether a node of the kind asked stands before an element under its parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Precedence {
    /// None does: the element is the first.
    Alone,
    /// One or more do.
    Preceded,
}
