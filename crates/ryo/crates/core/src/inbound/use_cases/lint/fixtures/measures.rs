//! What the whole per-file path must pass over the corpus: its allocation, its call count, its streaming, its output and its big blocks.

#![expect(
    clippy::expect_used,
    reason = "a measure that cannot keep its store has nothing to measure: the panic is the failure of the row"
)]

use super::corpus::Corpus;
use super::corpus::files;
use super::runs::Written;
use super::runs::agrees;
use super::runs::given;
use super::runs::lint;
use super::runs::WORKERS;
use super::runs::lint_keeping;
use super::runs::run_query;
use super::runs::write_all;
use crate::domain::allocation_verdict::service::{differences, most};
use crate::domain::allocation_verdict::types::AllocatedBlocks;
use crate::domain::allocation_verdict::types::InputBytes;
use crate::domain::allocation_verdict::types::JobBlocks;
use crate::domain::allocation_verdict::types::ParserBlocks;
use crate::domain::corpus_copies::types::CopyDigits;
use crate::domain::corpus_copies::types::CopyNaming;
use crate::outbound::row_sink::fixtures::fake::BytesRowSinkFake;
use crate::outbound::row_sink::fixtures::fake::TallyRowSinkFake;
use crate::outbound::scheduler::fixtures::fake::CountingSchedulerFake;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use ryo_execution_runtime::outbound::facts;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use std::sync::Arc;
use std::sync::PoisonError;

/// How many copies of each input the allocation corpus holds.
pub(super) const FILES: u32 = 60;

/// How many times the rows of every file the larger call-count corpus has.
pub(super) const TIMES: usize = 10;

/// How many files the big-blocks run lints: more than its rules and its one worker own tables.
const BLOCK_FILES: u32 = 300;

/// The rows beyond the first the fake language reads of each file of the big-blocks run: a file's tables stay well under a megabyte,
/// and what every file hands the rule across them is several megabytes, so a table of the whole project made again for each file is
/// a big block each time.
const BLOCK_ROWS: u32 = 1_000;

/// The rules of the big-blocks run, by id: one over each file, and one across them that joins every function of every file with
/// itself by its id, which no file can answer alone.
const BLOCK_RULES: [(&str, &str); 2] = [
    (
        "each-file",
        "let names = project(from=\"function\", columns=[\"name\", \"file\"]);\nemit names subject file key(name);",
    ),
    (
        "across-files",
        "let rows = join(left=\"function\", right=\"function\", left-on=[\"id\"], right-on=[\"id\"], output={id:[left,id],file:[left,file]});\nlet counted = group-count(from=rows, keys=[\"file\"], count=\"functions\");\nemit counted subject file key(file);",
    ),
];

