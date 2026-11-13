//! The types of the `dependency_origin` concept: the closed set of words, as an enum.

/// Where a dependency says the package it needs comes from, as its manifest writes it: how the package is found, not whether the
/// repository holds it (the row's `target` says that). A requirement that names a package by a version range is `registry` even when a
/// workspace member of that name answers it.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
    strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub enum DependencyOrigin {
    /// A package index, by the package's name and a version.
    Registry,
    /// A directory, by its path.
    Path,
    /// A git repository, by its URL.
    Git,
    /// An archive, by its URL.
    Url,
    /// A member of the manifest's own workspace, by its name.
    Workspace,
}
