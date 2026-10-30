//! The words of the `explanations` concept: the headings and the fixed lines `explain` says.

/// The heading of the scopes a path lies in, or the workspace has.
pub const SCOPES: &str = "Scopes";

/// The heading of the directories a walk of the repository never goes into.
pub const SKIPPED: &str = "Skipped directories";

/// What the section says before it lists them: a skip is for the walk's own descent, not for what the workspace names.
pub const SKIPPED_NOTE: &str = "A root or an include the workspace names is read even inside one of these; only a directory the walk comes upon is skipped.";

/// What separates those who declare one directory skipped.
pub const DECLARERS: &str = ", ";

/// The heading of what places a path.
pub const PLACEMENT: &str = "Placement";

/// The heading of the rules that read a path.
pub const RULES: &str = "Rules";

/// The heading of a definition's params.
pub const PARAMS: &str = "Params";

/// The heading of what a definition holds.
pub const SHAPE: &str = "Shape";

/// How the workspace's own directory is shown.
pub const HERE: &str = crate::domain::paths::constants::CURRENT;

/// The command that writes what an entry places, as a line of `explain` shows it to be copied.
pub const SCAFFOLD: &str = "tm scaffold";

/// What a path no template entry places is told.
pub const UNPLACED: &str =
    "- placed by no template entry: in a closed directory this is a `#closed` finding";

/// What separates the lines of a section on a page.
pub const LINE_BREAK: &str = "\n";

/// What a param that says nothing of itself is said to mean.
pub const UNSAID: &str = "";

/// The heading of the topics `explain` answers.
pub const TOPICS_TITLE: &str = "Topics";

/// The heading of how each kind of asset is written.
pub const AUTHORING_TITLE: &str = "Authoring";

/// What opens a rule's query on its page.
pub const QUERY_OPEN: &str = "```ryoql";

/// What closes it.
pub const QUERY_CLOSE: &str = "```";

/// The noun for one governed file, in the line that counts what the workspace governs.
pub const FILE_ONE: &str = "file";

/// The noun for any other count of governed files.
pub const FILE_MANY: &str = "files";

/// The noun for one rule in the lint call, in the line that counts what the workspace governs.
pub const RULE_ONE: &str = "rule";

/// The noun for any other count of rules in the lint call.
pub const RULE_MANY: &str = "rules";
