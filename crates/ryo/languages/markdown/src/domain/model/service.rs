//! The `model` service: free functions named by verb that work out the sections of a file from its headings, and the section that
//! holds an element, with no IO and no ports.

use crate::domain::nodes::types::Node;
use ryo::languages::{self, Tree};

use super::types::{Holding, Scoped, Section, SectionPlace, Sections, Up};
use crate::domain::syntax::service::{kind, level_of};
use crate::domain::syntax::types::Kind;

/// Refills `sections` with those of the tree: one for each heading, held by the nearest heading above it that is shallower, ended by the
/// next heading as shallow or shallower.
pub(crate) fn settle(tree: &Tree<'_>, sections: &mut Sections) {
    sections.0.clear();
    let document_end = languages::end_at(tree.elements, languages::Position(0));
    for at in languages::positions(tree.elements) {
        let Some(level) = level_of(tree, at) else {
            continue;
        };
        let start = languages::start_at(tree.elements, at);
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
            .map_or(languages::Position(0), |section| section.heading);
        sections.0.push(Section {
            heading: at,
            parent,
            end: document_end,
            level,
            up: open.map_or(Up::Document, Up::Section),
        });
    }
}

/// What holds the element at `at`: the section of the deepest heading above it that has not ended by the time it starts, or the document.
/// A heading is held by the section around it, not by its own.
#[must_use]
pub(crate) fn holding(tree: &Tree<'_>, sections: &Sections, at: languages::Position) -> Holding {
    let start = languages::start_at(tree.elements, at);
    let before = sections.0.partition_point(|section| section.heading < at);
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

/// The node the element at `at` is held by, as an id: its section's heading, or the document.
#[must_use]
pub(crate) fn owner(
    tree: &Tree<'_>,
    sections: &Sections,
    at: languages::Position,
) -> languages::NodeId {
    match holding(tree, sections, at) {
        Holding::Section(section) => languages::node_id(tree.file, section.heading),
        Holding::Document => languages::node_id(tree.file, languages::Position(0)),
    }
}

/// The section a heading opens.
#[must_use]
pub(crate) fn opened_by(sections: &Sections, heading: languages::Position) -> Option<Section> {
    match sections
        .0
        .binary_search_by_key(&heading, |section| section.heading)
    {
        Ok(place) => sections.0.get(place).copied(),
        // The search answers where the heading would go: no section is opened by it.
        Err(_) => None,
    }
}

/// What the element at `at` opens a scope as: the document, a section when it is a heading, nothing otherwise.
#[must_use]
pub(crate) fn scoped(tree: &Tree<'_>, sections: &Sections, at: languages::Position) -> Scoped {
    if kind(tree, at) == Kind::Node(Node::Root) {
        Scoped::Document
    } else {
        opened_by(sections, at).map_or(Scoped::Nothing, Scoped::Section)
    }
}
