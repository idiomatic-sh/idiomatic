//! What a tool's process is given and held to.

use std::time::Duration;

/// Why an empty command could not start.
pub(super) const EMPTY: &str = "the command is empty";

/// How often a running child is looked at.
pub(super) const POLL: Duration = Duration::from_millis(10);

/// The most a tool may write to one stream: one byte past it shows the overflow.
pub(super) const MAX_OUTPUT: u64 = 1 << 30;

/// Variables no tool sees. A tool inherits asymptote's environment, which asymptote never reads, so `PATH`, `HOME`
/// and the toolchains' homes reach it; these would instead change what it reports, so a laptop and CI disagree about one
/// commit, or swap the gate's own program. Cargo's own and a pinned toolchain go too: a tool runs the toolchain the
/// repository selects.
pub(super) const REMOVED: [&str; 14] = [
    "CARGO",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CLICOLOR_FORCE",
    "CLIPPY_CONF_DIR",
    "FORCE_COLOR",
    "RUFF_CONFIG",
    "RUSTC",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTC_WRAPPER",
    "RUSTDOCFLAGS",
    "RUSTFLAGS",
    "RUSTFMT",
    "RUSTUP_TOOLCHAIN",
];

/// Variables every tool gets: machine-readable output with no escape sequences, in a locale that never reorders or
/// translates a message asymptote keys on.
pub(super) const SET: [(&str, &str); 3] = [("LANG", "C"), ("LC_ALL", "C"), ("NO_COLOR", "1")];

/// The variable that lists the directories a program is found in.
#[cfg(windows)]
pub(super) const PATH: &str = "PATH";

/// The extensions Windows runs a program by, in the order its default `PATHEXT` lists them.
#[cfg(windows)]
pub(super) const EXTENSIONS: [&str; 4] = [".com", ".exe", ".bat", ".cmd"];
