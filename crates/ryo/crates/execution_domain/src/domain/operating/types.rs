//! The types of the `operating` concept: what an operator reads, writes and fails with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;

use crate::domain::table_model::types::Rows;
use ryo_values::domain::query_bindings::types::ParameterBinding;

use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// An application-owned, file-scoped semantic demand port. Operators borrow it;
/// they never own a provider session or its cache.
pub trait SemanticLookup: std::fmt::Debug {
    /// Resolve demanded keys in order; the current application deduplicates each batch.
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: SourceView<'_>,
    ) -> Result<Vec<RenderOutcome>, OperatingError>;
}

/// The families of comparison a `select` tests with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Family {
    /// Equal and not equal.
    Equality,
    /// Less, greater, and their or-equal.
    Ordering,
    /// Numbers written as text.
    FloatText,
    /// Exact canonical numeric text and whole numbers.
    Numeric,
    /// In and not in.
    Membership,
    /// Glob matches.
    Glob,
    /// Starts with and ends with.
    Affix,
    /// Regular expression searches.
    Regex,
}

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
    pub matcher: crate::domain::matching::types::PatternMatcher<'a>,
    /// Finite authored literal matchers owned by this worker's prepared request.
    pub prepared_matchers:
        Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    /// Finite prepared literal-kind metadata for the current exact vocabulary.
    pub prepared_kinds: Option<&'a crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>,

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

/// What a comparison decided about a name of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Memo {
    /// Not decided yet.
    Unknown,
    /// The comparison fails for the name.
    Fails,
    /// The comparison holds for the name.
    Holds,
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

/// How many names a file has: the length of a comparison's run of the memo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameCount(pub usize);

/// A comparison of a `select`, by its place among the select's comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonNumber(pub usize);

/// One of the inputs of a step, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPlace(pub usize);

/// Whether a comparison asks of the kind of an element something fixed, which the sets of kinds decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindNaming {
    /// It names a kind.
    Names,
    /// It does not.
    Silent,
}

/// Whether the steps run before the files were read, or after: after, the text of the workers is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterTheFiles {
    /// While the files are read.
    Before,
    /// After the files were read.
    After,
}

use ryo_values::domain::rows::types::{RowCount, Table};
