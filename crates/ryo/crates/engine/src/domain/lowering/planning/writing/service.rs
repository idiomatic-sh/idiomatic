//! The services of the `writing` part: a planned branch written as a rule's branch program, as a call-form pattern's
//! program, or as one pipeline of a table. The two emitters read the same accesses; a program runs per candidate or per key,
//! a pipeline's stages are loops over a batch's columns and hold no program.

use crate::domain::lowering::costing::types::PlanEstimate;
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::pins_of;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm};
use crate::domain::lowering::planning::deferring::service::deferred;
use crate::domain::lowering::planning::emitting::service::atom_written;
use crate::domain::lowering::planning::forms::service::{stage_of, step_of};
use crate::domain::lowering::planning::operands::service::{operand, uses_of};
use crate::domain::lowering::planning::types::BranchStart;
use crate::domain::lowering::planning::types::Enumerated;
use crate::domain::lowering::planning::types::PatternColumns;
use crate::domain::lowering::planning::types::WrittenUnit;
use crate::domain::lowering::planning::types::{
    Emitter, Environment, PlannedBranch, UnitRole, WrittenAccess,
};
use crate::domain::steps::stages::types::{Pipeline, PlacedStage};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, PlacedStep, ProgramStep, RegisterList, StepOperand,
    StepOperands, StepProgram, TestOperation,
};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The accesses of a branch in its planned order, from an environment.
fn accesses<'a>(
    drafts: &'a mut Drafts,
    kinds: &'a KindVocabulary,
    branch: &FlatBranch,
    plan: &PlannedBranch,
    unit: WrittenUnit,
    start: BranchStart<'_>,
) -> Emitter<'a> {
    let mut environment = start.environment;
    environment.pins = pins_of(branch);
    let mut emitter = Emitter {
        drafts,
        kinds,
        role: unit.role,
        unit: unit.definition,
        row: branch.row.clone(),
        uses: uses_of(branch, start.gives),
        environment,
        written: start.written,
        missed: Count(0),
    };
    // SPIKE (restricted tables): a rule's branch and a table's pipeline start from nothing given, so what an ask of theirs
    // is keyed by comes from the atoms written before it: each ask keeps them. A pattern's program starts from its formals.
    let sited = matches!(unit.role, UnitRole::Branch | UnitRole::Pipeline);
    let outer = std::mem::take(&mut emitter.drafts.asking);
    for (place, at) in plan.order.iter().enumerate() {
        if let Some(atom) = branch.atoms.get(at.0) {
            if sited {
                let rows = plan.before.get(place).copied().unwrap_or(PlanEstimate(0));
                emitter.drafts.asking.atom = Some((atom.clone(), rows));
            }
            atom_written(&mut emitter, atom);
            if sited {
                emitter.drafts.asking.prefix.push(atom.clone());
            }
        }
    }
    emitter.drafts.asking = outer;
    // Each read waits until its cell is first needed.
    emitter.written = deferred(std::mem::take(&mut emitter.written));
    emitter
}

/// A program of steps, none of them numbered yet: the numbering pass gives each its plan point.
fn program(steps: impl IntoIterator<Item = ProgramStep>) -> StepProgram {
    StepProgram(
        steps
            .into_iter()
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    )
}

/// The operands of a branch's row: one for each column. None when a column's variable was given by no access.
fn row_operands(environment: &Environment, row: &[FlatTerm]) -> Option<Vec<StepOperand>> {
    row.iter().map(|term| operand(environment, *term)).collect()
}

/// A rule's branch as a program: its accesses, then the row handed over. None when a column of its row was never given.
pub(crate) fn branch_program(
    drafts: &mut Drafts,
    kinds: &KindVocabulary,
    branch: &FlatBranch,
    plan: &PlannedBranch,
    target: EmitTarget,
) -> Option<StepProgram> {
    let unit = WrittenUnit {
        role: UnitRole::Branch,
        definition: None,
    };
    let emitter = accesses(drafts, kinds, branch, plan, unit, BranchStart::default());
    if emitter.missed.0 > 0 {
        return None;
    }
    let columns = StepOperands(row_operands(&emitter.environment, &branch.row)?.into());
    let steps = emitter.written.into_iter().map(step_of);
    Some(program(
        steps.chain([ProgramStep::Emit { target, columns }]),
    ))
}

