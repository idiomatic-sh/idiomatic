//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{finish, json_input, json_output, lower};
use json_input::{Project as Request, Hosted, HostCell, Parameters, read_parameters};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

include!("../../../ryo/crates/engine/benches/vec/check_session/hosted.rs");

#[path = "batch.rs"]
mod batch;


#[cfg(feature = "product-session")]
pub use batch::calls::{link_executor, measure_dispatch, configure_bench_check, configured_workers, exit_process, HostPhase, host_snapshot, host_wall, capture, capture_then, capture_retained, capture_one, print_measurement};
