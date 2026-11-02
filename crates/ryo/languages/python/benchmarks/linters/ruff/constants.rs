//! The pinned tool's process protocol and boundary refusals.

/// Run Ruff lint diagnostics.
pub(crate) const CHECK: &str = "check";
/// Disable ambient project and user configuration.
pub(crate) const ISOLATED: &str = "--isolated";
/// Select the diagnostic wire representation.
pub(crate) const OUTPUT_FORMAT: &str = "--output-format";
/// Machine-readable Ruff diagnostic format.
pub(crate) const JSON: &str = "json";
/// Supply an explicit retained reference cache directory.
pub(crate) const CACHE_DIR: &str = "--cache-dir";
/// Reference-specific state directory below one observation.
pub(crate) const CACHE_NAME: &str = "ruff";
/// Separate explicit input paths from options.
pub(crate) const END_OPTIONS: &str = "--";
/// Ask Ruff to enumerate its selected source files.
pub(crate) const SHOW_FILES: &str = "--show-files";
/// Discovery root relative to the explicit working directory.
pub(crate) const CURRENT_DIRECTORY: &str = ".";
/// Environment control for Rayon worker count.
pub(crate) const WORKERS_ENV: &str = "RAYON_NUM_THREADS";
/// Pin the reference to one worker for comparable execution.
pub(crate) const ONE_WORKER: &str = "1";
/// Refusal preventing an empty explicit selection from becoming directory discovery.
pub(crate) const EMPTY_INPUT: &str = "empty input list would make Ruff walk the directory";
/// Refusal for duplicated requested source paths.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate requested source file";
/// Refusal when discovery escapes the corpus root.
pub(crate) const DISCOVERED_OUTSIDE: &str = "discovered file outside corpus";
/// Refusal for paths outside the text-based benchmark contract.
pub(crate) const NON_UTF8_PATH: &str = "non-UTF8 corpus path";
/// Refusal when discovery supplies no source files.
pub(crate) const EMPTY_DISCOVERY: &str = "empty discovered corpus";
/// Select the exact compared rule set.
pub(crate) const SELECT: &str = "--select";
/// Pin syntax and target-gated diagnostics to the prepared Python 3.10 oracle.
pub(crate) const TARGET_VERSION: &str = "--target-version";
/// Project grammar errors and syntax validation that has no dedicated rule.
/// Future directives parse into a complete AST and retain their `syntax-error`
/// facts for feature gating, while F404/F407 consume their directive validation
/// diagnostics. Other validation errors, such as invalid `nonlocal` and
/// annotation expressions, continue to project as generic invalid syntax.
pub(crate) const FRONTEND_DIAGNOSTIC_QUERY: &str = "let diagnostics = select id, file, span from \"syntax-error\" where (phase = \"parse\" and (impact = \"incomplete\" or impact = \"lexically-complete\" or impact = \"preserved\")) or (phase = \"validation\" and impact = \"preserved\" and code != \"future-feature-not-defined\" and code != \"future-directive-invalid\");\nemit diagnostics subject file key(id);\n";
/// Separator in the reference rule-selection argument.
pub(crate) const RULE_SEPARATOR: &str = ",";
