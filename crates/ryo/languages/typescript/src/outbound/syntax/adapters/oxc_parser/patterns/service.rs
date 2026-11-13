//! The patterns of the adapter: which elements carry a regular-expression pattern, and the trees `oxc_regular_expression` parses of
//! them, written under each carrier with the properties the domain reads.

use super::super::constants::{NO_FLAGS, PATTERN_CONSTRUCTOR, UNKNOWN_FLAGS};
use super::super::projection::service::declaration_start;
use super::super::types::{
    Carrier, CarrierIndex, Carries, DecodedText, Form, Hosting, PatternFlags, PatternWriter,
    VisitorStack,
};
use crate::domain::syntax::constants::{PATTERN_ERROR, PATTERN_FLAGS, PATTERN_VALUE};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::PatternNode;
use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Argument, Expression},
};
use oxc_regular_expression::{
    ConstructorParser, LiteralParser, Options,
    visit::{RegExpAstKind, Visit},
};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan as _, Span};
use ryo::languages::{self, ByteSlice, EmbeddedMapping, NameText, Offset};

impl<'a> Visit<'a> for PatternWriter<'_, '_> {
    fn enter_node(&mut self, kind: RegExpAstKind<'a>) {
        let Some(node) = pattern_node(kind) else {
            return;
        };
        languages::enter_node(
            self.builder,
            kind_bits(Kind::Pattern(node)),
            Offset(kind.span().start),
        );
        let number = match kind {
            RegExpAstKind::Character(character) => Some(character.value),
            RegExpAstKind::IndexedReference(reference) => Some(reference.index),
            _ => None,
        };
        if let Some(number) = number {
            // A number formats into retained storage without failure.
            let _written =
                languages::node_property_fmt(self.builder, PATTERN_VALUE, format_args!("{number}"));
        }
    }

    fn leave_node(&mut self, kind: RegExpAstKind<'a>) {
        if pattern_node(kind).is_some() {
            languages::leave_node(self.builder, Offset(kind.span().end));
        }
    }
}

/// The kind of pattern element the library's node is; nothing for the library's wrappers, which the grammar gives no element.
fn pattern_node(kind: RegExpAstKind<'_>) -> Option<PatternNode> {
    Some(match kind {
        RegExpAstKind::Pattern(_) => PatternNode::Pattern,
        RegExpAstKind::Disjunction(_) => PatternNode::Disjunction,
        RegExpAstKind::Alternative(_) => PatternNode::Alternative,
        RegExpAstKind::Term(_) | RegExpAstKind::CharacterClassContents(_) => return None,
        RegExpAstKind::LookAroundAssertion(_) => PatternNode::LookAroundAssertion,
        RegExpAstKind::Quantifier(_) => PatternNode::Quantifier,
        RegExpAstKind::CapturingGroup(_) => PatternNode::CapturingGroup,
        RegExpAstKind::IgnoreGroup(_) => PatternNode::IgnoreGroup,
        RegExpAstKind::BoundaryAssertion(_) => PatternNode::BoundaryAssertion,
        RegExpAstKind::Character(_) => PatternNode::Character,
        RegExpAstKind::Dot(_) => PatternNode::Dot,
        RegExpAstKind::CharacterClassEscape(_) => PatternNode::CharacterClassEscape,
        RegExpAstKind::UnicodePropertyEscape(_) => PatternNode::UnicodePropertyEscape,
        RegExpAstKind::CharacterClass(_) => PatternNode::CharacterClass,
        RegExpAstKind::CharacterClassRange(_) => PatternNode::CharacterClassRange,
        RegExpAstKind::CharacterClassStringDisjunction(_) => PatternNode::ClassStringDisjunction,
        RegExpAstKind::CharacterClassString(_) => PatternNode::ClassString,
        RegExpAstKind::IndexedReference(_) => PatternNode::IndexedReference,
        RegExpAstKind::NamedReference(_) => PatternNode::NamedReference,
    })
}

/// Notes the string a call of `RegExp` takes first as a possible carrier, with the flags its second argument gives: whether the name
/// is the global constructor is known only once the file's names are resolved.
pub(in super::super) fn constructed<'a>(projection: &mut VisitorStack<'a>, kind: AstKind<'a>) {
    let (callee, arguments) = match kind {
        AstKind::CallExpression(call) => (&call.callee, &call.arguments),
        AstKind::NewExpression(call) => (&call.callee, &call.arguments),
        _ => return,
    };
    let Expression::Identifier(name) = callee else {
        return;
    };
    if name.name != PATTERN_CONSTRUCTOR {
        return;
    }
    let Some(Argument::StringLiteral(pattern)) = arguments.first() else {
        return;
    };
    let flags = match arguments.get(1) {
        None => PatternFlags::Absent,
        Some(Argument::StringLiteral(flags)) => {
            PatternFlags::Quoted(flags.span, DecodedText(flags.value.to_string()))
        }
        Some(_) => PatternFlags::Unknown,
    };
    projection.carriers.push(Carrier {
        span: pattern.span,
        form: Form::Constructor(&**name),
        flags,
        host: Hosting::Unwritten,
    });
}

