//! The cases of the loads behind a run's file scan, over the engine port's fake: the statistics a run names reach the
//! engine's `load` as the bytes they are; a run that names none gives none; and the rules are loaded once for each linked
//! language, however many workers scan however many files.

use crate::application::file_scan::service::{emit_columns, place_columns, ran, worker};
use crate::application::file_scan::types::{ScanMeasuring, ScanRun};
use crate::application::pack_load::service::harnessed;
use crate::application::pack_load::types::{Harness, HarnessRequest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::digest::factory::blake3;
use crate::outbound::engine::fixtures::fake::EngineFake;
use ryo_engine::domain::lowering::fixtures::named;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::descriptions::types::{
    KindVocabularies, KindVocabulary, KindWord, LanguageName,
};
use ryo_values::domain::stored_facts::types::Digest;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// What a row loads: the languages linked, and the statistics the run names.
type Input = (&'static [&'static str], &'static str);
/// What the fake was asked: its lowerings before and after the files, and the statistics its last lowering was given.
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("statistics_reach_load"),
        word: Word::Succeeds,
        input: || (&["words"], "ryo-statistics\n0.0.0\nfirst\n"),
        expect: Expect::Value(Cow::Borrowed(
            "1 rule sets; 1 lowerings; 1 lowerings after the files, 12 rows found; statistics \"ryo-statistics\\n0.0.0\\nfirst\\n\"",
        )),
    },
    Case {
        name: CaseName("a_run_that_names_no_statistics_gives_load_none"),
        word: Word::Succeeds,
        input: || (&["words"], ""),
        expect: Expect::Value(Cow::Borrowed(
            "1 rule sets; 1 lowerings; 1 lowerings after the files, 12 rows found; statistics \"\"",
        )),
    },
    Case {
        name: CaseName("one_load_per_language"),
        word: Word::Succeeds,
        input: || (&["words", "marks"], ""),
        expect: Expect::Value(Cow::Borrowed(
            "2 rule sets; 2 lowerings; 2 lowerings after the files, 24 rows found; statistics \"\"",
        )),
    },
];

fn subject((languages, statistics): Input) -> Result<Value, Failure> {
    let (fake, digest, plan) = (EngineFake::default(), blake3(), named("one tree rule"));
    let vocabularies = KindVocabularies(
        languages
            .iter()
            .map(|language| KindVocabulary {
                language: LanguageName((*language).into()),
                kinds: vec![KindWord("word".into())],
                fields: Vec::new(),
                fixed: Vec::new(),
            })
            .collect(),
    );
    let harness = Harness {
        engine: &fake,
        digest: &digest,
        vocabularies: &vocabularies,
        named: None,
    };
    let request = HarnessRequest {
        plans: &[&plan],
        bindings: &[],
        statistics: ByteSlice(statistics.as_bytes()),
        given: Digest([1; 16]),
        stored: &[],
        declared: Default::default(),
    };
    let loads = harnessed(EngineMode::New, Some(harness), request)
        .map_err(|error| Cow::Owned(error.to_string()))?;
    let before = fake.lowered.load(Ordering::Relaxed);
    // Two workers a rule set, three files a worker: nothing is loaded for a worker or for a file.
    let words: Arc<dyn WalkEngine> = Arc::new(Walking);
    let mut found = 0_usize;
    for load in &loads {
        let (columns, places) = (
            emit_columns(&fake, &load.programs),
            place_columns(&load.accepted, &load.pack.emits),
        );
        let run = ScanRun {
            programs: &load.programs,
            columns: &columns,
            places: &places,
        };
        for _ in 0..2 {
            let mut worker = worker(&words, &fake, &load.programs, ScanMeasuring::Plain);
            let files: Vec<WalkRequest<'_>> = (0..3)
                .map(|number| WalkRequest {
                    path: SourcePath("a.walk"),
                    file: FileId(number),
                    source: ByteSlice(b"one two"),
                })
                .collect();
            let _counted = ran(&fake, &run, &mut worker, &files);
            found = found.saturating_add(worker.report.found.len());
        }
    }
    let given = fake
        .statistics
        .lock()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    Ok(Cow::Owned(format!(
        "{} rule sets; {before} lowerings; {} lowerings after the files, {found} rows found; statistics {given:?}",
        loads.len(),
        fake.lowered.load(Ordering::Relaxed)
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
