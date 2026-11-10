//! The exact diagnostic of a `Cargo.toml` the reader refuses, for the project in `manifests/invalid-manifest/`.

use ryo::languages::contract::{ManifestDiagnosticExpectedRow, ManifestExpected, file, name, span};

/// The rows every manifest relation must give for the project in `manifests/invalid-manifest/`: the one diagnostic of the refused
/// `Cargo.toml`.
pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("invalid-manifest").rows(&[ManifestDiagnosticExpectedRow {
        file: file("Cargo.toml"),
        span: span((1, 1), (1, 1)),
        location_kind: name("file-origin"),
        phase: name("parse"),
        schema: name("cargo-manifest"),
        code: name("unreadable-manifest"),
        message: name("unclosed table, expected `]`"),
    }])
}
