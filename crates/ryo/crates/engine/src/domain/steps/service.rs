//! The services of the `steps` concept: the cost class of a step, a stage and a per-row meaning, each a property of the item
//! alone. Every `match` here names every variant: a new step, stage or meaning does not compile without its class.

use super::types::{
    AxisDirection, CallMeaning, CostClass, NearestEdges, ProgramStep, RangeBound, StepBody,
    StepOperand, StepOperands, StepRows,
};
use crate::domain::steps::stages::types::{FillStage, ProbeRange, UseMode};

/// The class of a read by values: a register among them is a key, and what it reaches is its fan-out; values that are all
/// constants name a bucket of the file.
const fn values_class(operands: &StepOperands) -> CostClass {
    let mut rest: &[StepOperand] = &*operands.0;
    while let [first, later @ ..] = rest {
        match first {
            StepOperand::Register(_) => return CostClass::Fanout,
            StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => {}
        }
        rest = later;
    }
    CostClass::File
}

/// The class of one end of a range: a register's value is the candidate's own.
const fn bound_class(bound: &RangeBound) -> CostClass {
    match bound.value {
        StepOperand::Register(_) => CostClass::Fanout,
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => CostClass::File,
    }
}

/// The class of a step's range: a key when both ends are there and each is a register's value, a two-sided range the candidate
/// gives; else none.
const fn range_class(low: Option<&RangeBound>, high: Option<&RangeBound>) -> CostClass {
    match (low, high) {
        (Some(low), Some(high)) => greater(bound_class(low), bound_class(high)),
        (Some(_) | None, None) | (None, Some(_)) => CostClass::File,
    }
}

/// The class of a probe's range: a key when both ends are a register's value.
const fn probe_range_class(range: Option<&ProbeRange>) -> CostClass {
    match range {
        Some(range) => greater(bound_class(&range.low), bound_class(&range.high)),
        None => CostClass::File,
    }
}

/// The class of a read that either its values or its range may key: keyed by one is keyed.
const fn either(values: CostClass, range: CostClass) -> CostClass {
    match values {
        CostClass::Fanout => CostClass::Fanout,
        CostClass::Constant | CostClass::Group | CostClass::File => range,
    }
}

/// The greater of two classes, in the order the classes are declared in.
const fn greater(left: CostClass, right: CostClass) -> CostClass {
    match left {
        CostClass::File => CostClass::File,
        CostClass::Group => match right {
            CostClass::File => CostClass::File,
            CostClass::Constant | CostClass::Fanout | CostClass::Group => CostClass::Group,
        },
        CostClass::Fanout => match right {
            CostClass::File | CostClass::Group => right,
            CostClass::Constant | CostClass::Fanout => CostClass::Fanout,
        },
        CostClass::Constant => right,
    }
}

/// The class of a body where only its having a row is asked, or its rows are a walk's edges: an inline body's steps are classed
/// one by one, a use that binds a column reaches what its key reaches, a use that binds nothing and collects nothing asks one
/// decision of the file, and a use that collects rows with nothing bound is every row of the definition.
const fn body_class(body: &StepBody) -> CostClass {
    match body {
        StepBody::Inline(_) => CostClass::Constant,
        StepBody::Definition {
            bound, collected, ..
        } => match (&*bound.0, &*collected.0) {
            ([_, ..], [] | [_, ..]) => CostClass::Fanout,
            ([], []) => CostClass::Constant,
            ([], [_, ..]) => CostClass::File,
        },
    }
}

/// The class of a body whose rows go on: a use that binds nothing is every row of the definition, whatever it collects.
const fn rows_body_class(body: &StepBody) -> CostClass {
    match body {
        StepBody::Inline(_) => CostClass::Constant,
        StepBody::Definition { bound, .. } => match &*bound.0 {
            [] => CostClass::File,
            [_, ..] => CostClass::Fanout,
        },
    }
}

/// The class of a read of rows by a key of class `keyed`: a written table is the same for every file, so what is read of it is
/// a fan-out; of any other source, the key's own class.
const fn match_class(source: StepRows, keyed: CostClass) -> CostClass {
    match source {
        StepRows::Written(_) => CostClass::Fanout,
        StepRows::Node(_) | StepRows::Filled(_) | StepRows::Made(_) | StepRows::Definition(_) => {
            keyed
        }
    }
}

