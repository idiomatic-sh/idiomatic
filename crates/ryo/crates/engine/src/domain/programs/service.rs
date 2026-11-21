//! The `programs` service: what the host reads of a rule set's programs.

use super::types::{Programs, ProgramsEngine};
use crate::domain::code::types::UnitKey;
use crate::domain::findings::types::RuleColumns;
use crate::domain::made_tables::types::{FillStore, RuleSetView};
use crate::domain::scratch::types::{LoopState, Measuring, ScanScratch, ScratchEngine, UnitInFile};
use ryo_language::domain::facts::service::file_words;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::inbound::use_cases::walk::types::RelationNeeds;
use ryo_relations::domain::relations::types::{Count, KindBits};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The relations some program of the rule set reads: what the host hands every walk, so a language builds only what is read.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::needs;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(needs(&Programs::default()).0.is_empty());
/// ```
#[must_use]
pub fn needs(programs: &Programs) -> RelationNeeds<'_> {
    RelationNeeds(&programs.0.reads)
}

/// The rules the rule set runs, in rule order: the ones `load` accepted.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[must_use]
pub fn accepted(programs: &Programs) -> Vec<SelectedRuleOrdinal> {
    programs.0.code.rules.iter().map(|rule| rule.rule).collect()
}

/// What each column of each accepted rule's rows holds, in the order of [`accepted`]: what the host reads a found row by.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::columns;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(columns(&Programs::default()).is_empty());
/// ```
#[must_use]
pub fn columns(programs: &Programs) -> Vec<RuleColumns> {
    programs
        .0
        .code
        .rules
        .iter()
        .map(|rule| rule.columns.clone())
        .collect()
}

/// What the fill side reads of a rule set, built once per scan: its code, the words of its kinds and fields, and its
/// regular expressions.
pub(crate) fn rule_set(programs: &ProgramsEngine) -> RuleSetView<'_> {
    RuleSetView {
        code: &programs.code,
        kinds: &programs.kinds,
        fields: &programs.fields,
        regexes: &programs.regexes,
    }
}

/// A worker's per-file state for `programs`: its registers, one for each the programs name, its words over the programs'
/// constants, and an empty store of fills.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::programs::service::scratch_for;
///
/// let _scratch = scratch_for(&Programs::default());
/// ```
#[must_use]
pub fn scratch_for(programs: &Programs) -> ScanScratch {
    let registers = usize::try_from(programs.0.code.counts.registers.0).unwrap_or(usize::MAX);
    ScanScratch(ScratchEngine {
        state: LoopState {
            registers: vec![CellWord(0); registers],
            ..LoopState::default()
        },
        store: FillStore::default(),
        words: file_words(&programs.0.words),
    })
}

/// A worker's per-file state for `programs` that also counts: the runs of each plan point, what each unit was asked and gave,
/// its time, and the sizes of every relation read, which it fills whole in every file. What it counted is read by
/// `counting::service::counted`. The counting is the executor's track's: until it counts, this is the plain state.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::scratch_measuring;
/// use ryo_engine::domain::programs::types::Programs;
///
/// let _scratch = scratch_measuring(&Programs::default());
/// ```
#[must_use]
pub fn scratch_measuring(programs: &Programs) -> ScanScratch {
    let mut scratch = scratch_for(programs);
    let code = &programs.0.code;
    scratch.0.state.measure = Some(Box::new(Measuring {
        points: vec![Count(0); usize::try_from(code.counts.points.0).unwrap_or(usize::MAX)],
        calls: vec![Count(0); code.units.len()],
        keys: vec![Count(0); code.units.len()],
        rows: vec![Count(0); code.units.len()],
        build: vec![Count(0); code.units.len()],
        nanos: vec![Count(0); code.units.len()],
        idle_nanos: vec![Count(0); code.units.len()],
        file: vec![UnitInFile::default(); code.units.len()],
        table_units: code.tables.iter().map(|table| table.unit).collect(),
        kinds: vec![Count(0); programs.0.kinds.len()],
        literals: vec![Count(0); code.sizes.len()],
        ..Measuring::default()
    }));
    scratch
}

/// The step of `rule`'s own plan whose rows the unit named `unit` makes: what a work notice names (`WorkNotice.step`). A unit
/// is named by a merged node, shared by every rule that reaches it; the step is the first of the rule's plan with that node.
/// A unit whose node is no step of the rule's plan (a cut that is one side of a step) answers the step the rule emits; a rule
/// these programs do not run, which has no notice, answers the first step.
#[must_use]
pub(crate) fn unit_step(programs: &Programs, rule: SelectedRuleOrdinal, unit: &UnitKey) -> StepId {
    let (UnitKey::Pattern(node, ..) | UnitKey::Rule(node) | UnitKey::Table(node, _)) = unit;
    let Some(steps) = programs.0.steps.iter().find(|steps| steps.rule == rule) else {
        return StepId(0);
    };
    let place = steps.nodes.0.iter().position(|each| each == node);
    match place.map(u32::try_from) {
        Some(Ok(place)) => StepId(place),
        // No step of the plan bears the node, or the plan has more steps than a step is numbered by.
        Some(Err(_)) | None => steps.emitted,
    }
}

/// The cell a node row's `Kind` column holds for a kind code: the word of the kind's name. None for a code the language's
/// vocabulary states no kind at.
#[must_use]
pub(crate) fn kind_word(programs: &Programs, kind: KindBits) -> Option<CellWord> {
    programs.0.kinds.get(usize::from(kind.0)).copied()
}

/// The cell a node row's `Field` column holds for a field code: the word of the field's name. Code 0, no field, is the
/// empty text's word. None for a code the language's vocabulary states no field at.
#[must_use]
pub(crate) fn field_word(programs: &Programs, field: FieldCode) -> Option<CellWord> {
    programs.0.fields.get(usize::from(field.0)).copied()
}
