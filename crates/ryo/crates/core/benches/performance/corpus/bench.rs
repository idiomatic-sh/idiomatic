//! core's corpus workload.
//!
//! The corpus hex's adapters back to back: the disk adapter walking a small tree and the memory adapter holding the same files each list
//! the files and read every one back, so the faster is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot set up its own input has nothing to measure: the panic is the failure"
)]

use ryo_core::outbound::corpus::factory::{ignore, memory};
use ryo_core::outbound::corpus::interface::CorpusPort;
use ryo_core::outbound::corpus::types::SkippedDirectories;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::{CorpusFile, CorpusListing, FileBytes};

/// How many files the tree holds.
const FILES: usize = 200;

/// The corpus adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The files on disk below a root, walked.
    Ignore,
    /// The same files, held in memory.
    Memory,
}

impl Variant {
    /// Every adapter, run on the same tree.
    const ALL: [Self; 2] = [Self::Ignore, Self::Memory];

    /// The adapter this variant is over the tree at `root` holding `files`, built through its factory function.
    fn build(self, root: &std::path::Path, files: &[CorpusFile]) -> Box<dyn CorpusPort> {
        match self {
            Self::Ignore => Box::new(ignore(
                RootPath(root.display().to_string().into()),
                CorpusListing::Walked,
                Vec::new(),
                SkippedDirectories::default(),
            )),
            Self::Memory => Box::new(memory(files.to_vec())),
        }
    }
}

/// The tree's files: `FILES` small source files over ten directories.
fn files() -> Vec<CorpusFile> {
    (0..FILES)
        .map(|number| CorpusFile {
            executable: Default::default(),
            path: FilePath(format!("src/{}/file_{number}.rs", number % 10).into()),
            bytes: FileBytes(format!("fn function_{number}() {{}}\n").into_bytes()),
        })
        .collect()
}

/// Lists the tree's files and reads every one back into one buffer.
#[divan::bench(args = Variant::ALL)]
fn inventory_and_read(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    let files = files();
    let root = tempfile::tempdir().expect("the root is made");
    for file in &files {
        let path = root.path().join(&*file.path.0);
        std::fs::create_dir_all(path.parent().expect("a file is in a directory"))
            .expect("the directory is made");
        std::fs::write(path, &file.bytes.0).expect("the file is written");
    }
    let adapter = variant.build(root.path(), &files);
    bencher.bench_local(|| {
        let mut buffer = FileBytes::default();
        let inventory = adapter.inventory().expect("the tree is listed");
        inventory
            .0
            .iter()
            .map(|entry| {
                adapter.read(entry, &mut buffer).expect("the file is read");
                buffer.0.len()
            })
            .sum::<usize>()
    });
}
