//! SWC constructs the native DOM. A source wrapper keeps its subtree and the tokenizer's
//! source-ordered leaves separately: repaired DOM order never invents lexical order or ranges.

use super::{
    constants::{HTML_WHITESPACE, NOT_UTF8},
    schemas::published,
    types::{PropertyValue, Recording, SourceLength},
};
use crate::domain::{
    bases::{
        service::{based, resolved},
        types::{Targeting, Whose},
    },
    extras::types::Extra,
    nodes::types::Node,
    syntax::{
        constants::{BASE, HREF, TARGET_PROPERTY},
        service::{kind_bits, property_id, property_node, url_valued},
        types::{Kind, Property, Reference},
    },
    tokens::types::Token,
};
use crate::outbound::syntax::{interface::SyntaxPort, types::SourceText};
use ryo::languages::{self, Offset};
use swc_atoms::Atom;
use swc_common::{BytePos, Span, input::StringInput};
use swc_html_ast::{Child, Element, TokenAndSpan};
use swc_html_parser::{
    error::Error,
    lexer::{Lexer, State},
    parser::{Parser, ParserConfig, input::ParserInput},
};

/// The standards-compliant HTML tree builder.
#[derive(Debug)]
pub(crate) struct SyntaxSwcHtmlParserAdapter;

impl<I: ParserInput> Iterator for Recording<'_, I> {
    type Item = TokenAndSpan;
    fn next(&mut self) -> Option<Self::Item> {
        let token = self.input.next()?;
        self.tokens.push(token.clone());
        Some(token)
    }
}
impl<I: ParserInput> ParserInput for Recording<'_, I> {
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

/// The offset a length of the text reaches.
fn offset(value: SourceLength) -> Offset {
    Offset(u32::try_from(value.0).unwrap_or(u32::MAX))
}

/// Where `span` starts in the file: the library counts bytes from 1.
fn start(span: Span) -> Offset {
    Offset(span.lo.0.saturating_sub(1))
}

/// Where `span` ends in the file: the library counts bytes from 1.
fn end(span: Span) -> Offset {
    Offset(span.hi.0.saturating_sub(1))
}

/// Opens a node of `kind` at `span`, or one the parser implied, with no bytes of its own, when the span is the library's dummy.
fn enter(builder: &mut languages::TableBuilder<'_>, kind: Node, span: Span) {
    if span.is_dummy() {
        languages::enter_implied_node(builder, kind_bits(Kind::Node(kind)));
    } else {
        languages::enter_node(builder, kind_bits(Kind::Node(kind)), start(span));
    }
}

/// Both the typed fields used by readers and the published node value come directly from SWC. Each property is also a node of its
/// own under the node, with no bytes of its own, whose value is the property's: the structure a rule reads the name or the value
/// of an element or an attribute by. A property the DOM leaves empty has none, except a value, which an attribute, a text and a
/// comment always have.
fn properties(builder: &mut languages::TableBuilder<'_>, values: &[PropertyValue<'_>]) {
    for PropertyValue { key, value } in values {
        languages::node_property(builder, property_id(*key), *value);
    }
    let published = published(values);
    languages::node_property(
        builder,
        languages::PropertyId(0),
        languages::NameText(&published.0),
    );
    for PropertyValue { key, value } in values {
        if value.0.is_empty() && *key != Property::Value {
            continue;
        }
        languages::enter_implied_node(builder, kind_bits(Kind::Node(property_node(*key))));
        languages::node_property(builder, languages::PropertyId(0), *value);
        languages::leave_node(builder, languages::NO_SPAN.end);
    }
}

/// The `href` of the first `<base>` element of the document that has one, in tree order, template contents left out.
fn base_href(children: &[Child]) -> Option<languages::WrittenTarget<'_>> {
    children.iter().find_map(|item| {
        let Child::Element(value) = item else {
            return None;
        };
        if value.tag_name == BASE
            && let Some(href) = value
                .attributes
                .iter()
                .find(|attribute| attribute.name == HREF && attribute.namespace.is_none())
                .and_then(|attribute| attribute.value.as_deref())
        {
            return Some(languages::WrittenTarget(href.trim_matches(HTML_WHITESPACE)));
        }
        base_href(&value.children)
    })
}

