//! The repository hex's adapters back to back: the disk and the memory adapter each make a project, write a tree of files
//! into it, walk it and read every file back, so the faster is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot set up its own input has nothing to measure: the panic is the failure"
)]

use tastemaker_core::domain::files::types::{
    DirectoryName, FileText, MachinePath, SkippedDirectory,
};
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::outbound::repository::factory::{disk, memory};
use tastemaker_core::outbound::repository::interface::RepositoryPort;

/// How many files each round writes, walks and reads.
const FILES: usize = 200;

/// The directory a store lives in, which no walk enters.
const STORE: &str = ".tastemaker_cache";

/// The repository adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The file system.
    Disk,
    /// Scratch directories held in memory.
    Memory,
}

impl Variant {
    /// Every adapter, run on the same tree.
    const ALL: [Self; 2] = [Self::Disk, Self::Memory];

    /// The adapter this variant is, built through its factory function.
    fn build(self) -> Box<dyn RepositoryPort> {
        let skipped: Vec<SkippedDirectory> = Vec::new();
        let store = DirectoryName(STORE.into());
        match self {
            Self::Disk => Box::new(disk(skipped, store)),
            Self::Memory => Box::new(memory(skipped, store)),
        }
    }
}

fn main() {
    divan::main();
}

/// Makes a project, writes `FILES` files into it, walks it and reads every file back.
#[divan::bench(args = Variant::ALL)]
fn project_walk_and_read(variant: Variant) -> usize {
    static ROUNDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let parent = std::env::temp_dir().join("tastemaker-repository-bench");
    let round = ROUNDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = MachinePath(format!("{}/{}-{round}", parent.display(), std::process::id()).into());
    let adapter = variant.build();
    let project = adapter.project(&root).expect("the project is made");
    for number in 0..FILES {
        project
            .write(
                &RepoPath(format!("src/{}/file-{number}.txt", number % 10).into()),
                &FileText(format!("text {number}\n").into()),
            )
            .expect("the file is written");
    }
    let walked = project
        .walk(&RepoPath::default())
        .expect("the tree is walked");
    walked
        .files
        .iter()
        .map(|file| project.read(file).expect("the file is read").0.len())
        .sum()
}
