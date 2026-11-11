//! The `format_specification` service's cases: the standard format specifications of the brace strings and f-strings of a parsed
//! source, with their place, their value, their outcome, their selector and their conversion.

use super::service::observe;
use super::types::ObservationCompleteness;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::brace_format::service::prepare;
use crate::domain::model::types::Declarations;
use crate::domain::semantic_syntax::{
    service::{positions, visibility},
    types::Visibility,
};
use crate::fixtures::reading::parsed;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
use crate::settings::PythonParserSettings;
use ryo::languages::{self, ByteSlice, Parse as _, Tree};
use std::sync::Arc;

/// A source.
type Input = &'static str;
/// Each specification as `source#ordinal<parent> value outcome selector!conversion`, joined by `|`, then whether the reading was whole.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_brace_string_field_has_its_spec_and_selector"),
        word: Word::Succeeds,
        input: || "'{0.a[1]:>10}'.format(v)\n",
        expect: Expect::Value("brace-string#0<-> Some(\">10\") Recognized \"0.a[1]\"!\"\"|whole"),
    },
    Case {
        name: CaseName("a_nested_brace_field_makes_its_parent_spec_dynamic"),
        word: Word::Succeeds,
        input: || "'{:{w}d}'.format(v, w=3)\n",
        expect: Expect::Value("brace-string#0<-> Some(\"{w}d\") Dynamic \"\"!\"\"|whole"),
    },
    Case {
        name: CaseName("an_fstring_spec_keeps_its_conversion"),
        word: Word::Succeeds,
        input: || "f'{x!r:>8}'\n",
        expect: Expect::Value("fstring#0<-> Some(\">8\") Recognized \"\"!\"r\"|whole"),
    },
    Case {
        name: CaseName("an_fstring_spec_with_a_field_in_it_is_dynamic"),
        word: Word::Succeeds,
        input: || "f'{x:>{width}}'\n",
        expect: Expect::Value("fstring#0<-> None Dynamic \"\"!\"\"|whole"),
    },
    Case {
        name: CaseName("an_unknown_presentation_type_is_invalid"),
        word: Word::Succeeds,
        input: || "f'{x:q}'\n",
        expect: Expect::Value(
            "fstring#0<-> Some(\"q\") InvalidType(PresentationCharacter('q')) \"\"!\"\"|whole",
        ),
    },
];

/// Observes the specifications of the source.
fn subject(source: Input) -> Result<Value, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parser = ParseContract::new(
        PythonParserSettings::default(),
        Arc::new(ruff_python_parser(PythonParserSettings::default())),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let braces = prepare::<RuffLiteralBraceFormat>(
        &tree,
        &model.brace_format,
        &mut file.symbols,
        positions(&tree),
        |node, symbols| (visibility(&tree, &model, node, symbols) == Visibility::Visible).into(),
    );
    let observations = observe(&tree, &braces, model.language_target);
    let mut texts: Vec<String> = observations
        .rows
        .iter()
        .map(|spec| {
            format!(
                "{}#{}<{}> {:?} {:?} {:?}!{:?}",
                spec.source_kind.0,
                spec.ordinal.0,
                spec.parent
                    .map_or_else(|| "-".to_owned(), |parent| parent.0.to_string()),
                spec.value.as_ref().map(|value| value.0.as_str()),
                spec.outcome,
                spec.selector.0,
                spec.conversion.0
            )
        })
        .collect();
    texts.push(
        match observations.completeness {
            ObservationCompleteness::Whole => "whole",
            ObservationCompleteness::Partial => "partial",
        }
        .to_owned(),
    );
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
