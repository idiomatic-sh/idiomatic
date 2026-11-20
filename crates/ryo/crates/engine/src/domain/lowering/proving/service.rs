//! The services of the `proving` part: the check of a lowered pack against the invariant of scope-engine3 2.5 part 3, by the
//! contract's own class functions. A fault is the lowering's own: the rule it belongs to is refused, never run.

use super::types::{ProductStated, ProofFault, ProvedUnit, Unproved};
use crate::domain::lowered::types::{FillInput, LoweredPack, MadeFill, OrderedFill, PatternForm};
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::service::{stage_class, step_class};
use crate::domain::steps::stages::types::{FillStage, Pipeline};
use crate::domain::steps::types::{
    CostClass, DefinitionId, MadeTableId, NearestEdges, ProgramStep, StepBody, StepOperand,
    StepProgram, StepRows,
};

/// Where a program stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// A rule's branch: its first step may be file-sized, and may read a definition's rows whole.
    Branch,
    /// Under a candidate or a key: an inline body, a call-form pattern, a slot of a `Fork`.
    Keyed,
}

/// Whether a count fits the sixteen bits a word's operands are counted in.
fn operands_fit(count: usize) -> Result<(), ProofFault> {
    if count > usize::from(u16::MAX) {
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

/// The inline programs a step runs, and how many operands its widest list holds.
fn held(step: &ProgramStep) -> (Vec<&StepProgram>, usize) {
    let of = |body: &StepBody| match body {
        StepBody::Inline(_) => 0,
        StepBody::Definition { values, .. } => values.0.len(),
    };
    match step {
        ProgramStep::Exists { body, .. } => (inline(body).into_iter().collect(), of(body)),
        ProgramStep::Or {
            bodies, outputs, ..
        } => {
            let widest = bodies.iter().map(of).max().unwrap_or(0);
            let count = widest.max(bodies.len()).max(outputs.0.len());
            (bodies.iter().filter_map(inline).collect(), count)
        }
        ProgramStep::Nearest {
            edges, condition, ..
        } => {
            let walked = match edges {
                NearestEdges::Tree(_) => None,
                NearestEdges::Derived { edges, .. } => inline(edges),
            };
            (walked.into_iter().chain(inline(condition)).collect(), of(condition))
        }
        ProgramStep::Closure { edges, .. } => (inline(edges).into_iter().collect(), of(edges)),
        ProgramStep::Fork { slots } => (slots.iter().collect(), slots.len()),
        ProgramStep::EachMatch { values, .. } | ProgramStep::EachRange { values, .. } => {
            (Vec::new(), values.0.len())
        }
        ProgramStep::Call {
            arguments, outputs, ..
        } => (Vec::new(), arguments.0.len().max(outputs.0.len())),
        ProgramStep::Yield { row } => (Vec::new(), row.0.len()),
        ProgramStep::Latest { key, .. } => (Vec::new(), key.0.len()),
        ProgramStep::Emit { columns, .. } => (Vec::new(), columns.0.len()),
        ProgramStep::EachKind { kinds, .. } => (Vec::new(), kinds.0.len()),
        ProgramStep::EachRow { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. } => (Vec::new(), 0),
    }
}

/// Whether a step reads a definition's rows whole: only `EachRow` may, and only first in a rule's branch.
const fn whole_read(step: &ProgramStep) -> bool {
    match step {
        ProgramStep::EachRow { source, .. }
        | ProgramStep::EachMatch { source, .. }
        | ProgramStep::EachRange { source, .. } => matches!(source, StepRows::Definition(_)),
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
        | ProgramStep::Emit { .. } => false,
    }
}

/// Whether a test's operands can be written: the left a register or a constant, the right a register, a constant or a set.
/// The file is no operand of a test: a program puts it in a register first.
const fn test_written(left: &StepOperand, right: &StepOperand) -> bool {
    let left = matches!(left, StepOperand::Register(_) | StepOperand::Word(_));
    left && !matches!(right, StepOperand::File)
}

/// Holds a program to the invariant: a file-sized step, and a whole read of a definition's rows, stand only first in a
/// rule's branch; every program a step runs stands under a key. Programs are walked from a list, never by recursion.
fn program_proved(program: &StepProgram, standing: Standing) -> Result<(), ProofFault> {
    let mut waiting = vec![(program, standing)];
    while let Some((program, standing)) = waiting.pop() {
        for (place, placed) in program.0.iter().enumerate() {
            let anchors = standing == Standing::Branch && place == 0;
            let sized = step_class(&placed.step) == CostClass::File;
            let whole = whole_read(&placed.step);
            let row_read = matches!(placed.step, ProgramStep::EachRow { .. });
            if (sized && !anchors) || (whole && !(anchors && row_read)) {
                return Err(ProofFault::Misplaced);
            }
            if let ProgramStep::Test { left, right, .. } = &placed.step
                && !test_written(left, right)
            {
                return Err(ProofFault::Misplaced);
            }
            let (programs, count) = held(&placed.step);
            operands_fit(count)?;
            waiting.extend(programs.into_iter().map(|program| (program, Standing::Keyed)));
        }
    }
    Ok(())
}

/// Holds a pipeline to the invariant: a file-sized stage is its scan, standing first, or a product stage, and a product
/// stage stands only in a table whose plan states a keyless join. A scan stands nowhere but first.
fn pipeline_proved(pipeline: &Pipeline, products: ProductStated) -> Result<(), ProofFault> {
    operands_fit(pipeline.row.0.len())?;
    for (place, placed) in pipeline.stages.iter().enumerate() {
        let sized = stage_class(&placed.stage) == CostClass::File;
        let held = match &placed.stage {
            FillStage::Scan { values, .. } => {
                operands_fit(values.0.len())?;
                place == 0
            }
            FillStage::Product { .. } => products == ProductStated::Stated,
            FillStage::Probe { values, .. } | FillStage::Use { values, .. } => {
                operands_fit(values.0.len())?;
                !sized
            }
            FillStage::Keep { left, right, .. } => !sized && test_written(left, right),
            FillStage::Latest { .. }
            | FillStage::Read { .. }
            | FillStage::Call { .. }
            | FillStage::Render { .. }
            | FillStage::Walk { .. } => !sized,
        };
        if !held || (place == 0 && !matches!(placed.stage, FillStage::Scan { .. })) {
            return Err(ProofFault::Misplaced);
        }
    }
    Ok(())
}

/// The pipelines of a fill, in its inputs' order. The contract's own list (`lowered::service::fill_pipelines`) is private to
/// its module: this is the same list, and goes when that one is `pub(crate)`.
fn fill_pipelines(fill: &MadeFill) -> Vec<&Pipeline> {
    let inputs: Vec<&FillInput> = match fill {
        MadeFill::Rows(pipelines, _) => return pipelines.iter().collect(),
        MadeFill::Group(_, input) | MadeFill::Ordered(OrderedFill::Sorted(input)) => vec![input],
        MadeFill::Ordered(OrderedFill::Match(left, right)) => vec![left, right],
        MadeFill::Graph(_, inputs) => inputs.iter().collect(),
        MadeFill::Constant(_) => Vec::new(),
    };
    inputs.into_iter().flat_map(|input| &input.pipelines).collect()
}

/// Holds a lowered pack to the invariant, unit by unit: the first unit that does not hold it is answered. `products` says,
/// for each made table, whether its own plan states a join with no key within the file.
pub(crate) fn proved(pack: &LoweredPack, products: &[ProductStated]) -> Result<(), Unproved> {
    for (place, rule) in pack.rules.iter().enumerate() {
        for branch in &rule.branches {
            program_proved(&branch.program, Standing::Branch).map_err(|fault| Unproved {
                unit: ProvedUnit::Rule(place),
                fault,
            })?;
        }
    }
    for (place, definition) in pack.definitions.iter().enumerate() {
        let unit = ProvedUnit::Definition(DefinitionId(u32::try_from(place).unwrap_or(u32::MAX)));
        for pattern in &definition.patterns {
            let held = match &pattern.form {
                PatternForm::Call(program) => program_proved(program, Standing::Keyed),
                PatternForm::Table(_) => Ok(()),
            };
            held.and_then(|()| operands_fit(pattern.bound.0.len().max(pattern.collected.0.len())))
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
