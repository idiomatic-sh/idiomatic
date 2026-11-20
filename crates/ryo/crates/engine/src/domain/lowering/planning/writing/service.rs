//! The services of the `writing` part: a planned branch written as a rule's branch program, as a call-form pattern's
//! program, or as one pipeline of a table. The two emitters read the same accesses; a program runs per candidate or per key,
//! a pipeline's stages are loops over a batch's columns and hold no program.

use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::pins_of;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm};
use crate::domain::lowering::planning::emitting::service::{atom_written, operand, uses_of};
use crate::domain::lowering::planning::types::{
    Emitted, Emitter, Environment, PlannedBranch, UnitRole,
};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage, TreeWalk, UseMode};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, NearestEdges, PlacedStep, Polarity, ProgramStep,
    Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands, StepProgram,
    TestOperation,
};
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The accesses of a branch in its planned order, from an environment.
fn accesses<'a>(
    drafts: &'a mut Drafts,
    kinds: &'a KindVocabulary,
    branch: &FlatBranch,
    plan: &PlannedBranch,
    unit: (UnitRole, Option<DefinitionId>),
    start: (Environment, Vec<Emitted>, Option<&ColumnMask>),
) -> Emitter<'a> {
    let mut environment = start.0;
    environment.pins = pins_of(branch);
    let mut emitter = Emitter {
        drafts,
        kinds,
        role: unit.0,
        unit: unit.1,
        row: branch.row.clone(),
        uses: uses_of(branch, start.2),
        environment,
        written: start.1,
        missed: 0,
    };
    for at in &plan.order {
        if let Some(atom) = branch.atoms.get(*at) {
            atom_written(&mut emitter, atom);
        }
    }
    emitter
}

/// One access as a step of a program.
fn step_of(access: Emitted) -> ProgramStep {
    match access {
        Emitted::Rows {
            source,
            columns,
            values,
            kinds,
            out,
        } => match kinds {
            Some((relation, kinds)) => ProgramStep::EachKind {
                relation,
                kinds,
                out,
            },
            None if columns.0.is_empty() => ProgramStep::EachRow { source, out },
            None => ProgramStep::EachMatch {
                source,
                columns,
                values,
                out,
            },
        },
        Emitted::Use {
            definition,
            bound,
            collected,
            values,
            outputs,
            mode,
        } => {
            let body = StepBody::Definition {
                definition,
                bound,
                collected,
                values,
            };
            match mode {
                UseMode::Present => ProgramStep::Exists {
                    polarity: Polarity::Present,
                    body,
                    memo: StepMemo::None,
                },
                UseMode::Absent => ProgramStep::Exists {
                    polarity: Polarity::Absent,
                    body,
                    memo: StepMemo::None,
                },
                UseMode::Rows => ProgramStep::Or {
                    bodies: [body].into(),
                    outputs,
                    memo: StepMemo::None,
                },
            }
        }
        Emitted::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => ProgramStep::Latest {
            table,
            key,
            bound,
            side,
            out,
        },
        Emitted::Read {
            source,
            column,
            row,
            out,
        } => ProgramStep::Read {
            source,
            column,
            row,
            out,
        },
        Emitted::Test { test, left, right } => ProgramStep::Test { test, left, right },
        Emitted::Axis {
            relation,
            direction,
            start,
            out,
        } => ProgramStep::Axis {
            relation,
            direction,
            start,
            out,
        },
        Emitted::Children {
            relation,
            parent,
            out,
        } => ProgramStep::Children {
            relation,
            parent,
            field: None,
            out,
        },
        Emitted::Within {
            relation,
            ancestor,
            descendant,
        } => ProgramStep::Within {
            relation,
            ancestor,
            descendant,
        },
        Emitted::Nearest {
            relation,
            domain,
            column,
            element,
            start,
            out,
        } => ProgramStep::Nearest {
            edges: NearestEdges::Tree(relation),
            condition: StepBody::Definition {
                definition: domain,
                bound: ColumnMask([column].into()),
                collected: ColumnMask::default(),
                values: StepOperands([StepOperand::Register(element)].into()),
            },
            element,
            start,
            out,
            memo: StepMemo::None,
        },
        Emitted::Let { from, out } => ProgramStep::Let { from, out },
    }
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
    let start = (Environment::default(), Vec::new(), None);
    let emitter = accesses(drafts, kinds, branch, plan, (UnitRole::Branch, None), start);
    if emitter.missed > 0 {
        return None;
    }
    let columns = StepOperands(row_operands(&emitter.environment, &branch.row)?.into());
    let steps = emitter.written.into_iter().map(step_of);
    Some(program(
        steps.chain([ProgramStep::Emit { target, columns }]),
    ))
}

