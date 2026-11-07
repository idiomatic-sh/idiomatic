//! The types of the `lint_store` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The directory a product keeps what its runs leave for the next in.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheDirectory(pub Box<str>);

/// Which build of the program and of the languages linked to it runs: what a store keeps is used only by the build that made it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildIdentity(pub Box<str>);

/// Whether a store keeps anything: one that does not asks nothing of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreKeeping {
    /// It keeps what a run gives it for the next.
    Keeps,
    /// It keeps nothing: what it is given is let go, and it reads back as empty.
    Discards,
}

/// Where a store is kept and which build keeps it: worked out once by the edge, as plain values, and handed to the factory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Store {
    /// The cache directory.
    pub directory: CacheDirectory,
    /// The build.
    pub build: BuildIdentity,
}
