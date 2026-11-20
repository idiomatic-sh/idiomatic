//! The types of the `file_pass` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
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
    /// Spike (ryo-fmx3, host3): by node, the one root kind (its place in `kinds`) it starts from, or `u32::MAX` when it starts
    /// from several (its list is then its own, in `lists`) or from none.
    pub(crate) single: Vec<u32>,
    /// Spike (ryo-fmx3, host3): the open file's tree nodes of each root kind, by its place in `kinds`, filled only for the
    /// kinds the file holds.
    pub(crate) kind_lists: Vec<Vec<NodeId>>,
    /// Spike (ryo-fmx3, host3): the root kinds whose list the open file filled, cleared at the next file.
    pub(crate) kinds_filled: Vec<u32>,
    /// Spike (ryo-fmx3, host3): the nodes of several root kinds whose own list the open file filled, cleared at the next file.
    pub(crate) nodes_filled: Vec<u32>,
    /// Spike (ryo-fmx3, host3): the kind names of the last file mapped, as texts, and each root kind's number among them:
    /// a file naming the same kinds reuses the numbers.
    pub(crate) names_seen: Vec<Box<str>>,
    /// Spike (ryo-fmx3, host3): each root kind's number in the files named as `names_seen`, by its place in `kinds`.
    pub(crate) numbers: Vec<Option<u16>>,
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
    /// File-specific ports available after the provider proves the source condition.
    pub(crate) conditional: Option<Box<ConditionalFileProgram>>,
    /// The planned program.
    pub(crate) program: ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
    /// The operators that run its nodes.
    pub(crate) operators: std::sync::Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
    /// Each rule's compiled plan, shared with the compilation, in rule order.
    pub(crate) plans: Box<[std::sync::Arc<ryo_values::domain::plans::types::Plan>]>,
    /// What each step of each rule reads, in rule order.
    pub(crate) reads: Box<[ryo_execution_domain::domain::trust::types::read_sets::value::PlanReadSets]>,
    /// What files ran in and gave back, for the next files: one per file running at once.
    pub(crate) scratch: std::sync::Mutex<Vec<PassScratch>>,
    /// Each rule's id, in rule order: what the node counters name a node's rules by.
    pub(crate) ids: Box<[ryo_values::domain::runs::types::RuleId]>,
    /// Spike (ryo-fmx3): the host relations each rule's steps in a file read, in rule order, found once for the program.
    pub(crate) hosted_steps: Box<[Box<[ryo_values::domain::plan_parts::types::HostedNumber]>]>,
    /// Spike (ryo-8y7q): by node, the last rule turn in a file that can ask for it, found at the first file.
    pub(crate) turns: std::sync::OnceLock<std::sync::Arc<[u32]>>,
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
    /// Each rule's named outputs' rows in the open file, or what refused them, by rule and output, for the bound observations of
    /// the rules after it.
    pub(crate) answers: Vec<
        Vec<
            Option<
                Result<NodeAnswer, ryo_execution_domain::domain::operating::errors::OperatingError>,
            >,
        >,
    >,
    /// The relations the open file's sources are asked for: those of the rules that run on it.
    pub(crate) demand: Vec<ryo_relations::domain::relations::types::Relation>,
    /// Whether a rule that runs on the open file can run each node, by node.
    pub(crate) live: Vec<NodeInFile>,
}