/// What a call-form pattern is of: the definition's formals, and the columns it binds and collects.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatternColumns<'a> {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// One register per column of the definition.
    pub(crate) formals: &'a [Register],
    /// The columns that arrive bound.
    pub(crate) bound: &'a ColumnMask,
    /// The columns a row gives back.
    pub(crate) collected: &'a ColumnMask,
}

/// The environment a pattern's branch starts from: each bound column's cell is in its formal. A variable bound twice, and a
/// constant in a bound column, are tested against the formal first.
#[must_use]
pub(crate) fn pattern_start(
    branch: &FlatBranch,
    pattern: PatternColumns<'_>,
) -> (Environment, Vec<Emitted>) {
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
            (_, Some(right)) => tests.push(Emitted::Test {
                test: TestOperation::Equal,
                left: StepOperand::Register(*formal),
                right,
            }),
            (FlatTerm::Text(_) | FlatTerm::Number(_), None) => {}
        }
    }
    (environment, tests)
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
    let (environment, tests) = pattern_start(branch, pattern);
    // What a pattern gives is its collected columns: a column nothing collects is read for nothing.
    let mut read = pattern.collected.0.to_vec();
    read.extend(pattern.bound.0.iter().copied());
    let read = ColumnMask(read.into());
    let start = (environment, tests, Some(&read));
    let unit = (UnitRole::Pattern, Some(pattern.definition));
    let emitter = accesses(drafts, kinds, branch, plan, unit, start);
    if emitter.missed > 0 {
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
            let slots: Vec<StepProgram> = branches
                .into_iter()
                .chain([last])
                .map(program)
                .collect();
            program([ProgramStep::Fork {
                slots: slots.into(),
            }])
        }
        (None, _) => program([]),
    }
}

/// One access as a stage of a pipeline: the first enumeration is the scan; a later read by a register is a probe, and one
/// with no register is every row of its source, a product. None for a `Let`, which no pipeline holds.
fn stage_of(access: Emitted, first: bool) -> Option<FillStage> {
    Some(match access {
        Emitted::Rows {
            source,
            columns,
            values,
            out,
            ..
        } => {
            let keyed = values
                .0
                .iter()
                .any(|value| matches!(value, StepOperand::Register(_)));
            match (first, keyed) {
                (true, _) => FillStage::Scan {
                    source,
                    columns,
                    values,
                    out,
                },
                (false, true) => FillStage::Probe {
                    source,
                    columns,
                    values,
                    range: None,
                    mode: UseMode::Rows,
                    out,
                },
                (false, false) => FillStage::Product { source, out },
            }
        }
        Emitted::Use {
            definition,
            bound,
            collected,
            values,
            outputs,
            mode,
        } => FillStage::Use {
            definition,
            bound,
            collected,
            values,
            mode,
            outputs,
        },
        Emitted::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => FillStage::Latest {
            table,
            key,
            bound,
            side,
            out,
        },
        Emitted::Read {
            source,
            column,
            row,
            out,
        } => FillStage::Read {
            source,
            column,
            row,
            out,
        },
        Emitted::Test { test, left, right } => FillStage::Keep { test, left, right },
        Emitted::Axis {
            relation,
            direction,
            start,
            out,
        } => FillStage::Walk {
            relation,
            walk: TreeWalk::Axis(direction),
            start,
            out,
        },
        Emitted::Children {
            relation,
            parent,
            out,
        } => FillStage::Walk {
            relation,
            walk: TreeWalk::Children(None),
            start: parent,
            out,
        },
        // A pipeline holds no test of containment, no walk with a condition and no `Let`.
        Emitted::Within { .. } | Emitted::Nearest { .. } | Emitted::Let { .. } => return None,
    })
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
    let start = (Environment::default(), Vec::new(), None);
    let emitter = accesses(drafts, kinds, branch, plan, (UnitRole::Pipeline, unit), start);
    if emitter.missed > 0 {
        return None;
    }
    let row = StepOperands(row_operands(&emitter.environment, &branch.row)?.into());
    let mut stages = Vec::with_capacity(emitter.written.len());
    for (place, access) in emitter.written.into_iter().enumerate() {
        stages.push(PlacedStage {
            point: None,
            stage: stage_of(access, place == 0)?,
        });
    }
    Some(Pipeline {
        stages: stages.into(),
        row,
    })
}
