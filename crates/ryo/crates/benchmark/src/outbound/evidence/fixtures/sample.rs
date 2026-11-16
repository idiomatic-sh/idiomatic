//! The owned sample data the evidence contract rows are written in.
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, PublicationSample, SampleName,
};

/// Owned test data, not a certification or measurement proof.
#[derive(Clone)]
pub struct SampleInput {
    pub(super) inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus,
    pub(super) name: SampleName,
    pub(super) measurement: Measurement,
}
impl SampleInput {
    pub(super) fn publication(&self) -> PublicationSample<'_> {
        PublicationSample {
            inventory: self.inventory,
            name: &self.name,
            measurement: &self.measurement,
        }
    }
}

/// One project's costs over three trials, each condition's values distinct, so a measure read under another name shows.
#[must_use]
pub fn costs() -> crate::domain::measurements::types::project_cost::value::ProjectCosts {
    use crate::domain::measurements::types::project_cost::value::{
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
#[must_use]
pub fn timings() -> crate::outbound::evidence::types::RuleTimings {
    crate::outbound::evidence::types::RuleTimings(std::collections::BTreeMap::from([(
        ryo::linter::RuleId::try_from("F401").unwrap_or_else(|_| unreachable!("F401 is a rule id")),
        crate::outbound::evidence::types::RuleTiming {
            total: std::time::Duration::from_millis(40),
            slowest: std::time::Duration::from_millis(9),
            compile: std::time::Duration::from_millis(3),
        },
    )]))
}

/// One shape grown 1.9 times, 120 ms at the whole size.
#[must_use]
pub fn growth() -> crate::domain::growth::types::ShapeGrowths {
    use crate::domain::growth::types::{DoublingRatio, GrowthShape, ShapeGrowth, ShapeGrowths};
    ShapeGrowths(std::collections::BTreeMap::from([(
        GrowthShape("F401/module/assign".into()),
        ShapeGrowth {
            ratio: DoublingRatio(1.9),
            larger: std::time::Duration::from_millis(120),
            peak: ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(1 << 20),
        },
    )]))
}
