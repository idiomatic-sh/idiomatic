//! One cell of a row as a case states it in a constant, and the rows a read gave kept as such cases hold them.

use super::expected::{Cell, Point, Stretch};

/// One cell of a row as a case states it in a constant: the cells of `Cell`, with the text borrowed so that a constant can hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    /// No source location.
    Unlocated,
    /// A node, by its place in the file's element table.
    Node(u32),
    /// A name, by its text.
    Name(&'static str),
    /// A value the language cannot resolve statically, by the name of its reason.
    Unknown(&'static str),
    /// A file, by its path.
    File(&'static str),
    /// A stretch of a file, from one place to another.
    Span(Point, Point),
    /// A whole number.
    Number(i64),
    /// A yes or no.
    Flag(bool),
}

/// The rows a case wants, each in full: the cells of every column.
pub type Wanted = &'static [&'static [Want]];

/// A node, by its place in the file's element table.
#[must_use]
pub const fn want_node(place: u32) -> Want {
    Want::Node(place)
}

/// A value the language cannot resolve statically, by the name of its reason.
#[must_use]
pub const fn want_unknown(reason: &'static str) -> Want {
    Want::Unknown(reason)
}

/// A name, by its text.
#[must_use]
pub const fn want_name(text: &'static str) -> Want {
    Want::Name(text)
}

/// A file, by its path.
#[must_use]
pub const fn want_file(path: &'static str) -> Want {
    Want::File(path)
}

/// A stretch of a file, from `(line, column)` to `(line, column)`.
#[must_use]
pub const fn want_span(from: (u32, u32), to: (u32, u32)) -> Want {
    Want::Span(
        Point {
            line: from.0,
            column: from.1,
        },
        Point {
            line: to.0,
            column: to.1,
        },
    )
}

/// A whole number.
#[must_use]
pub const fn want_number(number: i64) -> Want {
    Want::Number(number)
}

/// A yes or no.
#[must_use]
pub const fn want_flag(flag: bool) -> Want {
    Want::Flag(flag)
}

/// The rows a read gave, as a case holds them: kept for the rest of the run, which is a test's.
#[must_use]
pub fn kept(rows: Vec<Vec<Cell>>) -> Wanted {
    let rows: Vec<&'static [Want]> = rows
        .into_iter()
        .map(|row| {
            let cells: Vec<Want> = row
                .into_iter()
                .map(|cell| match cell {
                    Cell::Unlocated => Want::Unlocated,
                    Cell::Node(place) => Want::Node(place),
                    Cell::Name(text) => Want::Name(Box::leak(text.into_boxed_str())),
                    Cell::Unknown(reason) => Want::Unknown(Box::leak(reason.into_boxed_str())),
                    Cell::File(path) => Want::File(Box::leak(path.into_boxed_str())),
                    Cell::Span(from, to) => Want::Span(from, to),
                    Cell::Number(number) => Want::Number(number.0),
                    Cell::Flag(flag) => Want::Flag(flag),
                })
                .collect();
            &*Box::leak(cells.into_boxed_slice())
        })
        .collect();
    Box::leak(rows.into_boxed_slice())
}

/// An explicitly absent source location.
#[must_use]
pub const fn unlocated() -> Stretch {
    Stretch {
        located: false,
        from: Point { line: 0, column: 0 },
        to: Point { line: 0, column: 0 },
    }
}
