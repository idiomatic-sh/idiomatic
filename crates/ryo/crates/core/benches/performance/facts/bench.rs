//! core's facts workload.
//!
//! The facts hex's adapters back to back: the memory adapter and the spill adapter, in a directory and in the system's temporary one,
//! each give two workers the rows of a few files and the words they keep, and finish them, so the fastest is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot set up its own input has nothing to measure: the panic is the failure"
)]

use ryo_execution::outbound::facts::factory::{memory, spill, spill_scratch};
use ryo_execution::outbound::facts::interface::FactsPort;
use ryo_execution::outbound::facts::interface::FactsWorker;
use ryo_execution::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::records::types::{
    FrontierCoverage, FrontierOrdinal, FrontierProof, ProducerRule,
};
use ryo_values::domain::stored_facts::types::{Bucket, Digest, PackedWords};

/// How many files each worker reads.
const FILES: u64 = 50;

/// How many workers read the files.
const WORKERS: usize = 2;

/// The bucket the files keep their words in.
const BUCKET: Bucket = Bucket(3);

/// The facts adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// Everything in memory.
    Memory,
    /// The kept words in a file of a directory.
    Spill,
    /// The kept words in a file of the system's temporary directory.
    SpillScratch,
}

impl Variant {
    /// Every adapter, run on the same files.
    const ALL: [Self; 3] = [Self::Memory, Self::Spill, Self::SpillScratch];

    /// The adapter this variant is, built through its factory function; the spill adapter keeps its file in `directory`.
    fn build(self, directory: &std::path::Path) -> Box<dyn FactsPort> {
        match self {
            Self::Memory => Box::new(memory()),
            Self::Spill => Box::new(spill(directory.to_path_buf())),
            Self::SpillScratch => Box::new(spill_scratch()),
        }
    }
}

/// Each worker writes a row of `FILES` files, keeps words for each and releases it; the finish brings the workers' words together and
/// the bucket is read back.
#[divan::bench(args = Variant::ALL)]
fn read_keep_and_finish(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    let directory = tempfile::tempdir().expect("the directory is made");
    let adapter = variant.build(directory.path());
    let words = PackedWords((0..64).collect());
    let proof = FrontierProof {
        rule: ProducerRule(0),
        input: FrontierOrdinal(0),
        rule_digest: Digest([0; 16]),
        coverage: FrontierCoverage::Complete,
    };
    bencher.bench_local(|| {
        let workers = (0..WORKERS)
            .map(|at| {
                let mut worker = adapter.worker().expect("the worker is opened");
                for file in 0..FILES {
                    let id = FileId(
                        u32::try_from(
                            u64::try_from(at).expect("a small worker number") * FILES + file,
                        )
                        .expect("a small file id"),
                    );
                    for column in worker
                        .columns_mut()
                        .expect("the columns are given")
                        .function
                        .iter_mut()
                    {
                        column.0.push(file);
                    }
                    worker
                        .keep(
                            BUCKET,
                            WordsToKeep {
                                words: &words,
                                file: id,
                                proof: &proof,
                            },
                        )
                        .expect("the words are kept");
                    worker.release().expect("the rows are released");
                }
                worker
            })
            .collect::<Vec<Box<dyn FactsWorker>>>();
        let finished = adapter
            .finish(&mut workers.into_iter())
            .expect("the words are finished");
        finished
            .kept(BUCKET)
            .expect("the bucket is read")
            .words
            .0
            .0
            .len()
    });
}
