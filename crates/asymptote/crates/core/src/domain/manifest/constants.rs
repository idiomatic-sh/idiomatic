//! The words of the manifest's closed sets.

/// The git hooks a client runs, which `[hooks]` may name.
pub const HOOKS: [&str; 19] = [
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-auto-gc",
    "post-rewrite",
    "post-index-change",
    "sendemail-validate",
    "fsmonitor-watchman",
    "p4-pre-submit",
    "post-update",
];
/// The hook, and its group, `asymptote init` writes into the `asymptote.toml` it creates.
pub const DEFAULT_HOOK: &str = "pre-commit";
/// The group the default hook runs.
pub const DEFAULT_GROUP: &str = "lint";
/// What joins the groups when they are listed.
pub const LISTED_WITH: &str = ", ";
/// The manifest's file, at the product's root: a check records it beside its inputs, so `up` and `down` refuse once it
/// moved.
pub const MANIFEST: &str = "asymptote.toml";
/// The path a workspace root's own product is printed under, from the root itself.
pub const ROOT_PATH: &str = ".";
