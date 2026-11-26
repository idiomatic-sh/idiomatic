//! The parse contract's own operations: the kind of a source.
use super::constants::STUB_EXTENSION;
use crate::outbound::syntax::types::PythonSourceKind;
use ryo::languages;

/// Select stubs by the exact lowercase `.pyi` suffix used by the existing parser contract.
pub(crate) fn source_kind(path: languages::SourcePath<'_>) -> PythonSourceKind {
    if std::path::Path::new(path.0)
        .extension()
        .is_some_and(|extension| extension == std::ffi::OsStr::new(STUB_EXTENSION))
    {
        PythonSourceKind::Stub
    } else {
        PythonSourceKind::Python
    }
}
