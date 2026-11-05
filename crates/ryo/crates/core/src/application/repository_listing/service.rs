//! Acquire the inventory and previous store concurrently, then validate the budget.
use super::types::RepositoryListing;
use crate::application::job_execution::service::run_indexed;
use crate::domain::budget::errors::BudgetError;
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
    memory::{errors::MemoryError, interface::MemoryPort},
    scheduler::{errors::SchedulerError, interface::SchedulerPort, types::WorkerCount},
};
use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ListingError {
    #[error(transparent)]
    Corpus(#[from] CorpusError),
    #[error(transparent)]
    Store(#[from] LintStoreError),
    #[error(transparent)]
    Memory(#[from] MemoryError),
    #[error(transparent)]
    Budget(#[from] BudgetError),
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
}

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
) -> Result<RepositoryListing, ListingError> {
    let opened = run_indexed(scheduler, workers, 2, |job| {
        if job.0 == 0 {
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
