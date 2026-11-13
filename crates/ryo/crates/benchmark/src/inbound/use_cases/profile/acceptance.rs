//! The public profile run over the children and profiler doubles.
use cucumber::then;
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark::inbound::use_cases::profile::{
    fixtures::ChildrenFake,
    interface::ProfileUseCase as _,
    types::{Profile, ProfileReport, ProfileRequest},
    use_case::ProfileUseCaseImpl,
};
use ryo_benchmark::outbound::profile_tools::fixtures::fake::ProfileToolsFake;
use ryo_benchmark::outbound::profile_tools::types::{DumpTotal, HeapBytes};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::sync::Arc;

/// The profile run of `profile-ryo request.json out`, its worker writing `totals`.
fn profile(totals: Vec<DumpTotal>) -> Result<ProfileReport, ReportsError> {
    let tools = ProfileToolsFake {
        totals,
        ..ProfileToolsFake::default()
    };
    ProfileUseCaseImpl::new(Arc::new(ChildrenFake::default()), Arc::new(tools)).profile(
        ProfileRequest {
            worker: "target/worker".into(),
            arguments: ["profile-ryo", "request.json", "out"]
                .map(|word| BenchmarkArgument(word.into()))
                .to_vec(),
        },
    )
}

#[then("profile records the heap and CPU profiles and answers the trace command")]
fn records(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let report = profile(vec![DumpTotal {
        dump: "jeprof.1.0.f.heap".into(),
        bytes: HeapBytes(64),
    }]);
    assert!(report.is_ok_and(|report| {
        matches!(report.profile, Profile::Recorded(_))
            && report.command.last()
                == Some(&BenchmarkArgument(
                    std::path::Path::new("out")
                        .join("trace.json")
                        .to_string_lossy()
                        .into_owned(),
                ))
    }));
}

#[then("profile refuses a worker that writes no heap dump")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(profile(Vec::new()).is_err_and(|error| error.to_string().contains("heap-profile")));
}
