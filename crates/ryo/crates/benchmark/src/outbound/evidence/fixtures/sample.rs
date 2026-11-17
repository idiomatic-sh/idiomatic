//! The owned sample data the evidence contract is written in.

/// A complete report of one file and no finding, what the comparison evidence is written of.
#[must_use]
pub fn report() -> ryo_benchmark_contracts::domain::reports::types::RunReport {
    use ryo_benchmark_contracts::domain::reports::types::{
        CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext, RuleCoverage, RunReport,
    };
    RunReport {
        execution_metadata: ExecutionMetadata::Complete,
        project_context: ProjectContext::NotRequested,
        skipped: Vec::new(),
        coverage: CoverageAccounting::Accounted(RuleCoverage {
            selected: PairCount(1),
            evaluated: PairCount(1),
            skipped: PairCount(0),
            unknown_or_unread: PairCount(0),
        }),
        diagnostics: vec![],
        files: vec![ryo::linter::RelativePath("input".into())],
    }
}

/// One project's costs over three trials, each condition's values distinct, so a measure read under another name shows.
#[must_use]
pub fn costs() -> ryo_benchmark_contracts::domain::costs::types::ProjectCosts {
    use ryo_benchmark_contracts::domain::costs::types::{
        ProjectCost, ProjectCosts, RunCost, StoreBytes, StoreFiles, StoreSize,
    };
    use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
    use std::time::Duration;
    let run = |cpu: u64, wall: u64, peak: u64| RunCost {
        cpu: [cpu, cpu + 2, cpu + 1].map(Duration::from_millis).to_vec(),
        wall: [wall, wall + 2, wall + 1]
            .map(Duration::from_millis)
            .to_vec(),
        peak: [peak, peak + 2, peak + 1].map(ResidentBytes).to_vec(),
    };
    ProjectCosts(vec![ProjectCost {
        project: "django".into(),
        cold: run(1_000, 1_500, 1_000_000),
        warm: run(20, 40, 200_000),
        edited: run(60, 90, 300_000),
        store: StoreSize {
            bytes: StoreBytes(30),
            files: StoreFiles(3),
        },
    }])
}

/// One rule's verification times: 40 ms in all, 9 ms at most, 3 ms to compile.
///
/// # Panics
///
/// Never: `F401` is a rule id.
#[must_use]
pub fn timings() -> crate::outbound::evidence::types::RuleTimings {
    crate::outbound::evidence::types::RuleTimings(std::collections::BTreeMap::from([(
        ryo::linter::RuleId::try_from("F401").expect("F401 is a rule id"),
        crate::outbound::evidence::types::RuleTiming {
            total: std::time::Duration::from_millis(40),
            slowest: std::time::Duration::from_millis(9),
            compile: std::time::Duration::from_millis(3),
        },
    )]))
}

/// One shape grown 1.9 times, 120 ms at the whole size.
#[must_use]
pub fn growth() -> ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths {
    use ryo_benchmark_contracts::domain::measurements::types::{
        DoublingRatio, GrowthShape, ShapeGrowth, ShapeGrowths,
    };
    ShapeGrowths(std::collections::BTreeMap::from([(
        GrowthShape("F401/module/assign".into()),
        ShapeGrowth {
            ratio: DoublingRatio(1.9),
            larger: std::time::Duration::from_millis(120),
            peak: ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(1 << 20),
        },
    )]))
}
