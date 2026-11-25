//! The `walked_model` service: free functions that work out the sections of a walked file from its headings, the section that holds
//! an element, and what a construct of the file defines, with no IO and no ports.

use crate::domain::file_view::service::{kind, level_of, named_by, span_at, written};
use crate::domain::file_view::types::Syntax;
use crate::domain::labels::types::{Defined, HtmlText, LabelText};
use crate::domain::model::types::{Holding, Scoped, Section, SectionPlace, Sections, Up};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, NodeId, Position, node_table};

/// Whether `at` is a token of the table: tokens are numbered after every node.
fn is_token(syntax: &Syntax<'_>, at: Position) -> bool {
    u64::from(at.0) >= u64::try_from(node_table::node_count(syntax.table).0).unwrap_or(u64::MAX)
}

/// Works out the sections of the file into `sections`: one for each heading that has a depth, in the order of the headings, each ending
/// where the next heading as shallow or shallower starts, or where the document ends.
pub(crate) fn settle(syntax: &Syntax<'_>, sections: &mut Sections) {
    sections.0.clear();
    let document_end = span_at(syntax, Position(0)).end;
    let nodes = u32::try_from(node_table::node_count(syntax.table).0).unwrap_or(u32::MAX);
    for id in 0..nodes {
        let at = Position(id);
        let Some(level) = level_of(syntax, at) else {
            continue;
        };
        let start = span_at(syntax, at).start;
        // The sections still open are the last one and those that hold it; those as deep as this heading or deeper end where it starts.
        let mut open = sections.0.len().checked_sub(1).map(SectionPlace);
        while let Some(place) = open {
            let Some(section) = sections.0.get_mut(place.0) else {
                break;
            };
            if section.level < level {
                break;
            }
            section.end = start;
            open = match section.up {
                Up::Section(above) => Some(above),
                Up::Document => None,
            };
        }
        let parent = open
            .and_then(|place| sections.0.get(place.0))
            .map_or(Position(0), |section| section.heading);
        sections.0.push(Section {
            heading: at,
            parent,
            end: document_end,
            level,
            up: open.map_or(Up::Document, Up::Section),
        });
    }
}

/// What holds the element at `at`: the innermost section that starts before it and ends after it start, or the document.
#[must_use]
pub(crate) fn holding(syntax: &Syntax<'_>, sections: &Sections, at: Position) -> Holding {
    let start = span_at(syntax, at).start;
    let before = if is_token(syntax, at) {
        // a token is numbered after every node: it is after a heading that starts before it
        sections
            .0
            .partition_point(|section| span_at(syntax, section.heading).start < start)
    } else {
        sections.0.partition_point(|section| section.heading < at)
    };
    let mut candidate = before.checked_sub(1).map(SectionPlace);
    while let Some(place) = candidate {
        let Some(&section) = sections.0.get(place.0) else {
            break;
        };
        if start < section.end {
            return Holding::Section(section);
        }
        candidate = match section.up {
            Up::Section(above) => Some(above),
            Up::Document => None,
        };
    }
    Holding::Document
}

/// The element that owns the element at `at`: the heading of the section that holds it, or the document.
#[must_use]
pub(crate) fn owner(syntax: &Syntax<'_>, sections: &Sections, at: Position) -> NodeId {
    match holding(syntax, sections, at) {
        Holding::Section(section) => languages::node_id(syntax.file, section.heading),
        Holding::Document => languages::node_id(syntax.file, Position(0)),
    }
}

/// The section the heading at `heading` opens, when it opens one.
#[must_use]
pub(crate) fn opened_by(sections: &Sections, heading: Position) -> Option<Section> {
    match sections
        .0
        .binary_search_by_key(&heading, |section| section.heading)
    {
        Ok(place) => sections.0.get(place).copied(),
        // The search answers where the heading would go: no section is opened by it.
        Err(_) => None,
    }
}

/// What the element at `at` is the scope of: the document, the section its heading opens, or nothing.
#[must_use]
pub(crate) fn scoped(syntax: &Syntax<'_>, sections: &Sections, at: Position) -> Scoped {
    if kind(syntax, at) == Kind::Node(Node::Root) {
        Scoped::Document
    } else {
        opened_by(sections, at).map_or(Scoped::Nothing, Scoped::Section)
    }
}

/// What the construct at `at` defines: a section, a label, the anchors of raw HTML, or nothing.
#[must_use]
pub(crate) fn defined<'a>(syntax: &Syntax<'a>, at: Position) -> Defined<'a> {
    let of = kind(syntax, at);
    if of == Kind::Node(Node::Definition) {
        crate::domain::file_view::service::decoded(syntax, at)
            .map_or(Defined::Nothing, |label| Defined::Label(LabelText(label)))
    } else if of == Kind::Node(Node::FootnoteDefinition) {
        named_by(syntax, at).map_or(Defined::Nothing, |label| Defined::Label(LabelText(label)))
    } else if of == Kind::Node(Node::Html) {
        Defined::Html(HtmlText(written(syntax, at)))
    } else if level_of(syntax, at).is_some() {
        Defined::Section
    } else {
        Defined::Nothing
    }
}