/// Writes an element: its name and namespace, its attributes, the first of each expanded name, a URL-valued one with its target
/// resolved against `at`, its children and its template contents.
fn element(builder: &mut languages::TableBuilder<'_>, value: &Element, at: &Targeting<'_>) {
    enter(builder, Node::Element, value.span);
    let namespace = value.namespace.to_string();
    properties(
        builder,
        &[
            PropertyValue {
                key: Property::Name,
                value: languages::NameText(&value.tag_name),
            },
            PropertyValue {
                key: Property::Namespace,
                value: languages::NameText(&namespace),
            },
        ],
    );
    // SWC retains duplicate source attributes; the DOM keeps the first expanded name.
    let mut names = std::collections::HashSet::new();
    for attribute in &value.attributes {
        if !names.insert((attribute.namespace, attribute.name.clone())) {
            continue;
        }
        enter(builder, Node::Attribute, attribute.span);
        let attribute_namespace = attribute
            .namespace
            .map(|each| each.to_string())
            .unwrap_or_default();
        let written = attribute.value.as_deref().unwrap_or_default();
        properties(
            builder,
            &[
                PropertyValue {
                    key: Property::Name,
                    value: languages::NameText(&attribute.name),
                },
                PropertyValue {
                    key: Property::Namespace,
                    value: languages::NameText(&attribute_namespace),
                },
                PropertyValue {
                    key: Property::Prefix,
                    value: languages::NameText(attribute.prefix.as_deref().unwrap_or_default()),
                },
                PropertyValue {
                    key: Property::Value,
                    value: languages::NameText(written),
                },
            ],
        );
        // What a URL-valued attribute points at is resolved as a link is; its value as written stays its `value`.
        if url_valued(
            [
                languages::NameText(&namespace),
                languages::NameText(&value.tag_name),
            ],
            [
                languages::NameText(&attribute_namespace),
                languages::NameText(&attribute.name),
            ],
        ) == Reference::Url
        {
            let whose = if value.tag_name == BASE {
                Whose::Base
            } else {
                Whose::Other
            };
            let target = resolved(
                at,
                languages::WrittenTarget(written.trim_matches(HTML_WHITESPACE)),
                whose,
            );
            languages::node_property(builder, TARGET_PROPERTY, languages::NameText(&target.0));
        }
        languages::leave_node(builder, end(attribute.span));
    }
    for item in &value.children {
        child(builder, item, at);
    }
    if let Some(fragment) = &value.content {
        enter(builder, Node::DocumentFragment, fragment.span);
        for item in &fragment.children {
            child(builder, item, at);
        }
        languages::leave_node(builder, end(fragment.span));
    }
    languages::leave_node(builder, end(value.span));
}

/// Writes a child of an element or of the document, in the file at `at`: an element, a doctype, a text or a comment.
fn child(builder: &mut languages::TableBuilder<'_>, value: &Child, at: &Targeting<'_>) {
    let kind = super::schemas::child_kind(value);
    match value {
        Child::Element(value) => element(builder, value, at),
        Child::DocumentType(value) => {
            enter(builder, kind, value.span);
            properties(
                builder,
                &[
                    PropertyValue {
                        key: Property::Name,
                        value: languages::NameText(value.name.as_deref().unwrap_or_default()),
                    },
                    PropertyValue {
                        key: Property::PublicId,
                        value: languages::NameText(value.public_id.as_deref().unwrap_or_default()),
                    },
                    PropertyValue {
                        key: Property::SystemId,
                        value: languages::NameText(value.system_id.as_deref().unwrap_or_default()),
                    },
                ],
            );
            languages::leave_node(builder, end(value.span));
        }
        Child::Text(value) => {
            enter(builder, kind, value.span);
            properties(
                builder,
                &[PropertyValue {
                    key: Property::Value,
                    value: languages::NameText(&value.data),
                }],
            );
            languages::leave_node(builder, end(value.span));
        }
        Child::Comment(value) => {
            enter(builder, kind, value.span);
            properties(
                builder,
                &[PropertyValue {
                    key: Property::Value,
                    value: languages::NameText(&value.data),
                }],
            );
            languages::leave_node(builder, end(value.span));
        }
    }
}

