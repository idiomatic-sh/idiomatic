//! Test support split from adapter_test: nesting, mdast_name, is_phrasing and more.

use super::super::adapter::SyntaxPulldownCmarkAdapter;
use super::walk::{Shape, shape};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::SourcePath;
use ryo::languages::{NodeTable, Position, node_table};

/// The nodes under `at`, each written `kind(held held)`, its tokens left out.
pub(in super::super) fn nesting(elements: &NodeTable, at: Position) -> String {
    let held: Vec<String> = node_table::children_of(elements, at).iter().copied()
        .filter(|each| !node_table::is_token(elements, *each))
        .map(|each| nesting(elements, each))
        .collect();
    let name = kind_name(kind_of(node_table::kind_of(elements, at))).0;
    if held.is_empty() {
        name.to_owned()
    } else {
        format!("{name}({})", held.join(" "))
    }
}

/// The type mdast gives a kind of our trees; nothing for the kinds mdast has no node for (tokens, markup, the task marks).
pub(in super::super) fn mdast_name(kind: Kind) -> Option<String> {
    match kind {
        Kind::Node(_) => {
            let mut camel = String::new();
            let mut upper = false;
            for each in kind_name(kind).0.chars() {
                if each == '-' {
                    upper = true;
                } else if upper {
                    camel.push(each.to_ascii_uppercase());
                    upper = false;
                } else {
                    camel.push(each);
                }
            }
            Some(camel)
        }
        Kind::Token(Token::LineEnding) => Some("text".to_owned()),
        Kind::Extra(Extra::Yaml) => Some("yaml".to_owned()),
        Kind::Extra(Extra::Toml) => Some("toml".to_owned()),
        Kind::Extra(Extra::Math) => Some("math".to_owned()),
        Kind::Extra(Extra::InlineMath) => Some("inlineMath".to_owned()),
        Kind::Extra(_) | Kind::Token(_) => None,
    }
}

/// Whether the element at `at` is part of a line of text: what mdast puts in a paragraph.
pub(in super::super) fn is_phrasing(elements: &NodeTable, at: Position) -> bool {
    match kind_of(node_table::kind_of(elements, at)) {
        Kind::Node(
            Node::Text
            | Node::Emphasis
            | Node::Strong
            | Node::Delete
            | Node::Link
            | Node::LinkReference
            | Node::Image
            | Node::ImageReference
            | Node::InlineCode
            | Node::Break
            | Node::FootnoteReference,
        )
        | Kind::Extra(Extra::InlineMath)
        | Kind::Token(Token::LineEnding) => true,
        Kind::Node(Node::Html) => node_table::is_token(elements, at),
        Kind::Node(
            Node::Root
            | Node::Paragraph
            | Node::Heading
            | Node::Blockquote
            | Node::Code
            | Node::List
            | Node::ListItem
            | Node::FootnoteDefinition
            | Node::Table
            | Node::TableRow
            | Node::TableCell
            | Node::Definition
            | Node::ThematicBreak,
        )
        | Kind::Token(_)
        | Kind::Extra(_) => false,
    }
}

/// Our tree at `at` projected onto mdast: the nodes it is, the tokens and the markup dropped, the text of a line and the lines of a
/// paragraph one `text` as mdast joins them, and the lines of a tight list item or a footnote in the `paragraph` mdast puts them in.
pub(in super::super) fn project(elements: &NodeTable, at: Position, out: &mut Vec<Shape>) {
    let kind = kind_of(node_table::kind_of(elements, at));
    let Some(name) = mdast_name(kind) else {
        return;
    };
    let mut children: Vec<Shape> = Vec::new();
    // mdast keeps the alternative text of an image as a field, `alt`, and the text of front matter as `value`, not as nodes under them.
    if matches!(
        kind,
        Kind::Node(Node::Image | Node::ImageReference) | Kind::Extra(Extra::Yaml | Extra::Toml)
    ) {
        out.push(shape(&name, children));
        return;
    }
    let wraps = matches!(kind, Kind::Node(Node::ListItem | Node::FootnoteDefinition));
    let mut lines: Vec<Shape> = Vec::new();
    for each in held(elements, at) {
        let mut one = Vec::new();
        project(elements, each, &mut one);
        if wraps && is_phrasing(elements, each) {
            lines.append(&mut one);
            continue;
        }
        if !one.is_empty() {
            if !lines.is_empty() {
                children.push(shape("paragraph", std::mem::take(&mut lines)));
            }
            children.append(&mut one);
        }
    }
    if !lines.is_empty() {
        children.push(shape("paragraph", lines));
    }
    out.push(Shape { name, children });
}

/// The elements under `at` as mdast holds them: mdast has no section, so the elements a section holds are held by what holds it.
pub(in super::super) fn held(elements: &NodeTable, at: Position) -> Vec<Position> {
    let mut found = Vec::new();
    for each in node_table::children_of(elements, at).iter().copied() {
        if kind_of(node_table::kind_of(elements, each)) == Kind::Extra(Extra::Section) {
            found.extend(held(elements, each));
        } else {
            found.push(each);
        }
    }
    found
}

/// Our tree of `text`, projected onto mdast.
pub(in super::super) fn projected(text: &str) -> Option<Shape> {
    let elements = crate::outbound::syntax::adapters::pulldown_cmark::fixtures::projection::walked_table(text);
    let mut out = Vec::new();
    project(&elements, Position(0), &mut out);
    out.pop().map(joined)
}

/// The shape with the `text` nodes that follow one another joined into one, at every depth: mdast has one `text` for a run of text,
/// where the library has one for each line, entity and escape.
pub(in super::super) fn joined(node: Shape) -> Shape {
    let mut children: Vec<Shape> = Vec::new();
    for child in node.children.into_iter().map(joined) {
        let after_text = children.last().is_some_and(|last| last.name == "text");
        if !(after_text && child.name == "text") {
            children.push(child);
        }
    }
    Shape {
        name: node.name,
        children,
    }
}

/// The node table the adapter walks `text` into.
pub(in super::super) fn walked_table(text: &str) -> NodeTable {
    let mut table = NodeTable::default();
    let mut builder = node_table::builder(&mut table, ryo::languages::Count(64));
    SyntaxPulldownCmarkAdapter.walk(
        SourceText(text.as_bytes()),
        SourcePath("a.md"),
        &mut crate::domain::file_view::types::Decodings::default(),
        &mut builder,
    );
    let _ = node_table::finished(&mut builder);
    table
}
