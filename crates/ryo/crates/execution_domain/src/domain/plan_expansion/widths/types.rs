//! The `widths` service's types: what is known so far of the widths of the steps of a plan being written.
use ryo_values::domain::plan_parts::types::RowWidth;

/// How many steps a width was counted for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepsCounted(pub(crate) usize);

/// What is known of one step's width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum StepWidth {
    /// Not counted yet.
    #[default]
    Uncounted,
    /// Counted: so many columns, or none where the plan does not say.
    Counted(Option<RowWidth>),
}

/// The widths of the steps of one plan, each counted once: a step that many steps read, or that one step reads twice, is
/// counted the first time and answered from here after. Without it a count visits a step once for each path that reaches
/// it, and a ladder of joins of a step with itself doubles the paths at every level.
#[derive(Debug, Default)]
pub(crate) struct StepWidths {
    /// What is known of each step's width, by the step's number.
    pub(crate) known: Vec<StepWidth>,
    /// How many steps were counted; the expansion's case holds it to one a step.
    pub(crate) counted: StepsCounted,
}
