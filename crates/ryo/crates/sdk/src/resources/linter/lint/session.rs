//! The session the SDK's lint runs on: the executor of the check, called in-process with this crate's typed requests. What names an SDK
//! type (the entry that admits a request, the call cache, the linked languages, the request's named outputs) is here, above the
//! executor crate, which knows no SDK type.

#![allow(
    dead_code,
    reason = "the session's diagnostic entry points are read by the images' tools, not by the lint"
)]

mod call_cache;
mod entry;
mod linked;
mod measure;
mod modules;
mod sdk_request;

use crate::resources::linter as sdk;
use call_cache::{CallCache, PathFilter};
use linked::Linked;
use ryo_executor::bench_support::thread_cpu;
use ryo_executor::check_session::hosted::{Hosted, Parameters};
use ryo_executor::check_session::project::Authority;
use ryo_executor::check_session::replay::{self, Active, Batch, Input, LiveAdmission};
use ryo_values::domain::finish_plans::expressions::types::{ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use super::executor::{finding, report, unknown, unread};

type Reply = Result<sdk::LintReport, sdk::LintError>;

pub(super) use entry::{execute_round, execute_rows, release_session, take_not_file_local};
/// Adds worker CPU nanoseconds to the measurement the images print.
#[cfg(feature = "bench-hooks")]
pub fn add_work_nanos(nanos: u64) {
    measure::add_work_nanos(nanos);
}

/// Adds dispatch CPU nanoseconds to the measurement the images print.
#[cfg(feature = "bench-hooks")]
pub fn add_dispatch_nanos(nanos: u64) {
    measure::add_dispatch_nanos(nanos);
}

/// Prints the counters the session kept, as the images' tools read them.
#[cfg(feature = "bench-hooks")]
pub fn print_measurement() {
    measure::print_measurement();
}
use measure::MEASURED;

/// Whether the session reads a file listing in reverse order: a diagnostic setting of the images, off in the product.
static REVERSE_FILES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sets the diagnostic order of file listings (the images' `--reverse-files`).
#[cfg(feature = "bench-hooks")]
pub fn configure_reverse_files(reverse: bool) {
    REVERSE_FILES.store(reverse, Ordering::Relaxed);
}

fn reverse_files() -> bool {
    REVERSE_FILES.load(Ordering::Relaxed)
}

/// The languages the session reads, each as the executor's `Source`.
pub(crate) fn natives(languages: impl IntoIterator<Item = crate::resources::languages::Language>) -> Vec<Linked> {
    languages.into_iter().map(Linked::from).collect()
}
