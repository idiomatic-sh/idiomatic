//! The exact diagnostic of a `package.json` the reader refuses, for the project in `manifests/invalid-manifest/`.

use ryo::languages::contract::{ManifestDiagnosticExpectedRow, ManifestExpected, file, name, span};

/// The rows every manifest relation must give for the project in `manifests/invalid-manifest/`: the one diagnostic of the refused
/// `package.json`.
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("invalid-manifest").rows(&[ManifestDiagnosticExpectedRow {
        file: file("package.json"),
        span: span((1, 1), (1, 1)),
        location_kind: name("file-origin"),
        phase: name("parse"),
        schema: name("package.json"),
        code: name("unreadable-manifest"),
        message: name("EOF while parsing an object at line 1 column 1"),
    }])
}
