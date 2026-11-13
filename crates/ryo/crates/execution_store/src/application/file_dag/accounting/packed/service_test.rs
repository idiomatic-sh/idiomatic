//! The `packed` service's cases: the bytes and the allocation identity of a relation's packed words and their arenas.

use super::service::packed_backings;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords, PackedWords};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    /// Words with room for 32 bytes, referencing an arena with room for 16 and an empty one.
    WordsAndArenas,
}
type Input = Scenario;
/// Each backing rendered: its tag and bytes, and whether it is keyed by the allocation it describes.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("packed_words_and_each_nonempty_arena_are_backings_of_their_capacity"),
    word: Word::Succeeds,
    input: || Scenario::WordsAndArenas,
    expect: Expect::Value(
        "2 backings: tag 22 bytes 72 keyed by its allocation true; tag 23 bytes 16 keyed by its allocation true",
    ),
}];

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let Scenario::WordsAndArenas = input;
    let mut words = Vec::with_capacity(32);
    words.extend_from_slice(&[1, 2, 3]);
    let words = Arc::new(KeptWords(PackedWords(words)));
    let mut arena = Vec::with_capacity(16);
    arena.push(7);
    let arena = ArenaBytes(arena);
    let empty = ArenaBytes(Vec::new());
    let backings = packed_backings(&words, &[&arena, &empty]);
    let owners = [Arc::as_ptr(&words).addr(), arena.0.as_ptr().addr()];
    let shown = backings
        .iter()
        .zip(owners)
        .map(|(backing, owner)| {
            format!(
                "tag {} bytes {} keyed by its allocation {}",
                backing.key.tag.0,
                backing.bytes.0,
                backing.key.address.0 == owner
            )
        })
        .collect::<Vec<_>>();
    Ok(Box::leak(
        format!("{} backings: {}", backings.len(), shown.join("; ")).into_boxed_str(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
