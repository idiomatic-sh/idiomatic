//! The types of the `file_pass` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::NodeId;
use ryo_values::domain::plan_parts::types::LiteralText;

/// The tree nodes each node of a planned program starts from in the open file, made once per file into buffers kept across
/// files.
#[derive(Debug, Default)]
pub(crate) struct FileRoots {
    /// Each root kind of the program with the nodes that start from it.
    pub(crate) kinds: Vec<RootKindNodes>,
    /// Where each node starts from, by node.
    pub(crate) starts: Vec<TreeStart>,
    /// The open file's tree nodes of each rooted node's kinds, in tree order, by node.
    pub(crate) lists: Vec<Vec<NodeId>>,
    /// Every tree node of the open file, in tree order.
    pub(crate) all: Vec<NodeId>,
}

/// A root kind of a program, held across files: its text and the nodes that start from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootKindNodes {
    /// The kind's text, as the plans write it.
    pub(crate) kind: LiteralText,
    /// The nodes that start from it.
    pub(crate) nodes: Vec<PlanNodeId>,
}

/// Whether a node starts from the tree nodes of its root kinds or from every tree node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TreeStart {
    /// It starts from its root kinds' tree nodes.
    FromKinds,
    /// It starts from every tree node.
    #[default]
    FromEveryNode,
}

/// A request's program as the file pass runs it, made once when the request is prepared: the planned program, the operators
/// that run it, the rules' plans and what each step of each rule reads.
#[derive(Debug)]
pub(crate) struct PassProgram {
    /// The planned program.
    pub(crate) program: ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
    /// The operators that run its nodes.
    pub(crate) operators: std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    /// Each rule's compiled plan, shared with the compilation, in rule order.
    pub(crate) plans: Box<[std::sync::Arc<ryo_values::domain::plans::types::Plan>]>,
    /// What each step of each rule reads, in rule order.
    pub(crate) reads: Box<[Box<[NodeReads]>]>,
    /// What files ran in and gave back, for the next files: one per file running at once.
    pub(crate) scratch: std::sync::Mutex<Vec<PassScratch>>,
}

/// What one file's run fills and gives back to the request for the next file: the nodes' rows, the tree nodes each node starts
/// from and the rules' named outputs, each grown to the program once and refilled per file, so a file allocates only for what
/// it runs.
#[derive(Debug, Default)]
pub(crate) struct PassScratch {
    /// The nodes' rows.
    pub(crate) buffers:
        ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
    /// The tree nodes each node starts from.
    pub(crate) roots: FileRoots,
    /// Each rule's named outputs, in rule order.
    pub(crate) outputs: Vec<Vec<OutputState>>,
}

/// What one step's rows rest on: every relation read on the way to them, and the reads whose absence of rows a step on the way
/// trusts.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NodeReads {
    /// Every relation read on the way to the step's rows, narrowed as the steps narrow it.
    pub(crate) uses: Vec<ryo_execution_domain::domain::trust::types::RelationRead>,
    /// The reads whose absence of rows the step or a step it reads trusts.
    pub(crate) needs: Vec<ryo_execution_domain::domain::trust::types::RelationRead>,
}

/// The rows one file's run reads besides its nodes': the fact lookup's relations, the tree nodes each root starts from, and the
/// first relation the lookup refused.
pub(crate) struct PassInputs<'p, 'a, 's> {
    /// The file's facts.
    pub(crate) lookup: &'p crate::application::file_dag::fact_lookup::service::FactLookup<'a, 's>,
    /// The tree nodes each node starts from.
    pub(crate) roots: &'p FileRoots,
    /// The lookup's first refusal in this file, answered as the facts' own stop.
    pub(crate) refused: std::cell::RefCell<Option<crate::outbound::facts::errors::FactsError>>,
    /// The rows of an input no stage before the fourth serves: never read, since such a rule is refused.
    pub(crate) unserved: ryo_values::domain::rows::types::Table,
}

/// What one file's kernels borrow besides rows: its symbols, bytes and tree, and the paths of the run's files.
pub(crate) struct PassResources<'r> {
    /// The file's symbols, where a made text is interned.
    pub(crate) symbols: &'r mut ryo_language::domain::symbols::types::Symbols,
    /// The file's bytes.
    pub(crate) source: ryo_values::domain::files::types::SourceView<'r>,
    /// The path of each file of the run.
    pub(crate) paths: &'r [ryo_language::domain::files::types::FilePath],
    /// The file's tree.
    pub(crate) elements: &'r ryo_language::domain::elements::types::Elements,
    /// The file's semantic session, when its language has one: what renders syntax.
    pub(crate) semantic: Option<&'r dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    /// The engine's own reason for refusing a rendering, which the run's error reports in place of the operators' fault.
    pub(crate) refusal: Option<ryo_execution_domain::domain::operating::errors::OperatingError>,
}

