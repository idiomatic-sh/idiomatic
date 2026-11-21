//! The files of the fill bench: the nodes a workload's file holds, and its text.

use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};

/// How many bytes a token of a workload's file covers.
const TOKEN_BYTES: u32 = 2;

/// A file of about `elements` elements: a root (kind 1) whose children (kind 2) each hold seven tokens (kind 3), each
/// token two bytes of the text.
pub(crate) fn tree(elements: usize) -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    let groups = u32::try_from(elements / 8).unwrap_or(u32::MAX).max(1);
    let wide = TOKEN_BYTES.saturating_mul(7);
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    for group in 0..groups {
        let start = group.saturating_mul(wide);
        enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(start));
        for place in 0..7_u32 {
            let at = start.saturating_add(place.saturating_mul(TOKEN_BYTES));
            token(
                &mut writer,
                KindBits(3),
                FieldCode(0),
                Offset(at),
                Offset(at.saturating_add(TOKEN_BYTES)),
            );
        }
        leave_node(&mut writer, Offset(start.saturating_add(wide)));
    }
    leave_node(&mut writer, Offset(groups.saturating_mul(wide)));
    let _table = finished(&mut writer);
    made
}

/// A file of `elements` elements under one root: tokens whose kinds are sixteen (1 to 16) in an order that does not
/// repeat from one element to the next, as a real file's nodes change kind.
pub(crate) fn mixed_tree(elements: usize) -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(17));
    let count = u32::try_from(elements).unwrap_or(u32::MAX).max(2);
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    let mut state = 12_345_u32;
    for at in 1..count {
        // A small generator: the kind is its high bits.
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let kind = u16::try_from((state >> 28) & 15)
            .unwrap_or(0)
            .saturating_add(1);
        token(
            &mut writer,
            KindBits(kind),
            FieldCode(0),
            Offset(at),
            Offset(at.saturating_add(1)),
        );
    }
    leave_node(&mut writer, Offset(count));
    let _table = finished(&mut writer);
    made
}

/// The text of the file `tree` describes: for each group fourteen small letters that spell the group's number, so
/// every group's text is its own, a token is two letters, and no group holds a space.
pub(crate) fn bench_text(elements: usize) -> Vec<u8> {
    let groups = (elements / 8).max(1);
    let mut text = Vec::with_capacity(groups.saturating_mul(14));
    for group in 0..groups {
        let mut rest = group;
        for _ in 0..14 {
            text.push(b'a'.saturating_add(u8::try_from(rest % 26).unwrap_or(0)));
            rest /= 26;
        }
    }
    text
}
