//! The names of the `language` concept: the file names the language reads, text that came from disk.

/// The endings of the paths of Python files: modules, stubs and the modules Windows runs without a console.
pub(crate) const EXTENSIONS: [&str; 3] = [".py", ".pyi", ".pyw"];

/// The directories the language never reads: virtual environments, bytecode caches (PEP 3147) and the tool caches and environments that ruff's default `exclude` lists (`.venv`, `venv`, `.tox`, `.nox`, `.eggs`, `.mypy_cache`, `.pytest_cache`, `.ruff_cache`).
pub(crate) const SKIPS: &[ryo::languages::SkippedDirectory] = &[
    ryo::languages::SkippedDirectory(".venv"),
    ryo::languages::SkippedDirectory("venv"),
    ryo::languages::SkippedDirectory("__pycache__"),
    ryo::languages::SkippedDirectory(".tox"),
    ryo::languages::SkippedDirectory(".nox"),
    ryo::languages::SkippedDirectory(".eggs"),
    ryo::languages::SkippedDirectory(".mypy_cache"),
    ryo::languages::SkippedDirectory(".pytest_cache"),
    ryo::languages::SkippedDirectory(".ruff_cache"),
];

/// What the language is called, in listings: the name of its directory.
pub(crate) const NAME: &str = "python";

/// The reasons the language declares for a name it cannot resolve statically, each named once: what a row's unknown value says.
pub(crate) const UNKNOWN_REASONS: &[ryo::languages::UnknownName] = &[
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(16),
        text: ryo::languages::ReasonName("wildcard-import"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(17),
        text: ryo::languages::ReasonName("dynamic-decorator"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(18),
        text: ryo::languages::ReasonName("dynamic-base"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(19),
        text: ryo::languages::ReasonName("star-base"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(20),
        text: ryo::languages::ReasonName("dynamic-callee"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(21),
        text: ryo::languages::ReasonName("interpolated-string"),
    },
    ryo::languages::UnknownName {
        reason: ryo::languages::UnknownReason(22),
        text: ryo::languages::ReasonName("unnamed-requirement"),
    },
];
