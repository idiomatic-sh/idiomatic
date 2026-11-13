//! Read decoded brace fields and syntax-owned f-string specs, without call/type policy.
use super::constants;
use super::types::{
    ConversionText, Observation, ObservationCompleteness, Observations, ObservedPiece,
    ObservedSpan, ObservedText, OwnedField, SelectorPath, SpanEnd, SpanStart, SpecOrdinal,
    SpecSource, SpecValue,
};
use crate::domain::brace_format::types::CachedBraceFormat;
use crate::domain::format_syntax::types::RetainedFormatIndex;
use crate::domain::{
    brace_format::types::{BraceFieldContext, BraceFormatResult, BraceFormatState},
    fields::service::field_of,
    format_parsing::service::decoded_into,
    format_syntax::types::{DecodedInput, FormatScratch},
    labels::types::Field,
    nodes::types::Node,
    standard_format_spec::{
        service::recognize,
        types::{FormatSpecText, StandardSpecOutcome},
    },
    syntax::{
        service::{kind, parent},
        types::Kind,
    },
};
use ryo::languages::{self, NameText, Position, Tree};
/// The actual f-string owner, excluding synthetic format-spec JoinedStr nodes.
fn fstring_owner(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let mut above = parent(tree, at);
    let mut child = at;
    while above != child {
        if kind(tree, above) == Kind::Node(Node::JoinedStr)
            && field_of(tree, above) != Field::FormatSpec
        {
            return Some(above);
        }
        child = above;
        above = parent(tree, above);
    }
    None
}
/// The span of `piece` appended to the observations' text.
fn appended(text: &mut ObservedText, piece: ObservedPiece<'_>) -> ObservedSpan {
    let start = SpanStart(text.0.len());
    text.0.push_str(piece.0);
    ObservedSpan {
        start,
        end: SpanEnd(text.0.len()),
    }
}

/// What an observation's span reads in the observations' text.
pub(crate) fn text_of(observations: &Observations, span: ObservedSpan) -> NameText<'_> {
    NameText(
        observations
            .text
            .0
            .get(span.start.0..span.end.0)
            .unwrap_or(constants::NO_TEXT),
    )
}

/// Extract observations after the caller has lazily prepared the independent brace grammar. `result` is the caller's, kept between
/// files, so a file whose specifications fit what an earlier one grew allocates nothing.
pub(crate) fn observe(
    tree: &Tree<'_>,
    braces: &RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
    target: crate::domain::standard_library::types::TargetVersion,
    result: &mut Observations,
) {
    result.rows.clear();
    result.text.0.clear();
    result.completeness = if braces.state == BraceFormatState::Complete {
        ObservationCompleteness::Whole
    } else {
        ObservationCompleteness::Partial
    };
    for occurrence in &braces.occurrences {
        // F-string specs have their own AST fields and must not be reinterpreted as brace strings.
        if kind(tree, occurrence.node) != Kind::Node(Node::Constant) {
            continue;
        }
        let Some(BraceFormatResult::Parsed(parsed)) = occurrence
            .cached
            .and_then(|at| braces.values.get(at.0))
            .map(|value| &value.result)
        else {
            continue;
        };
        for field in &parsed.fields {
            let Some(spec) = &field.format_spec else {
                continue;
            };
            let nested = parsed.fields.iter().any(|nested| matches!(nested.context, BraceFieldContext::FormatSpec { parent_ordinal } if parent_ordinal == field.ordinal));
            let value = appended(&mut result.text, ObservedPiece(&spec.0));
            let selector = appended(&mut result.text, ObservedPiece(&field.selector_text.0));
            let conversion = {
                let start = SpanStart(result.text.0.len());
                if let Some(character) = field.conversion {
                    result.text.0.push(character.0);
                }
                ObservedSpan {
                    start,
                    end: SpanEnd(result.text.0.len()),
                }
            };
            result.rows.push(Observation {
                id: occurrence.node,
                ordinal: SpecOrdinal(field.ordinal.0),
                parent: match field.context {
                    BraceFieldContext::Literal => None,
                    BraceFieldContext::FormatSpec { parent_ordinal } => {
                        Some(SpecOrdinal(parent_ordinal.0))
                    }
                },
                origin: occurrence.node,
                source_kind: SpecSource(constants::BRACE_STRING),
                value: Some(SpecValue(value)),
                outcome: recognize(FormatSpecText(&spec.0), nested.into(), target.into()),
                selector: SelectorPath(selector),
                conversion: ConversionText(conversion),
            });
        }
    }
    result.owners.clear();
    for field in (0..languages::end_of(tree.elements).0).map(Position) {
        if kind(tree, field) != Kind::Node(Node::FormattedValue) {
            continue;
        }
        if let Some(owner) = fstring_owner(tree, field) {
            result.owners.push(OwnedField { owner, field });
        }
    }
    // Preorder positions put an owner before its fields, so grouping by owner position keeps each owner's first-field order.
    result.owners.sort_unstable();
    let owners = std::mem::take(&mut result.owners);
    for group in owners.chunk_by(|left, right| left.owner == right.owner) {
        let Some(owner) = group.first().map(|each| each.owner) else {
            continue;
        };
        for (ordinal, OwnedField { field, .. }) in group.iter().enumerate() {
            let Some(spec) = languages::children(tree.elements, *field)
                .find(|child| field_of(tree, *child) == Field::FormatSpec)
            else {
                continue;
            };
            let mut above = parent(tree, *field);
            let parent_ordinal = loop {
                if above == owner {
                    break None;
                }
                if let Some(found) = group.iter().position(|each| each.field == above) {
                    break Some(found);
                }
                let next = parent(tree, above);
                if next == above {
                    break None;
                }
                above = next;
            };
            let (value, outcome) =
                match decoded_into(tree, spec, FormatScratch(&mut result.decoded.0)) {
                    Ok(DecodedInput::Static) => {
                        let outcome = recognize(
                            FormatSpecText(&result.decoded.0),
                            false.into(),
                            target.into(),
                        );
                        (
                            Some(SpecValue(appended(
                                &mut result.text,
                                ObservedPiece(&result.decoded.0),
                            ))),
                            outcome,
                        )
                    }
                    Ok(DecodedInput::Dynamic) => (None, StandardSpecOutcome::Dynamic),
                    Err(()) => {
                        result.completeness = ObservationCompleteness::Partial;
                        continue;
                    }
                };
            result.rows.push(Observation {
                id: owner,
                ordinal: SpecOrdinal(ordinal),
                parent: parent_ordinal.map(SpecOrdinal),
                origin: spec,
                source_kind: SpecSource(constants::FSTRING),
                value,
                outcome,
                selector: SelectorPath::default(),
                conversion: ConversionText(appended(
                    &mut result.text,
                    ObservedPiece(
                        languages::property_at(
                            tree.elements,
                            *field,
                            crate::domain::syntax::constants::INTERPOLATION_CONVERSION,
                        )
                        .map_or(constants::NO_TEXT, |value| value.0),
                    ),
                )),
            });
        }
    }
    result.owners = owners;
}
