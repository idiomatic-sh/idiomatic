//! The types of the `grammar` concept: plain data, public fields and derives, with no constructor and no inherent impl. The parser
//! library's tree of a file is held in one table, each node knowing its parent, its first and last child and its next sibling.

use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// A place among the bytes of the text the library read, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ByteIndex(pub usize);

/// A place among the nodes of the library's tree, in the order the library visits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Index(pub usize);

/// The text of the file the library read, as it is written: what the spans of its nodes are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GoText<'a>(pub &'a str);

/// What a node of the library's tree is in Go: the word of the Go syntax, not the library's name for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// The whole file.
    Source,
    /// Text the library could not read.
    Broken,
    /// A comment.
    Comment,
    /// A name: an identifier, a field, a package or a label name, the blank identifier, `nil`, `true`, `false` or `iota`.
    Name,
    /// The name of a type.
    TypeName,
    /// The `.` of a dot import, which `go/ast` has as an identifier.
    Dot,
    /// A literal of a basic type other than a string: a node of `go/ast` over a token of this kind.
    Literal(Token),
    /// A string, interpreted or raw.
    Text,
    /// A group the library makes that `go/ast` does not: its children are its parent's.
    Transparent,
    /// A list of statements: a group of the library's, with the end `go/ast` gives its last statement.
    StatementList,
    /// The clause of a `for` that ranges.
    RangeClause,
    /// What the escapes and the content of a literal are made of, which the literal's token holds.
    Inside,
    /// A keyword, an operator or a piece of punctuation, as `go/token` spells it.
    Spelled(Token),
    /// A keyword, an operator or a piece of punctuation `go/token` has no constant for.
    Unspelled,
    /// A node of `go/ast` over the same bytes.
    Node(Node),
    /// `func name(...)`: a `FuncDecl` with its `FuncType`.
    Function,
    /// `func (r T) name(...)`: a `FuncDecl` with its `FuncType`.
    Method,
    /// `func(...) { ... }`: a `FuncLit` with its `FuncType`.
    Closure,
    /// `func(...) T`: a `FuncType`, with its results.
    Signature,
    /// `name(...) T` in an interface: a `Field` holding a `FuncType` that starts at the parameters.
    MethodElement,
    /// `A | B` or `~A`: a `Field` in an interface, a type argument or a constraint, a chain of `BinaryExpr` when it has several.
    Union,
    /// `(a, b int)`: a `FieldList`, whose names the library may have split from their type.
    Parameters,
    /// `T[A, B]` as a type: an `IndexExpr` or an `IndexListExpr`.
    GenericType,
    /// `f[A, B]` as an expression, a generic function instantiated.
    Instantiation,
    /// `f(...)` and `f[T](...)`: a `CallExpr`, and the instantiation inside it.
    Call,
    /// `T(x)`: a call, and the `*` of a conversion to a pointer type.
    Conversion,
    /// `*x` and `-x`: a `StarExpr` or a `UnaryExpr`.
    Unary,
    /// `{ ... }` of a type: an `InterfaceType` and its `FieldList`.
    Interface,
    /// A field of a struct: a `Field`, a `StarExpr` over an embedded pointer.
    FieldDeclaration,
    /// `name ...T`: a `Field` holding an `Ellipsis`.
    Variadic,
    /// `[...]T`: an `ArrayType` whose length is an `Ellipsis`.
    ImplicitArray,
    /// `const`, `var` and `type`: a `GenDecl`, in a `DeclStmt` where a statement is.
    Declaration,
    /// `import`: a `GenDecl`.
    Import,
    /// `switch x {`: a `SwitchStmt`, and a `BlockStmt` for its clauses.
    Switch,
    /// `switch x := y.(type) {`: a `TypeSwitchStmt`, its assignment and its `BlockStmt`.
    TypeSwitch,
    /// `select {`: a `SelectStmt`, and a `BlockStmt` for its clauses.
    Select,
    /// `for`: a `ForStmt` or a `RangeStmt`.
    Loop,
    /// `x := <-c` and `<-c` in a `select`: an `AssignStmt` or an `ExprStmt`.
    Receive,
    /// `{ ... }` of a composite literal: nothing when the literal's type is before it, a `CompositeLit` when it stands alone.
    Elements,
    /// A statement the library names after the keyword it starts with: a `BranchStmt`.
    Branch(Token),
    /// `L:`: a `LabeledStmt`, and the empty statement `go/parser` makes after a label that has none.
    Labeled,
    /// A kind of node the adapter does not know.
    Unknown,
}

/// The part a node plays in its parent, where the library names one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// The name something declares.
    Name,
    /// A type.
    Type,
    /// The results of a signature.
    Result,
    /// The parameters of a signature.
    Parameters,
    /// The type arguments of a call or an instantiation.
    TypeArguments,
    /// The value a type switch examines.
    Value,
    /// The name a type switch binds.
    Alias,
    /// The left side of a receive.
    Left,
}

/// Whether the library names a node in its grammar, or writes it by its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Naming {
    /// A named node: a construct of the grammar.
    Named,
    /// An anonymous node: a keyword, an operator or a piece of punctuation.
    Anonymous,
}

/// Whether a node is in the file, or one the library made up to recover from an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Presence {
    /// It is read from the file.
    Read,
    /// It is a token the parser expected and did not find, which has no text.
    Missing,
}

/// A node of the library's tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Raw {
    /// What it is in Go.
    pub form: Form,
    /// The part it plays in its parent.
    pub role: Option<Role>,
    /// Whether the library names it.
    pub naming: Naming,
    /// Whether the file has it.
    pub presence: Presence,
    /// Where it starts.
    pub start: ByteIndex,
    /// Where it ends.
    pub end: ByteIndex,
    /// The node it is under.
    pub parent: Option<Index>,
    /// The first node under it.
    pub first: Option<Index>,
    /// The last node under it.
    pub last: Option<Index>,
    /// The node after it, under the same parent.
    pub next: Option<Index>,
    /// The node before it, under the same parent.
    pub previous: Option<Index>,
}

/// The library's tree of a file: its nodes in the order the library visits them, the first the root, and the text they span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Concrete<'a> {
    /// The nodes.
    pub nodes: Vec<Raw>,
    /// The text the nodes span.
    pub text: GoText<'a>,
}
