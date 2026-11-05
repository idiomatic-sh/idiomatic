//! Acquire the inventory and previous store concurrently, then validate the budget.
use super::types::RepositoryListing;
use crate::domain::{
    budget::service::{fit, repository_size},
    render::types::Answering,
    repository::service::by_path,
};
use crate::outbound::{
    corpus::{errors::CorpusError, interface::CorpusPort},
    lint_store::{
        errors::LintStoreError,
        interface::{LintStorePort, ReadStore},
    },
    memory::interface::MemoryPort,
    scheduler::{
        interface::SchedulerPort,
        types::{JobCount, JobNumber, WorkerCount},
    },
};
use ryo_values::domain::files::types::CorpusInput;
use std::sync::{Arc, Mutex, PoisonError};

use super::constants::OPEN_TASK_ANSWERED;
use super::errors::RepositoryListingError;

enum Opened {
    Files(Result<Vec<CorpusInput>, CorpusError>),
    Stored(Result<Option<Arc<dyn ReadStore>>, LintStoreError>),
}

pub(crate) fn list(
    corpus: &dyn CorpusPort,
    store: &dyn LintStorePort,
    memory: &dyn MemoryPort,
    scheduler: &dyn SchedulerPort,
    workers: WorkerCount,
    answering: &Answering,
) -> Result<RepositoryListing, RepositoryListingError> {
    let opened = open(scheduler, workers, |job| {
        if job == INVENTORY_JOB {
            Opened::Files(corpus.inventory().map(|inventory| inventory.0))
        } else if *answering == Answering::Findings {
            Opened::Stored(store.stored().map(Some))
        } else {
            Opened::Stored(Ok(None))
        }
    })?;
    let (mut listed, mut stored) = (Ok(Vec::new()), Ok(None));
    for each in opened {
        match each {
            Opened::Files(files) => listed = files,
            Opened::Stored(kept) => stored = kept,
        }
    }
    let mut files = listed?;
    let budget = memory.budget()?;
    fit(repository_size(&files), budget)?;
    by_path(&mut files);
    let read = stored?;
    Ok(RepositoryListing {
        files,
        stored: read
            .as_ref()
            .map_or_else(Default::default, |read| read.tables()),
        read,
        budget,
    })
}

const OPEN_JOBS: u32 = 2;
const INVENTORY_JOB: JobNumber = JobNumber(0);

#[expect(
    clippy::expect_used,
    reason = "a completed scheduler must answer each requested opening job"
)]
fn open(
    scheduler: &dyn SchedulerPort,
    workers: WorkerCount,
    work: impl Fn(JobNumber) -> Opened + Sync,
) -> Result<Vec<Opened>, RepositoryListingError> {
    let slots: Vec<Mutex<Option<Opened>>> = std::iter::repeat_with(|| Mutex::new(None))
        .take(OPEN_JOBS as usize)
        .collect();
    let mut jobs: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
    for _ in 0..workers.0.clamp(1, OPEN_JOBS) {
        let (slots, work) = (&slots, &work);
        jobs.push(Box::new(move |job| {
            let answer = work(job);
            #[expect(
                clippy::indexing_slicing,
                reason = "scheduler jobs are within the requested count"
            )]
            let slot = &slots[job.0 as usize];
            *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(answer);
        }));
    }
    scheduler.run(&mut jobs, JobCount(OPEN_JOBS))?;
    drop(jobs);
    Ok(slots
        .into_iter()
        .map(|slot| {
            slot.into_inner()
                .unwrap_or_else(PoisonError::into_inner)
                .expect(OPEN_TASK_ANSWERED)
        })
        .collect())
}
