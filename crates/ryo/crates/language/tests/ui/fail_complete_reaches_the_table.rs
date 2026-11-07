//! complete_reaches_the_table: the compiler rejects this.

use ryo_language::domain::elements::types::Completion;

/// A language that writes into the table when it completes it, past `name_kind` and `write_notes`.
fn completes(completion: &mut Completion<'_>) {
    completion.elements.kind.0.push(0);
}

fn main() {}
