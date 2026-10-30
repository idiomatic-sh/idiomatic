//! The types of the `signatures` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::Position;

/// The parts of a function, each the element that holds it. The trees are those of `go/ast` nested by the bytes they span, so the
/// name, the receiver and the lists of a declaration are all under its `FuncType`, which starts at `func`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Signature {
    /// The `FieldList` of the receiver.
    pub receiver: Option<Position>,
    /// The `Ident` of the name; a function literal and a function type have none.
    pub name: Option<Position>,
    /// The `FieldList` of the type parameters.
    pub type_parameters: Option<Position>,
    /// The `FieldList` of the parameters.
    pub parameters: Option<Position>,
    /// The `FieldList` of the results.
    pub results: Option<Position>,
    /// The `BlockStmt` of the body.
    pub body: Option<Position>,
}

/// What a list of fields is, by what it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Listing {
    /// The fields of a struct.
    Members,
    /// The methods and the type elements of an interface.
    Methods,
    /// The receiver of a method.
    Receiver,
    /// The type parameters of a function or a type.
    TypeParameters,
    /// The parameters of a function.
    Parameters,
    /// The results of a function.
    Results,
}

/// Where a list of parameters belongs: the function, or the method of an interface, that is called with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Callable {
    /// The declaration, the literal or the interface method.
    pub callable: Position,
}

/// What a parameter is in Go's own words, the `refinement` of its row: `variadic`, `result`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Refinement(pub &'static str);

/// A parameter, a receiver or a named result, as the `parameter` relation says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Parameter {
    /// How it takes its argument.
    pub kind: ryo::languages::ParameterKind,
    /// The same in Go's own words, the `refinement` of its row.
    pub word: Refinement,
    /// The element whose span is the parameter's: its name, or its type when it has none.
    pub named: Position,
    /// The name, or the unknown value of a parameter with none.
    pub name: ryo::languages::Symbol,
    /// Its place in its list, from zero.
    pub ordinal: ryo::languages::Integer,
    /// The function or function literal whose parameter it is.
    pub callable: Position,
}
