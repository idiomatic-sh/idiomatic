//! The words the `facts` port's contract and its adapters' tests keep: the proof every kept stretch carries and the words to keep.

use ryo_execution::inbound::use_cases::execution::types::facts::WordsToKeep;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::stored_facts::types::PackedWords;

/// The proof of complete coverage that a kept stretch carries when a test says nothing of coverage.
pub(crate) static FRONTIER_PROOF: std::sync::LazyLock<ryo_values::domain::records::types::FrontierProof> =
    std::sync::LazyLock::new(|| ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(0),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    });

/// The words to keep, of the first file, with the complete proof.
pub(crate) fn test_words(words: &PackedWords) -> WordsToKeep<'_> {
    WordsToKeep {
        words,
        file: FileId(0),
        proof: &FRONTIER_PROOF,
    }
}
