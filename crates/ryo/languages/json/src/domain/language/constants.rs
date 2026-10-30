//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The ending of the paths of json files.
pub(crate) const EXTENSION: &str = ".json";

/// The ending of the paths of files written as json with comments.
pub(crate) const COMMENTED_EXTENSION: &str = ".jsonc";

/// The ending of the paths of VS Code workspace files, which are json with comments.
pub(crate) const WORKSPACE_EXTENSION: &str = ".code-workspace";

/// The ending of the paths of web app manifests, which are json as the standard defines it.
pub(crate) const MANIFEST_EXTENSION: &str = ".webmanifest";

/// The files that are json with comments though their names end in no `.json`: the configurations of Babel, `ESLint` and swc.
pub(crate) const COMMENTED_DOTFILES: [&str; 3] = [".babelrc", ".eslintrc", ".swcrc"];

/// The beginnings of the names of the project configurations TypeScript and VS Code read as json with comments though their names
/// end in `.json`: `tsconfig.json` and `jsconfig.json`, and the configurations that extend them, as VS Code's file patterns
/// `tsconfig.*.json` and `jsconfig.*.json` (and the same with a hyphen) name them.
pub(crate) const COMMENTED_PREFIXES: [&str; 4] =
    ["tsconfig.", "tsconfig-", "jsconfig.", "jsconfig-"];

/// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
/// ecosystem. None: RFC 8259 defines a data format and no package, environment or build directory, so every directory may hold json.
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "json";
