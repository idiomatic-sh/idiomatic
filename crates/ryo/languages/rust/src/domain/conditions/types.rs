//! The types of the `conditions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A platform condition as Cargo shows it: a triple as it is, a `cfg(...)` expression with one space after a comma and around an `=`; empty when there is none.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlatformCondition(pub Box<str>);

impl std::ops::Deref for PlatformCondition {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for PlatformCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
