//! Independent traversal of native SWC nodes and tokenizer deliveries, including their reported ranges.
//! This does not call the production writer or its schemas.
use crate::domain::{
    extras::types::Extra,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
    tokens::types::Token,
};
use ryo::languages::{
    NO_SPAN, Offset,
    contract::{Leafness, Walked},
};
use swc_atoms::Atom;
use swc_common::{BytePos, Span, input::StringInput};
use swc_html_ast::{Child, Element, TokenAndSpan};
use swc_html_parser::{
    error::Error,
    lexer::{Lexer, State},
    parser::{Parser, ParserConfig, input::ParserInput},
};

/// The parser's input, watched: every token the tokenizer delivers becomes a leaf, adjacent characters merged into one.
struct Observed<'a, I> {
    /// The tokenizer the parser reads through it.
    input: I,
    /// The leaves the deliveries made so far, in the order they came.
    leaves: &'a mut Vec<Walked>,
}

impl<I: ParserInput> Iterator for Observed<'_, I> {
    type Item = TokenAndSpan;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.input.next()?;
        let kind = match item.token {
            swc_html_ast::Token::Doctype { .. } => Some(Token::Doctype),
            swc_html_ast::Token::StartTag { .. } => Some(Token::StartTag),
            swc_html_ast::Token::EndTag { .. } => Some(Token::EndTag),
            swc_html_ast::Token::Comment { .. } => Some(Token::Comment),
            swc_html_ast::Token::Character { .. } => Some(Token::Characters),
            swc_html_ast::Token::Eof => None,
        };
        if let Some(kind) = kind {
            let start = Offset(item.span.lo.0.saturating_sub(1));
            let end = Offset(item.span.hi.0.saturating_sub(1));
            let bits = kind_bits(Kind::Token(kind));
            let merged = self.leaves.last_mut().filter(|previous| {
                kind == Token::Characters && previous.kind == bits && start <= previous.end
            });
            match merged {
                Some(previous) => previous.end = previous.end.max(end),
                None if start < end => self.leaves.push(Walked {
                    kind: bits,
                    leaf: Leafness::Token,
                    start,
                    end,
                    children: Vec::new(),
                }),
                // A delivery that spans no bytes is no leaf.
                None => {}
            }
        }
        Some(item)
    }
}

impl<I: ParserInput> ParserInput for Observed<'_, I> {
    fn start_pos(&mut self) -> BytePos {
        self.input.start_pos()
    }
    fn last_pos(&mut self) -> BytePos {
        self.input.last_pos()
    }
    fn take_errors(&mut self) -> Vec<Error> {
        self.input.take_errors()
    }
    fn set_last_start_tag_name(&mut self, tag_name: &Atom) {
        self.input.set_last_start_tag_name(tag_name);
    }
    fn set_input_state(&mut self, state: State) {
        self.input.set_input_state(state);
    }
    fn set_adjusted_current_node_to_html_namespace(&mut self, value: bool) {
        self.input
            .set_adjusted_current_node_to_html_namespace(value);
    }
}

/// A node of `kind` over `span`, holding `children`; one the parser implied has no span.
fn node(kind: Node, span: Span, children: Vec<Walked>) -> Walked {
    let (start, end) = if span.is_dummy() {
        (NO_SPAN.start, NO_SPAN.end)
    } else {
        (
            Offset(span.lo.0.saturating_sub(1)),
            Offset(span.hi.0.saturating_sub(1)),
        )
    };
    Walked {
        kind: kind_bits(Kind::Node(kind)),
        leaf: Leafness::Node,
        start,
        end,
        children,
    }
}

/// The nodes of a node's DOM scalar properties, in the order given: each a node with no bytes of its own, one the DOM leaves empty
/// left out unless it is a value.
fn properties(values: &[(Node, &str)]) -> Vec<Walked> {
    values
        .iter()
        .filter(|(kind, value)| !value.is_empty() || *kind == Node::Value)
        .map(|(kind, _)| Walked {
            kind: kind_bits(Kind::Node(*kind)),
            leaf: Leafness::Node,
            start: NO_SPAN.start,
            end: NO_SPAN.end,
            children: Vec::new(),
        })
        .collect()
}

