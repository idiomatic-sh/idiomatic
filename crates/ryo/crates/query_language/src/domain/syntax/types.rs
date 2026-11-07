//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. The operations of the language and the arguments they take are enums;
//! their outside text is in `constants.rs`.

use ryo_values::domain::query_operators::types::QueryOperator;

/// What an argument of a call is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgumentRole {
    /// It names a relation or a step the call reads.
    Input,
    /// It is anything else the call takes.
    Other,
}

/// The arguments a call can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ArgumentSlot {
    /// Syntax node whose text is requested.
    Node,
    /// Embedding role for rendered syntax.
    Context,
    /// Maximum physical display columns.
    MaxLine,
    /// Physical tab stop width.
    TabWidth,
    /// Ordered rendered text columns for a physical width check.
    Texts,
    /// Literal separators around and between the rendered columns.
    Literals,
    /// Root entity column.
    Root,
    /// Outgoing-edge order column.
    Order,
    /// Edge identity column.
    Edge,
    /// Span supplying the start.
    Start,
    /// Span supplying the end.
    End,
    /// Physical source span to search.
    Span,
    /// Output name for the span's zero-based start byte.
    StartOutput,
    /// Output name for the span's exclusive end byte.
    EndOutput,
    /// One-based physical line of an input span's first byte.
    BaseLine,
    /// Optional output name for each occurrence's physical start line.
    LineOutput,
    /// Named regex group to project from each physical match.
    CaptureName,
    /// Output name for the named group's absolute source span.
    CaptureOutput,
    /// Optional output name for the group's physical text.
    CaptureTextOutput,
    /// Optional output name for the group's start line.
    CaptureLineOutput,
    /// Constant replacement text.
    Replacement,
    /// Left numeric ordering column.
    LeftOrder,
    /// Right numeric ordering column.
    RightOrder,
    /// `from`.
    From,
    /// `columns`.
    Columns,
    /// `values`.
    Values,
    /// `left`.
    Left,
    /// `right`.
    Right,
    /// `left-on`.
    LeftOn,
    /// `right-on`.
    RightOn,
    /// `output`.
    Output,
    /// `keys`.
    Keys,
    /// `distinct`.
    Distinct,
    /// `count`.
    Count,
    /// `digest`.
    Digest,
    /// `path`.
    Path,
    /// `pattern`.
    Pattern,
    /// `captures`.
    Captures,
    /// `text`.
    Text,
    /// `word`.
    Word,
    /// `relation`.
    Relation,
    /// `direction`.
    Direction,
    /// `domain`.
    Domain,
    /// `edges`.
    Edges,
    /// `source`.
    Source,
    /// `destination`.
    Destination,
    /// `column`.
    Column,
    /// `sets`.
    Sets,
}

/// What the language says about one operation: the arguments that name an input, in the order
/// the engine reads its inputs, and the other arguments it cannot be written without.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OperationShape {
    /// The operation.
    pub operator: QueryOperator,
    /// The arguments that name an input.
    pub inputs: &'static [ArgumentSlot],
    /// The other required arguments.
    pub required: &'static [ArgumentSlot],
}
