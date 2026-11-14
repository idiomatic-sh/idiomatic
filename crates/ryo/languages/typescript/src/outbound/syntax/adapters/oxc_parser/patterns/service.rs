//! The patterns of the adapter: which elements carry a regular-expression pattern, and the trees `oxc_regular_expression` parses of
//! them, written under each carrier with the properties the domain reads.

use super::super::constants::{EMPTY_PATTERN, NO_FLAGS, PATTERN_CONSTRUCTOR, UNKNOWN_FLAGS};
use super::super::projection::service::declaration_start;
use super::super::types::{
    Carrier, CarrierIndex, Carries, DecodedText, Form, Hosting, PatternFlags, PatternWriter, Reach,
    VisitorStack,
};
use crate::domain::syntax::constants::{
    PATTERN_ERROR, PATTERN_ERROR_END, PATTERN_ERROR_START, PATTERN_FLAGS, PATTERN_FLAGS_ERROR,
    PATTERN_VALUE,
};
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
        let number = if let RegExpAstKind::Character(character) = kind {
            Some(character.value)
        } else if let RegExpAstKind::IndexedReference(reference) = kind {
            Some(reference.index)
        } else {
            None
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

/// Notes the string a call of `RegExp` takes first as a possible carrier, with the flags its second argument gives; or, when the first
/// is no string, the string the second is, whose flags alone are read. Whether the name is the global constructor is known only once
/// the file's names are resolved.
pub(in super::super) fn constructed<'a>(projection: &mut VisitorStack<'a>, kind: AstKind<'a>) {
    let (callee, arguments) = if let AstKind::CallExpression(call) = kind {
        (&call.callee, &call.arguments)
    } else if let AstKind::NewExpression(call) = kind {
        (&call.callee, &call.arguments)
    } else {
        return;
    };
    let Expression::Identifier(name) = callee else {
        return;
    };
    if name.name != PATTERN_CONSTRUCTOR {
        return;
    }
    let flags = match arguments.get(1) {
        None => PatternFlags::Absent,
        Some(Argument::StringLiteral(flags)) => {
            PatternFlags::Quoted(flags.span, DecodedText(flags.value.to_string()))
        }
        Some(_) => PatternFlags::Unknown,
    };
    let (span, form) = match (arguments.first(), &flags) {
        (Some(Argument::StringLiteral(pattern)), _) => (pattern.span, Form::Constructor(name)),
        (Some(_), PatternFlags::Quoted(span, _)) => (*span, Form::Flags),
        _ => return,
    };
    projection.carriers.push(Carrier {
        span,
        form,
        flags,
        host: Hosting::Unwritten,
    });
    projection
        .pending
        .push(CarrierIndex(projection.carriers.len() - 1));
}

/// Whether the node the walk enters carries a pattern: a pattern literal always, a string when it is the one the last pending call
/// noted.
pub(in super::super) fn carried(projection: &mut VisitorStack<'_>, kind: AstKind<'_>) -> Carries {
    if let AstKind::RegExpLiteral(literal) = kind {
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
    } else if let AstKind::StringLiteral(literal) = kind
        && projection.pending.last().is_some_and(|index| {
            projection
                .carriers
                .get(index.0)
                .is_some_and(|carrier| carrier.span == literal.span)
        })
        && let Some(index) = projection.pending.pop()
    {
        Carries::Pattern(index)
    } else {
        Carries::Nothing
    }
}

/// Parses the pattern of every carrier the projection wrote and writes it under the carrier. A pattern a literal or the global
/// constructor reads is written whole: the flags it was read with, then its elements with their numbers, or the place and message of
/// what the library refused. The global constructor is a `RegExp` whose identifier has no `REFERENCE_DECLARATION`, so no declaration
/// of the file binds it. Any other carrier, a string a `RegExp` some declaration of the file binds is given, or flags given with no
/// pattern string, is written only when the library refuses it, and never with elements.
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
        let (flags, flags_span) = match &carrier.flags {
            PatternFlags::Bare(span) => (slice(source, *span), Some(*span)),
            PatternFlags::Quoted(span, text) => (NameText(text.0.as_str()), Some(*span)),
            PatternFlags::Absent => (NameText(NO_FLAGS), None),
            PatternFlags::Unknown => (NameText(UNKNOWN_FLAGS), None),
        };
        let (reach, parsed) = match carrier.form {
            Form::Literal(pattern) => {
                let PatternFlags::Bare(bare) = carrier.flags else {
                    continue;
                };
                let parsed = LiteralParser::new(
                    allocator,
                    slice(source, pattern).0,
                    Some(slice(source, bare).0),
                    Options {
                        pattern_span_offset: pattern.start,
                        flags_span_offset: bare.start,
                    },
                )
                .parse();
                (Reach::Read, parsed)
            }
            Form::Constructor(name) => {
                let global = name
                    .reference_id
                    .get()
                    .is_some_and(|reference| declaration_start(scoping, reference).is_none());
                let reach = if global { Reach::Read } else { Reach::Checked };
                let parser =
                    constructor_parser(allocator, source, Some(carrier.span), &carrier.flags);
                (reach, parser.parse())
            }
            Form::Flags => (
                Reach::Checked,
                constructor_parser(allocator, source, None, &carrier.flags).parse(),
            ),
        };
        if reach == Reach::Checked && parsed.is_ok() {
            continue;
        }
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
                        let place = error.labels.as_slice().first().map(|label| {
                            Span::new(label.offset(), label.offset().saturating_add(label.len()))
                        });
                        refusal(logical, place, flags_span, format_args!("{error}"));
                    }
                }
            },
        );
    }
}

/// The parser of the pattern string a call of `RegExp` is given at `pattern`, or of an empty one when it is given none, with the flags
/// string it is given when it is one.
fn constructor_parser<'a>(
    allocator: &'a Allocator,
    source: NameText<'a>,
    pattern: Option<Span>,
    flags: &PatternFlags,
) -> ConstructorParser<'a> {
    let (flags_text, flags_start) = match flags {
        PatternFlags::Quoted(span, _) => (Some(slice(source, *span).0), span.start),
        PatternFlags::Bare(_) | PatternFlags::Absent | PatternFlags::Unknown => (None, 0),
    };
    let (pattern_text, pattern_start) = pattern.map_or((EMPTY_PATTERN, 0), |span| {
        (slice(source, span).0, span.start)
    });
    ConstructorParser::new(
        allocator,
        pattern_text,
        flags_text,
        Options {
            pattern_span_offset: pattern_start,
            flags_span_offset: flags_start,
        },
    )
}

/// Writes on a carrier what the library refused: its message, under the flags' property when the place it points at lies in the
/// flags and the pattern's otherwise, and that place.
fn refusal(
    logical: &mut languages::TableBuilder<'_>,
    place: Option<Span>,
    flags: Option<Span>,
    message: std::fmt::Arguments<'_>,
) {
    let in_flags = place
        .zip(flags)
        .is_some_and(|(place, flags)| flags.contains_inclusive(place));
    let property = if in_flags {
        PATTERN_FLAGS_ERROR
    } else {
        PATTERN_ERROR
    };
    // A message and offsets format into retained storage without failure.
    let _written = languages::node_property_fmt(logical, property, message);
    if let Some(place) = place {
        let _start = languages::node_property_fmt(
            logical,
            PATTERN_ERROR_START,
            format_args!("{}", place.start),
        );
        let _end =
            languages::node_property_fmt(logical, PATTERN_ERROR_END, format_args!("{}", place.end));
    }
}

/// The text of the source at `span`; empty where the span is not of it.
fn slice(source: NameText<'_>, span: Span) -> NameText<'_> {
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
