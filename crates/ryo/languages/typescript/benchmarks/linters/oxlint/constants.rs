//! The pinned tool's process protocol and boundary refusals.

/// Read the configuration file named next instead of any the working directory holds.
pub(crate) const CONFIG: &str = "-c";
/// The configuration file a run writes into its own state: a project's `oxlint.config.ts`, which this standalone executable
/// cannot load, or `.oxlintrc.json`, is never read.
pub(crate) const CONFIG_NAME: &str = "oxlint.json";
/// The configuration that file holds: nothing, so only the command line's rules run.
pub(crate) const EMPTY_CONFIG: &str = "{}";
/// Allow the rules or category named next: with `all`, every rule starts off.
pub(crate) const ALLOW: &str = "-A";
/// Every category, so that only the denied rules run.
pub(crate) const ALL: &str = "all";
/// Deny the rule named next, as `<plugin>/<rule>`.
pub(crate) const DENY: &str = "-D";
/// Select the diagnostic wire representation.
pub(crate) const FORMAT: &str = "--format";
/// Machine-readable oxlint diagnostic format.
pub(crate) const JSON: &str = "json";
/// Set the number of linting threads.
pub(crate) const THREADS: &str = "--threads";
/// Pin the reference to one worker for comparable execution.
pub(crate) const ONE_WORKER: &str = "1";
/// Read no configuration file nested below the working directory.
pub(crate) const NO_NESTED_CONFIG: &str = "--disable-nested-config";
/// Separate explicit input paths from options.
pub(crate) const END_OPTIONS: &str = "--";
/// What separates the plugin from the rule in a rule directory id: `eslint.no-debugger`.
pub(crate) const ID_SEPARATOR: &str = ".";
/// What separates the plugin from the rule in a `-D` argument: `eslint/no-debugger`.
pub(crate) const PLUGIN_SEPARATOR: &str = "/";
/// What the pinned executable's `--version` prints before its version.
pub(crate) const VERSION_PREFIX: &str = "Version: ";
/// The rule a diagnostic without a code, a parse error, is reported under.
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
/// Refusal preventing an empty explicit selection from becoming directory discovery.
pub(crate) const EMPTY_INPUT: &str = "empty input list would make oxlint walk the directory";
/// Refusal of a request carrying rule configuration, which this adapter does not project.
pub(crate) const UNSUPPORTED_RULE_CONTEXT: &str = "oxlint adapter projects no rule configuration";
/// Refusal for duplicated requested source paths.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate requested source file";
/// Refusal of a code that is not `<plugin>(<rule>)`.
pub(crate) const UNREADABLE_CODE: &str = "oxlint code is not <plugin>(<rule>):";
/// Refusal of a rule directory id that is not `<plugin>.<rule>`.
pub(crate) const UNREADABLE_RULE: &str = "rule id is not <plugin>.<rule>:";
/// Refusal of a code the run did not select.
pub(crate) const UNEXPECTED_RULE: &str = "unexpected rule";
/// Refusal of a diagnostic in a file the run was not given.
pub(crate) const UNREQUESTED_FILE: &str = "oxlint returned an unrequested file";
/// Refusal of a diagnostic that names no span.
pub(crate) const UNLABELLED_DIAGNOSTIC: &str = "oxlint diagnostic has no labelled span";
/// The label key of a span's first UTF-8 byte.
pub(crate) const SPAN_OFFSET: &str = "offset";
/// The label key of a span's length in UTF-8 bytes.
pub(crate) const SPAN_LENGTH: &str = "length";
/// Refusal of a label span without one of its numbers.
pub(crate) const SPAN_FIELD_MISSING: &str = "oxlint label span has no";
/// Refusal of a span whose end is past the largest offset.
pub(crate) const SPAN_OVERFLOW: &str = "oxlint span overflows";
/// Refusal of an exit status the diagnostics contradict.
pub(crate) const CONTRADICTORY_STATUS: &str = "oxlint exit status contradicts findings";
/// Refusal of a run that linted another number of files than it was given.
pub(crate) const FILES_NOT_LINTED: &str = "oxlint did not lint exactly the requested files";
/// Refusal of a run that enabled another number of rules than it was given: a misspelled rule enables none.
pub(crate) const RULES_NOT_ENABLED: &str = "oxlint did not enable exactly the selected rules";
/// Why the report has no execution accounting.
pub(crate) const NO_ACCOUNTING: &str = "oxlint JSON format has no rule/file execution accounting";
