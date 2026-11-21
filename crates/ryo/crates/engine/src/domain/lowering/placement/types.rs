//! The types of the `placement` part: what the rule reads of a pattern, and what it answers.

use crate::domain::code::types::MeasuredForm;
use crate::domain::code::types::ShapeChoice;
use ryo_relations::domain::relations::types::Count;

/// What the load was given of a sample: nothing, the first run's statistics, or the second's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SampleStage {
    /// No statistics: every flip candidate takes Call.
    Absent,
    /// The first run's: every flip candidate takes Table.
    First,
    /// The second's: every unit takes the form written for it.
    Second,
}

/// Whether a pattern has a program: one whose every access is keyed once its bound columns are given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallForm {
    /// It has: Call is admissible.
    Keyed,
    /// Some access of it has no key: a file-sized step would stand under a key.
    Scans,
}

/// Whether a definition's table holds a product stage: its own plan joins with no key within the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableForm {
    /// Every stage after its scans is keyed.
    Keyed,
    /// It holds a product stage.
    Product,
    /// One of its pipelines uses a pattern of the definition itself, in call form: a nearest walk, which no stage runs.
    Calls,
}

/// What the placement rule reads of one pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatternFacts {
    /// How many columns it binds.
    pub(crate) binds: Count,
    /// Whether it has a program.
    pub(crate) call: CallForm,
    /// Whether its definition's table holds a product stage.
    pub(crate) table: TableForm,
    /// How many call-form patterns its definition already holds.
    pub(crate) call_patterns: Count,
    /// The form the statistics kept for its unit, when they name it.
    pub(crate) measured: Option<MeasuredForm>,
}

/// The two forms of a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormChoice {
    /// Per distinct key, kept for the file.
    Call,
    /// From the definition's table, made once per file.
    Table,
}

/// What the rule answers: the form, and whether the shape of the pattern's plans left it open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatternPlacement {
    /// The form.
    pub(crate) form: FormChoice,
    /// `Open` for a flip candidate: the only patterns a sample may flip.
    pub(crate) shape: ShapeChoice,
}

/// What a load reads of its statistics while it lowers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoadSample<'a> {
    /// What they are of.
    pub(crate) stage: SampleStage,
    /// The statistics, when the load was given any.
    pub(crate) statistics: Option<&'a crate::domain::statistics::types::Statistics>,
    /// The language's kinds.
    pub(crate) kinds: &'a ryo_values::domain::descriptions::types::KindVocabulary,
}

/// A rule the build of a pack refuses: its place among the accepted rules, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RefusedPlace {
    /// The rule.
    pub(crate) rule: crate::domain::lowering::nodes::types::AcceptedPlace,
    /// Why.
    pub(crate) refusal: crate::domain::lowering::types::LoadRefusal,
}
