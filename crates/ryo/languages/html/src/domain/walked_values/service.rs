//! The `walked_values` service: free functions over a walked file that say what an element is as a row's cell: whether it holds a
//! section, a label, a URL or a run of prose, which section holds it, and the words of its title or its run.

use crate::domain::file_view::service::{children, kind, parent, property};
use crate::domain::file_view::types::HtmlFile;
use crate::domain::nodes::types::Node;
use crate::domain::runs::types::Gap;
use crate::domain::syntax::constants::{
    ANCHOR, ARTICLE, ASIDE, BREAK, CODE, HEADINGS, HTML_NAMESPACE, ID, INLINE, NAME, NAV, SCRIPT, SECTION, STYLE, TEMPLATE,
    WORD_SPACE,
};
use crate::domain::syntax::service::url_valued;
use crate::domain::syntax::types::{
    Anchor, Defined, Flow, Kind, Property, Readability, Reference, Rootness, Section, Wording,
};
use ryo::languages::{self, Position};

/// Whether `at` is the root of the tree.
#[must_use]
pub(crate) fn is_root(file: &HtmlFile<'_>, at: Position) -> Rootness {
    if parent(file, at) == at { Rootness::Root } else { Rootness::Inner }
}

/// Whether an element starts a document section.
#[must_use]
pub(crate) fn section(file: &HtmlFile<'_>, at: Position) -> Section {
    if kind(file, at) == Kind::Node(Node::Element)
        && matches!(property(file, at, Property::Name), ARTICLE | ASIDE | NAV | SECTION)
    {
        Section::Section
    } else {
        Section::Other
    }
}

/// The closest section containing a node, or the document.
#[must_use]
pub(crate) fn owner(file: &HtmlFile<'_>, at: Position) -> Position {
    let mut cursor = parent(file, at);
    while cursor != parent(file, cursor) && section(file, cursor) != Section::Section {
        cursor = parent(file, cursor);
    }
    cursor
}

/// An HTML identifier attribute defines an anchor.
#[must_use]
fn anchor(file: &HtmlFile<'_>, at: Position) -> Anchor {
    if !property(file, at, Property::Namespace).is_empty() || property(file, at, Property::Value).is_empty() {
        return Anchor::Other;
    }
    let name = property(file, at, Property::Name);
    let element = parent(file, at);
    if name == ID
        || (name == NAME
            && property(file, element, Property::Name) == ANCHOR
            && property(file, element, Property::Namespace) == HTML_NAMESPACE)
    {
        Anchor::Definition
    } else {
        Anchor::Other
    }
}

/// What the element or attribute at `at` defines.
#[must_use]
pub(crate) fn defined(file: &HtmlFile<'_>, at: Position) -> Defined {
    if anchor(file, at) == Anchor::Definition {
        return Defined::Label;
    }
    if section(file, at) == Section::Section
        && let Some(heading) = title(file, at)
    {
        return Defined::Section(heading);
    }
    Defined::Nothing
}

/// The heading that titles the section at `at`: the first `h1` to `h6` it holds, outside the sections it holds.
fn title(file: &HtmlFile<'_>, at: Position) -> Option<Position> {
    children(file, at).find_map(|child| {
        if kind(file, child) != Kind::Node(Node::Element) || section(file, child) == Section::Section {
            return None;
        }
        if HEADINGS.contains(&property(file, child, Property::Name)) {
            return Some(child);
        }
        title(file, child)
    })
}

/// The title of a section: the words of the text the heading holds, one space apart.
#[must_use]
pub(crate) fn section_name(file: &HtmlFile<'_>, heading: Position) -> String {
    let mut name = String::new();
    let mut any = Wording::Silent;
    title_words(file, heading, &mut name, &mut any);
    name
}

