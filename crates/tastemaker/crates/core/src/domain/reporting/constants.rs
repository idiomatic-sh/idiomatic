//! The names of the `reporting` concept.

/// Why a placed file is not compared: no linked language reads it or its rendered content.
pub const UNREAD: &str = "no linked language reads it, and it does not hold the template's text";

/// Why a placed file is not compared: a row of it the comparison reads came back without its node.
pub const DROPPED: &str = "a row the comparison reads came back without its node";

/// Why a placed file is not compared: its rendered content answered none of the relations the comparison reads.
pub const UNANSWERED: &str = "its rendered content answered no relation the comparison reads, and it does not hold the template's text";

/// What separates the parts of a finding written as one text.
pub const SEPARATOR: &str = "\u{1}";

/// A part of a finding with nothing in it.
pub const EMPTY: &str = "";
