//! The `packs` service's cases: a pack written for a digest reads back for it; another version, another digest or bytes that do
//! not decode are refused.

use super::constants::VERSION_WIDTH;
use super::errors::PacksError;
use super::service::{decoded, encoded};
use super::types::{PackBytes, PackDigest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::programs::service::accepted;
use crate::domain::programs::types::Programs;

/// What a row reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The pack, for the digest it was written for.
    Same,
    /// The pack, for another digest.
    OtherDigest,
    /// The pack with another engine version.
    OtherVersion,
    /// The pack cut short.
    Truncated,
    /// The pack with a byte after its end.
    Trailing,
}

type Input = Scenario;
/// The rules the read pack runs.
type Value = usize;
type Failure = PacksError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pack_reads_back_for_its_digest"),
        word: Word::Succeeds,
        input: || Scenario::Same,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_pack_for_other_rules_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OtherDigest,
        expect: Expect::Error(PacksError::Digest),
    },
    Case {
        name: CaseName("a_pack_of_another_engine_version_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OtherVersion,
        expect: Expect::Error(PacksError::Version),
    },
    Case {
        name: CaseName("a_pack_cut_short_does_not_decode"),
        word: Word::Refuses,
        input: || Scenario::Truncated,
        expect: Expect::Error(PacksError::Corrupt),
    },
    Case {
        name: CaseName("a_pack_with_bytes_after_it_does_not_decode"),
        word: Word::Refuses,
        input: || Scenario::Trailing,
        expect: Expect::Error(PacksError::Corrupt),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let digest = PackDigest([9; 32]);
    let mut bytes = encoded(&Programs::default(), digest).0;
    let asked = match scenario {
        Scenario::OtherDigest => PackDigest([8; 32]),
        Scenario::Same | Scenario::OtherVersion | Scenario::Truncated | Scenario::Trailing => {
            digest
        }
    };
    match scenario {
        Scenario::OtherVersion => {
            if let Some(byte) = bytes.get_mut(VERSION_WIDTH.0) {
                *byte = byte.wrapping_add(1);
            }
        }
        Scenario::Truncated => bytes.truncate(bytes.len().saturating_sub(1)),
        Scenario::Trailing => bytes.push(0),
        Scenario::Same | Scenario::OtherDigest => {}
    }
    decoded(PackBytes(&bytes), asked).map(|programs| accepted(&programs).len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Every rule set the load scenarios make, written and read back: the code read is the code written, the words made again
/// from the vocabulary are the load's, and the read pack writes the same bytes. A scenario the load refuses is skipped.
#[test]
fn a_loaded_rule_set_reads_back_whole() {
    use crate::inbound::use_cases::load::fixtures::{kinds, loaded_with};
    use crate::inbound::use_cases::load::interface::LoadUseCase as _;
    use crate::inbound::use_cases::load::types::LoadSource;
    use crate::inbound::use_cases::load::use_case::LoadUseCaseImpl;
    use crate::domain::statistics::types::StatisticsBytes;
    let scenes = [
        "one tree rule and one over the host's rows",
        "two rules, one sub-plan",
        "two rules, one sub-plan, two bound values",
        "a product within a file",
        "a keyed read, then the whole relation",
        "one walk over each tree",
    ];
    // The vocabulary the scenarios load against: the fixture language's, stating no fixed name.
    let vocabulary = ryo_values::domain::descriptions::types::KindVocabulary { fixed: Vec::new(), ..kinds() };
    let mut read_back = 0;
    for scene in scenes {
        let Ok(loaded) = loaded_with(scene, StatisticsBytes(&[])) else {
            continue;
        };
        let digest = PackDigest([7; 32]);
        let written = encoded(&loaded.programs, digest);
        let again = LoadUseCaseImpl
            .load(LoadSource::Pack(PackBytes(&written.0), digest), &vocabulary)
            .unwrap_or_else(|error| panic!("{scene}: the pack does not read back: {error:?}"));
        assert_eq!(again.programs.0.code, loaded.programs.0.code, "{scene}: the code read back");
        assert_eq!(again.programs.0.kinds, loaded.programs.0.kinds, "{scene}: the kinds' words");
        assert_eq!(again.programs.0.fields, loaded.programs.0.fields, "{scene}: the fields' words");
        assert_eq!(again.programs.0.node_needs, loaded.programs.0.node_needs, "{scene}: the need sites");
        assert_eq!(encoded(&again.programs, digest), written, "{scene}: the bytes written again");
        read_back += 1;
    }
    assert!(read_back >= 4, "too few scenarios loaded: {read_back}");
}
