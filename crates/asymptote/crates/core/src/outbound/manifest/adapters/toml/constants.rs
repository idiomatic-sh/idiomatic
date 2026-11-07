//! The names the manifest's file uses.

/// The file.
pub(super) const FILE: &str = "asymptote.toml";
/// The version of asymptote that runs, which a workspace root's `asymptote` must name.
pub(super) const INSTALLED: &str = env!("CARGO_PKG_VERSION");
/// What every `asymptote.toml` asymptote writes starts with.
pub(super) const HEADER: &str = "# The tools asymptote runs, each in a group, a label of your choosing.\n# See `asymptote check --help`.\n\n";
/// The `output` a SARIF tool names.
pub(super) const SARIF: &str = "sarif";
/// The `output` a Bencher Metric Format tool names.
pub(super) const BMF: &str = "bmf";
/// The inputs of a tool that names none: the product itself.
pub(super) const PRODUCT: &str = ".";
/// How long a tool that names no timeout may run.
pub(super) const TIMEOUT: u64 = 600;
/// The segment of a path that names the directory it is in.
pub(super) const CURRENT: &str = ".";
/// Why a member that matches no directory is refused.
pub(super) const NO_DIRECTORY: &str = "it is no directory, or its glob matches none";
/// Why a member with no manifest is refused.
pub(super) const NO_MANIFEST: &str = "it has no asymptote.toml";
/// Why a member that names members is refused.
pub(super) const NESTED: &str = "it declares `members`, and only the workspace root may";
/// Why a member that maps hooks is refused.
pub(super) const NESTED_HOOKS: &str = "it declares `hooks`, and only the workspace root may";
/// Why a hook that maps a group no tool declares is refused.
pub(super) const UNDECLARED: &str = "no tool of the workspace is in the group";
/// What joins the words of a reason.
pub(super) const SPACE: &str = crate::domain::hooks::constants::SPACE;
/// What a refusal of two tools of one id starts with, before the id.
pub(super) const TWICE: &str = "two tools are named `";
/// What it ends with, after the id.
pub(super) const TWICE_END: &str = "`";
/// The key of a tool's command line.
pub(super) const RUN_KEY: &str = "run";
/// The key of a tool's fix.
pub(super) const FIX_KEY: &str = "fix";
/// What a refusal of a captured tool whose inputs match nothing starts with, before the tool's id.
pub(super) const UNBOUND: &str = "the captured tool `";
/// What follows the id, before its inputs.
pub(super) const UNBOUND_INPUTS: &str = "` matches nothing on disk: its inputs ";
/// Why an entry whose name is no text is refused, before the name.
pub(super) const NOT_TEXT: &str = "an entry's name is no UTF-8 text: ";
/// What joins the inputs a refusal names.
pub(super) const INPUTS_JOIN: &str = ", ";
