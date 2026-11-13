//! The parser options a run's settings and a file's kind select.

use crate::outbound::syntax::types::PythonSourceKind;
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::ParseOptions;

/// The parser options derived from Python-owned settings and the source path's kind.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the shared syntax adapter carries immutable parser settings by reference"
)]
pub(super) fn options(
    source_kind: PythonSourceKind,
    settings: &PythonParserSettings,
) -> ParseOptions {
    let source_type = source_type(source_kind);
    let target_version = match settings.target_version() {
        PythonTargetVersion::Py37 => PythonVersion::PY37,
        PythonTargetVersion::Py38 => PythonVersion::PY38,
        PythonTargetVersion::Py39 => PythonVersion::PY39,
        PythonTargetVersion::Py310 => PythonVersion::PY310,
        PythonTargetVersion::Py311 => PythonVersion::PY311,
        PythonTargetVersion::Py312 => PythonVersion::PY312,
        PythonTargetVersion::Py313 => PythonVersion::PY313,
        PythonTargetVersion::Py314 => PythonVersion::PY314,
        PythonTargetVersion::Py315 => PythonVersion::PY315,
        PythonTargetVersion::Latest => PythonVersion::latest(),
    };
    ParseOptions::from(source_type).with_target_version(target_version)
}

/// Map the language-owned Python source kind to Ruff's parser source type.
pub(super) const fn source_type(source_kind: PythonSourceKind) -> PySourceType {
    match source_kind {
        PythonSourceKind::Python => PySourceType::Python,
        PythonSourceKind::Stub => PySourceType::Stub,
    }
}
