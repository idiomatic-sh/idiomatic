//! The `manifest_diagnostic` relation of the language: one row for each `go.mod` or `go.work` the tool refused, with what it said.

use super::constants::{
    FILE_ORIGIN, INVALID_MODULE, INVALID_WORKSPACE, MODULE_SCHEMA, PARSE, WORKSPACE_SCHEMA,
};
use crate::domain::links::service::modules_in;
use crate::domain::links::types::Linked;
use crate::domain::modules::service::diagnoses_in;
use crate::domain::modules::types::{Diagnosed, Manifestness};
use crate::domain::names::service::named;
use ryo::languages;

/// The reader the `project` contract hands the `manifest_diagnostic` relation to, which reads its rows from the manifests the tool refused.
pub(crate) struct ManifestDiagnosticItem;

// The tool says a refusal as text, with no place in it a byte span could be made of, so the row is at the origin of the file.
ryo::reads! {
    ManifestDiagnosticItem => languages::ManifestDiagnostic, manifests(Linked) |input, symbols|
    model diagnosed: Vec<Diagnosed> = (Some(diagnoses_in(&input.request.files, modules_in(input)))) {
        each (diagnosed) as diagnosis {
            row {
                file: value(diagnosis.file),
                span: value(languages::Span { start: languages::Offset(0), end: languages::Offset(0) }),
                location_kind: value(named(symbols, languages::NameText(FILE_ORIGIN))),
                phase: value(named(symbols, languages::NameText(PARSE))),
                schema: value(named(symbols, languages::NameText(match diagnosis.manifest {
                    Manifestness::Workspace => WORKSPACE_SCHEMA,
                    Manifestness::Module | Manifestness::Other => MODULE_SCHEMA,
                }))),
                code: value(named(symbols, languages::NameText(match diagnosis.manifest {
                    Manifestness::Workspace => INVALID_WORKSPACE,
                    Manifestness::Module | Manifestness::Other => INVALID_MODULE,
                }))),
                message: value(named(symbols, languages::NameText(&diagnosis.reason.0))),
            }
        }
    }
}
