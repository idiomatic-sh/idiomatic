//! The constants of the `packs` concept: the engine version a pack is written by, and the widths of its fields.

use ryo_relations::domain::relations::types::Count;

/// The engine version a pack is written by: a pack of another engine is refused, and rebuilt.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The width of the length before the engine version.
pub const VERSION_WIDTH: Count = Count(size_of::<u16>());

/// The width of the rule count.
pub const COUNT_WIDTH: Count = Count(size_of::<u32>());
