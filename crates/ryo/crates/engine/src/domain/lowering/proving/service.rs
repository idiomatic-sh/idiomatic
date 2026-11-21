//! The services of the `proving` part: the check of a lowered pack against the invariant of scope-engine3 2.5 part 3, by the
//! contract's own class functions. A fault is the lowering's own: the rule it belongs to is refused, never run.

use super::types::{
    HeldPrograms, ProductStated, ProgramStanding, ProofFault, ProvedUnit, Unproved, WholeRead,
};
use crate::domain::lowered::service::fill_pipelines;
use crate::domain::lowered::types::{LoweredPack, PatternForm};
use crate::domain::lowering::nodes::types::AcceptedPlace;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::service::{stage_class, step_class};
use crate::domain::steps::stages::types::{FillStage, Pipeline};
use crate::domain::steps::types::{
    CostClass, DefinitionId, MadeTableId, NearestEdges, ProgramStep, StepBody, StepOperand,
    StepProgram, StepRows,
};
use ryo_relations::domain::relations::types::Count;

/// Holds a count to the sixteen bits a word's operands are counted in.
fn operands_fit(count: Count) -> Result<(), ProofFault> {
    if count.0 > usize::from(u16::MAX) {
        Err(ProofFault::Past(PastWidth::WordOperands))
    } else {
        Ok(())
    }
}

/// The program a body holds in place; none for a use of a definition.
const fn inline(body: &StepBody) -> Option<&StepProgram> {
    match body {
        StepBody::Inline(program) => Some(program),
        StepBody::Definition { .. } => None,
    }
}

/// The values a use of a definition passes; none for an inline body.
const fn passed(body: &StepBody) -> Count {
    match body {
        StepBody::Inline(_) => Count(0),
        StepBody::Definition { values, .. } => Count(values.0.len()),
    }
}

/// What a step holds, from its inline programs and the operands of its widest list.
const fn of(programs: Vec<&StepProgram>, operands: Count) -> HeldPrograms<'_> {
    HeldPrograms { programs, operands }
}

/// The inline programs a step runs, and how many operands its widest list holds.
fn held(step: &ProgramStep) -> HeldPrograms<'_> {
    match step {
        ProgramStep::Exists { body, .. } => of(inline(body).into_iter().collect(), passed(body)),
        ProgramStep::Or {
            bodies, outputs, ..
        } => {
            let widest = bodies.iter().map(|body| passed(body).0).max().unwrap_or(0);
            let count = widest.max(bodies.len()).max(outputs.0.len());
            of(bodies.iter().filter_map(inline).collect(), Count(count))
        }
        ProgramStep::Nearest {
            edges, condition, ..
        } => {
            let walked = match edges {
                NearestEdges::Tree(_) => None,
                NearestEdges::Derived { edges, .. } => inline(edges),
            };
            let programs = walked.into_iter().chain(inline(condition));
            of(programs.collect(), passed(condition))
        }
        ProgramStep::Closure { edges, .. } => {
            of(inline(edges).into_iter().collect(), passed(edges))
        }
        ProgramStep::Fork { slots } => of(slots.iter().collect(), Count(slots.len())),
        ProgramStep::EachMatch { values, .. } | ProgramStep::EachRange { values, .. } => {
            of(Vec::new(), Count(values.0.len()))
        }
        ProgramStep::Call {
            arguments, outputs, ..
        } => of(Vec::new(), Count(arguments.0.len().max(outputs.0.len()))),
        ProgramStep::Yield { row } => of(Vec::new(), Count(row.0.len())),
        ProgramStep::Latest { key, .. } => of(Vec::new(), Count(key.0.len())),
        ProgramStep::Emit { columns, .. } => of(Vec::new(), Count(columns.0.len())),
        ProgramStep::EachKind { kinds, .. } => of(Vec::new(), Count(kinds.0.len())),
        ProgramStep::EachRow { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. } => of(Vec::new(), Count(0)),
    }
}

