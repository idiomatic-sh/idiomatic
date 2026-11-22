//! The `spelled` part's cases: a file's tokens read once for the texts asked, the own tree's apart from an embedded
//! segment's, and a token past ASCII that may be a name read as another text.

use super::service::{file_read, spelled, text_number};
use super::types::{SpelledFile, SpelledTexts};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node, token};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};

/// A file's source and the texts asked of it.
type Input = (&'static str, [&'static str; 2]);

/// For each text asked: spelled in the own tree, spelled anywhere.
type Value = [(bool, bool); 2];

/// A node over the source's words, each a token (split at spaces).
fn table_of(source: &str) -> NodeTable {
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(3));
    let end = Offset(u32::try_from(source.len()).unwrap_or(u32::MAX));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    let mut start = 0_usize;
    for word in source.split(' ') {
        let stop = start + word.len();
        token(
            &mut writer,
            KindBits(2),
            FieldCode(0),
            Offset(u32::try_from(start).unwrap_or(u32::MAX)),
            Offset(u32::try_from(stop).unwrap_or(u32::MAX)),
        );
        start = stop + 1;
    }
    leave_node(&mut writer, end);
    let _done = finished(&mut writer);
    table
}

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("a_text_a_token_spells_is_there_and_one_no_token_spells_is_not"),
        word: Word::Succeeds,
        input: || ("import os path", ["path", "sys"]),
        expect: Expect::Value([(true, true), (false, false)]),
    },
    Case {
        name: CaseName("a_text_inside_a_token_is_not_spelled_by_it"),
        word: Word::Succeeds,
        input: || ("os.path x", ["path", "x"]),
        expect: Expect::Value([(false, false), (true, true)]),
    },
    Case {
        name: CaseName("a_token_past_ascii_with_no_quote_makes_every_text_there"),
        word: Word::Succeeds,
        input: || ("ﬁle x", ["file", "sys"]),
        expect: Expect::Value([(true, true), (true, true)]),
    },
    Case {
        name: CaseName("a_string_token_past_ascii_changes_nothing"),
        word: Word::Succeeds,
        input: || ("'é' x", ["x", "sys"]),
        expect: Expect::Value([(true, true), (false, false)]),
    },
];

fn subject((source, asked): Input) -> Result<Value, ()> {
    let table = table_of(source);
    let mut texts = SpelledTexts::default();
    let numbers: Vec<u32> = asked.iter().map(|text| text_number(&mut texts, text)).collect();
    let mut file = SpelledFile::default();
    file_read(&texts, Some(source), &table, &mut file);
    let each = |at: usize| {
        numbers
            .get(at)
            .map_or((false, false), |number| (spelled(&file, *number, true), spelled(&file, *number, false)))
    };
    Ok([each(0), each(1)])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
