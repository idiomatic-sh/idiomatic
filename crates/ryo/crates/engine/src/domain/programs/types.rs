//! The types of the `programs` concept: a rule set's programs and the instruction set they are written in. Everything but
//! `Programs` itself is the crate's own: the lowering writes it, the packs encode it, the executor runs it.
use crate::domain::code::types::ProgramCode;
use crate::domain::scratch::types::NodeNeed;
use crate::domain::steps::types::{ConstantText, StepRegex};
use ryo_language::domain::facts::types::{BaseWords, CellWord};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_plans::types::NodeDigests;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The programs of a rule set: opaque; built only by `load`, encoded by `packs::service`.
///
/// ```compile_fail,E0603
/// let forged = ryo_engine::domain::programs::types::Programs(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[derive(Debug, Default)]
pub struct Programs(pub(crate) ProgramsEngine);

/// The private body of `Programs`: what a scan runs, after every load-time pass.
#[derive(Debug, Default)]
pub(crate) struct ProgramsEngine {
    /// The programs, encoded.
    pub(crate) code: ProgramCode,
    /// The run's base words, built once at load in this order: word 0 is the empty text; then the language's fixed names,
    /// each at its place; then the pack's constant texts in their order (`LoweredPack.texts`); then the names of the
    /// vocabulary's kinds and of its fields that are not words yet. A name that is already a word keeps that word: a kind
    /// named as a fixed name or as a constant text has one word. A file's own texts are words after these: a token's
    /// `Text` cell is made on its first read by `ryo_language::domain::facts::service::intern(words, text)`, given the
    /// scratch's `FileWords` and the bytes of the file the token's span covers (`node_table::service::span_of`), which
    /// answers the base word when the text has one and else interns it for the file.
    pub(crate) words: BaseWords,
    /// The regular expressions the programs match, by number, as their sources.
    pub(crate) regexes: Vec<StepRegex>,
    /// Every relation some program reads: what a language builds its model for (`RelationNeeds`).
    pub(crate) reads: Vec<Relation>,
    /// The word of each kind of the language, by kind code: a kind's code is its place among the vocabulary's kinds. What a
    /// node row's `Kind` column holds (`service::kind_word`).
    pub(crate) kinds: Vec<CellWord>,
    /// The word of each field of the language, by field code: a field's code is its place among the vocabulary's fields, and
    /// place 0 is "no field", whose name is the empty text: its cell is the empty text's word, word 0. What a node row's
    /// `Field` column holds (`service::field_word`).
    pub(crate) fields: Vec<CellWord>,
    /// For each rule of the code, in the code's rule order: the digests of its own plan's steps, by which a unit's plan step is
    /// found for that rule (`service::unit_step`). A unit is named by a merged node, which is no step of any one plan.
    pub(crate) steps: Vec<RuleSteps>,
    /// The pack's constant texts, in their order: the words after the fixed names. Kept so a pack can be written.
    pub(crate) texts: Vec<ConstantText>,
    /// For each need site of the code's rules' branches and tables, by site, whether its formula asks the node rows (none
    /// for any other place): a function of the code, found once at load (or read from the pack) and handed to every
    /// worker's state, which finds any other site at its first ask.
    pub(crate) node_needs: Vec<Option<NodeNeed>>,
}

/// One rule's plan as a work notice needs it: where each node is in the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleSteps {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// The step the plan emits: the rule's own unit.
    pub(crate) emitted: StepId,
    /// The digest of each step of the plan, in step order.
    pub(crate) nodes: NodeDigests,
}
