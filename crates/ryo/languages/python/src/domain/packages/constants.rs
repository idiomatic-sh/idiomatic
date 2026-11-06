//! The names of the `packages` concept.

/// The file PEP 518 names for a Python project's tables, which several package managers write theirs in.
pub(crate) const PYPROJECT: &str = "pyproject.toml";

/// What ends the requirement's name and opens its marker.
pub(crate) const MARKER: char = ';';

/// What a normalized name joins its parts with.
pub(crate) const JOINER: char = '-';

/// What separates the parts of a name as it is written.
pub(crate) const PARTS: [char; 3] = ['-', '_', '.'];

/// A part of a path that goes up one directory.
pub(crate) const UP: &str = "..";

/// The directory of a manifest at the root of the repository.
pub(crate) const ROOT: &str = "";

/// What separates the clauses of a version requirement: `>=1,<2`.
pub(crate) const CLAUSES: char = ',';

/// What opens a PEP 440 clause that admits one version: `==1.2.3`, unless it ends in a wildcard.
pub(crate) const EQUAL: &str = "==";

/// What opens a PEP 440 clause of arbitrary equality, `===1.2.3`, which admits the one version it spells.
pub(crate) const ARBITRARY_EQUAL: &str = "===";

/// What ends a PEP 440 version that admits a family: `==1.2.*`.
pub(crate) const WILDCARD: char = '*';

/// What joins the alternatives of a Poetry constraint: `^1.2 || ^2.0`.
pub(crate) const ALTERNATIVES: &str = "||";

/// What opens the extras of a requirement: `name[extra]`.
pub(crate) const EXTRAS: char = '[';

/// What closes the extras of a requirement.
pub(crate) const EXTRAS_END: char = ']';

/// What a PEP 508 version requirement may be enclosed in: `name (>=1)`.
pub(crate) const PARENTHESES: [char; 2] = ['(', ')'];

/// What separates a manifest from the reason it was refused in a cause.
pub(crate) const CAUSE_SEPARATOR: &str = ": ";

/// The marker of a requirement that carries none.
pub(crate) const NO_MARKER: &str = "";

/// What opens the direct reference of a PEP 508 requirement: `name @ url`.
pub(crate) const DIRECT: char = '@';

/// What a URL of a git repository starts with, as pip and PEP 508 write it.
pub(crate) const GIT_SCHEME: &str = "git+";

/// What ends a URL's scheme, before the host it names.
pub(crate) const AUTHORITY: &str = "://";

/// What a URL of a local file or directory starts with.
pub(crate) const FILE_SCHEME: &str = "file:";

/// Why a requested manifest cannot be given a validation row: its file identity is beyond the catalog.
pub(crate) const FILE_IDENTITY_EXCEEDS_CATALOG: &str =
    "requested manifest file identity exceeds the catalog range";

/// What precedes the path of a requested manifest the metadata validator did not observe.
pub(crate) const VALIDATOR_DID_NOT_OBSERVE: &str = "requested metadata validator did not observe ";
