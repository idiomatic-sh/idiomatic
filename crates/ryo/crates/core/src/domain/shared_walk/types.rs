//! The types of the `shared_walk` concept: the per-file `select` steps of the call's rules over the elements of a file, merged so
//! that the elements are visited once. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::types::Memo;
use crate::domain::operating::types::NameCount;
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::types::Mask;
use crate::domain::positions::types::{ClassPlace, Slots};
use crate::domain::step_classes::constants::ELEMENT_COLUMNS;
use crate::domain::step_classes::types::Merge;
use crate::domain::table_cells::types::{ColumnPlaces, TreeRows};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_scratch::types::{ComparisonNumbers, OperatorScratch, TextVerdicts};
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::Plan;

/// Whether the merge of the call's rules has been made for this worker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Merging {
    /// Not yet: it is made when the worker reads its first file.
    #[default]
    NotYet,
    /// It is.
    Done,
}

/// How many kinds of a file a class names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindCount(pub usize);

/// Whether a class was decided by the walk for the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Merged {
    /// No: its steps run on their own, as they would without the merge.
    #[default]
    Apart,
    /// Yes: its steps take its elements.
    Together,
}

/// What deciding a class of steps over one file works with, kept from file to file so it allocates once.
#[derive(Debug, Default)]
pub(crate) struct ClassWork {
    /// How many kinds the class names in this file.
    pub kinds: KindCount,
    /// The comparisons that ask of the text of an element something fixed.
    pub gate: ComparisonNumbers,
    /// What the gate decided for each text of the file.
    pub text_ok: TextVerdicts,
    /// The comparisons the kind sets and the gate have not decided.
    pub rest: ComparisonNumbers,
    /// The columns those comparisons read.
    pub needed: ColumnPlaces,
    /// What the comparisons decided of each name of the file.
    pub memo: Vec<Memo>,
    /// Whether the class is decided by the walk for this file.
    pub merged: Merged,
}

/// The merge of the call's rules, and what the walk of one file works with.
#[derive(Debug, Default)]
pub(crate) struct MergedWalk {
    /// Whether the classes are made.
    pub made: Merging,
    /// The classes of steps and the class of each step.
    pub merge: Merge,
}

/// Temporary dispatch and comparison work, dropped when the native pass returns.
#[derive(Debug, Default)]
pub(crate) struct WalkWork {
    /// For each class, what deciding it works with.
    pub work: Vec<ClassWork>,
    /// For each kind of the file, the classes that name it.
    pub by_kind: Vec<Vec<ClassPlace>>,
    /// The classes that name no kind: every element is looked at for them.
    pub every: Vec<ClassPlace>,
    /// The columns the classes of one kind read, together.
    pub union: ColumnPlaces,
}

/// File-live outputs of the native pass, borrowed by each independent rule invocation.
#[derive(Debug, Default)]
pub(crate) struct SharedClasses {
    /// Request metadata shared by the native walk and each rule invocation.
    pub matchers:
        Option<std::sync::Arc<crate::domain::matching::types::prepared::value::PreparedMatchers>>,
    /// Prepared literal masks for this file's exact vocabulary, without provider references.
    pub kind_vocabulary:
        Option<std::sync::Arc<crate::domain::kind_sets::types::PreparedVocabulary>>,

    /// Positions selected for each class, preserving their existing order and multiplicity.
    pub classes: Vec<Slots>,
    /// Which classes the pass successfully decided.
    pub merged: Vec<Merged>,
}

/// What one walk of a file reads and writes: the call's plans and what binds their parameters, the file's elements, names and bytes,
/// which rules govern it, and the room the lowered steps take the elements each class selected from.
pub(crate) struct Walking<'a> {
    /// The plans of the rules.
    pub plans: &'a [Plan],
    /// What each rule binds its parameters to.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The path of each file of the run.
    pub paths: &'a [FilePath],
    /// The rules that govern the file.
    pub mask: &'a Mask,
    /// The file.
    pub file: FileId,
    /// The elements of the file.
    pub elements: &'a Elements,
    /// The names of the file.
    pub symbols: &'a mut Symbols,
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The room the operators work in.
    pub scratch: &'a mut OperatorScratch,
    /// The room the lowered steps work in, which the walk leaves the elements each class selected in.
    pub room: &'a mut SharedClasses,
    /// Whether the steps that read only the tree run over its positions.
    pub steps: TreeSteps,
}

/// Whether the cells the classes of one element read have been read for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElementReading {
    /// Not yet: the first class that looks at the element reads them.
    NotYet,
    /// They have: the other classes take them as they are.
    Done,
}

/// What deciding an element for a class works with, kept for the whole pass over a file.
pub(crate) struct Pass<'a> {
    /// Literal matcher slots borrowed from the request, without row ownership.
    pub matchers: Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    /// What each rule binds its parameters to.
    pub bindings: &'a [Vec<ParameterBinding>],
    /// The path of each file of the run.
    pub paths: &'a [FilePath],
    /// The elements of the file, as rows.
    pub tree: TreeRows<'a>,
    /// How many names the file has.
    pub names: NameCount,
    /// The kinds of the columns of an element.
    pub element_kinds: &'a [CellKind],
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The names of the file.
    pub symbols: &'a mut Symbols,
    /// The room the operators work in.
    pub scratch: &'a mut OperatorScratch,
    /// The room the lowered steps work in.
    pub room: &'a mut Vec<Slots>,
    /// The cells of the element being decided.
    pub cells: [Value; ELEMENT_COLUMNS],
    /// Whether they have been read for it.
    pub read: ElementReading,
}
