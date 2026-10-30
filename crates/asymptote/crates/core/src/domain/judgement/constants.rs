//! How much of a failed tool's stderr is kept.

/// The lines of stderr a failure keeps.
pub const TAIL_LINES: usize = 20;
/// What joins the lines kept.
pub const NEWLINE: &str = "\n";
/// The word a violation the lock accepts is said with, and the name of the records an operation accepts.
pub const ACCEPTED: &str = "accepted";
/// The word a record some of whose violations no longer occur is said with.
pub const RESOLVED: &str = "resolved";
/// Why a tool's output that is empty is no report: no log, no measurement.
pub const NOTHING: &str = "the tool printed nothing";