/// The allocation row of the whole per-file path: the most blocks a file, after warm-up, allocates beyond what the parser library
/// does on its own for it (`library` does only that). It is a measure, not a bound: the target is one block, for the findings the
/// files leave, a vector that doubles now and then; asymptote's `memory-allocation` holds the measure to its best. The row also holds
/// that every rule gives its rows for every copy, and runs the streaming and output rows.
///
/// # Panics
///
/// When a rule does not give its fixture's rows, or the streaming or output rows fail.
pub fn allocation(corpus: &Corpus, library: &dyn Fn(&[u8])) -> u64 {
    let _profiler = dhat::Profiler::builder().testing().build();
    let scheduler = Arc::new(CountingSchedulerFake::default());
    let once = given(corpus, Arc::<CountingSchedulerFake>::clone(&scheduler));
    agrees(&once, corpus);

    // The copies: the first lint grows what a run keeps, the way a program's first files do; the second is the one counted.
    let copies = || {
        files(
            corpus,
            CopyNaming::Copy {
                digits: CopyDigits(3),
            },
            FILES,
            1,
        )
    };
    let counted = Arc::new(TallyRowSinkFake::default());
    for sink in [Arc::new(TallyRowSinkFake::default()), Arc::clone(&counted)] {
        let facts = Arc::new(facts::factory::memory());
        lint(
            corpus,
            Arc::<CountingSchedulerFake>::clone(&scheduler),
            facts,
            copies(),
            sink,
        );
    }
    let counted = counted
        .counts
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for (id, rows) in &once {
        let every_copy = rows.len() * usize::try_from(FILES).unwrap_or(usize::MAX);
        let gave = counted.get(id).copied().unwrap_or(0);
        assert_eq!(gave, every_copy, "{id} gives its rows for every copy");
    }
    let jobs = scheduler
        .blocks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();

    // The parser library alone, for the file each job read: the files are taken in the order of their paths.
    let mut listed = copies();
    listed.sort_by(|left, right| left.path.cmp(&right.path));
    let mut parser = ParserBlocks::default();
    for file in &listed {
        let content = InputBytes(file.bytes.0.clone());
        let _: &mut AllocatedBlocks = parser.0.entry(content).or_insert_with(|| {
            library(&file.bytes.0);
            let before = dhat::HeapStats::get().total_blocks;
            library(&file.bytes.0);
            AllocatedBlocks(dhat::HeapStats::get().total_blocks - before)
        });
    }
    let beyond = most(&differences(&JobBlocks(jobs), &listed, &parser)).0;
    streaming(corpus);
    output_path(corpus, library);
    beyond
}

/// The run the big-blocks row profiles: the fake language reads each of [`BLOCK_FILES`] files as a table of [`BLOCK_ROWS`] rows, for
/// a rule over each file and one across them, on one worker, keeping its store on disk as a lint does. What
/// is big in a worker is owned by the worker and kept between files, and what is big in a rule across files is the tables of its
/// plan, so the blocks of a megabyte or more the run allocates are bounded by the workers and the rules, never by the files. Answers
/// the workers it ran on and the rules, which bound them.
///
/// # Panics
///
/// When a file is left unread or a rule gives no row.
#[must_use]
pub fn blocks() -> (usize, usize) {
    let corpus = Corpus {
        language: Arc::new(crate::outbound::language::fixtures::fake::LanguageFake {
            extra_rows: BLOCK_ROWS,
            constant_name: true,
            ..Default::default()
        }),
        rules: BLOCK_RULES
            .iter()
            .map(|(id, query)| ((*id).to_owned(), ((*query).to_owned(), "**/*.rs".to_owned())))
            .collect(),
        ..super::corpus::fake_corpus()
    };
    let copies = files(
        &corpus,
        CopyNaming::Copy {
            digits: CopyDigits(3),
        },
        BLOCK_FILES,
        1,
    );
    let cache = tempfile::tempdir().expect("a cache directory");
    let found = Arc::new(TallyRowSinkFake::default());
    lint_keeping(
        &corpus,
        Arc::new(SchedulerFake::default()),
        Arc::new(facts::factory::memory()),
        copies,
        Arc::<TallyRowSinkFake>::clone(&found),
        Arc::new(crate::outbound::lint_store::factory::postcard(
            crate::outbound::lint_store::types::Store {
                directory: crate::outbound::lint_store::types::CacheDirectory(
                    cache.path().to_string_lossy().into(),
                ),
                build: crate::outbound::lint_store::types::BuildIdentity("blocks".into()),
            },
        )),
    );
    let counts = found.counts.lock().unwrap_or_else(PoisonError::into_inner);
    for (id, _) in BLOCK_RULES {
        assert!(
            counts.get(id).copied().unwrap_or(0) > 0,
            "the rule {id} gives its rows"
        );
    }
    (
        usize::try_from(WORKERS.0).unwrap_or(usize::MAX),
        BLOCK_RULES.len(),
    )
}