/// Whether a rule that runs on the open file can run a node: a step in each file of such a rule can; any other node runs
/// nothing in the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NodeInFile {
    /// No rule that runs on the file can run it.
    #[default]
    Unrunnable,
    /// A rule that runs on the file can run it.
    Runnable,
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
    /// The tables the running rule's bound observations in this file are bound to.
    pub(crate) bound: crate::application::file_dag::publications::types::BoundTables,
    /// Why each withheld input of this file is withheld.
    pub(crate) withholdings: crate::application::file_dag::frontiers::types::Withholdings,
    /// The host's rows of each relation a rule of this file read, in the file's symbols, by the relation's place.
    pub(crate) hosted: Vec<Option<ryo_values::domain::rows::types::Table>>,
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
    pub(crate) semantic:
        Option<&'r dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    /// The file's refused renderings, each the engine's own reason, where a refusal of this run is recorded.
    pub(crate) refusals:
        &'r mut Vec<ryo_execution_domain::domain::operating::errors::OperatingError>,
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
    /// Which route the complete file sources authorize.
    pub(crate) condition: SourceCondition,
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
    pub(crate) semantic:
        Option<&'f dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    /// The engine's reason for each rendering it refused in this file, by the `RenderRefusal` the refused node's fault carries:
    /// every reader of that node answers that node's reason.
    pub(crate) refusals: Vec<ryo_execution_domain::domain::operating::errors::OperatingError>,
    /// Each rule's named outputs in this file, in rule order.
    pub(crate) outputs: Vec<Vec<OutputState>>,
    /// Each rule's named outputs' rows in this file, or what refused them, by rule and output, for the bound observations of the
    /// rules after it.
    pub(crate) answers: Vec<
        Vec<
            Option<
                Result<NodeAnswer, ryo_execution_domain::domain::operating::errors::OperatingError>,
            >,
        >,
    >,
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
    /// The columns of each relation the host declares, by its place.
    pub(crate) hosted: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    /// Each rule's id, in rule order.
    pub(crate) ids: Vec<ryo_values::domain::runs::types::RuleId>,
}

/// What the pass reads of one file once its facts are open: the file, its intents and source, the partiality its rules see, its
/// semantic session and each rule's first frontier bucket.
pub(crate) struct OpenedFile<'a> {
    /// The file.
    pub(crate) file: ryo_relations::domain::relations::types::FileId,
    /// Exactly one intent for every prepared rule.
    pub(crate) intents: &'a [ryo_execution_domain::domain::rule_intent::types::FileRuleIntent],
    /// The file's source text.
    pub(crate) source: ryo_values::domain::files::types::SourceView<'a>,
    /// Whether evaluation sees the file's physical partiality or none of it.
    pub(crate) answering: crate::application::file_dag::program::file_session::types::Partiality,
    /// The semantic session the file is read with.
    pub(crate) semantic:
        crate::application::file_dag::program::file_session::types::SemanticSupport<'a>,
    /// The first frontier bucket of each rule, in rule order.
    pub(crate) first: &'a [ryo_values::domain::stored_facts::types::Bucket],
}

/// One move of answering a node: run a node, read an input of another kind before a refusal (admit a relation's facts, or
/// hold a frontier entry or a bound observation, a withheld one answering why), or answer a deferred refusal's empty rows with
/// no column.
#[derive(Clone, Copy, Debug)]
pub(crate) enum NodeMove {
    /// Run the node.
    Run(PlanNodeId),
    /// Read the input, which is no node.
    Read(PlanInput),
    /// Answer no column of the node to its readers: it holds no row and its refusal is deferred to its rows.
    Empty(PlanNodeId),
}

/// Whether the current subscriber listens to the node counters, so a run counts its nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CountersListened {
    /// It listens: the run counts each node and emits the counts at its end.
    Listening,
    /// It does not: the run counts nothing and emits nothing.
    Unheard,
}

/// Routes through the single physical graph and the evidence required to select them.
#[derive(Debug)]
pub(crate) struct ConditionalFileProgram {
    /// The ordered source predicate for this prepared program.
    pub(crate) absence: ryo_execution_domain::domain::executable_program::types::SourceAbsence,
    /// Public ports on the request's single prepared graph.
    pub(crate) routes:
        ryo_execution_domain::domain::executable_program::routes::types::FileProgramRoutes,
}

/// The outcome of inspecting a file for a prepared source condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceCondition {
    /// Some row could select the branch.
    Unproven,
    /// The complete file source proves the branch empty.
    Proven,
}
