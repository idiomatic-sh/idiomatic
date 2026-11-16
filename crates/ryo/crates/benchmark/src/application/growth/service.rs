//! Grow each shape: write its scene, lint the half and the whole program, and refuse a doubling over the limits.
use super::constants;
use crate::domain::growth::constants::{CPU_FLOOR, MEMORY_BOUND};
use crate::domain::growth::types::{
    GrownProgram, GrowthModule, GrowthPlan, RuleModules, ShapePrograms,
};
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::process_meter::interface::ProcessMeterPort;
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::ArtifactPath;
use crate::outbound::suite::types::ArtifactText;
use ryo_benchmark_contracts::domain::growth::types::{GrowthFrame, GrowthOperation, GrowthSize};
use ryo_benchmark_contracts::domain::measurements::types::{
    DoublingRatio, GrowthShape, ResidentBytes, ShapeGrowth, ShapeGrowths,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, InputDeclaration, ProviderSelection, QueryModuleName,
    RuleContextPreparation, SearchRoots, WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;
use std::time::Duration;

/// One lint of a grown program, as the use case composes it (prepare, load, configure, execute; refused unless complete).
pub type Lint<'a> = &'a dyn Fn(&WorkerRequest) -> Result<(), Error>;

/// The growth plan of a language: its table read through the suite, every rule with the query modules its profile
/// registers, over every operation in every frame the operation names, at each whole size and its half; held to the one
/// CPU floor and memory bound.
///
/// # Errors
/// A table or a rule profile the suite refuses.
pub fn plan(
    suite: &dyn SuitePort,
    source: crate::domain::growth::types::GrowthSource<'_>,
) -> Result<GrowthPlan, Error> {
    let table = suite.growth_table(ArtifactPath(source.table))?;
    let mut shapes = BTreeMap::new();
    let mut applicability = BTreeMap::new();
    for grown in &table.rules {
        let profile = suite.profile(ArtifactPath(&source.bench.join(&grown.profile)))?;
        applicability.extend(profile.rule_applicability);
        let modules = RuleModules(
            profile
                .query_modules
                .into_iter()
                .map(|(name, path)| (GrowthModule(name.0), path))
                .collect(),
        );
        for operation in &table.operations {
            for frame_name in &operation.frames {
                let Some(frame) = table.frames.get(frame_name) else {
                    return Err(Error::Invalid(format!(
                        "{}: {}",
                        frame_name.0,
                        constants::UNKNOWN_FRAME
                    )));
                };
                for whole in &table.sizes {
                    let name = [
                        grown.rule.to_string(),
                        frame_name.0.clone(),
                        operation.name.0.clone(),
                        whole.0.to_string(),
                    ]
                    .join(constants::SHAPE_SEPARATOR);
                    let _previous = shapes.insert(
                        GrowthShape(name),
                        ShapePrograms {
                            rule: grown.rule.clone(),
                            modules: modules.clone(),
                            half: program(frame, operation, GrowthSize(whole.0 / 2)),
                            whole: program(frame, operation, *whole),
                            size: *whole,
                        },
                    );
                }
            }
        }
    }
    Ok(GrowthPlan {
        floor: CPU_FLOOR,
        memory: MEMORY_BOUND,
        file: table.file.clone(),
        request: WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(Vec::new()),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: Default::default(),
            context_manifest: ContextSeal::Unsealed,
            rules: source.rules.to_owned(),
            root: source.bench.to_owned(),
            query_modules: BTreeMap::new(),
            rule_applicability: applicability,
            parser_configuration: source.parser.clone(),
            source_language: ProviderSelection::Unspecified,
            files: Vec::new(),
            state: WorkerState::Fresh,
        },
        shapes,
    })
}

/// The operation's program at `size` in the frame: the frame's head, each segment's repetitions with every line indented,
/// then the frame's tail.
fn program(frame: &GrowthFrame, operation: &GrowthOperation, size: GrowthSize) -> GrownProgram {
    let mut text = frame.head.0.clone();
    for segment in &operation.segments {
        let count = segment.repetitions.fixed.0 + segment.repetitions.per_size.0 * size.0;
        for (index, line) in segment.lines.iter().cycle().take(count).enumerate() {
            for written in line.0.replace(constants::INDEX, &index.to_string()).lines() {
                text.push_str(&frame.indent.0);
                text.push_str(written);
                text.push('\n');
            }
        }
    }
    text.push_str(&frame.tail.0);
    GrownProgram(text)
}

