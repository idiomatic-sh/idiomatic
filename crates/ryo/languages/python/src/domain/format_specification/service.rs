//! Read decoded brace fields and syntax-owned f-string specs, without call/type policy.
use super::types::{Observation, Observations};
use crate::domain::{
    brace_format::types::{
        BraceFieldContext, BraceFormatIndex, BraceFormatResult, BraceFormatState,
    },
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
use ryo::languages::{self, Position, Tree};
use super::constants;
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
/// Extract observations after the caller has lazily prepared the independent brace grammar.
pub(crate) fn observe(
    tree: &Tree<'_>,
    braces: &BraceFormatIndex,
    target: (i64, i64),
) -> Observations {
    let mut result = Observations {
        partial: braces.state != BraceFormatState::Complete,
        ..Default::default()
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
            result.rows.push(Observation {
                id: occurrence.node,
                ordinal: field.ordinal.0,
                parent: match field.context {
                    BraceFieldContext::Literal => None,
                    BraceFieldContext::FormatSpec { parent_ordinal } => Some(parent_ordinal.0),
                },
                origin: occurrence.node,
                source_kind: constants::BRACE_STRING,
                value: Some(spec.0.clone()),
                outcome: recognize(FormatSpecText(&spec.0), nested.into(), target.into()),
                selector: field.selector_text.0.clone(),
                conversion: field
                    .conversion
                    .map_or_else(String::new, |value| value.to_string()),
            });
        }
    }
    let mut owners = indexmap::IndexMap::<Position, Vec<Position>>::new();
    for field in crate::domain::semantic_syntax::service::positions(tree) {
        if kind(tree, field) != Kind::Node(Node::FormattedValue) {
            continue;
        }
        if let Some(owner) = fstring_owner(tree, field) {
            owners.entry(owner).or_default().push(field);
        }
    }
    for (owner, fields) in owners {
        for (ordinal, field) in fields.iter().enumerate() {
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
                if let Some(found) = fields.iter().position(|each| *each == above) {
                    break Some(found);
                }
                let next = parent(tree, above);
                if next == above {
                    break None;
                }
                above = next;
            };
            let mut decoded = String::new();
            let (value, outcome) = match decoded_into(tree, spec, FormatScratch(&mut decoded)) {
                Ok(DecodedInput::Static) => {
                    let outcome = recognize(FormatSpecText(&decoded), false.into(), target.into());
                    (Some(decoded), outcome)
                }
                Ok(DecodedInput::Dynamic) => (None, StandardSpecOutcome::Dynamic),
                Err(()) => {
                    result.partial = true;
                    continue;
                }
            };
            result.rows.push(Observation {
                id: owner,
                ordinal,
                parent: parent_ordinal,
                origin: spec,
                source_kind: constants::FSTRING,
                value,
                outcome,
                selector: String::new(),
                conversion: languages::property_at(
                    tree.elements,
                    *field,
                    crate::domain::syntax::constants::INTERPOLATION_CONVERSION,
                )
                .map_or_else(String::new, |value| value.0.to_owned()),
            });
        }
    }
    result
}
