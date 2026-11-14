//! Immutable plan layouts, shared execution accounting vocabulary and what an operator reads.

use ryo_operators::domain::table_scratch::types::Memo;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{AfterTheFiles, SemanticLookup};
use ryo_operators::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::stored_facts::types::ArenaBytes;

pub mod axis_walk;
pub mod graph_lookups;
pub mod graph_walks;
pub mod join_candidates;
pub mod lease_driver;
pub mod row_delivery;

/// How many rows a step was fed: the rows of each of its inputs, by the input's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsFed(pub [u64; 3]);

/// How many rows each step of a rule was fed, by the step's place.
pub struct LogicalCounts {
    /// The rows fed to each step, by the step's place.
    pub fed: Box<[RowsFed]>,
}

/// A step of a plan, by its place among the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepPlace(pub usize);

/// A plan, by where it is: the same plan has the same identity in every file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlanIdentity(pub usize);

/// Number of independent readers of the same publication.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderMultiplicity(pub usize);

/// What an operator reads besides its inputs and may write: what the host bound, the names and
/// bytes of the file the rows are of, and the paths of the files. An operator may add only derived
/// names; language fact readers alone own source-name interning.
#[derive(Debug)]
pub struct OperatorContext<'a> {
    /// Execution phase supplied by the owner, independent of available text arenas.
    pub phase: AfterTheFiles,
    /// The current file's application-owned semantic session, when available.
    pub semantic: Option<&'a dyn SemanticLookup>,
    /// The matching semantics the parent supplies for regular expressions and globs.
    pub matcher: ryo_operators::domain::patterns::types::PatternMatcher<'a>,
    /// Finite authored literal matchers owned by this worker's prepared request.
    pub prepared_matchers:
        Option<&'a ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    /// Finite prepared literal-kind metadata for the current exact vocabulary.
    pub prepared_kinds:
        Option<&'a crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>,

    /// What each parameter is bound to, by the parameter's place.
    pub bindings: &'a [ParameterBinding],
    /// Readable file names and a derived-only query interning capability.
    pub symbols: QuerySymbols<'a>,
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The path of each file of the run, by file number.
    pub paths: &'a [FilePath],
    /// The text each worker made, by worker number, when the steps run after the files were read.
    pub arenas: &'a [ArenaBytes],
    /// What a `select` has already decided about a name, for each comparison and each name of the
    /// file: kept between steps and files so it allocates once.
    pub memo: &'a mut Vec<Memo>,
    /// The room the operators read and sort rows in.
    pub scratch: &'a mut OperatorScratch,
}

/// Where the rows of a step go as they are made, one at a time. The context is the one of the step that makes them; the
/// step that takes them makes its own from it.
pub trait Emit {
    /// Takes one row.
    ///
    /// # Errors
    ///
    /// Whatever the step that takes it refuses.
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError>;
}
