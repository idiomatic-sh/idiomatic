//! core's `lint_store` workload.
//!
//! The `lint_store` hex's adapters back to back: the postcard adapter in a cache directory, the memory adapter and the discarding one each
//! keep a small store and read it back, so the fastest is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot set up its own input has nothing to measure: the panic is the failure"
)]

use ryo_core::outbound::lint_store::factory::{discard, memory, postcard};
use ryo_core::outbound::lint_store::fixtures::fake::stored;
use ryo_core::outbound::lint_store::interface::LintStorePort;
use ryo_core::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows};

/// The build the store is kept for.
const BUILD: &str = "bench";

/// The `lint_store` adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// A file of the cache directory.
    Postcard,
    /// Memory, for the life of the adapter.
    Memory,
    /// No store.
    Discard,
}

impl Variant {
    /// Every adapter, run on the same store.
    const ALL: [Self; 3] = [Self::Postcard, Self::Memory, Self::Discard];

    /// The adapter this variant is, built through its factory function; the postcard adapter keeps its file in `directory`.
    fn build(self, directory: &std::path::Path) -> Box<dyn LintStorePort> {
        match self {
            Self::Postcard => Box::new(postcard(Store {
                directory: CacheDirectory(directory.display().to_string().into()),
                build: BuildIdentity(BUILD.into()),
            })),
            Self::Memory => Box::new(memory()),
            Self::Discard => Box::new(discard()),
        }
    }
}

/// Keeps a store with a row in every table, then reads the tables and what the files handed back.
#[divan::bench(args = Variant::ALL)]
fn keep_and_read_back(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    let directory = tempfile::tempdir().expect("the directory is made");
    let adapter = variant.build(directory.path());
    bencher.bench_local(|| {
        adapter
            .stored()
            .expect("the store is read")
            .keep(stored(1), &KeptRows::default())
            .expect("the store is kept");
        let read = adapter.stored().expect("the store is read");
        let handed = read
            .handed(&BucketsWanted::All)
            .expect("the handed words are read");
        (read.tables(), handed)
    });
}
