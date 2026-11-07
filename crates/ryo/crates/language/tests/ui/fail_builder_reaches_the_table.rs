//! builder_reaches_the_table: the compiler rejects this.

use ryo_language::domain::elements::types::TableBuilder;

/// A language that writes into the table itself, past the four functions that honour a builder that builds nothing.
fn reads(builder: &mut TableBuilder<'_>) {
    builder.elements.kind.0.push(0);
}

fn main() {}