/// Whether the node the walk enters carries a pattern: a pattern literal always, a string when the call around it noted it.
pub(in super::super) fn carried(projection: &mut VisitorStack<'_>, kind: AstKind<'_>) -> Carries {
    match kind {
        AstKind::RegExpLiteral(literal) => {
            let start = literal.span.start.saturating_add(1);
            let end =
                start.saturating_add(u32::try_from(literal.regex.pattern.text.len()).unwrap_or(0));
            projection.carriers.push(Carrier {
                span: literal.span,
                form: Form::Literal(Span::new(start, end)),
                flags: PatternFlags::Bare(Span::new(end.saturating_add(1), literal.span.end)),
                host: Hosting::Unwritten,
            });
            Carries::Pattern(CarrierIndex(projection.carriers.len() - 1))
        }
        AstKind::StringLiteral(literal) => match projection.carriers.last() {
            Some(carrier)
                if carrier.span == literal.span
                    && matches!(carrier.form, Form::Constructor(_))
                    && carrier.host == Hosting::Unwritten =>
            {
                Carries::Pattern(CarrierIndex(projection.carriers.len() - 1))
            }
            _ => Carries::Nothing,
        },
        _ => Carries::Nothing,
    }
}

/// Parses the pattern of every carrier the projection wrote and writes it under the carrier: its elements with their numbers, and on
/// the carrier the flags it was read with, or the library's message when it refused the pattern. A string is a carrier only when the
/// `RegExp` it is passed to names the global constructor: its identifier has no `REFERENCE_DECLARATION`, so no declaration of the
/// file binds it.
pub(in super::super) fn written_patterns(
    allocator: &Allocator,
    source: NameText<'_>,
    carriers: &[Carrier<'_>],
    scoping: &Scoping,
    builder: &mut languages::TableBuilder<'_>,
) {
    for carrier in carriers {
        let Hosting::At(host) = carrier.host else {
            continue;
        };
        let parsed = match carrier.form {
            Form::Literal(pattern) => {
                let PatternFlags::Bare(flags) = carrier.flags else {
                    continue;
                };
                LiteralParser::new(
                    allocator,
                    slice(source, pattern).0,
                    Some(slice(source, flags).0),
                    Options {
                        pattern_span_offset: pattern.start,
                        flags_span_offset: flags.start,
                    },
                )
                .parse()
            }
            Form::Constructor(name) => {
                let global = name
                    .reference_id
                    .get()
                    .is_some_and(|reference| declaration_start(scoping, reference).is_none());
                if !global {
                    continue;
                }
                let (flags, flags_start) = match &carrier.flags {
                    PatternFlags::Quoted(span, _) => (Some(slice(source, *span).0), span.start),
                    PatternFlags::Bare(_) | PatternFlags::Absent | PatternFlags::Unknown => {
                        (None, 0)
                    }
                };
                ConstructorParser::new(
                    allocator,
                    slice(source, carrier.span).0,
                    flags,
                    Options {
                        pattern_span_offset: carrier.span.start,
                        flags_span_offset: flags_start,
                    },
                )
                .parse()
            }
        };
        let flags = match &carrier.flags {
            PatternFlags::Bare(span) => slice(source, *span),
            PatternFlags::Quoted(_, text) => NameText(text.0.as_str()),
            PatternFlags::Absent => NameText(NO_FLAGS),
            PatternFlags::Unknown => NameText(UNKNOWN_FLAGS),
        };
        languages::write_embedded(
            builder,
            host,
            ByteSlice(source.0.as_bytes()),
            EmbeddedMapping::Exact(Offset(0)),
            |logical| {
                languages::node_property(logical, PATTERN_FLAGS, flags);
                match parsed {
                    Ok(pattern) => PatternWriter { builder: logical }.visit_pattern(&pattern),
                    Err(error) => {
                        // A message formats into retained storage without failure.
                        let _written = languages::node_property_fmt(
                            logical,
                            PATTERN_ERROR,
                            format_args!("{error}"),
                        );
                    }
                }
            },
        );
    }
}

/// The text of the source at `span`; empty where the span is not of it.
fn slice<'s>(source: NameText<'s>, span: Span) -> NameText<'s> {
    NameText(
        source
            .0
            .get(
                usize::try_from(span.start).unwrap_or(usize::MAX)
                    ..usize::try_from(span.end).unwrap_or(usize::MAX),
            )
            .unwrap_or_default(),
    )
}