/// Whether a rule's named output was committed with its frontier in this file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OutputCommit {
    /// Not committed.
    #[default]
    Pending,
    /// Its frontier was kept and committed.
    Committed,
}

/// One named output of a rule in the open file: how ready it is and whether it was committed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OutputState {
    /// Its readiness, once its root ran.
    pub(crate) readiness: Option<ryo_values::domain::records::types::ProducerReadiness>,
    /// Whether it was committed.
    pub(crate) commit: OutputCommit,
}

/// One file's run through the planned program: the request and its program, the rows and buffers its nodes read and fill,
/// its partiality, its bytes and the readiness of its rules' named outputs.
pub(crate) struct PassFile<'f, 'p, 'a, 's> {
    /// The request.
    pub(crate) request: PassRequest<'f>,
    /// Its planned program.
    pub(crate) pass: &'f PassProgram,
    /// The rows the nodes read besides each other's.
    pub(crate) inputs: PassInputs<'p, 'a, 's>,
    /// The nodes' rows in this file.
    pub(crate) buffers:
        ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
    /// The file.
    pub(crate) file: ryo_relations::domain::relations::types::FileId,
    /// What the rules are evaluated with as partial.
    pub(crate) evaluation: &'f [ryo_execution_domain::domain::coverage::types::Partial],
    /// What publication sees as partial: always the physical partiality.
    pub(crate) publication: &'f [ryo_execution_domain::domain::coverage::types::Partial],
    /// The file's bytes.
    pub(crate) source: ryo_values::domain::files::types::SourceView<'f>,
    /// The path of each file of the run.
    pub(crate) paths: &'f [ryo_language::domain::files::types::FilePath],
    /// The file's semantic session, when its language has one.
    pub(crate) semantic: Option<&'f dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    /// Each rule's named outputs in this file, in rule order.
    pub(crate) outputs: Vec<Vec<OutputState>>,
}

/// What a rule's capture is bound to: the rule, the first bucket of each rule and the rule's capture intent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CaptureAt<'a> {
    /// The rule.
    pub(crate) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    /// The first bucket of each rule.
    pub(crate) first: &'a [ryo_values::domain::stored_facts::types::Bucket],
    /// What the rule captures.
    pub(crate) intent: ryo_execution_domain::domain::rule_intent::types::CaptureIntent,
}

/// What one file's run reads of its request: the prepared program, whether the file is the request's, the paths of the run's
/// files and the observer the steps are reported to.
#[derive(Clone)]
pub(crate) struct PassRequest<'r> {
    /// The prepared program: its rules' order, intents, frontiers and named outputs.
    pub(crate) program:
        &'r crate::application::file_dag::program::preparation::types::PreparedProgram,
    /// Whether the program runs the file: its intents pair with the rules and the file is the request's, or why not.
    pub(crate) admission:
        Result<(), ryo_execution_domain::domain::operating::errors::OperatingError>,
    /// The path of each file of the run.
    pub(crate) paths: &'r [ryo_language::domain::files::types::FilePath],
    /// Where each rule's steps are reported, when the request has an observer.
    pub(crate) observer: Option<&'r dyn crate::outbound::observer::interface::ObserverPort>,
}

/// A node's rows in the open file and how completely its reads covered them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodeAnswer {
    /// The rows.
    pub(crate) table: ryo_values::domain::rows::types::Table,
    /// Their coverage.
    pub(crate) coverage: ryo_execution_domain::domain::completion::types::CoverageState,
}

/// What the file pass's program is made of, kept by the prepared program: the rules' plans and bindings and the operators.
#[derive(Debug)]
pub(crate) struct PassSource {
    /// Each rule's compiled plan, shared with the compilation, in rule order.
    pub(crate) plans: Vec<std::sync::Arc<ryo_values::domain::plans::types::Plan>>,
    /// What each rule binds its parameters to.
    pub(crate) bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    /// The operators that plan and run the program.
    pub(crate) operators: std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
}

/// Whether a request holds any rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RulesHeld {
    /// It holds none.
    None,
    /// It holds one or more.
    Some,
}

/// The latest stage among a request's rules that no stage serves yet, and whether it holds a rule at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stages {
    /// The latest unserved stage, when a rule is not served.
    pub(crate) latest: Option<ryo_operators::domain::physical_plan::types::ServedAt>,
    /// Whether the request holds any rule.
    pub(crate) held: RulesHeld,
}
