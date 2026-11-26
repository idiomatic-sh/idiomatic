//! Parser-free values retained between literal-expression reads.



/// The syntax requested for a reconstructed source carrier.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum LiteralDialect {
    /// Ruff's Python string-annotation parser, independent of typing position.
    PythonStringAnnotation,
}


/// The reconstructed spelling handed to a grammar, borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CarrierInput<'a>(pub &'a str);


/// A complete syntactic outcome, or explicit unavailable carrier knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralParseOutcome {
    /// The selected grammar accepted the original carrier.
    Parsed,
    /// The carrier was complete but its annotation expression was invalid or empty.
    Malformed,
    /// Logical source or reconstruction was unavailable; never a negative parse result.
    Unavailable,
}




