//! The types of the `roles` concept: plain data, public fields and derives, with no constructor and no inherent impl. A role is the
//! field of a node of `go/ast` that holds an element, in kebab case, `Pos` cut from the end of a field that says where a token is
//! (`OpPos` is `op`), as `tests/fixtures/standard/dump.go` reads the fields with `reflect`.

/// The role of an element under its parent: the field of `go/ast` that holds it. One closed list, in the order of their names.
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
pub(crate) enum AstField {
    /// The edge has no field of `go/ast`: a comma, a semicolon a line end inserts, a bracket no field says where it is.
    #[default]
    None,
    /// `CallExpr.Args`.
    Args,
    /// `ChanType.Arrow` and `SendStmt.Arrow`.
    Arrow,
    /// `TypeSpec.Assign` and `TypeSwitchStmt.Assign`.
    Assign,
    /// `ChanType.Begin`.
    Begin,
    /// The `Body` of a function, a loop, a branch, a switch, a select or a clause.
    Body,
    /// `GoStmt.Call` and `DeferStmt.Call`.
    Call,
    /// `CaseClause.Case` and `CommClause.Case`.
    Case,
    /// `SendStmt.Chan`.
    Chan,
    /// `FieldList.Closing`.
    Closing,
    /// The `Colon` of a clause, a key-value pair or a labeled statement.
    Colon,
    /// `CommClause.Comm`.
    Comm,
    /// The `Cond` of an `if` or a `for`.
    Cond,
    /// `DeclStmt.Decl`.
    Decl,
    /// `File.Decls`.
    Decls,
    /// `DeferStmt.Defer`.
    Defer,
    /// `Ellipsis.Ellipsis` and `CallExpr.Ellipsis`.
    Ellipsis,
    /// `IfStmt.Else`.
    Else,
    /// `ArrayType.Elt` and `Ellipsis.Elt`.
    Elt,
    /// `CompositeLit.Elts`.
    Elts,
    /// `StructType.Fields`.
    Fields,
    /// `ForStmt.For` and `RangeStmt.For`.
    For,
    /// `CallExpr.Fun`.
    Fun,
    /// `FuncType.Func`.
    Func,
    /// `GoStmt.Go`.
    Go,
    /// `SliceExpr.High`.
    High,
    /// `IfStmt.If`.
    If,
    /// `IndexExpr.Index`.
    Index,
    /// `IndexListExpr.Indices`.
    Indices,
    /// The `Init` of an `if`, a `for` or a switch.
    Init,
    /// `InterfaceType.Interface`.
    Interface,
    /// `MapType.Key`, `KeyValueExpr.Key` and `RangeStmt.Key`.
    Key,
    /// `BranchStmt.Label` and `LabeledStmt.Label`.
    Label,
    /// The `Lbrace` of a block or a composite literal.
    Lbrace,
    /// The `Lbrack` of an array type, an index or a slice.
    Lbrack,
    /// `ArrayType.Len`.
    Len,
    /// `AssignStmt.Lhs`.
    Lhs,
    /// The `List` of a block, a field list or a case clause.
    List,
    /// `SliceExpr.Low`.
    Low,
    /// The `Lparen` of a call, a declaration, a parenthesized expression or a type assertion.
    Lparen,
    /// `MapType.Map`.
    Map,
    /// `SliceExpr.Max`.
    Max,
    /// `InterfaceType.Methods`.
    Methods,
    /// The `Name` of a file, a function, a type, an import or an identifier (`Ident.NamePos`).
    Name,
    /// The `Names` of a field or a value spec.
    Names,
    /// The `Op` of a unary or a binary expression (`OpPos`).
    Op,
    /// `FieldList.Opening`.
    Opening,
    /// `File.Package`.
    Package,
    /// `FuncType.Params`.
    Params,
    /// `ImportSpec.Path`.
    Path,
    /// `ForStmt.Post`.
    Post,
    /// `RangeStmt.Range`.
    Range,
    /// The `Rbrace` of a block or a composite literal.
    Rbrace,
    /// The `Rbrack` of an index or a slice.
    Rbrack,
    /// `FuncDecl.Recv`.
    Recv,
    /// `FuncType.Results` and `ReturnStmt.Results`.
    Results,
    /// `ReturnStmt.Return`.
    Return,
    /// `AssignStmt.Rhs`.
    Rhs,
    /// The `Rparen` of a call, a declaration, a parenthesized expression or a type assertion.
    Rparen,
    /// `SelectStmt.Select`.
    Select,
    /// `SelectorExpr.Sel`.
    Sel,
    /// `EmptyStmt.Semicolon`.
    Semicolon,
    /// `GenDecl.Specs`.
    Specs,
    /// `StarExpr.Star`.
    Star,
    /// `LabeledStmt.Stmt`.
    Stmt,
    /// `StructType.Struct`.
    Struct,
    /// The `Switch` of a switch or a type switch.
    Switch,
    /// `Field.Tag` and `SwitchStmt.Tag`.
    Tag,
    /// The `Tok` of a declaration, an assignment, a branch, an increment or a range (`TokPos`).
    Tok,
    /// The `Type` of a field, a spec, a function, a composite literal or a type assertion.
    Type,
    /// The `TypeParams` of a function or a type spec.
    TypeParams,
    /// `BasicLit.Value` (`ValuePos`), `MapType.Value`, `ChanType.Value`, `KeyValueExpr.Value`, `RangeStmt.Value` and `SendStmt.Value`.
    Value,
    /// `ValueSpec.Values`.
    Values,
    /// The `X` of an expression or a statement that has one.
    #[strum(serialize = "x")]
    FieldX,
    /// `BinaryExpr.Y`.
    #[strum(serialize = "y")]
    FieldY,
}

/// Whether what a role is chosen by holds of an element: it has no sibling of a kind before it, or it is of a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Holding {
    /// It holds.
    Holds,
    /// It does not.
    Fails,
}
