//! Publish actual requested manifest identities and genuine parser locations.
use super::constants;
use crate::domain::{
    links::types::Linked,
    manifest_validation::types::{Location, ManifestValidations, Phase},
};
use ryo::languages;
/// Independent metadata validator rows.
pub(crate) struct ManifestDiagnosticItem;
ryo::reads! {
    ManifestDiagnosticItem => languages::ManifestDiagnostic, manifests(Linked) |input, symbols|
    model validations: ManifestValidations = (Some(input.derived.manifest_validation.clone())) {
        each (&validations.incomplete) as reason {
            partial reason.0.as_ref() => languages::Missing::Any;
        }
        each (&validations.files) as file {
            each (&file.validation.incomplete) as reason {
                partial reason.0.as_ref() => languages::Missing::Any;
            }
            each (&file.validation.diagnostics) as diagnostic {
                row {
                    file: value(file.file),
                    span: value(match diagnostic.location { Location::Span(span) => span, Location::FileOrigin => languages::Span { start: languages::Offset(0), end: languages::Offset(0) } }),
                    location_kind: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match diagnostic.location { Location::Span(_) => constants::SPAN, Location::FileOrigin => constants::FILE_ORIGIN }))),
                    phase: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match diagnostic.phase { Phase::Parse => constants::PARSE, Phase::Validation => constants::VALIDATION }))),
                    schema: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(constants::PROJECT_SCHEMA))),
                    code: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match diagnostic.phase { Phase::Parse => constants::INVALID_TOML, Phase::Validation => constants::INVALID_PROJECT_METADATA }))),
                    message: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&diagnostic.message.0))),
                }
            }
        }
    }
}
