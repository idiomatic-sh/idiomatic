//! The `lowering` concept's failures.

use super::types::ArgumentFault;
use ryo_values::domain::program::types::{QueryOrigin, QuerySpan};

/// Why a program does not lower to a plan.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum LoweringError {
    /// A span projection has an extra or repeated named argument.
    #[error("span-between has an extra or repeated argument `{name}`")]
    SpanArgument {
        /// The unexpected argument.
        name: String,
        /// The containing call.
        span: QuerySpan,
    },
    /// Span offsets can only be composed while the file's bytes are available.
    #[error("span-between requires per-file input")]
    CrossFileSpan {
        /// The call that cannot run after the files.
        span: QuerySpan,
    },
    /// The diagnostic-only comparison origin table cannot represent another entry.
    #[error("too many comparison source origins")]
    TooManyComparisonOrigins {
        /// The comparison that exceeded the checked index space.
        span: QuerySpan,
    },
    /// A failure in a linked step attributed to its original module source.
    #[error("{error}")]
    Located {
        /// The original source and import/call trace.
        origin: QueryOrigin,
        /// The lowering refusal.
        error: Box<LoweringError>,
    },
    /// A fragment's declared relation shape does not match an actual input or output.
    #[error("fragment relation contract is not satisfied: {message}")]
    FragmentContract {
        /// Why the actual relation differs.
        message: String,
        /// The original declaration and call trace.
        origin: QueryOrigin,
    },
    /// An appended output would hide an existing column.
    #[error("output column `{name}` already exists")]
    DuplicateOutput {
        /// Repeated output name.
        name: String,
        /// Call containing the duplicate.
        span: QuerySpan,
    },
    /// A literal regular expression is invalid even if the query will read no rows.
    #[error("invalid regular expression: {reason}")]
    InvalidRegex {
        /// The pattern compiler's explanation.
        reason: String,
        /// The comparison containing the invalid pattern.
        span: QuerySpan,
    },
    /// A query reads a relation ryo does not own.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The relation's name as written.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A query reads a step that was not defined above it.
    #[error("no step named `{name}` above this line")]
    UnknownStep {
        /// The step's name as written.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A query names a column its input does not have.
    #[error("no column named `{name}` here")]
    UnknownColumn {
        /// The column's name as written.
        name: String,
        /// Where the query names it.
        span: QuerySpan,
    },
    /// A join's `on`, or a `where` after a join, names `alias.column` with an alias neither side of the join has.
    #[error("no side of the join is named `{alias}`; the join reads {sides}")]
    UnknownSide {
        /// The alias as written.
        alias: String,
        /// The aliases of the two sides, as a list.
        sides: String,
        /// Where the query names it.
        span: QuerySpan,
    },
    /// A `where` after a join names `alias.column` with a column that side does not have.
    #[error("the side `{alias}` of the join has no column named `{name}`")]
    UnknownSideColumn {
        /// The alias as written.
        alias: String,
        /// The column's name as written.
        name: String,
        /// Where the query names it.
        span: QuerySpan,
    },
    /// A query declares a parameter of a type the language does not have.
    #[error("no parameter type `{name}`")]
    UnknownParameterType {
        /// The type as written.
        name: String,
        /// Where it is declared.
        span: QuerySpan,
    },
    /// A comparison puts unlike types side by side.
    #[error("cannot compare these two sides")]
    KindMismatch {
        /// Where the comparison is.
        span: QuerySpan,
    },
    /// A function, `starts_with` or `ends_with` is given an argument that is not text.
    #[error("`{function}` takes text, and its argument {argument} is not text")]
    NotText {
        /// The function as written.
        function: String,
        /// The argument, counted from 1.
        argument: usize,
        /// Where it is applied.
        span: QuerySpan,
    },
    /// A function is given more or fewer arguments than it takes.
    #[error("`{function}` takes {takes}")]
    FunctionArity {
        /// The function as written.
        function: String,
        /// How many arguments it takes, in words.
        takes: &'static str,
        /// Where it is applied.
        span: QuerySpan,
    },
    /// A comparison puts a word next to a column that holds a closed set of words, and the word is not one of them.
    #[error(
        "`{word}` is not {} {column} of {owner}; the {column}s are {words}",
        crate::domain::comparing::constants::article(column)
    )]
    UnknownWord {
        /// The text as written.
        word: Box<str>,
        /// The column it is compared with, as the query names it.
        column: Box<str>,
        /// What the words are the words of: the relation that holds the column.
        owner: Box<str>,
        /// The words, in order, as a list.
        words: Box<str>,
        /// Where the comparison is.
        span: QuerySpan,
    },
    /// An argument has the wrong shape for its operation.
    #[error("`{operation}` cannot use its argument `{argument}`: {reason}")]
    BadArgument {
        /// The operation as written.
        operation: String,
        /// The argument as written.
        argument: String,
        /// Why.
        reason: ArgumentFault,
        /// Where the call is.
        span: QuerySpan,
    },
}

/// Why an argument cannot be used, as the `BadArgument` failure shows it: a value the failure carries, not a failure of its own.
impl std::fmt::Display for ArgumentFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ArgumentFault::Missing => "the argument is missing",
            ArgumentFault::ExpectedName => "expected one name",
            ArgumentFault::ExpectedNames => "expected a list of names",
            ArgumentFault::ExpectedInput => "expected a relation or a step",
            ArgumentFault::ExpectedMap => "expected a map",
            ArgumentFault::ExpectedSideAndColumn => "expected a side and a column",
            ArgumentFault::ExpectedSide => "expected left or right",
            ArgumentFault::SameColumns => "expected the same columns as the left input",
            ArgumentFault::MatchingKeys => "expected as many left keys as right keys, and at least one",
            ArgumentFault::ExpectedDirection => "expected parent, child, ancestor, descendant, nearest-ancestor, previous-sibling or next-sibling",
            ArgumentFault::SiblingOverTree => "a sibling direction walks the relation \"syntax-child\", the edges that keep the children's order",
            ArgumentFault::Undeclared => "an undeclared parameter",
            ArgumentFault::NumbersAndFlagsOnly => "only numbers and flags extend a table",
            ArgumentFault::NumbersAndParametersOnly => "only numbers and parameters extend a table",
            ArgumentFault::ExpectedTextColumn => "expected a column of text",
            ArgumentFault::ExpectedTables => "expected a parameter that holds a list of tables",
            ArgumentFault::ExpectedTextSets => "expected a parameter that holds a list of sets of text",
            ArgumentFault::NoSuchOperation => "no such operation",
            ArgumentFault::NotACall => "select is written as a select, not called",
        })
    }
}
