//! The `node_table` service's cases: the id-space laws (nodes in pre-order, then tokens in source order, then embedded segments),
//! subtree ends, children merged in source order, kinds present past the fourth word, and texts kept by id after tokens are
//! numbered.

use super::service::{
    acceptance_of, builder, children_of, decoded_of, decoded_token, diagnose, embed, enter_node,
    finished, ids_of_kind, kind_of, kinds_present, leave_node, node_count, node_property,
    ordinal_of, parent_of, property_of, set_syntax_acceptance, span_of, subtree_end_of,
    syntax_error_count, token,
};
use super::types::{EmbeddedText, FieldCode, NodeTable, NodeTableBuilder};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::elements::types::EmbeddedMapping;
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, PropertyId};
use ryo_relations::domain::terms::types::NameText;

/// What a row reads of the table it builds.
#[derive(Clone, Copy, Debug)]
enum Law {
    /// The kind of every id, then the number of nodes.
    Ids,
    /// The subtree end of every id.
    SubtreeEnds,
    /// The root's children.
    Children,
    /// The kinds present, as their codes, for a language of 330 kinds.
    KindsPresent,
    /// The decoded text of the second token, as its length.
    Decoded,
    /// An embedded segment: its root's id and parent, its token's start in the file and its logical text's length.
    Embedded,
    /// The other readers: the second node's ordinal, the ids of the first token's kind, the root's property's length, whether the
    /// parser rejected the file, and how many diagnostics it reported.
    Readers,
}

/// Writes `node(1)[ tok(10) node(2)[ tok(11) ] tok(12) ]` over the bytes `a b c` (`kinds` kinds).
fn small<'t>(table: &'t mut NodeTable, kinds: usize) -> NodeTableBuilder<'t> {
    let mut writer = builder(table, Count(kinds));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    token(
        &mut writer,
        KindBits(10),
        FieldCode(0),
        Offset(0),
        Offset(1),
    );
    enter_node(&mut writer, KindBits(2), FieldCode(3), Offset(2));
    decoded_token(
        &mut writer,
        KindBits(11),
        FieldCode(0),
        Offset(2),
        Offset(3),
        NameText("bee"),
    );
    leave_node(&mut writer, Offset(3));
    token(
        &mut writer,
        KindBits(12),
        FieldCode(0),
        Offset(4),
        Offset(5),
    );
    leave_node(&mut writer, Offset(5));
    writer
}

/// The numbers of a list of ids.
fn numbers(ids: &[Position]) -> Vec<u32> {
    ids.iter().map(|id| id.0).collect()
}

type Input = Law;
/// Up to six numbers a law reads, the rest zero.
type Value = [u32; 6];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nodes_take_the_first_ids_in_pre_order_then_tokens_in_source_order"),
        word: Word::Succeeds,
        input: || Law::Ids,
        expect: Expect::Value([1, 2, 10, 11, 12, 2]),
    },
    Case {
        name: CaseName("a_node_subtree_ends_after_its_last_descendant_node"),
        word: Word::Succeeds,
        input: || Law::SubtreeEnds,
        expect: Expect::Value([2, 2, 3, 4, 5, 0]),
    },
    Case {
        name: CaseName("children_merge_nodes_and_tokens_in_source_order"),
        word: Word::Succeeds,
        input: || Law::Children,
        expect: Expect::Value([2, 1, 4, 0, 0, 0]),
    },
    Case {
        name: CaseName("kinds_past_the_fourth_word_are_present"),
        word: Word::Succeeds,
        input: || Law::KindsPresent,
        expect: Expect::Value([3, 329, 6, 0, 0, 0]),
    },
    Case {
        name: CaseName("a_decoded_text_stays_with_its_token_once_tokens_are_numbered"),
        word: Word::Succeeds,
        input: || Law::Decoded,
        expect: Expect::Value([33, 0, 0, 0, 0, 0]),
    },
    Case {
        name: CaseName("an_embedded_segment_follows_the_tokens_under_its_host"),
        word: Word::Succeeds,
        input: || Law::Embedded,
        expect: Expect::Value([5, 1, 4, 2, 1, 0]),
    },
    Case {
        name: CaseName("the_readers_answer_ordinal_kind_ids_property_acceptance_and_errors"),
        word: Word::Succeeds,
        input: || Law::Readers,
        expect: Expect::Value([1, 1, 3, 1, 1, 0]),
    },
];

