//! The store's tables carry a checked product back whole, each text once, and a store whose rows name a text it does
//! not hold is refused.
use super::{KeptSchema, StoreSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::check::{fixtures, interface::CheckUseCase as _};
use crate::outbound::check_store::types::{Build, Stored};

#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether a checked product, its violations, inputs and file records among it, reads back as written.
    RoundTrip,
    /// Whether every text of the store is held once.
    Once,
    /// Whether a store whose texts were dropped is refused.
    Unnamed,
    /// Whether the store a failing check left, a failed run beside the completed run it replaced, reads back as written.
    AfterFailure,
}
type Input = Question;
type Value = bool;
/// Why the store could not be written.
type Failure = String;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_checked_product_reads_back_as_written"),
        word: Word::Succeeds,
        input: || Question::RoundTrip,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_text_is_held_once"),
        word: Word::Succeeds,
        input: || Question::Once,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_whose_rows_name_an_absent_text_is_refused"),
        word: Word::Succeeds,
        input: || Question::Unnamed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_kept_after_a_failing_run_reads_back_as_written"),
        word: Word::Succeeds,
        input: || Question::AfterFailure,
        expect: Expect::Value(true),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    let scenario = fixtures::scenario();
    drop(
        fixtures::use_case(&scenario)
            .check(fixtures::request())
            .map_err(|error| error.to_string())?,
    );
    let scenario = if matches!(question, Question::AfterFailure) {
        scenario.sources.write("src/a.rs", "fn a() {}\n");
        let failing = fixtures::with_sarif(
            scenario,
            crate::outbound::sarif::fixtures::fake::SarifFake::default(),
        );
        drop(
            fixtures::use_case(&failing)
                .check(fixtures::request())
                .map_err(|error| error.to_string())?,
        );
        failing
    } else {
        scenario
    };
    let stored = scenario.store.kept();
    let build = Build("build-1".into());
    let bytes = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&stored, &build)))
        .map_err(|error| error.to_string())?;
    let read: StoreSchema = postcard::from_bytes(&bytes).map_err(|error| error.to_string())?;
    match question {
        Question::RoundTrip | Question::AfterFailure => {
            // Read back and written again, the store is the same bytes: nothing changes shape on the way.
            let again = Stored::try_from(read).map_err(|()| "the store reads back".to_owned())?;
            let rewritten = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&again, &build)))
                .map_err(|error| error.to_string())?;
            // A failed run is kept only to say that it failed, not what it said: after a failing run the bytes are the same,
            // the failure's text is not.
            let same = matches!(question, Question::AfterFailure) || again == stored;
            Ok(same && rewritten == bytes)
        }
        Question::Once => {
            let texts = read.strings.len();
            let distinct: std::collections::BTreeSet<&String> = read.strings.iter().collect();
            Ok(texts > 0 && distinct.len() == texts)
        }
        Question::Unnamed => Ok(Stored::try_from(StoreSchema {
            strings: Vec::new(),
            ..read
        }) == Err(())),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
