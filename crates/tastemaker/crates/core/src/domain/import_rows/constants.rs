//! The names the built-in rules over the import relations read.

/// The relation of the names a `use` brings in.
pub const NAMES_RELATION: &str = "static-import";
/// The relation of the packages a manifest depends on.
pub const PACKAGES_RELATION: &str = "package-dependency";
/// The relation of the package each file belongs to.
pub const OWNERS_RELATION: &str = "file-package";
/// The column a row of the owners is the finding of, and the file it says the package of.
pub const PATH_COLUMN: &str = "path";
/// The key of a row of the owners.
pub const OWNER_KEY: &str = "\"path\"";
/// The column of the manifest a file's package is.
pub const PACKAGE: &str = "package";
/// The column a row of the edges and the packages is the finding of.
pub const SOURCE_COLUMN: &str = "source";
/// The column a row of the names is the finding of.
pub const FILE_COLUMN: &str = "file";
/// The key of a row of the names.
pub const NAME_KEY: &str = "\"occurrence\"";
/// The key of a row of the packages.
pub const PACKAGE_KEY: &str = "\"target\", \"binding\", \"kind\"";
/// The column of what a row reaches.
pub const TARGET: &str = "target";
/// The column of where a row is written.
pub const SPAN: &str = "span";
/// The column of the name a package is bound by.
pub const BINDING: &str = "binding";
/// The column of an edge's path as its file writes it at the edge's place, its whitespace left out.
pub const WRITTEN: &str = "written";

/// The column of the kind of a syntax node.
pub const KIND: &str = "kind";
/// The kind of the node that holds a block's statements: a `use` written in it binds within the block.
pub const BLOCK_KIND: &str = "stmt-list";
/// The kind of the node that holds an inline module's items: a `use` written outside it binds nothing within it.
pub const MODULE_KIND: &str = "item-list";
/// The nodes a `use` binds within: each block and each inline module, by its file and span.
pub const SCOPES: &str = r#"let rows = select id, kind, file, span from "syntax-tree" where glob_any(kind, "{stmt-list,item-list}");
emit rows subject "file" key("id");
"#;

/// Visibility restriction paths name an access scope, not an imported capability.
/// Exclude only their exact AST-proven file/span; other uses of the same path remain.
pub const SEMANTIC_EDGES: &str = r#"let nodes = select id, kind, file, span from "syntax-tree" where text = "";
let paths = select * from nodes where kind = "path";
let visibility = select * from nodes where kind = "visibility";
let nested = axis(from=visibility, relation="syntax-child", direction=descendant, domain=paths);
let scopes = select r.file as source, r.span as span from nested as l join paths as r on l.related = r.id;
let written = select * from "source-dependency";
let rows = antijoin(left=written, right=scopes, left-on=["source", "span"], right-on=["source", "span"]);
emit rows subject "source" key("target", "span");
"#;
