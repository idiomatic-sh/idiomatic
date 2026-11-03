//! Diagnostics of manifest syntax and metadata schemas, independent of any lint rule.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Span};
use crate::domain::terms::types::Symbol;
/// One manifest diagnostic schema.
#[derive(Clone, Copy, Debug)]
pub struct ManifestDiagnostic;
impl super::super::interface::sealed::Sealed for ManifestDiagnostic {}
/// Typed manifest diagnostic declaration.
#[macro_export]
#[doc(hidden)]
macro_rules! manifest_diagnostic {
    ($callback:path) => {
        $callback! {
            item ManifestDiagnostic,
            row { /// One metadata diagnostic.
                ManifestDiagnosticRow },
            column { /// Metadata diagnostic columns.
                ManifestDiagnosticColumn },
            expected { /// One independently expected diagnostic row.
                ManifestDiagnosticExpectedRow },
            class Manifest,
            columns {
                /// The real manifest identity.
                file File: FileId => File;
                /// A reported byte span, or the genuine file origin when no finer location exists.
                span Span: Span => Span;
                /// Whether the location is a reported source span or the file origin.
                location_kind LocationKind: Symbol => Symbol;
                /// The diagnostic stage: parsing or metadata validation.
                phase Phase: Symbol => Symbol;
                /// The provider's metadata schema identity.
                schema Metadata: Symbol => Symbol;
                /// A stable provider-owned diagnostic category.
                code Code: Symbol => Symbol;
                /// The original diagnostic detail.
                message Message: Symbol => Symbol;
            }
        }
    };
}
crate::manifest_diagnostic!(crate::relation_row);
