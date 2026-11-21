//! The constants of the `packs` concept: the engine version a pack is written by, and the widths of its fields.

use ryo_relations::domain::relations::types::Count;

/// The engine version a pack is written by: a pack of another engine is refused, and rebuilt.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The width of the length before the engine version.
pub const VERSION_WIDTH: Count = Count(size_of::<u16>());

/// The width of the rule count.
pub const COUNT_WIDTH: Count = Count(size_of::<u32>());

/// The byte a unit named as a definition's pattern is written with.
pub(crate) const PATTERN_KEY: u8 = 0;

/// The byte a unit named as a rule's own program is written with.
pub(crate) const RULE_KEY: u8 = 1;

/// The byte a unit named as a table's fill is written with.
pub(crate) const TABLE_KEY: u8 = 2;

/// The byte a unit in call form is written with; one more: in table form.
pub(crate) const CALL_FORM: u8 = 0;

/// The byte a unit whose shape is open is written with; one more: decided.
pub(crate) const OPEN_SHAPE: u8 = 0;
