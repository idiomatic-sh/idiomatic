//! The exact parse diagnostic of an invalid `pyproject.toml` project fixture.

use ryo::languages::contract::{ManifestDiagnosticExpectedRow, ManifestExpected, file, name, span};

pub(super) fn expected() -> ManifestExpected {
    ManifestExpected::of("invalid-project").rows(&[ManifestDiagnosticExpectedRow {
        file: file("pyproject.toml"),
        span: span((1, 9), (1, 9)),
        location_kind: name("span"),
        phase: name("parse"),
        schema: name("python-project-metadata"),
        code: name("invalid-toml"),
        message: name("unclosed table, expected `]`"),
    }])
}
