//! The exact diagnostic of a `go.mod` the tool refuses, for the project in `manifests/invalid-module/`.

use ryo::languages::contract::{ManifestDiagnosticExpectedRow, ManifestExpected, file, name, span};

/// The rows every manifest relation must give for the project in `manifests/invalid-module/`: the one diagnostic of the refused `go.mod`.
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("invalid-module").rows(&[ManifestDiagnosticExpectedRow {
        file: file("go.mod"),
        span: span((1, 1), (1, 1)),
        location_kind: name("file-origin"),
        phase: name("parse"),
        schema: name("go-module"),
        code: name("invalid-go-mod"),
        message: name("line 3: the `require (` block is not closed"),
    }])
}
