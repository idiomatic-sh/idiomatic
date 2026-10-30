//! The names of the arguments that more than one operator of RyoQL takes, written once: the grammar's slots and the
//! operators' descriptions read them from here.

/// The argument that names the columns an operator keeps, or the keys whose values fill them.
pub const COLUMNS: &str = "columns";
/// The argument that names the one column an operator reads.
pub const COLUMN: &str = "column";
/// The argument that names the column an operator writes its digest to.
pub const DIGEST: &str = "digest";