/// What bounds a step's work each time it runs.
#[must_use]
pub(crate) const fn step_class(step: &ProgramStep) -> CostClass {
    match step {
        ProgramStep::EachKind { .. } => CostClass::File,
        ProgramStep::EachMatch { source, values, .. } => match_class(*source, values_class(values)),
        ProgramStep::EachRange {
            source,
            values,
            low,
            high,
            ..
        } => match_class(
            *source,
            either(
                values_class(values),
                range_class(low.as_ref(), high.as_ref()),
            ),
        ),
        ProgramStep::EachRow { source, .. } => match_class(*source, CostClass::File),
        ProgramStep::Axis { direction, .. } => match direction {
            AxisDirection::Parent => CostClass::Constant,
            AxisDirection::Ancestor
            | AxisDirection::Descendant
            | AxisDirection::NextSibling
            | AxisDirection::PreviousSibling => CostClass::Fanout,
        },
        ProgramStep::Children { .. } | ProgramStep::Within { .. } | ProgramStep::Latest { .. } => {
            CostClass::Fanout
        }
        ProgramStep::Nearest {
            edges, condition, ..
        } => {
            let walked = match edges {
                NearestEdges::Tree => CostClass::Fanout,
                NearestEdges::Derived { edges, .. } => {
                    greater(CostClass::Fanout, body_class(edges))
                }
            };
            greater(walked, body_class(condition))
        }
        ProgramStep::Closure { edges, .. } => greater(CostClass::Fanout, body_class(edges)),
        ProgramStep::Call { meaning, .. } => call_class(*meaning),
        ProgramStep::Exists { body, .. } => body_class(body),
        ProgramStep::Or { bodies, .. } => {
            let (mut class, mut rest): (CostClass, &[StepBody]) = (CostClass::Constant, &**bodies);
            while let [first, later @ ..] = rest {
                class = greater(class, rows_body_class(first));
                rest = later;
            }
            class
        }
        ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. }
        | ProgramStep::Fork { .. }
        | ProgramStep::Yield { .. }
        | ProgramStep::Emit { .. } => CostClass::Constant,
    }
}

/// What bounds a stage's work for each row of its batch.
#[must_use]
pub(crate) const fn stage_class(stage: &FillStage) -> CostClass {
    match stage {
        FillStage::Scan { .. } | FillStage::Product { .. } => CostClass::File,
        FillStage::Probe {
            values,
            range,
            mode,
            ..
        } => match either(values_class(values), probe_range_class(range.as_ref())) {
            CostClass::Fanout => CostClass::Fanout,
            CostClass::Constant | CostClass::Group | CostClass::File => unkeyed_class(*mode),
        },
        FillStage::Use { bound, mode, .. } => match &*bound.0 {
            [] => unkeyed_class(*mode),
            [_, ..] => CostClass::Fanout,
        },
        FillStage::Latest { .. } | FillStage::Walk { .. } => CostClass::Fanout,
        FillStage::Call { meaning, .. } => call_class(*meaning),
        FillStage::Read { .. } | FillStage::Keep { .. } | FillStage::Render { .. } => {
            CostClass::Constant
        }
    }
}

/// The class of a probe or a use that nothing of the batch's row keys: every row of what it reads for each row of the batch when
/// its rows go on, which is a product and is written as one; one decision when it asks only whether a row is there.
const fn unkeyed_class(mode: UseMode) -> CostClass {
    match mode {
        UseMode::Rows => CostClass::File,
        UseMode::Present | UseMode::Absent => CostClass::Constant,
    }
}

/// What bounds a per-row meaning's work for one row: constant in rows; a regular expression is linear in the text it reads.
#[must_use]
pub(crate) const fn call_class(meaning: CallMeaning) -> CostClass {
    match meaning {
        CallMeaning::SpanBetween
        | CallMeaning::SpanBounds
        | CallMeaning::SourceMatches(_)
        | CallMeaning::Capture
        | CallMeaning::Replace(_)
        | CallMeaning::Words
        | CallMeaning::Rank
        | CallMeaning::Concat
        | CallMeaning::Lower
        | CallMeaning::FitsRendered => CostClass::Constant,
    }
}
