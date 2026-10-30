//! The names of the `targets` concept: where Cargo looks for the targets of a package.

/// The directory of a package's sources.
pub(crate) const SOURCE: &str = "src";

/// The directory below the sources that holds binaries.
pub(crate) const BIN: &str = "bin";

/// The directory of examples.
pub(crate) const EXAMPLES: &str = "examples";

/// The directory of tests.
pub(crate) const TESTS: &str = "tests";

/// The directory of benchmarks.
pub(crate) const BENCHES: &str = "benches";

/// The file of a library.
pub(crate) const LIBRARY: &str = "lib.rs";

/// The file of a binary, an example or a test that is a directory.
pub(crate) const MAIN: &str = "main.rs";

/// The file of a build script.
pub(crate) const BUILD: &str = "build.rs";

/// The extension of a source file.
pub(crate) const EXTENSION: &str = crate::domain::language::constants::EXTENSION;

/// What a name starts with when Cargo leaves the file out.
pub(crate) const HIDDEN: char = '.';
