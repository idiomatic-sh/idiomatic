//! The types of the `program` concept: the shape of RyoQL text once it is read. Plain data, public
//! fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::Integer;
use std::collections::BTreeMap;

/// A place in the query text, counted in bytes from its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(pub usize);

/// A stretch of the query text, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuerySpan {
    /// Where it starts.
    pub start: ByteOffset,
    /// Where it ends.
    pub end: ByteOffset,
}

/// RyoQL text as the port reads it: borrowed from the host's query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryView<'a>(pub &'a str);

/// Stable logical identity of one source in a query bundle.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuerySourceId(pub Box<str>);

/// One exact source in a query bundle, with its logical identity retained for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryDocumentSource {
    /// The stable logical identity used by imports and diagnostics.
    pub id: QuerySourceId,
    /// Exact UTF-8 query bytes held by this source.
    pub text: Box<str>,
}

/// The in-memory root and immutable named module sources for one query.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryBundle {
    /// The entry document compiled for this query.
    pub root: QueryDocumentSource,
    /// Explicitly registered immutable modules, keyed by logical identity.
    pub modules: BTreeMap<QuerySourceId, QueryDocumentSource>,
}

/// The name of a parameter, as the query writes it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenParameter(pub Box<str>);

/// The type of a parameter, as the query writes it: `integer`, `list<string>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeName(pub Box<str>);

/// A stable identity for one step definition within a linked query bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepIdentity(pub u64);

/// The name a `let` gives a step, or a step is read under; identity is separate from spelling.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepName(pub Box<str>, pub Option<StepIdentity>);

/// The name of a column, as the query writes it or keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnName(pub Box<str>);

/// The alias a source is given after `as`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AliasName(pub Box<str>);

/// The name of a relation, written in quotes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenRelation(pub Box<str>);

/// The name of an operation a call applies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationName(pub Box<str>);

/// The name of an argument of a call.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentName(pub Box<str>);

/// Text the query wrote between quotes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenLiteral(pub Box<str>);

/// A bare word the query wrote: a column, a step or a keyword such as `descendant`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordText(pub Box<str>);

/// A name the query wrote, bare or in quotes: how a column or an argument is written.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenName(pub Box<str>);

/// A bare word as the parsers read it: borrowed from the query text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BareWord<'a>(pub &'a str);

/// A name as the query spells it, borrowed from the program: a relation, a step, a column or a
/// parameter, whichever the program wrote in text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuerySpelling<'a>(pub &'a str);

/// A whole query: its parameters, its steps and the one `emit`.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Whether source-local identities and imports have been linked for a complete query bundle.
    pub linked: bool,
    /// The imported modules available to root-level calls.
    pub imports: Vec<ImportDeclaration>,
    /// Source attribution for original and linked syntax nodes.
    pub origins: Vec<QueryOrigin>,
    /// Typed signatures retained for every linked fragment invocation.
    pub fragment_contracts: Vec<FragmentContract>,
    /// Original source and call trace for each linked step definition.
    pub step_origins: BTreeMap<StepIdentity, QueryOrigin>,
    /// Original source for the root emit statement.
    pub emit_origin: Option<QueryOrigin>,
    /// The `param` lines.
    pub parameters: Vec<ParameterDeclaration>,
    /// The `let` lines, in order.
    pub steps: Vec<LetStep>,
    /// Explicit named observation roots, separate from diagnostics.
    pub observations: Vec<WrittenObservation>,
    /// The `emit` line.
    pub emit: WrittenEmit,
}

/// A fragment invocation's typed input/output contract after hygienic linking.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentContract {
    /// The first expanded definition, before which inputs are available.
    pub first: StepIdentity,
    /// The returned definition, whose columns must match the declared output.
    pub result: StepIdentity,
    /// Inputs bound at the call site.
    pub inputs: Vec<FragmentContractInput>,
    /// Declared output relation shape.
    pub output: RelationSignature,
    /// Where the output signature is written.
    pub output_span: QuerySpan,
    /// Original fragment identity and import/call trace.
    pub origin: QueryOrigin,
}

/// One input relation and the source actual bound at its call site.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentContractInput {
    /// The fragment's input name.
    pub name: WrittenName,
    /// The source actual passed to that input.
    pub value: ArgumentValue,
    /// The required relation shape.
    pub relation: RelationSignature,
    /// Where the input is declared.
    pub span: QuerySpan,
}

/// Where a linked syntax node came from, including the import and call path used to reach it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryOrigin {
    /// The source document that wrote the node.
    pub source_id: QuerySourceId,
    /// The original byte span in that document.
    pub span: QuerySpan,
    /// Import and call sites from the root to this source.
    pub trace: Vec<QueryTraceSite>,
}

/// One source boundary crossed while linking a fragment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryTraceSite {
    /// The source containing this import or call.
    pub source_id: QuerySourceId,
    /// The imported or invoked source.
    pub target_id: QuerySourceId,
    /// The original byte span of the import or invocation.
    pub span: QuerySpan,
}

