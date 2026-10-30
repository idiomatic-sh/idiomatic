//! The words of the `closing` concept: the relations tm hosts for the closed query, their columns, and the query that decides which
//! files of a closed directory are strays and the scope each is named after.

/// The id of the closed query.
pub const CLOSED_RULE: &str = "closed-stray";

/// Each closed directory: `group`, `prefix` and `last`.
pub const CLOSED_DIR: &str = "tastemaker.closed-dir";
/// Each file beneath a closed directory: `stray`, `group`, `path` and `home`.
pub const CLOSED_UNDER: &str = "tastemaker.closed-under";
/// Each file a template placed: `path`.
pub const PLACED_FILE: &str = "tastemaker.placed-file";
/// Each `tastemaker.toml`, which no entry places: `path`.
pub const OWN_FILE: &str = "tastemaker.own-file";
/// Each directory of an instance nested in a closed directory: `group` and `prefix`.
pub const NESTED_DIR: &str = "tastemaker.nested-dir";
/// Each directory a closed scope placed files in: `group`, `owner`, `prefix` and `depth`.
pub const OCCUPIED: &str = "tastemaker.occupied";

/// The number a closed directory has among the closed directories, from zero, in the order of their paths.
pub const GROUP: &str = "group";
/// A directory as a prefix of the paths beneath it: the path and a `/`, none for the root.
pub const PREFIX: &str = "prefix";
/// The scope that is named when no scope placed files in or above a stray's directory.
pub const LAST: &str = "last";
/// The number a file beneath a closed directory has among those of every closed directory.
pub const STRAY: &str = "stray";
/// A file's path from the root of the workspace.
pub const PATH: &str = "path";
/// The directory that holds a file, as a prefix.
pub const HOME: &str = "home";
/// The scope that placed files in a directory.
pub const OWNER: &str = "owner";
/// How many directories a directory is below the root.
pub const DEPTH: &str = "depth";

/// The files a closed directory holds that no template places, no `tastemaker.toml` is and no nested instance owns, each with the
/// scope it is named after: the scope that placed files in the nearest directory above it, the last declared of those on a tie, or
/// the last declared of the directory's when none did.
pub const CLOSED_QUERY: &str = r#"let under = select * from "tastemaker.closed-under";
let unplaced = antijoin(left=under, right="tastemaker.placed-file", left-on=["path"], right-on=["path"]);
let unowned = antijoin(left=unplaced, right="tastemaker.own-file", left-on=["path"], right-on=["path"]);
let within = select u."stray" as "stray" from unowned as u join "tastemaker.nested-dir" as n on u."group" = n."group" where starts_with(u."path", n."prefix");
let strays = antijoin(left=unowned, right=within, left-on=["stray"], right-on=["stray"]);
let near = select s."stray" as "stray", o."owner" as "owner", o."depth" as "depth" from strays as s join "tastemaker.occupied" as o on s."group" = o."group" where starts_with(s."home", o."prefix");
let deeper = select a."stray" as "stray", a."owner" as "owner", a."depth" as "depth" from near as a join near as b on a."stray" = b."stray" where b."depth" > a."depth";
let nearest = antijoin(left=near, right=deeper, left-on=["stray", "owner", "depth"], right-on=["stray", "owner", "depth"]);
let later = select a."stray" as "stray", a."owner" as "owner" from nearest as a join nearest as b on a."stray" = b."stray" where b."owner" > a."owner";
let innermost = antijoin(left=nearest, right=later, left-on=["stray", "owner"], right-on=["stray", "owner"]);
let found = project(from=innermost, columns=["stray", "owner"]);
let ownerless = antijoin(left=strays, right=found, left-on=["stray"], right-on=["stray"]);
let defaulted = select s."stray" as "stray", c."last" as "owner" from ownerless as s join "tastemaker.closed-dir" as c on s."group" = c."group";
let decided = union(left=found, right=defaulted);
emit decided subject "stray" key("stray");
"#;