/// Writes the tokens the lexer recorded, adjacent characters merged into one text token, and the bytes no token covers as text
/// the parser could not read.
fn lexical(builder: &mut languages::TableBuilder<'_>, tokens: &[TokenAndSpan], length: Offset) {
    let mut cursor = Offset(0);
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        let Some(kind) = super::schemas::token_kind(&token.token) else {
            index += 1;
            continue;
        };
        let from = start(token.span);
        let mut to = end(token.span);
        index += 1;
        if kind == Token::Characters {
            while let Some(next) = tokens.get(index).filter(|next| {
                matches!(next.token, swc_html_ast::Token::Character { .. })
                    && start(next.span) <= to
            }) {
                to = to.max(end(next.span));
                index += 1;
            }
        }
        if cursor < from {
            languages::token(builder, kind_bits(Kind::Extra(Extra::Error)), cursor, from);
        }
        if cursor <= from && from < to {
            languages::token(builder, kind_bits(Kind::Token(kind)), from, to);
        }
        cursor = cursor.max(to);
    }
    if cursor < length {
        languages::token(
            builder,
            kind_bits(Kind::Extra(Extra::Error)),
            cursor,
            length,
        );
    }
}

impl SyntaxPort for SyntaxSwcHtmlParserAdapter {
    fn read(
        &self,
        text: SourceText<'_>,
        at: languages::SourcePath<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let readable =
            std::str::from_utf8(text.0).map_or_else(|error| error.valid_up_to(), str::len);
        let source =
            std::str::from_utf8(text.0.get(..readable).unwrap_or_default()).unwrap_or_default();
        let lexer = Lexer::new(StringInput::new(
            source,
            BytePos(1),
            BytePos(offset(SourceLength(readable)).0.saturating_add(1)),
        ));
        let mut tokens = Vec::new();
        let mut parser = Parser::new(
            Recording {
                input: lexer,
                tokens: &mut tokens,
            },
            ParserConfig {
                scripting_enabled: true,
                ..ParserConfig::default()
            },
        );
        let document = parser.parse_document();
        let mut errors = parser.take_errors();
        drop(parser);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::SourceFile)), Offset(0));
        match document {
            Ok(document) => {
                enter(builder, Node::Document, document.span);
                properties(
                    builder,
                    &[PropertyValue {
                        key: Property::DocumentMode,
                        value: languages::NameText(&document.mode.to_string()),
                    }],
                );
                let targeting = based(at, base_href(&document.children));
                for item in &document.children {
                    child(builder, item, &targeting);
                }
                languages::leave_node(builder, end(document.span));
            }
            Err(error) => errors.push(error),
        }
        for error in errors {
            let message = error.message().into_owned();
            let span = error.into_inner().0;
            let location = if span.is_dummy() {
                languages::NO_SPAN
            } else {
                languages::Span {
                    start: start(span),
                    end: end(span),
                }
            };
            languages::diagnose(
                builder,
                languages::ReportText(&message),
                location.start,
                location.end,
            );
        }
        lexical(builder, &tokens, offset(SourceLength(readable)));
        if readable < text.0.len() {
            languages::token(
                builder,
                kind_bits(Kind::Extra(Extra::Error)),
                offset(SourceLength(readable)),
                offset(SourceLength(text.0.len())),
            );
            languages::diagnose(
                builder,
                languages::ReportText(NOT_UTF8),
                offset(SourceLength(readable)),
                offset(SourceLength(text.0.len())),
            );
        }
        languages::leave_node(builder, offset(SourceLength(text.0.len())));
    }
}
