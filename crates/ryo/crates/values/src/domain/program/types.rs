//! The types of the `program` concept: the shape of RyoQL text once it is read. Plain data, public
//! fields and derives, with no constructor and no inherent impl.

use crate::domain::descriptions::types::ColumnName;
use crate::domain::query_syntax::types::{ArgumentValue, Expression};
use crate::domain::query_tokens::types::{
    Attribution, Linking, QueryOrigin, QuerySourceId, QuerySpan, StepIdentity, StepName, TypeName,
    WrittenLiteral, WrittenName, WrittenParameter,
};
use ryo_relations::domain::relations::types::Span;
use std::collections::BTreeMap;

/// A whole query: its parameters, its steps and the one `emit`.
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenProgram {
    /// Whether source-local identities and imports have been linked for a complete query bundle.
    pub linked: Linking,
    /// The imported modules available to root-level calls.
    pub imports: Vec<ImportDeclaration>,
    /// Source attribution for original and linked syntax nodes.
    pub origins: Vec<QueryOrigin>,
    /// Typed signatures retained for every linked fragment invocation.
    pub fragment_contracts: Vec<FragmentContract>,
    /// Original source and call trace for each linked step definition.
    pub step_origins: BTreeMap<StepIdentity, QueryOrigin>,
    /// Original source for the root emit statement.
    pub emit_origin: Attribution,
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

/// `import suppress from "vendor/tool/suppress";`.
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
    pub root: WrittenProgram,
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
    pub parameter: WrittenParameter,
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
    pub origin: Attribution,
}

/// Which fault a query text was refused for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusedKind {
    /// The text is not RyoQL.
    Syntax,
    /// The query reads a relation ryo does not own.
    UnknownRelation,
    /// A comparison puts unlike types side by side.
    Type,
    /// The query is wrong in another way.
    Invalid,
    /// The catalog of relations could not be read.
    Catalog,
}

/// What a refusal says: the relation a query named that is not one, or the reason, as the compiler worded it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefusalMessage(pub String);

/// A query text that was refused: which fault, what it says, and where in the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusedQuery {
    /// Which fault.
    pub kind: RefusedKind,
    /// What it says.
    pub message: RefusalMessage,
    /// Where in the text; empty for a fault that is not in the text.
    pub span: Span,
    /// The original query-module source and import/call path, when the refusal came from a fragment.
    pub origin: Attribution,
}

impl std::fmt::Display for RefusedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = &self.message.0;
        match self.kind {
            RefusedKind::Syntax => write!(f, "{message} at byte {}", self.span.start.0),
            RefusedKind::UnknownRelation => write!(f, "no relation named `{message}`"),
            RefusedKind::Type | RefusedKind::Invalid => write!(f, "{message}"),
            RefusedKind::Catalog => write!(f, "the relation catalog failed: {message}"),
        }
    }
}
