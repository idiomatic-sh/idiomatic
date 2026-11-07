//! The names of the `links` concept: the files a specifier may name, and the text a finding about the links shows.

/// The endings a specifier without one is tried with, in the order TypeScript tries them: its own, then the declarations, then
/// JavaScript's.
pub(crate) const EXTENSIONS: [&str; 5] = [
    crate::domain::language::constants::PLAIN_EXTENSION,
    crate::domain::language::constants::JSX_EXTENSION,
    ".d.ts",
    ".js",
    ".jsx",
];

/// A JavaScript ending a specifier writes and the TypeScript endings of the file it names, in the order TypeScript tries them.
pub(crate) const REWRITES: [(&str, &[&str]); 4] = [
    (".js", &[".ts", ".tsx", ".d.ts"]),
    (".jsx", &[".tsx", ".d.ts"]),
    (".mjs", &[".mts", ".d.mts"]),
    (".cjs", &[".cts", ".d.cts"]),
];

/// The file a directory a specifier names is, without its ending.
pub(crate) const INDEX: &str = "index";

/// The name of a project configuration.
pub(crate) const CONFIGURATION: &str = "tsconfig.json";

/// What separates the parts of a path.
pub(crate) const SEPARATOR: char = '/';

/// A part of a path that stays where it is.
pub(crate) const CURRENT: &str = crate::domain::packages::constants::CURRENT;

/// A part of a path that goes up one directory.
pub(crate) const PARENT: &str = crate::domain::packages::constants::PARENT;

/// What starts an import of a package's own `imports`.
pub(crate) const PACKAGE_IMPORT: char = '#';

/// What stands for any text in a pattern of `paths`.
pub(crate) const WILDCARD: char = '*';

/// The directory at the root of the repository.
pub(crate) const ROOT: &str = "";

/// How many configurations `extends` is followed through, at most: a ring of them ends.
pub(crate) const EXTENDS_DEPTH: usize = 16;

/// Why an import that names a file of the repository no file is makes the relation read in part, after the specifier.
pub(crate) const UNRESOLVED_CAUSE: &str = "names a file of the repository that no file is";