/// `import suppress from "python/ruff/suppress";`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportDeclaration {
    /// The local name of the imported module.
    pub alias: WrittenName,
    /// Its logical id in the immutable query bundle.
    pub module: WrittenLiteral,
    /// Where the import is written.
    pub span: QuerySpan,
}

/// A relation shape written in a fragment signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationSignature {
    /// Columns in their declared order.
    pub columns: Vec<SignatureColumn>,
    /// Where the `relation<...>` type is written.
    pub span: QuerySpan,
}

/// One named column and its RyoQL cell type in a relation signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureColumn {
    /// The column name.
    pub name: ColumnName,
    /// The cell type name.
    pub kind: TypeName,
    /// Where the name and type are written.
    pub span: QuerySpan,
}

/// A fragment input relation declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentInput {
    /// The name the fragment body uses as its relation source.
    pub name: WrittenName,
    /// The required relation shape.
    pub relation: RelationSignature,
    /// Where the declaration is written.
    pub span: QuerySpan,
}

/// A reusable RyoQL fragment module, parsed without treating it as a root query.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentProgram {
    /// Modules visible to calls from this fragment.
    pub imports: Vec<ImportDeclaration>,
    /// Relation-valued inputs.
    pub inputs: Vec<FragmentInput>,
    /// Scalar parameters.
    pub parameters: Vec<ParameterDeclaration>,
    /// The declared output shape.
    pub output: RelationSignature,
    /// Where the output declaration is written.
    pub output_span: QuerySpan,
    /// The fragment's local steps.
    pub steps: Vec<LetStep>,
    /// The step returned as the fragment result.
    pub result: FragmentReturn,
}

/// Parsed root and module syntax before query-language linking runs.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedQueryBundle {
    /// Parsed root query.
    pub root: Program,
    /// Parsed module documents keyed by their bundle ids.
    pub fragments: BTreeMap<QuerySourceId, FragmentProgram>,
}

/// The step returned by a fragment and the source location of its declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentReturn {
    /// The local result step.
    pub step: StepName,
    /// Where the return declaration is written.
    pub span: QuerySpan,
}

/// `param max-lines: integer;`.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterDeclaration {
    /// The parameter's name.
    pub name: WrittenParameter,
    /// Its type, as written: `integer`, `list<string>`.
    pub kind: TypeName,
    /// Where it is written.
    pub span: QuerySpan,
}

/// `let long = select * from "filesystem.line-count";`.
#[derive(Clone, Debug, PartialEq)]
pub struct LetStep {
    /// The step's name.
    pub name: StepName,
    /// What it computes.
    pub expression: Expression,
    /// Where it is written.
    pub span: QuerySpan,
}

/// What a `let` computes.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    /// `select ... from ... join ... where ...`.
    Select(Select),
    /// `name(argument=value, ...)`.
    Call(Call),
}

/// `select columns from source join source on a = b where filters`.
#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    /// The columns kept: every one for `*`, or those listed.
    pub columns: Columns,
    /// What it reads.
    pub from: QuerySource,
    /// The join, when there is one.
    pub join: Joining,
    /// The `where` conditions, all of which must hold.
    pub filters: Vec<Condition>,
}

/// One condition of a `where`: a test, or the alternatives an `or` gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    /// One test: `name = "main"`.
    Test(Filter),
    /// `a or b and c`: at least one alternative holds, each a list of conditions that all hold. There are two or more.
    Any(Vec<Vec<Condition>>),
}

/// The columns a `select` keeps.
#[derive(Clone, Debug, PartialEq)]
pub enum Columns {
    /// `*`: every column of what it reads.
    Every,
    /// The columns the query lists.
    Listed(Vec<SelectColumn>),
}

/// Whether a `select` joins a second source to the one it reads.
#[derive(Clone, Debug, PartialEq)]
pub enum Joining {
    /// It reads one source.
    Alone,
    /// It joins this second source.
    Joined(Join),
}

/// The name a column of a `select` is kept under.
#[derive(Clone, Debug, PartialEq)]
pub enum Renaming {
    /// It keeps the name it is read under.
    Unchanged,
    /// `as` gives it this name.
    Renamed(ColumnName),
}

/// Whether the query writes an alias before a column or after a source.
#[derive(Clone, Debug, PartialEq)]
pub enum Alias {
    /// The query writes none.
    Unwritten,
    /// The query writes this one.
    Written(AliasName),
}

/// One column of a `select`, with the name it is kept under.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectColumn {
    /// The column read.
    pub column: ColumnReference,
    /// The name it is kept under.
    pub rename: Renaming,
}

/// A column, with the alias of the source it belongs to when the query writes one.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnReference {
    /// `l` in `l."id"`.
    pub alias: Alias,
    /// `id` in `l."id"`.
    pub column: ColumnName,
}

/// What a `select` or a `join` reads.
#[derive(Clone, Debug, PartialEq)]
pub struct QuerySource {
    /// The relation or the earlier step.
    pub reference: SourceReference,
    /// The alias after `as`.
    pub alias: Alias,
    /// Where it is written.
    pub span: QuerySpan,
}

