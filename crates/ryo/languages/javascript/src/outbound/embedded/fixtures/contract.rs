//! What every `embedded` adapter must pass.

use super::super::interface::EmbeddedPort;
use super::super::types::{Dialect, Markup};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::{Recognition, SourcePath};

/// What a row is given: the text of a file.
type Input = &'static str;
/// How many blocks it holds, and where the first starts.
type Value = (usize, usize);
/// An adapter refuses nothing: a file with no block has none.
type Failure = ();

/// The rows every embedded adapter answers alike: how many blocks a file holds and where the first starts.
const BLOCKS: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_file_has_no_block"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_file_with_no_script_has_no_block"),
        word: Word::Succeeds,
        input: || "<template><p>hi</p></template>",
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_script_is_a_block_that_starts_after_its_tag"),
        word: Word::Succeeds,
        input: || "<script>a</script>",
        expect: Expect::Value((1, 8)),
    },
];

/// Whether the adapter claims a path with this ending: a row's input is a file name.
const CLAIMS: &[Case<Input, Recognition, Failure>] = &[Case {
    name: CaseName("a_javascript_file_is_not_claimed"),
    word: Word::Succeeds,
    input: || "main.js",
    expect: Expect::Value(Recognition::Declines),
}];

/// Run the contract over `adapter`, which claims files ending in `own`.
pub(crate) fn check(adapter: &impl EmbeddedPort, own: Input) {
    run(BLOCKS, Word::Succeeds, |text| {
        let blocks = adapter.blocks(Markup(text)).0;
        Ok((
            blocks.len(),
            blocks.first().map_or(0, |block| block.start.0),
        ))
    });
    run(CLAIMS, Word::Succeeds, |path| {
        Ok(adapter.claims(SourcePath(path)))
    });
    let name = format!("component{own}");
    assert_eq!(
        adapter.claims(SourcePath(&name)),
        Recognition::Reads,
        "the adapter claims a file with its own ending"
    );
    let typed = adapter.blocks(Markup("<script lang=\"ts\">a</script>")).0;
    assert_eq!(
        typed.first().map(|block| block.dialect),
        Some(Dialect::Typed),
        "a block marked `lang=\"ts\"` is read as typed"
    );
}
