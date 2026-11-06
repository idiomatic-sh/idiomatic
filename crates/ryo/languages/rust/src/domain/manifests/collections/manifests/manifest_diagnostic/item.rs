//! The `manifest_diagnostic` relation of the language: one row for each `Cargo.toml` Cargo's reading refused, with what it said.

use super::constants::{
    CARGO_MANIFEST, FILE_ORIGIN, INCONSISTENT_WORKSPACE, PARSE, UNREADABLE_MANIFEST, VALIDATION,
};
use crate::domain::links::types::Linked;
use crate::domain::packages::service::{diagnoses_in, named};
use crate::domain::packages::types::{Diagnosed, Stage};
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `manifest_diagnostic` relation's rows to.
pub(crate) struct ManifestDiagnosticItem;

// The reader says a refusal as text, with no place in it a byte span could be made of, so the row is at the origin of the file; a
// manifest the run does not list has no file to say it of.
ryo::reads! {
    ManifestDiagnosticItem => languages::ManifestDiagnostic, manifests(Linked) |input, symbols|
    model diagnosed: Vec<Diagnosed> = (input.derived.packages.as_ref().map(|packages| diagnoses_in(&input.request.files, packages))) {
        each (&diagnosed) as diagnosis {
            row {
                file: value(diagnosis.file),
                span: value(languages::Span { start: languages::Offset(0), end: languages::Offset(0) }),
                location_kind: value(named(symbols, languages::NameText(FILE_ORIGIN))),
                phase: value(named(symbols, languages::NameText(match diagnosis.stage {
                    Stage::Parse => PARSE,
                    Stage::Validation => VALIDATION,
                }))),
                schema: value(named(symbols, languages::NameText(CARGO_MANIFEST))),
                code: value(named(symbols, languages::NameText(match diagnosis.stage {
                    Stage::Parse => UNREADABLE_MANIFEST,
                    Stage::Validation => INCONSISTENT_WORKSPACE,
                }))),
                message: value(named(symbols, languages::NameText(&diagnosis.reason.0))),
            }
        }
    }
}
