//! Emit the enumerated potential implicit-name catalog once per physical module.

use ryo::languages;

/// Enumerated Python implicit-name providers; entries are potential origins, not live values.
pub(crate) struct ImplicitSymbolItem;

#[derive(Clone, Copy)]
/// A potential Python environment-provided name and its target-version origin.
pub(crate) struct CatalogName {
    /// The spelling made available by Python.
    pub name: languages::NameText<'static>,
    /// First Python major version providing the name.
    pub major: languages::Integer,
    /// First Python minor version providing the name.
    pub minor: languages::Integer,
}

/// Names supplied by the builtins environment, including attributes of the builtins module.
/// These module attributes overlap with module execution globals without sharing origin.
pub(crate) const BUILTIN_ATTRIBUTES: &[CatalogName] = &[
    catalog(
        languages::NameText(super::constants::DOC),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::LOADER),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::NAME),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::PACKAGE),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::SPEC),
        languages::Integer(3),
        languages::Integer(4),
    ),
];

/// Selected known names installed by Python's module environment, not declarations.
/// This is an enumerated provider catalog, not an exhaustive claim about every import loader.
/// Version anchors: `__cached__` is in What's New 3.2; `__spec__` was added in 3.4; module
/// `__annotations__` begins with PEP 526 in 3.6; `__annotate__` was added in the 3.14 data model.
/// The baseline names predate Python 3.0: `__loader__` is from PEP 302 and `__package__` from
/// PEP 366. Sources: docs.python.org/3.2/whatsnew/3.2.html, docs.python.org/3.12/reference/datamodel.html,
/// docs.python.org/3.6/whatsnew/3.6.html, docs.python.org/3/reference/datamodel.html,
/// peps.python.org/pep-0302/, and peps.python.org/pep-0366/.
pub(crate) const MODULE_GLOBALS: &[CatalogName] = &[
    catalog(
        languages::NameText(super::constants::ANNOTATIONS),
        languages::Integer(3),
        languages::Integer(6),
    ),
    catalog(
        languages::NameText(super::constants::BUILTINS),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::CACHED),
        languages::Integer(3),
        languages::Integer(2),
    ),
    catalog(
        languages::NameText(super::constants::DOC),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::FILE),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::LOADER),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::NAME),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::PACKAGE),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::SPEC),
        languages::Integer(3),
        languages::Integer(4),
    ),
    catalog(
        languages::NameText(super::constants::WARNINGREGISTRY),
        languages::Integer(3),
        languages::Integer(0),
    ),
    catalog(
        languages::NameText(super::constants::ANNOTATE),
        languages::Integer(3),
        languages::Integer(14),
    ),
];

/// Build a static catalog row from a name and its first supported version.
const fn catalog(
    name: languages::NameText<'static>,
    major: languages::Integer,
    minor: languages::Integer,
) -> CatalogName {
    CatalogName { name, major, minor }
}
