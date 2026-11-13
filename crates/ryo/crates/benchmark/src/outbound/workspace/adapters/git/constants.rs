//! Git command protocol, ownership marker and refusal vocabulary.
/// Executable used for the Git repository protocol.
pub(super) const GIT: &str = "git";
/// Introduce one command-local Git configuration override.
pub(super) const CONFIG: &str = "-c";
/// Disable repository hooks during automated corpus operations.
pub(super) const DISABLE_HOOKS: &str = "core.hooksPath=/dev/null";
/// Permit explicitly configured local repositories used by contract fixtures.
pub(super) const LOCAL_TRANSPORT: &str = "protocol.file.allow=always";
/// Environment variable controlling interactive credential prompts.
pub(super) const PROMPT: &str = "GIT_TERMINAL_PROMPT";
/// Disable terminal prompting for unattended repository operations.
pub(super) const NO_PROMPT: &str = "0";
/// Refusal when patch input cannot reach the child process.
pub(super) const MISSING_STDIN: &str = "git stdin missing";
/// Inspect checkout cleanliness.
pub(super) const STATUS_COMMAND: &str = "status";
/// Request stable machine-readable status output.
pub(super) const PORCELAIN: &str = "--porcelain";
/// Include every untracked file in the cleanliness check.
pub(super) const UNTRACKED: &str = "--untracked-files=all";
/// Refusal protecting existing checkout edits from overwrite.
pub(super) const DIRTY: &str = "corpus is dirty; refusing to overwrite changes";
/// Resolve the current revision to its immutable object identity.
pub(super) const REV_PARSE: &str = "rev-parse";
/// Revision name for the currently checked-out commit.
pub(super) const HEAD: &str = "HEAD";
/// Refusal when checkout identity differs from the prepared handle.
pub(super) const HEAD_CHANGED: &str = "corpus HEAD changed";
/// Refusal for unsafe project names or non-commit revision pins.
pub(super) const UNPINNED: &str = "project requires a safe name and full commit revision";
/// Initialize a newly owned repository directory.
pub(super) const INIT: &str = "init";
/// Suppress routine Git progress output.
pub(super) const QUIET: &str = "--quiet";
/// Inspect or configure the source repository remote.
pub(super) const REMOTE: &str = "remote";
/// Add the configured source remote to a new repository.
pub(super) const ADD: &str = "add";
/// Remote name reserved for the pinned source repository.
pub(super) const ORIGIN: &str = "origin";
/// Fetch the explicitly pinned source revision.
pub(super) const FETCH: &str = "fetch";
/// Limit repository history to the requested commit.
pub(super) const SHALLOW: &str = "--depth=1";
/// Materialize the pinned revision in the working tree.
pub(super) const CHECKOUT: &str = "checkout";
/// Keep the checkout detached from mutable branch names.
pub(super) const DETACH: &str = "--detach";
/// Private marker recording the source repository of an owned cache clone.
pub(super) const OWNER: &str = ".git/ryo-benchmark-owner";
/// Refusal when a cache directory lacks the expected ownership marker.
pub(super) const UNOWNED: &str = "cache is not this benchmark's clone";
/// Read the configured remote URL for identity verification.
pub(super) const GET_URL: &str = "get-url";
/// Refusal when the remote differs from the pinned project repository.
pub(super) const REMOTE_CHANGED: &str = "corpus remote changed";
/// List tracked paths selected as corpus inputs.
pub(super) const LS_FILES: &str = "ls-files";
/// Separate paths with NUL bytes without quoting or line ambiguity.
pub(super) const NUL_PATHS: &str = "-z";
/// Refusal for source paths that cannot enter the text-based benchmark contract.
pub(super) const NON_UTF8: &str = "non-UTF8 corpus path";
/// Refusal when the repository contains no tracked corpus paths.
pub(super) const EMPTY: &str = "empty corpus";
/// Refusal when a handle already records an applied edit.
pub(super) const ALREADY_PATCHED: &str = "patch already applied";
/// Apply or validate patch bytes received on standard input.
pub(super) const APPLY: &str = "apply";
/// Inspect structural patch changes before accepting an edit.
pub(super) const SUMMARY: &str = "--summary";
/// Read patch bytes from standard input rather than a filesystem path.
pub(super) const STDIN: &str = "-";
/// Inspect changed-file counts to reject an empty edit.
pub(super) const NUMSTAT: &str = "--numstat";
/// Refusal for edits that create, delete, or structurally replace tracked paths.
pub(super) const INVALID_PATCH: &str = "edit patch must change existing tracked files only";
/// Validate patch applicability without changing the checkout.
pub(super) const CHECK: &str = "--check";
/// Capture tracked changes against the pinned commit.
pub(super) const DIFF: &str = "diff";
/// Preserve binary changes in the reversible patch representation.
pub(super) const BINARY: &str = "--binary";
/// Refusal when the applied patch changes no tracked source bytes.
pub(super) const NO_CHANGE: &str = "patch makes no tracked source edit";
/// Refusal protecting edits made after the benchmark applied its own patch.
pub(super) const EDITED: &str = "corpus changed after patch; refusing restore";
/// Undo only the exact patch retained by the current workspace handle.
pub(super) const REVERSE: &str = "--reverse";