/// The call-count row of the whole per-file path: a port is called for a file, never for a row.
/// The corpus and the same corpus with ten times the rows in every file call the facts port as often.
/// Its rules give the rows their fixtures state first, so a fixture nothing gives fails here.
///
/// # Panics
///
/// When a rule does not give its fixture's rows, the larger files do not have more rows, or the
/// port was called more often for them.
pub fn call_count(corpus: &Corpus) {
    // The rows first: each rule of the corpus gives what its fixture states, the operators' and the relations'.
    agrees(&given(corpus, Arc::new(SchedulerFake::default())), corpus);
    let counted = |times: usize| -> (usize, usize) {
        let facts = Arc::new(FactsFake::default());
        let found = Arc::new(TallyRowSinkFake::default());
        let files = files(corpus, CopyNaming::Input, 1, times);
        lint(
            corpus,
            Arc::new(SchedulerFake::default()),
            Arc::<FactsFake>::clone(&facts),
            files,
            Arc::<TallyRowSinkFake>::clone(&found),
        );
        let tallies = found.counts.lock().unwrap_or_else(PoisonError::into_inner);
        (tallies.values().sum(), facts.calls())
    };
    let ((few, few_calls), (many, many_calls)) = (counted(1), counted(TIMES));
    assert!(many > few, "the larger files do have more rows");
    assert_eq!(
        many_calls, few_calls,
        "the facts port was called more often for the larger files"
    );
}

/// The streaming row: what `run` and `lint` write does not wait in memory. Ten times the files
/// write ten times the bytes, and the most memory the process holds grows, beyond the findings the
/// lint keeps in its store for the next run, by less than a fifth of the bytes written, whatever the
/// rows are: the files themselves are what grows.
///
/// # Panics
///
/// When the larger run holds on to what it wrote.
pub fn streaming(corpus: &Corpus) {
    let warm = write_all(corpus, FILES);
    let before = dhat::HeapStats::get().max_bytes;
    let Written {
        bytes: written,
        kept,
    } = write_all(corpus, FILES * 10);
    assert!(written > warm.bytes, "ten times the files write more bytes");
    let grown = dhat::HeapStats::get()
        .max_bytes
        .saturating_sub(before)
        .saturating_sub(kept);
    assert!(
        written > 0 && grown * 5 < written,
        "the larger run grew the peak by {grown} bytes beyond the {kept} its store keeps while writing {written}: its output is held, not streamed"
    );
}

/// The output-path row: once the workers have grown their buffers, writing the rows of more files with ten times the rows allocates
/// nothing, but for the buffers that double now and then. Rows are formatted into the buffers a worker reuses, never into a string or
/// a vector of their own.
///
/// # Panics
///
/// When more files allocate more blocks than the doubling of the buffers explains.
pub(crate) fn output_path(corpus: &Corpus, library: &dyn Fn(&[u8])) {
    let blocks = || dhat::HeapStats::get().total_blocks;
    let beyond = |copies: u32, limit: u64| -> i64 {
        let before = blocks();
        run_query(
            corpus,
            copies,
            TIMES,
            limit,
            Arc::new(BytesRowSinkFake::default()),
        );
        let run = i64::try_from(blocks() - before).unwrap_or(i64::MAX);
        let parsing = blocks();
        for _ in 0..copies {
            for bytes in corpus.inputs.0.values() {
                library(&bytes.0.repeat(TIMES));
            }
        }
        run - i64::try_from(blocks() - parsing).unwrap_or(i64::MAX)
    };
    // Everything a file costs beyond the parser is the evaluation's, which the rows do not change:
    // what writing the rows costs is the difference between a run that writes them and one that writes none.
    let written = beyond(FILES * 4, u64::MAX) - beyond(FILES, u64::MAX);
    let counted = beyond(FILES * 4, 0) - beyond(FILES, 0);
    let extra = written - counted;
    assert!(
        extra <= 16 * i64::from(FILES.ilog2()),
        "{} more files with ten times the rows allocated {extra} blocks to write their rows: the output path allocates for rows",
        FILES * 3
    );
}
