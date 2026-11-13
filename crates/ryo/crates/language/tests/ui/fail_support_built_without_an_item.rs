//! support_built_without_an_item: the compiler rejects this.

use ryo_relations::domain::relations::collections::relations::function::item::Function;
use ryo_language::domain::answers::types::{Reader, Support};

/// An answer made beside the item, from a reader written for the purpose: only `reads::<Item>()` makes a `Reads`.
fn answers(reader: Reader<Function, ()>) -> Support<Reader<Function, ()>> {
    Support(reader)
}

fn main() {}
