//! The pinned tool's process protocol and boundary refusals.

/// A measured or compared run: every clippy lint off, each group a project's manifest could turn on included, then the selected
/// ones (`-W clippy::<lint>`, appended). The command line comes after the manifest's `[lints]` flags, so it outranks them.
pub(crate) const MEASURED_RUN: [&str; 16] = [
    CLIPPY,
    WORKSPACE,
    ALL_TARGETS,
    MESSAGE_FORMAT_JSON,
    OFFLINE,
    END_OPTIONS,
    ALLOW,
    ALL,
    ALLOW,
    PEDANTIC,
    ALLOW,
    NURSERY,
    ALLOW,
    RESTRICTION,
    ALLOW,
    CARGO_GROUP,
];
/// The discovery run: every clippy lint off, so it only fetches, checks and writes each crate's dependency file.
pub(crate) const DISCOVERY_RUN: [&str; 16] = [
    CLIPPY,
    WORKSPACE,
    ALL_TARGETS,
    MESSAGE_FORMAT_JSON,
    LOCKED,
    END_OPTIONS,
    ALLOW,
    ALL,
    ALLOW,
    PEDANTIC,
    ALLOW,
    NURSERY,
    ALLOW,
    RESTRICTION,
    ALLOW,
    CARGO_GROUP,
];
/// The subcommand the clippy command binary expects first, as the package manager passes it.
const CLIPPY: &str = "clippy";
/// The clippy command binary of the pinned toolchain, beside the verified `clippy-driver`.
pub(crate) const CLIPPY_COMMAND: &str = "cargo-clippy";
/// The package manager binary of the pinned toolchain.
pub(crate) const MANAGER: &str = "cargo";
/// The compiler binary of the pinned toolchain.
pub(crate) const RUSTC: &str = "rustc";
/// The variable naming the package manager the clippy command runs.
pub(crate) const MANAGER_VARIABLE: &str = "CARGO";
/// The variable naming the compiler the package manager runs.
pub(crate) const RUSTC_VARIABLE: &str = "RUSTC";
/// The variable naming the target directory every root shares.
pub(crate) const TARGET_VARIABLE: &str = "CARGO_TARGET_DIR";
/// The variable naming the directory clippy reads `clippy.toml` from, in place of the project's.
pub(crate) const CONF_VARIABLE: &str = "CLIPPY_CONF_DIR";
/// The variable pinning the package manager's parallel jobs.
pub(crate) const JOBS_VARIABLE: &str = "CARGO_BUILD_JOBS";
/// One job and one worker, for comparable execution.
pub(crate) const ONE_JOB: &str = "1";
/// Every package of the root's workspace.
const WORKSPACE: &str = "--workspace";
/// Every target: libraries, binaries, tests, benches, examples and build scripts.
const ALL_TARGETS: &str = "--all-targets";
/// One JSON message per line on stdout.
const MESSAGE_FORMAT_JSON: &str = "--message-format=json";
/// Refuse a lock change but fetch what the lock names: discovery, the one run that may reach the network.
const LOCKED: &str = "--locked";
/// Reach no network: every measured and compared run, after discovery fetched what the lock names.
const OFFLINE: &str = "--offline";
/// What separates the package manager's options from clippy's.
const END_OPTIONS: &str = "--";
/// Allow the lint or group named next.
const ALLOW: &str = "-A";
/// Every default clippy lint, so that only the warned ones run.
const ALL: &str = "clippy::all";
/// The pedantic group, off by default; a project's manifest may turn its lints on.
const PEDANTIC: &str = "clippy::pedantic";
/// The nursery group, off by default; a project's manifest may turn its lints on.
const NURSERY: &str = "clippy::nursery";
/// The restriction group, off by default; a project's manifest may turn its lints on (clap's `dbg_macro`).
const RESTRICTION: &str = "clippy::restriction";
/// The cargo group, off by default; a project's manifest may turn its lints on.
const CARGO_GROUP: &str = "clippy::cargo";
/// Warn on the lint named next.
pub(crate) const WARN: &str = "-W";
/// What a clippy code starts with: `clippy::needless_return`.
pub(crate) const LINT_PREFIX: &str = "clippy::";
/// The line reason that carries a compiler diagnostic.
pub(crate) const COMPILER_MESSAGE: &str = "compiler-message";
/// The level of a diagnostic that stops the build.
pub(crate) const ERROR_LEVEL: &str = "error";
/// The exit status of a build a compiler error stopped.
pub(crate) const BUILD_FAILED_STATUS: i32 = 101;
/// The directory of the target directory holding one directory for each package the check built, and in it one for each unit:
/// `debug/build/<package>/<unit>/out/<crate>-<unit>.d`, the layout of cargo 1.100.
pub(crate) const DEP_INFO_DIRECTORY: &str = "debug/build";
/// The directory of a unit holding its dependency file.
pub(crate) const UNIT_OUTPUT_DIRECTORY: &str = "out";
/// The extension of a dependency file.
pub(crate) const DEP_INFO_EXTENSION: &str = "d";
/// What separates a dependency file rule's target from its prerequisites.
pub(crate) const RULE_SEPARATOR: &str = ": ";
/// How a dependency file escapes a space inside a path, which otherwise separates two prerequisites.
pub(crate) const ESCAPED_SPACE: &str = "\\ ";
/// What stands for an escaped space while the prerequisites are split, a character no path holds.
pub(crate) const SPACE_PLACEHOLDER: &str = "\0";
/// A space, as the escaped one reads in its path.
pub(crate) const SPACE: &str = " ";
/// The extension of the sources discovery keeps.
pub(crate) const SOURCE_EXTENSION: &str = "rs";
/// The manifest a case root is given when it has none.
pub(crate) const MANIFEST_NAME: &str = "Cargo.toml";
/// The package name of a case's crate.
pub(crate) const CASE_PACKAGE: &str = "case";
/// The version of a case's crate.
pub(crate) const CASE_VERSION: &str = "0.0.0";
/// The edition of a case's crate.
pub(crate) const CASE_EDITION: &str = "2024";
/// What the verified executable's `--version` prints before its version.
pub(crate) const VERSION_PREFIX: &str = "clippy ";
/// What it prints after: the commit and date of the pinned toolchain's build.
pub(crate) const VERSION_STAMP: &str = " (0ef1bc3860 2026-11-09)";
/// The rule a parse error, a diagnostic without a code that stops the build, is reported under.
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
/// Refusal preventing an empty explicit selection.
pub(crate) const EMPTY_INPUT: &str = "empty input list names no source clippy should read";
/// Refusal of a request carrying rule configuration, which this adapter does not project.
pub(crate) const UNSUPPORTED_RULE_CONTEXT: &str = "clippy adapter projects no rule configuration";
/// Refusal of a code that is not `clippy::<lint>`.
pub(crate) const UNREADABLE_CODE: &str = "clippy code is not clippy::<lint>:";
/// Refusal of a dependency file without a rule.
pub(crate) const UNREADABLE_DEP_INFO: &str = "dependency file has no rule";
/// The code rustc gives a lint the command line names and no tool has: a misspelled `-W clippy::<lint>`.
pub(crate) const UNKNOWN_SELECTED_LINT: &str = "E0602";
/// The code rustc gives a lint a source attribute names and no tool has.
pub(crate) const UNKNOWN_ATTRIBUTE_LINT: &str = "unknown_lints";
/// Refusal of a run that named a lint clippy does not have.
pub(crate) const UNKNOWN_LINT: &str = "clippy has no lint named";
/// Refusal of a build that failed with no compiler error to report.
pub(crate) const FAILED_WITHOUT_ERROR: &str = "the build failed with no compiler error";
/// Refusal of a compiler error other than a parse error: a case must compile, or clippy's later lints never run.
pub(crate) const DOES_NOT_COMPILE: &str = "the root does not compile";
/// Refusal of a diagnostic in a file the run was not given.
pub(crate) const UNREQUESTED_FILE: &str = "clippy returned an unrequested file";
/// Refusal of a diagnostic that names no primary span.
pub(crate) const UNLABELLED_DIAGNOSTIC: &str = "clippy diagnostic has no primary span";
/// Refusal of a discovery run that did not finish.
pub(crate) const DISCOVERY_FAILED: &str = "clippy discovery run failed";
/// Refusal of a discovery that found no source under the root.
pub(crate) const NOTHING_DISCOVERED: &str = "clippy discovery found no source under the root";
/// Why the report has no execution accounting.
pub(crate) const NO_ACCOUNTING: &str =
    "clippy JSON messages have no rule/file execution accounting";