/// A name a query reads.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceReference {
    /// A relation, written in quotes: `"syntax-tree"`.
    Relation(WrittenRelation),
    /// An earlier step, written bare: `functions`.
    Step(StepName),
}

/// `join source on l.a = r.b and l.c = r.d`.
#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    /// The right side.
    pub source: QuerySource,
    /// The keys the rows are joined on, each a column of each side.
    pub on: Vec<JoinKey>,
}

/// One `l.a = r.b` of a join.
#[derive(Clone, Debug, PartialEq)]
pub struct JoinKey {
    /// The left key.
    pub left: ColumnReference,
    /// The right key.
    pub right: ColumnReference,
}

/// One `where` condition: `left test right`.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    /// The left operand.
    pub left: WrittenOperand,
    /// How they are compared.
    pub test: WrittenTest,
    /// The right operand.
    pub right: WrittenOperand,
    /// Where it is written.
    pub span: QuerySpan,
}

/// How a filter compares its operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrittenTest {
    /// `=`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    LessThan,
    /// `<=`.
    LessThanOrEqual,
    /// `>`.
    GreaterThan,
    /// `>=`.
    GreaterThanOrEqual,
    /// `in`.
    In,
    /// `not in`.
    NotIn,
    /// `glob_any(left, right)`.
    GlobAny,
    /// `not glob_any(left, right)`.
    NotGlobAny,
    /// `float_text_gte(left, right)`: numbers written as text, compared as numbers.
    FloatTextGreaterThanOrEqual,
    /// `starts_with(text, prefix)`.
    StartsWith,
    /// `ends_with(text, suffix)`.
    EndsWith,
    /// `matches(text, pattern)`: a regular expression searches the text.
    Matches,
    /// `numeric_gte(left, right)`: exact integer and binary64 numeric ordering.
    NumericGreaterThanOrEqual,
    /// `integer_successor(left, right)`: right is exactly left plus one; integers only.
    IntegerSuccessor,
    /// `natural_lt(left, right)`: case-sensitive natural text order; ASCII digit runs,
    /// fraction-style zero prefixes, Unicode whitespace separators, no normalization.
    NaturalLessThan,
}

/// One side of a filter.
#[derive(Clone, Debug, PartialEq)]
pub enum WrittenOperand {
    /// A column.
    Column(ColumnReference),
    /// `$name`.
    Parameter(WrittenParameter),
    /// `"text"`.
    Text(WrittenLiteral),
    /// `42`.
    Number(Integer),
    /// `pascal(pkg)`: a function applied to operands.
    Function(WrittenApplied),
}

/// A function a filter applies to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrittenFunction {
    /// `concat(a, b, ...)`.
    Concat,
    /// `pascal(text)`.
    Pascal,
    /// `camel(text)`.
    Camel,
    /// `snake(text)`.
    Snake,
    /// `kebab(text)`.
    Kebab,
    /// `lower(text)`: Unicode lowercase text without changing separators.
    Lower,
}

/// `concat(pascal(pkg), "Service")`: a function and the operands it is applied to.
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenApplied {
    /// The function.
    pub function: WrittenFunction,
    /// Its arguments, in the order written.
    pub arguments: Vec<WrittenOperand>,
    /// Where it is written.
    pub span: QuerySpan,
}

/// `name(argument=value, ...)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// The operation's name.
    pub name: OperationName,
    /// Its arguments, in the order written.
    pub arguments: Vec<Argument>,
    /// Where it is written.
    pub span: QuerySpan,
}

/// `name=value`.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The argument's name.
    pub name: ArgumentName,
    /// Its value.
    pub value: ArgumentValue,
}

/// The value of an argument.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentValue {
    /// `"text"`.
    Text(WrittenLiteral),
    /// `42`.
    Number(Integer),
    /// A bare word: a column, a step or a keyword such as `descendant`.
    Word(WordText),
    /// A relation input resolved to one exact step definition by the linker.
    Step(StepName),
    /// `$name`.
    Parameter(WrittenParameter),
    /// `[a, b]`.
    List(Vec<ArgumentValue>),
    /// `{key: value}`.
    Map(Vec<Argument>),
}

/// `emit step subject column key(columns);`.
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenEmit {
    /// The step whose rows answer the query.
    pub step: StepName,
    /// The column holding the path a row is about.
    pub subject: ColumnName,
    /// The columns that identify a row.
    pub key: Vec<ColumnName>,
    /// Where it is written.
    pub span: QuerySpan,
}

/// An explicit top-level named-output declaration. Linking resolves its source identity once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenObservation {
    /// The actual source step.
    pub step: StepName,
    /// The exported name.
    pub name: crate::domain::observations::types::ObservationName,
    /// The complete declaration's source range.
    pub span: QuerySpan,
    /// Attribution preserved through hygienic fragment expansion.
    pub origin: Option<QueryOrigin>,
}
