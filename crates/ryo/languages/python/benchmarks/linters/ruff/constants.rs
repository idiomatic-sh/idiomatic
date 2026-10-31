//! The pinned tool's process protocol and boundary refusals.

pub(crate) const CHECK: &str = "check";
pub(crate) const ISOLATED: &str = "--isolated";
pub(crate) const OUTPUT_FORMAT: &str = "--output-format";
pub(crate) const JSON: &str = "json";
pub(crate) const CACHE_DIR: &str = "--cache-dir";
pub(crate) const CACHE_NAME: &str = "ruff";
pub(crate) const END_OPTIONS: &str = "--";
pub(crate) const SHOW_FILES: &str = "--show-files";
pub(crate) const CURRENT_DIRECTORY: &str = ".";
pub(crate) const WORKERS_ENV: &str = "RAYON_NUM_THREADS";
pub(crate) const ONE_WORKER: &str = "1";
pub(crate) const EMPTY_INPUT: &str = "empty input list would make Ruff walk the directory";
pub(crate) const DUPLICATE_INPUT: &str = "duplicate requested source file";
pub(crate) const DISCOVERED_OUTSIDE: &str = "discovered file outside corpus";
pub(crate) const NON_UTF8_PATH: &str = "non-UTF8 corpus path";
pub(crate) const EMPTY_DISCOVERY: &str = "empty discovered corpus";
pub(crate) const SELECT: &str = "--select";
pub(crate) const RULE_SEPARATOR: &str = ",";
