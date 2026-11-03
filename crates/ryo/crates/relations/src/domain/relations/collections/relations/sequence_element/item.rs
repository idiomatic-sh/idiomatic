//! The `sequence_element` relation: one row for each position of a sequence of a data document.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer, NodeId, Span};
use crate::domain::terms::service::term_symbol;
use crate::domain::terms::types::{Symbol, ValueKind};

/// `sequence_element`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SequenceElement;

impl super::super::interface::sealed::Sealed for SequenceElement {}

/// The declaration of `sequence_element`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! sequence_element {
    ($callback:path) => {
        $callback! {
            item SequenceElement,
            row {
                /// One row of `sequence_element`.
                SequenceElementRow
            },
            column {
                /// The columns of `sequence_element`.
                SequenceElementColumn
            },
            expected {
                /// One row of `sequence_element` as a fixture states it.
                SequenceElementExpectedRow
            },
            class Keyed,
            columns {
                /// The value decoded when it is text, otherwise empty. Free text: a decoded string is whatever the file says.
                decoded Decoded: Symbol => Symbol;
                /// What the value is.
                kind Kind: ValueKind => Symbol(term_symbol);
                /// The element: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// Its place in the sequence.
                ordinal Ordinal: Integer => Number;
                /// The entry or element that holds the sequence.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the element is.
                span Span: Span => Span;
                /// The element as written. Source text: the element exactly as written, which a rule compares with a literal.
                text Text: Symbol => Symbol;
                /// The value as a yes or no when it is a boolean; lowered for every other kind.
                boolean Boolean: Flag => Flag;
                /// The value as a whole number when it is an integer; zero for every other kind, and for an integer too large to hold.
                integer Integer: Integer => Number;
                /// The value as a number when it is a float, in the shortest decimal form that reads back as it; empty for every other kind.
                float Float: Symbol => Symbol;
                /// The value as RFC 3339 writes it when it is a date-time; empty for every other kind.
                datetime Datetime: Symbol => Symbol;
                /// The base the value is written in when it is an integer; zero for every other kind.
                radix Radix: Integer => Number;
            }
        }
    };
}

crate::sequence_element!(crate::relation_row);
