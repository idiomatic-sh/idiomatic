//! The core's workloads: one `check` of the fixture workspace over the fakes of its ports, warm (its answer kept by the
//! runs before, so what is measured is what a run spends to find nothing changed) and cold (a use case built for each
//! round).

use tastemaker_core::inbound::use_cases::check::fixtures::{
    StoreFake, analysis, repository, request, use_case,
};
use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase as _;
use tastemaker_core::inbound::use_cases::check::types::Fixing;

/// The core's warm and cold `check`.
pub(crate) fn measured() -> Vec<crate::measure::Measured> {
    vec![warm(), cold()]
}

/// `check` over the fixture workspace and one use case, built once and run twice first, so each round measures a run
/// that finds nothing changed and answers what was kept.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a fixture that does not validate or check is a broken benchmark, and the message names which"
)]
fn warm() -> crate::measure::Measured {
    let check = use_case(repository(), analysis(), &StoreFake::default())
        .expect("the benchmark fixture validates");
    for _ in 0..2_u32 {
        let report = check
            .check(request(Fixing::Fix))
            .expect("the fixture checks");
        assert_eq!(
            report.report.findings.len(),
            4,
            "the fixture reports its four findings"
        );
    }
    crate::measure::rounds(
        "core/warm-check",
        || (),
        |()| {
            let report = check
                .check(request(Fixing::Report))
                .expect("the fixture checks");
            assert_eq!(
                report.report.findings.len(),
                4,
                "a warm check reports the fixture's four findings"
            );
        },
    )
}

/// `check` with its use case built for each round, so each round measures a first run.
#[expect(
    clippy::expect_used,
    reason = "a benchmark's setup fails by panicking: a fixture that does not validate or check is a broken benchmark, and the message names which"
)]
fn cold() -> crate::measure::Measured {
    crate::measure::rounds_of(
        "core/cold-check",
        crate::measure::COLD_ROUNDS,
        || {
            use_case(repository(), analysis(), &StoreFake::default())
                .expect("the benchmark fixture validates")
        },
        |check| {
            let report = check
                .check(request(Fixing::Fix))
                .expect("the fixture checks");
            assert_eq!(
                report.report.findings.len(),
                4,
                "a cold check reports the fixture's four findings"
            );
        },
    )
}
