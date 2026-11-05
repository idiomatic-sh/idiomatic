//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is a node
//! of the syntax tree that `go/ast` defines, named by its type in kebab case.

/// What a node of a Go syntax tree is. One closed list, each name once, in the order `go/ast` lists its types.
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
    /// `ast.ArrayType`.
    ArrayType,
    /// `ast.AssignStmt`.
    AssignStmt,
    /// `ast.BadDecl`.
    BadDecl,
    /// `ast.BadExpr`.
    BadExpr,
    /// `ast.BadStmt`.
    BadStmt,
    /// `ast.BasicLit`.
    BasicLit,
    /// `ast.BinaryExpr`.
    BinaryExpr,
    /// `ast.BlockStmt`.
    BlockStmt,
    /// `ast.BranchStmt`.
    BranchStmt,
    /// `ast.CallExpr`.
    CallExpr,
    /// `ast.CaseClause`.
    CaseClause,
    /// `ast.ChanType`.
    ChanType,
    /// `ast.CommClause`.
    CommClause,
    /// `ast.Comment`.
    Comment,
    /// `ast.CommentGroup`.
    CommentGroup,
    /// `ast.CompositeLit`.
    CompositeLit,
    /// `ast.DeclStmt`.
    DeclStmt,
    /// `ast.DeferStmt`.
    DeferStmt,
    /// `ast.Ellipsis`.
    Ellipsis,
    /// `ast.EmptyStmt`.
    EmptyStmt,
    /// `ast.ExprStmt`.
    ExprStmt,
    /// `ast.Field`.
    Field,
    /// `ast.FieldList`.
    FieldList,
    /// `ast.File`.
    File,
    /// `ast.ForStmt`.
    ForStmt,
    /// `ast.FuncDecl`.
    FuncDecl,
    /// `ast.FuncLit`.
    FuncLit,
    /// `ast.FuncType`.
    FuncType,
    /// `ast.GenDecl`.
    GenDecl,
    /// `ast.GoStmt`.
    GoStmt,
    /// `ast.Ident`.
    Ident,
    /// `ast.IfStmt`.
    IfStmt,
    /// `ast.ImportSpec`.
    ImportSpec,
    /// `ast.IncDecStmt`.
    IncDecStmt,
    /// `ast.IndexExpr`.
    IndexExpr,
    /// `ast.IndexListExpr`.
    IndexListExpr,
    /// `ast.InterfaceType`.
    InterfaceType,
    /// `ast.KeyValueExpr`.
    KeyValueExpr,
    /// `ast.LabeledStmt`.
    LabeledStmt,
    /// `ast.MapType`.
    MapType,
    /// `ast.Package`.
    Package,
    /// `ast.ParenExpr`.
    ParenExpr,
    /// `ast.RangeStmt`.
    RangeStmt,
    /// `ast.ReturnStmt`.
    ReturnStmt,
    /// `ast.SelectStmt`.
    SelectStmt,
    /// `ast.SelectorExpr`.
    SelectorExpr,
    /// `ast.SendStmt`.
    SendStmt,
    /// `ast.SliceExpr`.
    SliceExpr,
    /// `ast.StarExpr`.
    StarExpr,
    /// `ast.StructType`.
    StructType,
    /// `ast.SwitchStmt`.
    SwitchStmt,
    /// `ast.TypeAssertExpr`.
    TypeAssertExpr,
    /// `ast.TypeSpec`.
    TypeSpec,
    /// `ast.TypeSwitchStmt`.
    TypeSwitchStmt,
    /// `ast.UnaryExpr`.
    UnaryExpr,
    /// `ast.ValueSpec`.
    ValueSpec,
}
