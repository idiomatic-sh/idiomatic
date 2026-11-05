//! Repository opening preserves inventory order and reads the prior store once.
use super::service::list;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::render::types::Answering;
use crate::outbound::{
    corpus::fixtures::fake::CorpusFake, lint_store::fixtures::fake::LintStoreFake,
    memory::fixtures::fake::MemoryFake, scheduler::fixtures::fake::SchedulerFake,
};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusFile, ExecutablePermission, FileBytes};
use std::sync::atomic::Ordering;

type Input = fn() -> Result<Value, Failure>;
type Value = &'static str;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("opening_reads_both_sources_and_orders_inventory_with_one_worker"),
    word: Word::Succeeds,
    input: || open_both,
    expect: Expect::Value("a.py,b.py;1"),
}];

fn open_both() -> Result<Value, Failure> {
    let corpus = CorpusFake {
        files: ["b.py", "a.py"]
            .into_iter()
            .map(|path| CorpusFile {
                executable: ExecutablePermission::Unknown,
                path: FilePath(path.into()),
                bytes: FileBytes(Vec::new()),
            })
            .collect(),
        ..CorpusFake::default()
    };
    let store = LintStoreFake::default();
    let listing = list(
        &corpus,
        &store,
        &MemoryFake::default(),
        &SchedulerFake::default(),
        crate::outbound::scheduler::types::WorkerCount(1),
        &Answering::Findings,
    )
    .map_err(|_| ())?;
    let paths = listing
        .files
        .iter()
        .map(|file| {
            ryo_values::domain::files::service::input_path(file)
                .0
                .as_ref()
        })
        .collect::<Vec<&str>>()
        .join(",");
    let shown = format!("{paths};{}", store.reads.load(Ordering::Relaxed));
    assert!(listing.read.is_some());
    Ok(Box::leak(shown.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |subject| subject());
}
