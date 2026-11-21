//! The `EachKind` word's cases, on the fill side's fixture tree: a root of kind 1 (element 0), three children of kind 2
//! (1, 2, 3), three tokens of kind 3 (4, 5, 6) and one embedded element of kind 2 (7) past the file's own tree.

use super::item::EachKind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::service::each_kind;
use crate::domain::code::anchoring::types::EachKindWord;
use crate::domain::code::constants::{OWN_TREE, SEVERAL};
use crate::domain::code::records::service::push_list;
use crate::domain::code::types::{CodeEntry, PoolAt, ProgramCode, WordFlag, WordSlot};
use crate::domain::made_tables::fixtures::packs::nodes;
use crate::domain::programs::collections::instructions::interface::InstructionsItem;
use crate::domain::programs::fixtures::{Idle, framed_over, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::Register;
use ryo_relations::domain::relations::types::KindBits;
use std::borrow::Cow;

/// The kinds asked, in order, and the word's extent flag.
type Input = (&'static [u16], WordFlag);
/// The element bound at each turn; then 99 and where the loop goes on (the entry after the word).
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_kind_over_the_own_tree_leaves_out_the_embedded_element"),
        word: Word::Succeeds,
        input: || (&[2], OWN_TREE),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 99, 6])),
    },
    Case {
        name: CaseName("one_kind_over_every_tree_reaches_the_embedded_element"),
        word: Word::Succeeds,
        input: || (&[2], WordFlag(0)),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 7, 99, 6])),
    },
    Case {
        name: CaseName("several_kinds_are_read_in_the_words_order"),
        word: Word::Succeeds,
        input: || (&[3, 1], OWN_TREE),
        expect: Expect::Value(Cow::Borrowed(&[4, 5, 6, 0, 99, 6])),
    },
    Case {
        name: CaseName("a_kind_no_element_has_binds_nothing"),
        word: Word::Succeeds,
        input: || (&[9], OWN_TREE),
        expect: Expect::Value(Cow::Borrowed(&[99, 0])),
    },
];

fn subject((kinds, extent): Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let word = match kinds {
        [one] => each_kind(EachKindWord {
            flag: extent,
            kind: KindBits(*one),
            out: Register(0),
            kinds: PoolAt(0),
        }),
        several => {
            let slots: Vec<WordSlot> = several
                .iter()
                .map(|kind| WordSlot(u32::from(*kind)))
                .collect();
            each_kind(EachKindWord {
                flag: WordFlag(extent.0 | SEVERAL.0),
                kind: KindBits(0),
                out: Register(0),
                kinds: push_list(&mut code.pool, &slots),
            })
        }
    };
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let (tree, mut held) = (nodes(), held(1));
    let (bound, _found) = framed_over(&tree, &programs, &mut held, &Idle, |frame| {
        let _flow = EachKind::run(frame, word, CodeEntry(5));
        let mut cursor = frame.state.frames.pop().unwrap_or_default();
        let (mut bound, mut onward) = (Vec::new(), 0_u64);
        while let Some(next) = EachKind::next(frame, word, &mut cursor) {
            let handle = frame.state.registers.first().map_or(0, |cell| cell.0);
            bound.push(handle & u64::from(u32::MAX));
            onward = u64::from(next.0);
        }
        bound.extend([99, onward]);
        bound
    });
    Ok(Cow::Owned(bound))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