/// The environment a pattern's branch starts from: each bound column's cell is in its formal. A variable bound twice, and a
/// constant in a bound column, are tested against the formal first.
#[must_use]
pub(crate) fn pattern_start(
    branch: &FlatBranch,
    pattern: PatternColumns<'_>,
) -> BranchStart<'static> {
    let (mut environment, mut tests) = (Environment::default(), Vec::new());
    for column in &pattern.bound.0 {
        let place = usize::from(column.0);
        let (Some(term), Some(formal)) = (branch.row.get(place), pattern.formals.get(place)) else {
            continue;
        };
        match (*term, operand(&environment, *term)) {
            (FlatTerm::File, _) => {}
            (FlatTerm::Variable(variable), None) => {
                let _new = environment.registers.insert(variable, *formal);
            }
            (_, Some(right)) => tests.push(WrittenAccess::Test {
                test: TestOperation::Equal,
                left: StepOperand::Register(*formal),
                right,
            }),
            (FlatTerm::Text(_) | FlatTerm::Number(_), None) => {}
        }
    }
    BranchStart {
        environment,
        written: tests,
        gives: None,
    }
}

/// A call-form pattern's branch as a program: the bound cells arrive in their formals, and each row ends in a `Yield` of the
/// collected columns' formals, ascending. None when a collected column was never given.
pub(crate) fn pattern_program(
    drafts: &mut Drafts,
    kinds: &KindVocabulary,
    branch: &FlatBranch,
    plan: &PlannedBranch,
    pattern: PatternColumns<'_>,
) -> Option<Vec<ProgramStep>> {
    let mut start = pattern_start(branch, pattern);
    // What a pattern gives is its collected columns: a column nothing collects is read for nothing.
    let mut read = pattern.collected.0.to_vec();
    read.extend(pattern.bound.0.iter().copied());
    let read = ColumnMask(read.into());
    start.gives = Some(&read);
    let unit = WrittenUnit {
        role: UnitRole::Pattern,
        definition: Some(pattern.definition),
    };
    let emitter = accesses(drafts, kinds, branch, plan, unit, start);
    if emitter.missed.0 > 0 {
        return None;
    }
    let mut steps: Vec<ProgramStep> = Vec::new();
    let mut row = Vec::with_capacity(pattern.collected.0.len());
    for column in &pattern.collected.0 {
        let place = usize::from(column.0);
        let formal = *pattern.formals.get(place)?;
        let from = operand(&emitter.environment, *branch.row.get(place)?)?;
        if from != StepOperand::Register(formal) {
            steps.push(ProgramStep::Let { from, out: formal });
        }
        row.push(formal);
    }
    steps.push(ProgramStep::Yield {
        row: RegisterList(row.into()),
    });
    let written = emitter.written.into_iter().map(step_of);
    Some(written.chain(steps).collect())
}

/// A pattern's branches as one program: a pattern of one branch is that branch's steps; of several, a `Fork` of them.
#[must_use]
pub(crate) fn pattern_whole(mut branches: Vec<Vec<ProgramStep>>) -> StepProgram {
    match (branches.pop(), branches.is_empty()) {
        (Some(only), true) => program(only),
        (Some(last), false) => {
            let slots: Vec<StepProgram> = branches.into_iter().chain([last]).map(program).collect();
            program([ProgramStep::Fork {
                slots: slots.into(),
            }])
        }
        (None, _) => program([]),
    }
}

/// A branch as one pipeline of a table: its accesses as stages, then the row written. None when the branch has no stage
/// form: a column never given, or a value only a program can make.
pub(crate) fn pipeline(
    drafts: &mut Drafts,
    kinds: &KindVocabulary,
    branch: &FlatBranch,
    plan: &PlannedBranch,
    unit: Option<DefinitionId>,
) -> Option<Pipeline> {
    let unit = WrittenUnit {
        role: UnitRole::Pipeline,
        definition: unit,
    };
    let emitter = accesses(drafts, kinds, branch, plan, unit, BranchStart::default());
    if emitter.missed.0 > 0 {
        return None;
    }
    let row = StepOperands(row_operands(&emitter.environment, &branch.row)?.into());
    let mut stages = Vec::with_capacity(emitter.written.len());
    // The first access is the pipeline's scan.
    let mut first = Enumerated::NotYet;
    for access in emitter.written {
        stages.push(PlacedStage {
            point: None,
            stage: stage_of(access, first)?,
        });
        first = Enumerated::Already;
    }
    Some(Pipeline {
        stages: stages.into(),
        row,
    })
}
