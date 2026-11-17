//! The constants of the `imports` concept.

/// The check an entry's `imports` is: `<identity>#imports`.
pub const IMPORTS: &str = "imports";

/// The file whose check an import that could not be read is: `tastemaker.toml#unknown`.
pub const WORKSPACE_FILE: &str = crate::domain::definitions::constants::WORKSPACE_FILE;

/// The prefix of the built-in rules that read the imports.
pub const RULE: &str = "imports";

/// The characters a package's name and the code that names it may each write for the other: a name is compared with
/// each of them read as the first.
pub const JOINERS: [char; 2] = ['_', '-'];

/// What the stem of a test twin ends with.
pub const TEST_SUFFIX: &str = "_test";

/// The directory of integration tests.
pub const TESTS: &str = "tests";

/// The directory of benches.
pub const BENCHES: &str = "benches";
