//! Packing writes the minimal varints with no spare capacity, and a table is one frame only when its bytes are exactly that frame.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_words::service::{
    encode_table, encoded_size, pack, single_table_frame, unpacked, with_encoded_word,
};
use crate::domain::table_words::types::{EncodedSize, PackedSlice, WordSlice};
use crate::domain::rows::types::Table;
use ryo_relations::domain::relations::types::{Bits, Words};

#[derive(Clone, Copy)]
enum Scenario {
    Empty,
    Zero,
    OneByteLimit,
    TwoByteStart,
    TwoByteLimit,
    ThreeByteStart,
    Largest,
    Mixed,
    Twice,
    NoRoomForAnyWord,
    NoRoomForTheLargest,
    RoomForTheLargest,
    OneFrame,
    AdjacentFrames,
    ShortenedFrame,
    ShiftedFrame,
}
type Input = Scenario;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Packed,
    NoRoom,
    Room,
    Whole,
    Split,
}
type Value = Outcome;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_words_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("zero_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::Zero,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("the_largest_one_byte_word_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::OneByteLimit,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("the_first_two_byte_word_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::TwoByteStart,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("the_largest_two_byte_word_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::TwoByteLimit,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("the_first_three_byte_word_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::ThreeByteStart,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("the_largest_word_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::Largest,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("words_of_every_width_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::Mixed,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("a_table_written_twice_packs_as_the_legacy_varints_without_slack"),
        word: Word::Succeeds,
        input: || Scenario::Twice,
        expect: Expect::Value(Outcome::Packed),
    },
    Case {
        name: CaseName("a_total_with_no_room_takes_no_word"),
        word: Word::Succeeds,
        input: || Scenario::NoRoomForAnyWord,
        expect: Expect::Value(Outcome::NoRoom),
    },
    Case {
        name: CaseName("a_total_ten_short_of_the_limit_takes_no_largest_word"),
        word: Word::Succeeds,
        input: || Scenario::NoRoomForTheLargest,
        expect: Expect::Value(Outcome::NoRoom),
    },
    Case {
        name: CaseName("a_total_eleven_short_of_the_limit_takes_the_largest_word"),
        word: Word::Succeeds,
        input: || Scenario::RoomForTheLargest,
        expect: Expect::Value(Outcome::Room),
    },
    Case {
        name: CaseName("one_table_is_one_frame"),
        word: Word::Succeeds,
        input: || Scenario::OneFrame,
        expect: Expect::Value(Outcome::Whole),
    },
    Case {
        name: CaseName("adjacent_frames_are_not_one_frame"),
        word: Word::Succeeds,
        input: || Scenario::AdjacentFrames,
        expect: Expect::Value(Outcome::Split),
    },
    Case {
        name: CaseName("a_frame_cut_short_is_not_a_frame"),
        word: Word::Succeeds,
        input: || Scenario::ShortenedFrame,
        expect: Expect::Value(Outcome::Split),
    },
    Case {
        name: CaseName("a_frame_started_late_is_not_a_frame"),
        word: Word::Succeeds,
        input: || Scenario::ShiftedFrame,
        expect: Expect::Value(Outcome::Split),
    },
];

fn legacy(words: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for word in words {
        let mut value = *word;
        while value >= 128 {
            bytes.push((value & 127) as u8 | 128);
            value >>= 7;
        }
        bytes.push(value as u8);
    }
    bytes
}

fn packing(words: &[u64]) -> Outcome {
    let packed = pack(WordSlice(words));
    assert_eq!(packed.0, legacy(words));
    assert_eq!(
        encoded_size(WordSlice(words)),
        Some(EncodedSize(packed.0.len()))
    );
    assert_eq!(
        packed.0.capacity(),
        packed.0.len(),
        "no guessed allocation ratio or shrink-copy"
    );
    assert!(unpacked(&packed).map(|word| word.0).eq(words.iter().copied()));
    Outcome::Packed
}

fn room(total: usize, word: u64) -> Outcome {
    match with_encoded_word(EncodedSize(total), Bits(word)) {
        Some(_) => Outcome::Room,
        None => Outcome::NoRoom,
    }
}

fn frames() -> Vec<u8> {
    let mut words = Words::default();
    encode_table(&Table::default(), &[], &mut words);
    pack(WordSlice(&words.0)).0
}

fn framing(bytes: &[u8]) -> Outcome {
    if single_table_frame(PackedSlice(bytes)) == crate::domain::table_words::types::Framing::Whole {
        Outcome::Whole
    } else {
        Outcome::Split
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let frame = frames();
    let joined = [frame.clone(), frame.clone()].concat();
    let shifted = frame.len() - 1;
    Ok(match input {
        Scenario::Empty => packing(&vec![]),
        Scenario::Zero => packing(&vec![0]),
        Scenario::OneByteLimit => packing(&vec![127]),
        Scenario::TwoByteStart => packing(&vec![128]),
        Scenario::TwoByteLimit => packing(&vec![16383]),
        Scenario::ThreeByteStart => packing(&vec![16384]),
        Scenario::Largest => packing(&vec![u64::MAX]),
        Scenario::Mixed => packing(&vec![0, 127, 128, 16383, 16384, u64::MAX]),
        Scenario::Twice => {
            packing(&super::twice().0)
        }
        Scenario::NoRoomForAnyWord => room(usize::MAX, 0),
        Scenario::NoRoomForTheLargest => room(usize::MAX - 9, u64::MAX),
        Scenario::RoomForTheLargest => {
            assert_eq!(
                with_encoded_word(EncodedSize(usize::MAX - 10), Bits(u64::MAX)),
                Some(EncodedSize(usize::MAX))
            );
            room(usize::MAX - 10, u64::MAX)
        }
        Scenario::OneFrame => framing(&frame),
        Scenario::AdjacentFrames => framing(&joined),
        Scenario::ShortenedFrame => framing(&joined[..shifted]),
        Scenario::ShiftedFrame => framing(&joined[shifted..]),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
