//! Budget, content, grammar identity and current-source occurrence regressions.
use super::constants::KEY_CAPACITY;
use super::constants::MEMO_ENTRIES;
use super::constants::SCRATCH_CAPACITY;
use super::service;
use super::types::DialectName;
use super::types::EntryCount;
use super::types::GrammarMemo;
use super::types::Lookup;
use super::types::MemoKey;
use super::types::Scratch;
use super::types::Taken;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::format_syntax::types::FormatParseResult;
use crate::domain::format_syntax::types::FormatSummary;
use crate::domain::format_syntax::types::ParsedFormat;

struct FirstGrammar;

struct SecondGrammar;

fn bounds_reject_large_keys_and_full_memos() {
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("small".into()), 7_u32);
    assert_eq!(
        service::take(&mut memo, Lookup("small")),
        Some(Taken {
            key: MemoKey("small".into()),
            value: 7
        })
    );
    service::admit(&mut memo, MemoKey("x".repeat(KEY_CAPACITY + 1)), 8);
    assert!(memo.entries.is_empty());
    service::scratch(&mut memo, Scratch("x".repeat(SCRATCH_CAPACITY * 2)));
    assert!(memo.scratch.0.is_empty());
    assert!(memo.scratch.0.capacity() <= SCRATCH_CAPACITY);
}

fn parser_identity_and_exact_content_prevent_stale_reuse() {
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("original".into()), 1_u32);
    assert_eq!(service::take(&mut memo, Lookup("edited")), None);
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    assert_eq!(service::take(&mut memo, Lookup("original")), None);
    service::admit(&mut memo, MemoKey("dialect".into()), 3_u32);
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("different-dialect-v1"));
    assert_eq!(service::take(&mut memo, Lookup("dialect")), None);
    for round in 0_u32..100_u32 {
        service::empty(&mut memo);
        for item in 0..100 {
            service::admit(&mut memo, MemoKey(format!("{round}:{item}")), item);
        }
        assert!(service::len(&memo) <= EntryCount(MEMO_ENTRIES));
    }
    service::empty(&mut memo);
    assert!(memo.entries.is_empty());
    assert!(service::len(&memo) <= EntryCount(MEMO_ENTRIES));
}

fn memo_admission_prefers_source_insertion_order_when_the_memo_is_full() {
    use crate::domain::format_syntax::types::{
        CachedFormat, FormatArgument, FormatFieldValue, FormatParseResult, FormatParseState,
        FormatQuantityMode, FormatSummary, ParsedFormat, RetainedFormatIndex,
    };
    for _ in 0_usize..12_usize {
        let mut index =
            RetainedFormatIndex::<FormatParseResult, CachedFormat, FormatParseState>::default();
        let names: Vec<String> = (0..=MEMO_ENTRIES)
            .map(|ordinal| format!("input-{ordinal}"))
            .collect();
        for name in &names {
            assert_eq!(
                service::reserve(&mut index.by_input),
                super::types::Reservation::Granted
            );
            let previous = index.by_input.insert(
                name.clone().into(),
                crate::domain::format_syntax::types::FormatResultIndex(index.values.len()),
            );
            assert!(previous.is_none());
            index.values.push(CachedFormat {
                result: FormatParseResult::Parsed(ParsedFormat {
                    fields: vec![FormatFieldValue {
                        ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0),
                        argument: FormatArgument::Mapping(
                            crate::domain::format_syntax::types::FormatMappingKey("x".into()),
                        ),
                        conversion: crate::domain::format_syntax::types::FormatCharacter('s'),
                        flags: Default::default(),
                        width: FormatQuantityMode::None,
                        precision: FormatQuantityMode::None,
                    }],
                    summary: FormatSummary::default(),
                }),
            });
        }
        crate::domain::format_parsing::service::clear(&mut index);
        assert!(index.memo.entries.contains_key(names[0].as_str()));
        assert!(
            index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES - 1].as_str())
        );
        assert!(
            !index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES].as_str())
        );
        assert!(service::len(&index.memo) <= EntryCount(MEMO_ENTRIES));
    }
}

fn ordered_lookup_releases_large_dense_and_hash_storage_on_reset() {
    let mut map = indexmap::IndexMap::new();
    for ordinal in 0_usize..256_usize {
        assert_eq!(
            service::reserve(&mut map),
            super::types::Reservation::Granted
        );
        let previous = map.insert(ordinal.to_string(), ordinal);
        assert!(previous.is_none());
    }
    assert!(map.capacity() > super::constants::MAP_CAPACITY);
    map.clear();
    service::reset_map(&mut map);
    assert_eq!(map.capacity(), 0);
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("bounds_reject_large_keys_and_full_memos"),
        word: Word::Succeeds,
        input: || bounds_reject_large_keys_and_full_memos,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_identity_and_exact_content_prevent_stale_reuse"),
        word: Word::Succeeds,
        input: || parser_identity_and_exact_content_prevent_stale_reuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("memo_admission_prefers_source_insertion_order_when_the_memo_is_full"),
        word: Word::Succeeds,
        input: || memo_admission_prefers_source_insertion_order_when_the_memo_is_full,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_lookup_releases_large_dense_and_hash_storage_on_reset"),
        word: Word::Succeeds,
        input: || ordered_lookup_releases_large_dense_and_hash_storage_on_reset,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

#[cfg(test)]
mod parser_change_test;
#[cfg(test)]
mod reader_resets_test;