/// Lint every shape of `plan` at both sizes through `lint`, fastest of five. Each program's scene is a fresh directory
/// under `scenes` (`SuitePort::retained_directory`), its modules copied in (`copy_file`) and its program written as the
/// plan's one source file (`write_text`); `meter` takes the CPU time around each lint and resets and reads the peak.
///
/// # Errors
/// A scene or a lint that fails, or a shape over the CPU limit (2.2) or the plan's memory bound.
pub fn grow(
    lint: Lint<'_>,
    suite: &dyn SuitePort,
    meter: &dyn ProcessMeterPort,
    scenes: ArtifactPath<'_>,
    plan: &GrowthPlan,
) -> Result<ShapeGrowths, Error> {
    if plan.shapes.is_empty() {
        return Err(Error::Invalid(constants::NO_SHAPE.into()));
    }
    let ports = Ports {
        lint,
        suite,
        meter,
        scenes,
    };
    let mut grown = BTreeMap::new();
    let mut failures = Vec::new();
    for (shape, programs) in &plan.shapes {
        let half = ports.size(plan, programs, &programs.half)?;
        let whole = ports.size(plan, programs, &programs.whole)?;
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
    match failures.into_iter().reduce(|first, next| {
        Error::Invalid(format!("{first}{}{next}", constants::FAILURE_SEPARATOR))
    }) {
        None => Ok(ShapeGrowths(grown)),
        Some(failure) => Err(failure),
    }
}

/// The ports one growth run reads and writes through, and where its scenes go.
struct Ports<'a> {
    lint: Lint<'a>,
    suite: &'a dyn SuitePort,
    meter: &'a dyn ProcessMeterPort,
    scenes: ArtifactPath<'a>,
}

/// What one size of a shape took: its fastest CPU time and its highest peak.
struct SizeCost {
    cpu: Duration,
    peak: ResidentBytes,
}

impl Ports<'_> {
    /// Write `program` in a fresh scene with the rule's modules, and lint it the plan's number of times.
    fn size(
        &self,
        plan: &GrowthPlan,
        programs: &ShapePrograms,
        program: &GrownProgram,
    ) -> Result<SizeCost, Error> {
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
            ArtifactText(&program.0),
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
            (self.lint)(&request)?;
            let finished = self.meter.cpu()?;
            cpu = cpu.min(finished.0.saturating_sub(started.0));
            peak = ResidentBytes(peak.0.max(self.meter.peak()?.0));
        }
        Ok(SizeCost { cpu, peak })
    }
}

/// Each bound `growth` breaks: the CPU limit, then the memory bound at the whole size, then the memory doubling over the
/// floored half-size peak.
fn over(
    shape: &GrowthShape,
    plan: &GrowthPlan,
    programs: &ShapePrograms,
    half: &SizeCost,
    growth: &ShapeGrowth,
) -> Vec<Error> {
    let mut failures = Vec::new();
    if growth.ratio.0 > constants::DOUBLING_LIMIT.0 {
        failures.push(Error::Invalid(format!(
            "{}: its CPU ratio {:.2} is over {} ({:?} at {} after {:?} at half, floor {:?})",
            shape.0,
            growth.ratio.0,
            constants::DOUBLING_LIMIT.0,
            growth.larger,
            programs.size.0,
            half.cpu,
            plan.floor.0
        )));
    }
    let memory = &plan.memory;
    let size = u64::try_from(programs.size.0).unwrap_or(u64::MAX);
    let bound = memory
        .base
        .0
        .saturating_add(memory.per_thousand.0.saturating_mul(size) / constants::THOUSAND);
    if growth.peak.0 > bound {
        failures.push(Error::Invalid(format!(
            "{}: peak {} bytes at {} passes the bound of {bound} bytes",
            shape.0, growth.peak.0, programs.size.0
        )));
    }
    let floored = ResidentBytes(half.peak.0.max(memory.floor.0));
    if peak_ratio(growth.peak, floored).0 > memory.doubling.0 {
        failures.push(Error::Invalid(format!(
            "{}: peak {} bytes at {} grew over {} times the {} bytes at half",
            shape.0, growth.peak.0, programs.size.0, memory.doubling.0, floored.0
        )));
    }
    failures
}

/// The whole-size peak over the floored half-size peak, counted in thousands of bytes: exact for any peak under four
/// terabytes, the largest counted as that.
fn peak_ratio(whole: ResidentBytes, half: ResidentBytes) -> DoublingRatio {
    let thousands = |bytes: ResidentBytes| {
        f64::from(u32::try_from(bytes.0 / constants::THOUSAND).unwrap_or(u32::MAX))
    };
    DoublingRatio(thousands(whole) / thousands(half))
}

/// One growth lint, as the worker runs a request: prepare, load, configure and execute; refused unless it completes.
///
/// # Errors
/// A request the engine refuses, or a lint that does not complete.
pub(crate) fn engine_lint(engine: &dyn EnginePort, request: &WorkerRequest) -> Result<(), Error> {
    let prepared = crate::application::worker_request::service::prepare(request.clone())?;
    let session = engine.load(&prepared.request)?;
    let configured =
        crate::application::worker_request::service::configure(&prepared.request, session)?;
    if configured.execute()?.report.execution_metadata == ExecutionMetadata::Complete {
        Ok(())
    } else {
        Err(Error::Invalid(constants::GROWTH_INCOMPLETE.into()))
    }
}

/// Read the language's growth table, plan it and grow every shape, each program linted through `engine`.
///
/// # Errors
/// A table or a profile the suite refuses, a scene or a lint that fails, or a shape over its bounds.
pub(crate) fn grow_source(
    engine: &dyn EnginePort,
    suite: &dyn SuitePort,
    meter: &dyn ProcessMeterPort,
    scenes: ArtifactPath<'_>,
    source: crate::domain::growth::types::GrowthSource<'_>,
) -> Result<ShapeGrowths, Error> {
    let lint = |request: &WorkerRequest| engine_lint(engine, request);
    grow(&lint, suite, meter, scenes, &plan(suite, source)?)
}
