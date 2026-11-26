//! The pinned product calls the shared session in-process, through its typed SDK edge. The executor itself is the `ryo_executor` crate; what
//! names an SDK type (the linked entry, the request and report conversions, the call cache, the linked languages) is here, above it.
extern crate ryo_sdk as ryo;

mod calls;
mod linked;
#[path = "request_output.rs"]
mod json_output;

pub use calls::{link_executor, measure_dispatch, configure_bench_check, configured_workers, exit_process, HostPhase, host_snapshot, host_wall, capture, capture_then, capture_retained, capture_one, print_measurement};

// A pair of private executable aliases selects the gate's receipt collection.
// The check and the SDK have no runtime switch for their execution path.
fn proof_alias() -> Option<&'static std::ffi::OsStr> {
    use std::sync::OnceLock;
    static NAME: OnceLock<Option<std::ffi::OsString>> = OnceLock::new();
    NAME.get_or_init(|| {
        let arg = std::env::args_os().next()?;
        std::path::Path::new(&arg).file_name().map(ToOwned::to_owned)
    }).as_deref()
}

fn proof_enabled() -> bool {
    proof_alias().is_some_and(|name| name == "tm-sdk-serial-proof" || name == "tm-sdk-rounds-proof")
}

pub fn serial_host() -> bool {
    proof_alias().is_some_and(|name| name == "tm-sdk-serial" || name == "tm-sdk-serial-proof")
}
