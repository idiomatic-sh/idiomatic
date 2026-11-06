//! The pinned tool's process protocol and boundary refusals.

/// Run Ruff lint diagnostics.
pub const CHECK: &str = "check";
/// Disable ambient project and user configuration.
pub const ISOLATED: &str = "--isolated";
/// Ordered import search roots used by the pinned isolated Ruff invocation.
pub const ISOLATED_PROJECT_ROOTS: [&str; 2] = ["", "src"];
/// Refusal when an explicit host project scope differs from Ruff's isolated scope.
pub const UNSUPPORTED_PROJECT_ROOTS: &str =
    "reference profile cannot match the declared project search roots";
/// Select the diagnostic wire representation.
pub const OUTPUT_FORMAT: &str = "--output-format";
/// Machine-readable Ruff diagnostic format.
pub const JSON: &str = "json";
/// Supply an explicit retained reference cache directory.
pub const CACHE_DIR: &str = "--cache-dir";
/// Reference-specific state directory below one observation.
pub const CACHE_NAME: &str = "ruff";
/// Separate explicit input paths from options.
pub const END_OPTIONS: &str = "--";
/// Ask Ruff to enumerate its selected source files.
pub const SHOW_FILES: &str = "--show-files";
/// Discovery root relative to the explicit working directory.
pub const CURRENT_DIRECTORY: &str = ".";
/// Environment control for Rayon worker count.
pub const WORKERS_ENV: &str = "RAYON_NUM_THREADS";
/// Pin the reference to one worker for comparable execution.
pub const ONE_WORKER: &str = "1";
/// Refusal preventing an empty explicit selection from becoming directory discovery.
pub const EMPTY_INPUT: &str = "empty input list would make Ruff walk the directory";
/// Refusal for duplicated requested source paths.
pub const DUPLICATE_INPUT: &str = "duplicate requested source file";
/// Refusal when discovery escapes the corpus root.
pub const DISCOVERED_OUTSIDE: &str = "discovered file outside corpus";
/// Refusal for paths outside the text-based benchmark contract.
pub const NON_UTF8_PATH: &str = "non-UTF8 corpus path";
/// Refusal when discovery supplies no source files.
pub const EMPTY_DISCOVERY: &str = "empty discovered corpus";
/// Select the exact compared rule set.
pub const SELECT: &str = "--select";
/// Pin syntax and target-gated diagnostics to the prepared Python 3.10 oracle.
pub const TARGET_VERSION: &str = "--target-version";
/// Separator in the reference rule-selection argument.
pub const RULE_SEPARATOR: &str = ",";
/// Text `linux`.
pub const LINUX: &str = "linux";
/// Text `/proc/sys/kernel/osrelease`.
pub const PROC_SYS_KERNEL_OSRELEASE: &str = "/proc/sys/kernel/osrelease";
/// Text `/proc/version`.
pub const PROC_VERSION: &str = "/proc/version";
/// Text `microsoft`.
pub const MICROSOFT: &str = "microsoft";
/// Text `/.dockerenv`.
pub const DOCKERENV: &str = "/.dockerenv";
/// Text `/run/.containerenv`.
pub const RUN_CONTAINERENV: &str = "/run/.containerenv";
/// Text `/proc/self/cgroup`.
pub const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
/// Text `docker`.
pub const DOCKER: &str = "docker";
/// Text `/container`.
pub const CONTAINER: &str = "/container";
/// Text `/lxc`.
pub const LXC: &str = "/lxc";
/// Text `kubepods`.
pub const KUBEPODS: &str = "kubepods";
