//! The runtime's facts finish, counted in heap blocks and bytes in a process of its own: a read of a stored
//! population lends the worker's words, never a copy of them.
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsPort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// How many packed words the stored population of the facts finish case holds.
const WORDS: usize = 4096;

/// What one read of a stored population from the memory facts finish allocated.
#[derive(Debug, PartialEq)]
struct FinishRead {
    /// The words the read handed back.
    kept_words: usize,
    /// Whether the read made at most two blocks: one for the sources and one for the joined list, none for the words.
    /// Before one copy (checkpoint 322) the same read made four: the clone of the words and the sources, then the
    /// concatenation of each.
    within_two_blocks: bool,
    /// Whether the read allocated as many bytes as the words hold, that is, copied them.
    copied_the_words: bool,
}

type Input = usize;
type Value = FinishRead;
type Failure = FactsError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_facts_finish_reads_a_stored_population_without_copying_its_words"),
    word: Word::Succeeds,
    input: || WORDS,
    expect: Expect::Value(FinishRead {
        kept_words: WORDS,
        within_two_blocks: true,
        copied_the_words: false,
    }),
}];

/// Keeps `words` packed words in one memory worker, finishes it, and counts what one read of the bucket allocates.
fn subject(words: Input) -> Result<Value, Failure> {
    let facts = ryo_execution_runtime::outbound::facts::factory::memory();
    let worker = facts.worker()?;
    let proof = ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(0),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    };
    worker.keep(
        ryo_values::domain::stored_facts::types::Bucket(3),
        ryo_execution_runtime::outbound::facts::types::WordsToKeep {
            words: &ryo_values::domain::stored_facts::types::PackedWords(vec![1; words]),
            file: ryo_relations::domain::relations::types::FileId(0),
            proof: &proof,
        },
    )?;
    let finished = facts.finish(&mut std::iter::once(worker))?;
    let before = dhat::HeapStats::get();
    let kept = finished.kept(ryo_values::domain::stored_facts::types::Bucket(3))?;
    let after = dhat::HeapStats::get();
    let bytes = after.total_bytes - before.total_bytes;
    Ok(FinishRead {
        kept_words: kept.words.0.0.len(),
        within_two_blocks: after.total_blocks - before.total_blocks <= 2,
        copied_the_words: usize::try_from(bytes).map_or(true, |bytes| bytes >= words),
    })
}

fn main() {
    // This target is alone in its process; its rows count blocks and bytes, never elapsed time.
    // libtest-mimic answers libtest's command line (`--list`, `--exact`, `--skip`, filters), one trial per case.
    let mut arguments = libtest_mimic::Arguments::from_args();
    // dhat counts the whole process's heap, so the trials run one at a time, on this thread.
    arguments.test_threads = Some(1);
    let profiler = (!arguments.list).then(|| dhat::Profiler::builder().testing().build());
    let trials = CASES
        .iter()
        .map(|case| {
            libtest_mimic::Trial::test(case.name.0, move || {
                run(std::slice::from_ref(case), Word::Succeeds, subject);
                Ok(())
            })
        })
        .collect();
    let conclusion = libtest_mimic::run(&arguments, trials);
    drop(profiler);
    // A run that names no filter, skip or listing executes a case: an empty table is refused.
    let unfiltered = !arguments.list
        && !arguments.ignored
        && arguments.filter.is_none()
        && arguments.skip.is_empty();
    assert!(
        !unfiltered || conclusion.num_passed + conclusion.num_failed > 0,
        "the allocation target must execute an actual case"
    );
    conclusion.exit();
}
