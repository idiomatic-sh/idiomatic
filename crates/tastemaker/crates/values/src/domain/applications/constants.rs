//! The ids and the built-in query of the applications.

/// What joins a placed rule's definition id and the digest of what it is placed with, when one definition is placed more
/// than once: the lint call's rule id is the definition's own.
pub const PLACING_SEPARATOR: &str = "~";
/// The prefix of a condition's id in the lint call: the scope's directory and the condition's name follow.
pub const CONDITION_PREFIX: &str = "condition:";
/// The id of the built-in rule that reads the comments of the files that may hold an ignore.
pub const COMMENTS: &str = "tastemaker-comments";
/// The built-in rule: every comment, with its span, whether it is documentation, and its text, so the lines that hold code
/// can be told from those that hold only comments, a finding shown past the documentation its node starts with, and an
/// ignore read.
pub const COMMENTS_QUERY: &str = r#"let comments = select * from "comment";
emit comments subject "file" key("span", "doc", "text");
"#;
/// The relation the host declares, the placed files.
pub const PLACED: &str = "tastemaker.placed";
/// The reserved param of a rule with exceptions.
pub const EXCEPTIONS: &str = "exceptions";
/// The separator of a span's offsets, as the engine writes a span.
pub const SPAN_SEPARATOR: &str = "..";
/// The glob over everything beneath the analysis root.
pub const EVERYTHING: &str = "**";

/// The most globs one rule handed whole to ryo carries: more cost ryo more to match than matching the files here.
pub const CARRIED_GLOBS: usize = 32;

/// The id of the built-in rule that reads the syntax the contracts compare.
pub const DECLARATIONS: &str = "declarations";

/// What separates the segments of a path.
pub const PATH_SEPARATOR: &str = "/";

/// The checks every rule a workspace applies runs, besides its own findings: `#unknown` and `#coverage`. The rule itself is
/// evaluated only where it reaches a file.
pub const EVALUATED_CHECKS: [&str; 2] = ["unknown", "coverage"];
