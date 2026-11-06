//! The `manifest_diagnostic` relation of the language: one row for each manifest the reader refused, and for each `catalog:` requirement
//! no catalog has an entry for, with what was said.

use super::constants::{
    FILE_ORIGIN, PARSE, UNREADABLE_MANIFEST, UNRESOLVED_CATALOG_ENTRY, VALIDATION,
};
use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::diagnoses_in;
use crate::domain::packages::types::{Diagnosed, Stage};
use ryo::languages;

/// Answers the `manifest-diagnostic` rows of the repository's manifests.
pub(crate) struct ManifestDiagnosticItem;

// The reader says a refusal as text, with no place in it a byte span could be made of, so the row is at the origin of the file.
ryo::reads! {
    ManifestDiagnosticItem => languages::ManifestDiagnostic, manifests(Linked) |input, symbols|
    model diagnosed: Vec<Diagnosed> = (Some(diagnoses_in(&input.request.files, packages_in(input)))) {
        each (&diagnosed) as diagnosis {
            row {
                file: value(diagnosis.file),
                span: value(languages::Span { start: languages::Offset(0), end: languages::Offset(0) }),
                location_kind: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(FILE_ORIGIN))),
                phase: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match diagnosis.stage {
                    Stage::Parse => PARSE,
                    Stage::Validation => VALIDATION,
                }))),
                schema: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&diagnosis.read_as.0))),
                code: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(match diagnosis.stage {
                    Stage::Parse => UNREADABLE_MANIFEST,
                    Stage::Validation => UNRESOLVED_CATALOG_ENTRY,
                }))),
                message: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&diagnosis.reason.0))),
            }
        }
    }
}
