//! The support of the group's tests.
use super::super::super::constants::KEY_CAPACITY;
use super::super::super::constants::MEMO_ENTRIES;
use super::super::super::constants::SCRATCH_CAPACITY;
use super::super::super::service;
use super::super::super::types::DialectName;
use super::super::super::types::EntryCount;
use super::super::super::types::GrammarMemo;
use super::super::super::types::Lookup;
use super::super::super::types::MemoKey;
use super::super::super::types::Scratch;

pub(super) struct FirstGrammar;

pub(super) struct SecondGrammar;

pub(super) fn bounds_reject_large_keys_and_full_memos() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("small".into()), 7_u32);
    observed.push(format!("{:?}", service::take(&mut memo, Lookup("small"))));
    service::admit(&mut memo, MemoKey("x".repeat(KEY_CAPACITY + 1)), 8);
    observed.push(format!("{}", memo.entries.is_empty()));
    service::scratch(&mut memo, Scratch("x".repeat(SCRATCH_CAPACITY * 2)));
    observed.push(format!("{}", memo.scratch.0.is_empty()));
    observed.push(format!("{}", memo.scratch.0.capacity() <= SCRATCH_CAPACITY));
    observed.join("|")
}

pub(super) fn parser_identity_and_exact_content_prevent_stale_reuse() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("original".into()), 1_u32);
    observed.push(format!("{:?}", service::take(&mut memo, Lookup("edited"))));
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    observed.push(format!(
        "{:?}",
        service::take(&mut memo, Lookup("original"))
    ));
    service::admit(&mut memo, MemoKey("dialect".into()), 3_u32);
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("different-dialect-v1"));
    observed.push(format!("{:?}", service::take(&mut memo, Lookup("dialect"))));
    for round in 0_u32..100_u32 {
        service::empty(&mut memo);
        for item in 0..100 {
            service::admit(&mut memo, MemoKey(format!("{round}:{item}")), item);
        }
        observed.push(format!(
            "{}",
            service::len(&memo) <= EntryCount(MEMO_ENTRIES)
        ));
    }
    service::empty(&mut memo);
    observed.push(format!("{}", memo.entries.is_empty()));
    observed.push(format!(
        "{}",
        service::len(&memo) <= EntryCount(MEMO_ENTRIES)
    ));
    observed.join("|")
}

pub(super) fn memo_admission_prefers_source_insertion_order_when_the_memo_is_full() -> String {
    let mut observed: Vec<String> = Vec::new();
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
            observed.push(format!("{:?}", service::reserve(&mut index.by_input)));
            let previous = index.by_input.insert(
                name.clone().into(),
                crate::domain::format_syntax::types::FormatResultIndex(index.values.len()),
            );
            observed.push(format!("{}", previous.is_none()));
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
        observed.push(format!(
            "{}",
            index.memo.entries.contains_key(names[0].as_str())
        ));
        observed.push(format!(
            "{}",
            index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES - 1].as_str())
        ));
        observed.push(format!(
            "{}",
            !index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES].as_str())
        ));
        observed.push(format!(
            "{}",
            service::len(&index.memo) <= EntryCount(MEMO_ENTRIES)
        ));
    }
    observed.join("|")
}

pub(super) fn ordered_lookup_releases_large_dense_and_hash_storage_on_reset() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut map = indexmap::IndexMap::new();
    for ordinal in 0_usize..256_usize {
        observed.push(format!("{:?}", service::reserve(&mut map)));
        let previous = map.insert(ordinal.to_string(), ordinal);
        observed.push(format!("{}", previous.is_none()));
    }
    observed.push(format!(
        "{}",
        map.capacity() > super::super::super::constants::MAP_CAPACITY
    ));
    map.clear();
    service::reset_map(&mut map);
    observed.push(format!("{:?}", map.capacity()));
    observed.join("|")
}

pub(super) fn the_memo_refuses_at_the_entry_bound_and_releases_oversized_capacity() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    for item in 0..MEMO_ENTRIES {
        service::admit(&mut memo, MemoKey(format!("held-{item}")), item);
    }
    observed.push(format!("{:?}", service::len(&memo)));
    service::admit(&mut memo, MemoKey("past-the-bound".into()), MEMO_ENTRIES);
    observed.push(format!("{:?}", service::len(&memo)));
    observed.push(format!(
        "{}",
        service::take(&mut memo, Lookup("past-the-bound")).is_none()
    ));
    observed.push(format!("{:?}", service::take(&mut memo, Lookup("held-0"))));
    observed.push(format!("{:?}", service::len(&memo)));
    memo.entries.reserve(MEMO_ENTRIES * 8);
    observed.push(format!("{}", memo.entries.capacity() > MEMO_ENTRIES));
    service::empty(&mut memo);
    observed.push(format!("{:?}", service::len(&memo)));
    observed.push(format!("{}", memo.entries.capacity() <= MEMO_ENTRIES));
    observed.join("|")
}
