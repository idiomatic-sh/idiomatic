//! The names of the arguments that more than one operator of RyoQL takes, written once: the grammar's slots and the
//! operators' descriptions read them from here.

/// The argument that names the columns an operator keeps, or the keys whose values fill them.
pub const COLUMNS: &str = "columns";
/// The argument that names the one column an operator reads.
pub const COLUMN: &str = "column";
/// The argument that names the column an operator writes its digest to.
pub const DIGEST: &str = "digest";

/// Literal replacement text supplied to a text transformation.
pub const REPLACEMENT: &str = "replacement";

/// Left equality key columns.
pub const LEFT_ON: &str = "left-on";

/// Right equality key columns.
pub const RIGHT_ON: &str = "right-on";

/// Explicit output names or projection.
pub const OUTPUT: &str = "output";

/// The argument that names the relation an operator walks, also a grammar keyword.
pub const RELATION: &str = "relation";

/// The argument that names the column holding an edge's destination.
pub const DESTINATION: &str = "destination";

/// A constant matching pattern.
pub const PATTERN: &str = "pattern";

/// The query operator that selects relation rows, also a grammar keyword.
pub const SELECT: &str = "select";
