//! The allocation rows of the whole per-file path, over the fake language. The first: `lint` over the core's own corpus and every
//! operator's rule, and the most blocks a file allocates for it beyond what the parser does on its own, which for the fake is
//! nothing. The second: the blocks of a megabyte or more a run allocates are bounded by its workers and its rules, never by its
//! files, with the breach planted beside it. No language's corpus is read here: a language's own allocation test is
//! `tests/allocation.rs` of the language. The measuring is the core's contract, the same for every product. It runs alone in its
//! process, with `dhat` as the global allocator, so nothing else counts. Run as a test it holds the rows (the rules give their
//! rows, streaming and output); run with `--bmf` it prints the measure as one Bencher Metric Format document on stdout, which
//! asymptote holds to its best.

use ryo_core::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use ryo_core::inbound::use_cases::lint::fixtures::{corpus, measures};

/// The allocator that counts every block, so the row measures what the path allocates.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

/// What the parser does for a file by itself: the fake language reads a file without allocating.
fn library(_bytes: &[u8]) {}

/// A megabyte: a site whose blocks average this many bytes or more allocates big blocks.
const BIG: u32 = 1 << 20;

/// How many tables a worker, or a rule, owns at most: the steps and inputs of the widest plan, and its scratch.
const TABLES: usize = 64;

/// The big blocks the plant allocates beside the run, as a run that allocated a table again for each of a thousand files would.
const PLANTED: usize = 1_000;

/// What is allocated beside the run, under the same profile.
#[derive(Clone, Copy, Debug)]
enum Beside {
    /// Nothing.
    Nothing,
    /// The plant: [`PLANTED`] blocks of [`BIG`] bytes.
    BlockPerFile,
}

/// The big blocks a run allocated, beyond its bound.
#[derive(Debug, PartialEq, Eq)]
struct Over {
    /// The blocks of a megabyte or more.
    big: u64,
    /// The most its workers and rules own.
    most: u64,
}

type Input = Beside;
/// Whether the profile saw the run's allocations, the bound held.
type Value = bool;
type Failure = Over;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_big_blocks_of_a_run_are_bounded_by_the_workers_and_the_rules"),
        word: Word::Succeeds,
        input: || Beside::Nothing,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_big_block_allocated_again_for_each_file_breaks_the_bound"),
        word: Word::Refuses,
        input: || Beside::BlockPerFile,
        expect: Expect::Refusal(|over| selects(Matched(over.big > over.most))),
    },
];

/// Profiles the big-blocks run, with `beside` allocated under the same profile, and counts the blocks allocated at the sites whose
/// blocks average a megabyte or more, from the profile `dhat` writes.
#[expect(
    clippy::expect_used,
    reason = "a profile that cannot be written or read has nothing to count: the panic is the failure of the row"
)]
fn big_blocks(beside: Beside) -> Result<Value, Failure> {
    let file = std::env::temp_dir().join(format!("ryo-core-blocks-{}.json", std::process::id()));
    let ((workers, rules), gmax) = {
        let _profiler = dhat::Profiler::builder().file_name(&file).build();
        let bound = measures::blocks();
        if let Beside::BlockPerFile = beside {
            for _ in 0..PLANTED {
                drop(std::hint::black_box(Vec::<u8>::with_capacity(
                    usize::try_from(BIG).unwrap_or(usize::MAX),
                )));
            }
        }
        // The bytes live when the run held most, read before the profiler ends and writes its file.
        (bound, dhat::HeapStats::get().max_bytes)
    };
    let text = std::fs::read_to_string(&file).expect("the profile");
    // The profile is a temporary file: one left behind, when it cannot be removed, changes no count.
    drop(std::fs::remove_file(&file));
    let sites: serde_json::Value = serde_json::from_str(&text).expect("a profile");
    let big: u64 = sites
        .get("pps")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|site| {
            let (bytes, blocks) = (site.get("tb")?.as_f64()?, site.get("tbk")?.as_f64()?);
            (blocks > 0.0_f64 && bytes / blocks >= f64::from(BIG)).then_some(site.get("tbk")?.as_u64()?)
        })
        .sum();
    let most = u64::try_from((workers + rules) * TABLES).unwrap_or(u64::MAX);
    if big > most {
        return Err(Over { big, most });
    }
    // A profile that saw no allocation counted nothing: the allocator is the profiler's.
    Ok(gmax > 0)
}

/// Runs the cases on libtest-mimic, which answers libtest's command line and reports as cargo's harness does; a red exits 101, as
/// a failed cargo test does. The cases run on this thread, one after the other, so no other thread allocates beside them.
#[expect(
    clippy::print_stdout,
    reason = "with `--bmf` the document on stdout is what the asymptote tool reads"
)]
fn main() {
    if std::env::args().any(|each| each == "--bmf") {
        let beyond = measures::allocation(&corpus::fake_corpus(), &library);
        println!("{{\"fake\":{{\"blocks-per-file-beyond-parser\":{{\"value\":{beyond}}}}}}}");
    } else {
        let mut arguments = libtest_mimic::Arguments::from_args();
        arguments.test_threads = Some(1);
        let cases = vec![
            libtest_mimic::Trial::test(
                "allocation::core::reading_allocates_as_the_parser_library_does",
                || {
                    let _beyond = measures::allocation(&corpus::fake_corpus(), &library);
                    Ok(())
                },
            ),
            libtest_mimic::Trial::test("allocation::core::big_blocks::succeeds", || {
                run(CASES, Word::Succeeds, big_blocks);
                Ok(())
            }),
            libtest_mimic::Trial::test("allocation::core::big_blocks::refuses", || {
                run(CASES, Word::Refuses, big_blocks);
                Ok(())
            }),
        ];
        libtest_mimic::run(&arguments, cases).exit();
    }
}
