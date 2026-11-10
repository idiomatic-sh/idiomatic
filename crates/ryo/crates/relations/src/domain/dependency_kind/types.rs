//! The types of the `dependency_kind` concept: the closed set of words, as an enum.

/// How a package depends on another.
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
pub enum DependencyKind {
    /// Needed to build and to run.
    Normal,
    /// Needed to test.
    Dev,
    /// Needed to build only.
    Build,
    /// Expected of whoever uses the package.
    Peer,
}