fn subject(law: Input) -> Result<Value, Failure> {
    let mut table = NodeTable::default();
    let read: Vec<u32> = match law {
        Law::Ids => {
            let mut writer = small(&mut table, 16);
            let read = finished(&mut writer);
            let mut kinds: Vec<u32> = (0..5)
                .map(|id| u32::from(kind_of(read, Position(id)).0))
                .collect();
            kinds.push(u32::try_from(node_count(read).0).unwrap_or(u32::MAX));
            kinds
        }
        Law::SubtreeEnds => {
            let mut writer = small(&mut table, 16);
            let read = finished(&mut writer);
            (0..5)
                .map(|id| subtree_end_of(read, Position(id)).0)
                .collect()
        }
        Law::Children => {
            let mut writer = small(&mut table, 16);
            numbers(children_of(finished(&mut writer), Position(0)))
        }
        Law::KindsPresent => {
            let mut writer = builder(&mut table, Count(330));
            enter_node(&mut writer, KindBits(329), FieldCode(0), Offset(0));
            token(&mut writer, KindBits(3), FieldCode(0), Offset(0), Offset(1));
            leave_node(&mut writer, Offset(1));
            let present = kinds_present(finished(&mut writer));
            (0..330_u32)
                .filter(|code| {
                    present
                        .0
                        .get(usize::try_from(code / 64).unwrap_or(usize::MAX))
                        .is_some_and(|word| word.0 & (1_u64 << (code % 64)) != 0)
                })
                .chain(std::iter::once(
                    u32::try_from(present.0.len()).unwrap_or(u32::MAX),
                ))
                .collect()
        }
        Law::Decoded => {
            let mut writer = small(&mut table, 16);
            let read = finished(&mut writer);
            (0..5)
                .filter_map(|id| {
                    decoded_of(read, Position(id))
                        .map(|text| id * 10 + u32::try_from(text.0.len()).unwrap_or(0))
                })
                .collect()
        }
        Law::Embedded => {
            let mut writer = small(&mut table, 16);
            let text = EmbeddedText {
                text: NameText("xy"),
                mapping: EmbeddedMapping::Exact(Offset(4)),
            };
            let root = embed(&mut writer, Position(1), text);
            enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
            token(
                &mut writer,
                KindBits(10),
                FieldCode(0),
                Offset(0),
                Offset(2),
            );
            leave_node(&mut writer, Offset(2));
            let read = finished(&mut writer);
            let token_id = Position(root.0.0 + 1);
            vec![
                root.0.0,
                parent_of(read, root.0).0,
                span_of(read, token_id).start.0,
                u32::try_from(decoded_of(read, token_id).map_or(0, |text| text.0.len()))
                    .unwrap_or(0),
                u32::try_from(children_of(read, Position(1)).len()).unwrap_or(0),
            ]
        }
        Law::Readers => {
            let mut writer = builder(&mut table, Count(16));
            enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
            node_property(&mut writer, PropertyId(7), NameText("abc"));
            token(
                &mut writer,
                KindBits(10),
                FieldCode(0),
                Offset(0),
                Offset(1),
            );
            enter_node(&mut writer, KindBits(2), FieldCode(3), Offset(2));
            diagnose(&mut writer, Offset(2), Offset(3), NameText("odd"));
            leave_node(&mut writer, Offset(3));
            leave_node(&mut writer, Offset(5));
            set_syntax_acceptance(&mut writer, SyntaxAcceptance::Rejected);
            let read = finished(&mut writer);
            vec![
                ordinal_of(read, Position(1)).0,
                u32::try_from(ids_of_kind(read, KindBits(10)).len()).unwrap_or(0),
                u32::try_from(
                    property_of(read, Position(0), PropertyId(7)).map_or(0, |text| text.0.len()),
                )
                .unwrap_or(0),
                u32::from(acceptance_of(read) == SyntaxAcceptance::Rejected),
                u32::try_from(syntax_error_count(read).0).unwrap_or(0),
            ]
        }
    };
    let mut padded = [0; 6];
    for (slot, number) in padded.iter_mut().zip(read) {
        *slot = number;
    }
    Ok(padded)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