/// Writes the words of the text the element at `at` holds, one space apart.
fn title_words(file: &HtmlFile<'_>, at: Position, name: &mut String, any: &mut Wording) {
    for child in children(file, at) {
        match kind(file, child) {
            Kind::Node(Node::Text) => {
                for word in property(file, child, Property::Value).split_whitespace() {
                    if *any == Wording::Words {
                        name.push_str(WORD_SPACE.0);
                    }
                    name.push_str(word);
                    *any = Wording::Words;
                }
            }
            Kind::Node(Node::Element) => title_words(file, child, name, any),
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}

/// URL-valued attributes are references; event handlers are not HTML code relations.
#[must_use]
pub(crate) fn reference(file: &HtmlFile<'_>, at: Position) -> Reference {
    let element = parent(file, at);
    url_valued(
        [
            languages::NameText(property(file, element, Property::Namespace)),
            languages::NameText(property(file, element, Property::Name)),
        ],
        [
            languages::NameText(property(file, at, Property::Namespace)),
            languages::NameText(property(file, at, Property::Name)),
        ],
    )
}

/// The section that holds the run of the element at `at`: the element itself when it is one.
#[must_use]
pub(crate) fn run_owner(file: &HtmlFile<'_>, at: Position) -> Position {
    if section(file, at) == Section::Section { at } else { owner(file, at) }
}

/// Human-readable text excludes script and style data and template contents: the element at `at` and what it sits in.
fn readable(file: &HtmlFile<'_>, at: Position) -> Readability {
    let mut cursor = at;
    loop {
        if matches!(property(file, cursor, Property::Name), SCRIPT | STYLE | TEMPLATE) {
            return Readability::Excluded;
        }
        if cursor == parent(file, cursor) {
            return Readability::Prose;
        }
        cursor = parent(file, cursor);
    }
}

/// Whether the element at `at` is part of the run around it or holds a run of its own.
fn flow(file: &HtmlFile<'_>, at: Position) -> Flow {
    if INLINE.contains(&property(file, at, Property::Name)) { Flow::Inline } else { Flow::Block }
}

/// Whether the element at `at` holds a run of its own with a word a reader reads.
#[must_use]
pub(crate) fn run_words(file: &HtmlFile<'_>, at: Position) -> Wording {
    if kind(file, at) != Kind::Node(Node::Element)
        || flow(file, at) == Flow::Inline
        || readable(file, at) == Readability::Excluded
    {
        return Wording::Silent;
    }
    let mut any = Wording::Silent;
    gather(file, None, at, &mut any, &mut Gap::Closed);
    any
}

/// The text of the run the element at `at` holds: its inline markup joined, its code, its breaks and the blocks it holds each a
/// space, and the white space between words one space.
#[must_use]
pub(crate) fn run_name(file: &HtmlFile<'_>, at: Position) -> String {
    let mut name = String::new();
    let mut any = Wording::Silent;
    gather(file, Some(&mut name), at, &mut any, &mut Gap::Closed);
    name
}

/// Walks the text of the run at `at`, writing each word into `name` when it is given, and noting in `any` that a word was read.
fn gather(file: &HtmlFile<'_>, mut name: Option<&mut String>, at: Position, any: &mut Wording, gap: &mut Gap) {
    for child in children(file, at) {
        match kind(file, child) {
            Kind::Node(Node::Text) => {
                let text = property(file, child, Property::Value);
                if text.starts_with(char::is_whitespace) {
                    *gap = Gap::Open;
                }
                let mut words = text.split_whitespace().peekable();
                while let Some(word) = words.next() {
                    if let Some(name) = name.as_deref_mut() {
                        if *gap == Gap::Open && *any == Wording::Words {
                            name.push_str(WORD_SPACE.0);
                        }
                        name.push_str(word);
                    }
                    *any = Wording::Words;
                    *gap = if words.peek().is_some() || text.ends_with(char::is_whitespace) { Gap::Open } else { Gap::Closed };
                }
            }
            Kind::Node(Node::Element) => {
                let tag = property(file, child, Property::Name);
                if tag == CODE || tag == BREAK || flow(file, child) == Flow::Block {
                    *gap = Gap::Open;
                } else if readable(file, child) == Readability::Prose {
                    gather(file, name.as_deref_mut(), child, any, gap);
                }
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
}
