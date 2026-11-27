//! The vector executor: the check's rules lowered to plans and run over the columns of the walked files, one session for every
//! request of a run.
//!
//! The split line of this crate is "what names an SDK type": the executor and its session, its columns, its store and its
//! replay of recorded requests live here, below the SDK; the files that name an SDK type (the linked entry, the request and
//! report conversions, the call cache) stay in the session package above it.

pub mod bench_support;
pub mod check_session;
pub mod finish;
pub(crate) mod first;
pub(crate) mod glob_pattern;
pub(crate) mod host_columns;
pub(crate) mod languages;
pub mod lower;
pub(crate) mod pair_checks;
pub(crate) mod pair_skip;
pub(crate) mod pair_summary;
pub(crate) mod reader;
pub(crate) mod reader_run;
pub mod request_input;

use bench_support::thread_cpu;
use first::{Vocab, span_bytes};
use glob_pattern::glob_pattern;
use request_input::rule_context;
