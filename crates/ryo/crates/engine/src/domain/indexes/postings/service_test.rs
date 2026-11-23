//! Probe both postings encodings, including repeated and colliding composite keys.
use super::{service, types::PostingBuffers};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

type Input = (usize, u64);
type Value = Cow<'static, [u32]>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("zero_rows_offer_no_candidates"),
        word: Word::Succeeds,
        input: || (0, 1),
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("dense_repeated_keys_follow_source_order"),
        word: Word::Succeeds,
        input: || (4, 1),
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
    Case {
        name: CaseName("sparse_repeated_keys_follow_source_order"),
        word: Word::Succeeds,
        input: || (4, u64::MAX),
        expect: Expect::Value(Cow::Borrowed(&[0, 2])),
    },
];

fn subject((count, value): Input) -> Result<Value, Failure> {
    let mut group = super::types::KeyGroup::from_buf([3_u32]);
    group.push(1);
    group.push(3);
    assert_eq!(group.as_slice(), &[3, 1, 3]);
    let tuple = [1_u64, 2, 3, 4, 5];
    let mut groups = rustc_hash::FxHashMap::default();
    for cells in [&tuple[..4], &tuple[..]] {
        groups.insert(super::types::PartitionKey::from_slice(cells), cells.len());
        assert_eq!(groups.get(cells), Some(&cells.len()));
    }
    let mut buffers = PostingBuffers::default();
    let result = service::build(&mut buffers, Count(count), |row| {
        CellWord(if row.0 % 2 == 0 { value } else { 9 })
    })
    .ok_or(())?;
    let published = service::share(&mut buffers, result);
    let mut list = super::types::PackedPositions::default();
    service::collect_rows(
        &published,
        [CellWord(value), CellWord(5), CellWord(value)].into_iter(),
        &mut list,
    );
    assert_eq!(list.0, if count == 0 { vec![] } else { vec![0, 2, 0, 2] });
    let answer =
        service::probe(&published, CellWord(value)).map_or_else(Vec::new, |rows| rows.0.to_vec());
    assert!(service::probe(&published, CellWord(5)).is_none());
    service::release(&mut buffers, published);
    let empty_tuple = service::key(std::iter::empty());
    let next = service::build(&mut buffers, Count(2), |_| empty_tuple).ok_or(())?;
    assert_eq!(service::probe(&next, CellWord(0)).ok_or(())?.0, &[0, 1]);
    service::retire(&mut buffers, next);
    let keys = [
        [0, 0],
        [1, 0x517c_c1b7_2722_0a95_u64.rotate_left(5)],
        [0, 0],
    ];
    let columns = [
        keys.iter().map(|key| key[0]).collect(),
        keys.iter().map(|key| key[1]).collect(),
    ];
    let collides = service::build_columns(
        &mut buffers,
        Count(keys.len()),
        &columns,
        super::types::PostingColumns(&[0, 1]),
        |&word| CellWord(word),
    )
    .ok_or(())?;
    let candidates = service::probe(&collides, CellWord(0)).ok_or(())?.0;
    assert_eq!(candidates, &[0, 1, 2]);
    let exact: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|&r| service::equal(keys[r as usize].iter(), keys[0].iter()))
        .collect();
    assert_eq!(exact, [0, 2]);
    Ok(Cow::Owned(answer))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