/// An element: its name and namespace, its attributes, the first of each expanded name, its children and its template contents.
fn element(value: &Element) -> Walked {
    let mut names = std::collections::HashSet::new();
    let element_namespace = value.namespace.to_string();
    let mut children = properties(&[
        (Node::Name, &value.tag_name),
        (Node::Namespace, &element_namespace),
    ]);
    children.extend(
        value
            .attributes
            .iter()
            .filter(|attribute| names.insert((attribute.namespace, attribute.name.clone())))
            .map(|attribute| {
                let attribute_namespace = attribute
                    .namespace
                    .map(|each| each.to_string())
                    .unwrap_or_default();
                node(
                    Node::Attribute,
                    attribute.span,
                    properties(&[
                        (Node::Name, &attribute.name),
                        (Node::Namespace, &attribute_namespace),
                        (
                            Node::Prefix,
                            attribute.prefix.as_deref().unwrap_or_default(),
                        ),
                        (Node::Value, attribute.value.as_deref().unwrap_or_default()),
                    ]),
                )
            }),
    );
    children.extend(value.children.iter().map(visit));
    if let Some(content) = &value.content {
        children.push(node(
            Node::DocumentFragment,
            content.span,
            content.children.iter().map(visit).collect(),
        ));
    }
    node(Node::Element, value.span, children)
}

/// The node of a child of an element or of the document.
fn visit(value: &Child) -> Walked {
    match value {
        Child::Element(value) => element(value),
        Child::Text(value) => node(
            Node::Text,
            value.span,
            properties(&[(Node::Value, &value.data)]),
        ),
        Child::Comment(value) => node(
            Node::Comment,
            value.span,
            properties(&[(Node::Value, &value.data)]),
        ),
        Child::DocumentType(value) => node(
            Node::DocumentType,
            value.span,
            properties(&[
                (Node::Name, value.name.as_deref().unwrap_or_default()),
                (
                    Node::PublicId,
                    value.public_id.as_deref().unwrap_or_default(),
                ),
                (
                    Node::SystemId,
                    value.system_id.as_deref().unwrap_or_default(),
                ),
            ]),
        ),
    }
}

/// Compare every native DOM and tokenizer range, including absent locations and reordered nodes.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let end = Offset(u32::try_from(text.len()).unwrap_or(u32::MAX));
    let lexer = Lexer::new(StringInput::new(
        text,
        BytePos(1),
        BytePos(end.0.saturating_add(1)),
    ));
    let mut leaves = Vec::new();
    let mut parser = Parser::new(
        Observed {
            input: lexer,
            leaves: &mut leaves,
        },
        ParserConfig {
            scripting_enabled: true,
            ..ParserConfig::default()
        },
    );
    let document = parser.parse_document();
    drop(parser);
    let mut children = document.map_or_else(
        |_| Vec::new(),
        |document| {
            let mode = document.mode.to_string();
            let mut children = properties(&[(Node::DocumentMode, &mode)]);
            children.extend(document.children.iter().map(visit));
            vec![node(Node::Document, document.span, children)]
        },
    );
    let mut cursor = Offset(0);
    for leaf in leaves {
        if cursor < leaf.start {
            children.push(gap(cursor, leaf.start));
        }
        cursor = leaf.end;
        children.push(leaf);
    }
    if cursor < end {
        children.push(gap(cursor, end));
    }
    Walked {
        kind: kind_bits(Kind::Node(Node::SourceFile)),
        leaf: Leafness::Node,
        start: Offset(0),
        end,
        children,
    }
}

/// A token of text the tokenizer delivered nothing for, over `start..end`.
fn gap(start: Offset, end: Offset) -> Walked {
    Walked {
        kind: kind_bits(Kind::Extra(Extra::Error)),
        leaf: Leafness::Token,
        start,
        end,
        children: Vec::new(),
    }
}
