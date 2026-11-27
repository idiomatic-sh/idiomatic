//! A request answered as rows, for `query run` alone: every row its queries emit, the first `limit` of them rendered and all of them
//! counted. It is the SDK's own, not the linter's: nothing outside the crate calls it, and the linter's public surface stays `new` and `lint`.

use std::sync::Arc;

use ryo_core::outbound::memory::interface::MemoryPort as _;
use ryo_values::domain::runs::types::{Budget, MemoryBytes};

use super::errors::LintError;
use super::types::{LintReport, LintRequest, Linter, Port};
use crate::domain::capacity::types::MemoryBudget;

/// The budget as the memory port takes it: none means the machine's.
fn budgeted(memory: MemoryBudget) -> Budget {
    match memory {
        MemoryBudget::Bytes(bytes) => Budget::Bytes(MemoryBytes(bytes.get())),
        MemoryBudget::Machine => Budget::Machine,
    }
}

/// Answers `request` (a walked corpus) as rows over `linter`: the files are listed once, the bytes of the listing are held against
/// `memory` before any file is read, the rows are answered with `findings` holding at most `limit` of them and `rows_total` counting
/// every one, and the process is held against `memory` again when the run ends.
///
/// # Errors
///
/// `Io` of the corpus when the walk fails, `TooLarge` when the files do not fit `memory`, `MemoryBudgetExceeded` when the process holds
/// more than `memory` before or after the run, and what the linter refuses a request for.
pub(crate) fn answer(linter: &Linter, request: LintRequest, limit: usize, memory: MemoryBudget) -> Result<LintReport, LintError> {
    let engine = linter.engine();
    let rules: Vec<_> = request.rules.keys().cloned().collect();
    let (request, unreadable, size) = super::walk::listed_sized(request, engine)?;
    let port = ryo_core::outbound::memory::factory::sysinfo(budgeted(memory));
    let allowed = port
        .budget()
        .map_err(|error| LintError::Io { what: Port::Corpus, reason: format!("memory: {error}") })?
        .0;
    if size > allowed {
        return Err(LintError::TooLarge { size, budget: allowed });
    }
    let within = || -> Result<(), LintError> {
        let used = port
            .resident_bytes()
            .map_err(|error| LintError::Io { what: Port::Corpus, reason: format!("memory: {error}") })?
            .0;
        if used > allowed {
            return Err(LintError::MemoryBudgetExceeded { used, budget: allowed, plans: rules.clone() });
        }
        Ok(())
    };
    within()?;
    let Some(workers) = std::num::NonZeroUsize::new(engine.workers.0 as usize) else {
        return Err(LintError::Io { what: Port::Scheduler, reason: "the worker pool is empty".into() });
    };
    let mut request = Some(request);
    let mut answered = None;
    let mut job = || {
        if let Some(request) = request.take() {
            answered = Some(super::session::execute_rows(engine.session, request, &engine.languages, workers, &engine.scheduler, limit));
        }
    };
    let report = match Arc::clone(&engine.scheduler).on_owner(&mut job) {
        Ok(()) => answered.unwrap_or_else(|| Err(LintError::Execution { reason: "the executor answered no request".into() }))?,
        Err(reason) => return Err(LintError::Io { what: Port::Scheduler, reason: reason.to_string() }),
    };
    within()?;
    Ok(super::walk::naming_unread(report, unreadable))
}
