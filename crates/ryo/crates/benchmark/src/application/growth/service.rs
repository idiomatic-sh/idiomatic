//! Grow each shape: write its scene, lint the half and the whole program, and refuse a doubling over the limits.
use super::constants;
use crate::domain::growth::types::{GrowthPlan, ShapePrograms};
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::process_meter::interface::ProcessMeterPort;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::ArtifactPath;
use crate::outbound::suite::types::ArtifactText;
use ryo_benchmark_contracts::domain::measurements::types::{
    DoublingRatio, GrowthShape, ResidentBytes, ShapeGrowth, ShapeGrowths,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, QueryModuleName, WorkerRequest,
};
use std::collections::BTreeMap;
use std::time::Duration;

/// Lint every shape of `plan` at both sizes through `engine`, fastest of five. Each program's scene is a fresh directory
/// under `scenes` (`SuitePort::retained_directory`), its modules copied in (`copy_file`) and its program written as the
/// plan's one source file (`write_text`); `meter` takes the CPU time around each lint and resets and reads the peak.
///
/// # Errors
/// A scene or a lint that fails, or a shape over the CPU limit (2.2) or the plan's memory bound.
pub fn grow(
    engine: &dyn EnginePort,
    suite: &dyn SuitePort,
    meter: &dyn ProcessMeterPort,
    scenes: ArtifactPath<'_>,
    plan: &GrowthPlan,
) -> Result<ShapeGrowths, Error> {
    if plan.shapes.is_empty() {
        return Err(Error::Invalid(constants::NO_SHAPE.into()));
    }
    let ports = Ports {
        engine,
        suite,
        meter,
        scenes,
    };
    let mut grown = BTreeMap::new();
    let mut failures = Vec::new();
    for (shape, programs) in &plan.shapes {
        let half = ports.size(plan, programs, &programs.half.0)?;
        let whole = ports.size(plan, programs, &programs.whole.0)?;
        let growth = ShapeGrowth {
            ratio: DoublingRatio(
                whole.cpu.as_secs_f64() / half.cpu.max(plan.floor.0).as_secs_f64(),
            ),
            larger: whole.cpu,
            peak: whole.peak,
        };
        failures.extend(over(shape, plan, programs, &half, &growth));
        let _previous = grown.insert(shape.clone(), growth);
    }
    if failures.is_empty() {
        Ok(ShapeGrowths(grown))
    } else {
        Err(Error::Invalid(failures.join("\n")))
    }
}

/// The ports one growth run reads and writes through, and where its scenes go.
struct Ports<'a> {
    engine: &'a dyn EnginePort,
    suite: &'a dyn SuitePort,
    meter: &'a dyn ProcessMeterPort,
    scenes: ArtifactPath<'a>,
}

/// What one size of a shape took: its fastest CPU time and its highest peak.
struct Sized {
    cpu: Duration,
    peak: ResidentBytes,
}

impl Ports<'_> {
    /// Write `program` in a fresh scene with the rule's modules, and lint it the plan's number of times.
    fn size(
        &self,
        plan: &GrowthPlan,
        programs: &ShapePrograms,
        program: &str,
    ) -> Result<Sized, Error> {
        let scene = self
            .suite
            .retained_directory(self.scenes, ArtifactText(constants::SCENE))?
            .0;
        for path in programs.modules.0.values() {
            let destination = scene.join(path);
            if let Some(parent) = destination.parent() {
                let _created = self.suite.ensure_directory(ArtifactPath(parent))?;
            }
            self.suite.copy_file(
                ArtifactPath(&plan.request.root.join(path)),
                ArtifactPath(&destination),
            )?;
        }
        self.suite.write_text(
            ArtifactPath(&scene.join(&plan.file.0)),
            ArtifactText(program),
        )?;
        let request = WorkerRequest {
            selected_rules: CohortSelection::Explicit(vec![programs.rule.clone()]),
            query_modules: programs
                .modules
                .0
                .iter()
                .map(|(name, path)| (QueryModuleName(name.0.clone()), path.clone()))
                .collect(),
            files: vec![ryo::linter::RelativePath(plan.file.0.as_str().into())],
            root: scene,
            ..plan.request.clone()
        };
        let mut cpu = Duration::MAX;
        let mut peak = ResidentBytes(0);
        for _lint in 0..constants::LINTS {
            self.meter.reset_peak()?;
            let started = self.meter.cpu()?;
            self.lint(&request)?;
            let finished = self.meter.cpu()?;
            cpu = cpu.min(finished.0.saturating_sub(started.0));
            peak = ResidentBytes(peak.0.max(self.meter.peak()?.0));
        }
        Ok(Sized { cpu, peak })
    }

    /// One lint of `request`, as the worker runs it: prepare, load, configure and execute; refused unless complete.
    fn lint(&self, request: &WorkerRequest) -> Result<(), Error> {
        let prepared = crate::application::worker_request::service::prepare(request.clone())?;
        let session = self.engine.load(&prepared.request)?;
        let configured =
            crate::application::worker_request::service::configure(&prepared.request, session)?;
        let report = configured.execute()?.report;
        if report.execution_metadata == ExecutionMetadata::Complete {
            Ok(())
        } else {
            Err(Error::Invalid(format!(
                "{}: the run is not complete: {:?}",
                request.root.display(),
                report.execution_metadata
            )))
        }
    }
}

/// Each bound `growth` breaks: the CPU limit, then the memory bound at the whole size, then the memory doubling over the
/// floored half-size peak.
fn over(
    shape: &GrowthShape,
    plan: &GrowthPlan,
    programs: &ShapePrograms,
    half: &Sized,
    growth: &ShapeGrowth,
) -> Vec<String> {
    let mut failures = Vec::new();
    if growth.ratio.0 > constants::DOUBLING_LIMIT.0 {
        failures.push(format!(
            "{}: its CPU ratio {:.2} is over {} ({:?} at {} after {:?} at half, floor {:?})",
            shape.0,
            growth.ratio.0,
            constants::DOUBLING_LIMIT.0,
            growth.larger,
            programs.size.0,
            half.cpu,
            plan.floor.0
        ));
    }
    let memory = &plan.memory;
    let size = u64::try_from(programs.size.0).unwrap_or(u64::MAX);
    let bound = memory
        .base
        .0
        .saturating_add(memory.per_thousand.0.saturating_mul(size) / constants::THOUSAND);
    if growth.peak.0 > bound {
        failures.push(format!(
            "{}: peak {} bytes at {} passes the bound of {bound} bytes",
            shape.0, growth.peak.0, programs.size.0
        ));
    }
    let floored = ResidentBytes(half.peak.0.max(memory.floor.0));
    if kilobytes(growth.peak) / kilobytes(floored) > memory.doubling.0 {
        failures.push(format!(
            "{}: peak {} bytes at {} grew over {} times the {} bytes at half",
            shape.0, growth.peak.0, programs.size.0, memory.doubling.0, floored.0
        ));
    }
    failures
}

/// Bytes in thousands, as a ratio's number: exact for any peak under four terabytes, the largest counted as that.
fn kilobytes(bytes: ResidentBytes) -> f64 {
    f64::from(u32::try_from(bytes.0 / constants::THOUSAND).unwrap_or(u32::MAX))
}