/// Whether a step reads a definition's rows whole: only `EachRow` may, and only first in a rule's branch.
const fn whole_read(step: &ProgramStep) -> WholeRead {
    match step {
        ProgramStep::EachRow { source, .. }
        | ProgramStep::EachMatch { source, .. }
        | ProgramStep::EachRange { source, .. } => match source {
            StepRows::Definition(_) => WholeRead::Whole,
            StepRows::Node(_) | StepRows::Filled(_) | StepRows::Made(_) | StepRows::Written(_) => {
                WholeRead::Keyed
            }
        },
        ProgramStep::EachKind { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Nearest { .. }
        | ProgramStep::Closure { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Call { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. }
        | ProgramStep::Exists { .. }
        | ProgramStep::Or { .. }
        | ProgramStep::Fork { .. }
        | ProgramStep::Yield { .. }
        | ProgramStep::Latest { .. }
        | ProgramStep::Emit { .. } => WholeRead::Keyed,
    }
}

/// Holds a test's operands to what can be written: the left a register or a constant, the right a register, a constant or a
/// set. The file is no operand of a test: a program puts it in a register first.
const fn test_proved(left: &StepOperand, right: &StepOperand) -> Result<(), ProofFault> {
    match (left, right) {
        (
            StepOperand::Register(_) | StepOperand::Word(_),
            StepOperand::Register(_) | StepOperand::Word(_) | StepOperand::Words(_),
        ) => Ok(()),
        (
            StepOperand::Words(_) | StepOperand::File,
            StepOperand::Register(_)
            | StepOperand::Word(_)
            | StepOperand::Words(_)
            | StepOperand::File,
        )
        | (StepOperand::Register(_) | StepOperand::Word(_), StepOperand::File) => {
            Err(ProofFault::Misplaced)
        }
    }
}

/// Holds one step to its place: a file-sized step, and a whole read of a definition's rows, stand only first in a rule's
/// branch, and a whole read is an `EachRow`.
fn step_proved(step: &ProgramStep, anchors: ProgramStanding) -> Result<(), ProofFault> {
    let sized = step_class(step) == CostClass::File;
    let whole = whole_read(step) == WholeRead::Whole;
    let row_read = matches!(step, ProgramStep::EachRow { .. });
    let first = anchors == ProgramStanding::Branch;
    if (sized && !first) || (whole && !(first && row_read)) {
        return Err(ProofFault::Misplaced);
    }
    match step {
        ProgramStep::Test { left, right, .. } => test_proved(left, right),
        ProgramStep::EachKind { .. }
        | ProgramStep::EachMatch { .. }
        | ProgramStep::EachRange { .. }
        | ProgramStep::EachRow { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Nearest { .. }
        | ProgramStep::Closure { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Call { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. }
        | ProgramStep::Exists { .. }
        | ProgramStep::Or { .. }
        | ProgramStep::Fork { .. }
        | ProgramStep::Yield { .. }
        | ProgramStep::Latest { .. }
        | ProgramStep::Emit { .. } => Ok(()),
    }
}

/// Holds a program to the invariant: every step to its place, and every program a step runs to standing under a key.
/// Programs are walked from a list, never by recursion.
fn program_proved(program: &StepProgram, standing: ProgramStanding) -> Result<(), ProofFault> {
    let mut waiting = vec![(program, standing)];
    while let Some((program, standing)) = waiting.pop() {
        for (place, placed) in program.0.iter().enumerate() {
            // Only a branch's first step stands as a branch's anchor.
            let anchors = if place == 0 {
                standing
            } else {
                ProgramStanding::Keyed
            };
            step_proved(&placed.step, anchors)?;
            let held = held(&placed.step);
            operands_fit(held.operands)?;
            let under = held.programs.into_iter();
            waiting.extend(under.map(|program| (program, ProgramStanding::Keyed)));
        }
    }
    Ok(())
}

/// Holds one stage to its place: a file-sized stage is the pipeline's scan, standing first, or a product stage, and a
/// product stage stands only in a table whose plan states a keyless join. A scan stands nowhere but first.
fn stage_proved(
    stage: &FillStage,
    first: ProgramStanding,
    products: ProductStated,
) -> Result<(), ProofFault> {
    let sized = stage_class(stage) == CostClass::File;
    let starts = first == ProgramStanding::Branch;
    let misplaced = match stage {
        FillStage::Scan { values, .. } => {
            operands_fit(Count(values.0.len()))?;
            !starts
        }
        FillStage::Product { .. } => starts || products != ProductStated::Stated,
        FillStage::Probe { values, .. } | FillStage::Use { values, .. } => {
            operands_fit(Count(values.0.len()))?;
            starts || sized
        }
        FillStage::Keep { left, right, .. } => {
            test_proved(left, right)?;
            starts || sized
        }
        FillStage::Latest { .. }
        | FillStage::Read { .. }
        | FillStage::Call { .. }
        | FillStage::Render { .. }
        | FillStage::Walk { .. } => starts || sized,
    };
    if misplaced {
        Err(ProofFault::Misplaced)
    } else {
        Ok(())
    }
}

/// Holds a pipeline to the invariant, stage by stage: its first stage stands as its scan.
fn pipeline_proved(pipeline: &Pipeline, products: ProductStated) -> Result<(), ProofFault> {
    operands_fit(Count(pipeline.row.0.len()))?;
    for (place, placed) in pipeline.stages.iter().enumerate() {
        let first = if place == 0 {
            ProgramStanding::Branch
        } else {
            ProgramStanding::Keyed
        };
        stage_proved(&placed.stage, first, products)?;
    }
    Ok(())
}

/// Holds a lowered pack to the invariant, unit by unit: the first unit that does not hold it is answered. `products` says,
/// for each made table, whether its own plan states a join with no key within the file.
pub(crate) fn proved(pack: &LoweredPack, products: &[ProductStated]) -> Result<(), Unproved> {
    for (place, rule) in pack.rules.iter().enumerate() {
        for branch in &rule.branches {
            program_proved(&branch.program, ProgramStanding::Branch).map_err(|fault| Unproved {
                unit: ProvedUnit::Rule(AcceptedPlace(place)),
                fault,
            })?;
        }
    }
    for (place, definition) in pack.definitions.iter().enumerate() {
        let unit = ProvedUnit::Definition(DefinitionId(u32::try_from(place).unwrap_or(u32::MAX)));
        for pattern in &definition.patterns {
            let held = match &pattern.form {
                PatternForm::Call(program) => program_proved(program, ProgramStanding::Keyed),
                PatternForm::Table(_) => Ok(()),
            };
            let columns = Count(pattern.bound.0.len().max(pattern.collected.0.len()));
            held.and_then(|()| operands_fit(columns))
                .map_err(|fault| Unproved { unit, fault })?;
        }
    }
    for (place, table) in pack.tables.iter().enumerate() {
        let unit = ProvedUnit::Table(MadeTableId(u32::try_from(place).unwrap_or(u32::MAX)));
        let stated = products.get(place).copied().unwrap_or(ProductStated::Keyed);
        for pipeline in fill_pipelines(&table.fill) {
            pipeline_proved(pipeline, stated).map_err(|fault| Unproved { unit, fault })?;
        }
    }
    Ok(())
}
