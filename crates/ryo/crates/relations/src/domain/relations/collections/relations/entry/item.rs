//! The `entry` relation: one row for each key of a data document.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer, NodeId, Span};
use crate::domain::terms::service::term_symbol;
use crate::domain::terms::types::{Symbol, ValueKind};

/// `entry`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Entry;

impl super::super::interface::sealed::Sealed for Entry {}

/// The declaration of `entry`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! entry {
    ($callback:path) => {
        $callback! {
            item Entry,
            row {
                /// One row of `entry`.
                EntryRow
            },
            column {
                /// The columns of `entry`.
                EntryColumn
            },
            expected {
                /// One row of `entry` as a fixture states it.
                EntryExpectedRow
            },
            class Keyed,
            columns {
                /// The value decoded when it is text, otherwise empty. Free text: a decoded string is whatever the file says.
                decoded Decoded: Symbol => Symbol;
                /// The key, decoded.
                key Key: Symbol => Symbol;
                /// What the value is.
                kind Kind: ValueKind => Symbol(term_symbol);
                /// The entry: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// Its place within its owner.
                ordinal Ordinal: Integer => Number;
                /// The document or mapping that holds it.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the key is.
                span Span: Span => Span;
                /// The value as written; empty when the key opens a mapping. Source text: the value exactly as written, which a rule compares with a literal.
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

crate::entry!(crate::relation_row);
