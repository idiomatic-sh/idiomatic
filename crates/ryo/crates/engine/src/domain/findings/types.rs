//! The types of the `findings` concept: the rows a scan hands over, the frontier rows the project pass reads back, and what a scan
//! counted.

use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{InputNumber, StepId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Where the host takes found rows as they are complete.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FoundCells, FoundRows, FoundTarget, WorkNotice};
/// use ryo_language::domain::facts::types::FileWords;
///
/// struct Counted(usize);
/// impl FoundRows for Counted {
///     fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
///         self.0 += 1;
///     }
///     fn noticed(&mut self, _notice: WorkNotice) {}
/// }
/// let mut counted = Counted(0);
/// let target = FoundTarget::Rule(ryo_values::domain::rule_requests::types::SelectedRuleOrdinal(0));
/// counted.found(target, FoundCells(&[]), &FileWords::default());
/// assert_eq!(counted.0, 1);
/// ```
pub trait FoundRows {
    /// One found row of a rule, or of a frontier the project pass reads; its word cells mean something only with `words`, during
    /// this call: the host copies each word column to owned text (`facts::service::text`) before it keeps the row.
    fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, words: &FileWords);

    /// What the engine has to say of a rule's work in the file, beside its rows: the work passed its allowance, or a relation
    /// it read is past its declared size. Nothing stops a rule: it runs to its end and its rows are complete, whatever a notice
    /// says. What a run does with one (a warning in a user's run, a failure in a gate's) is the host's.
    fn noticed(&mut self, notice: WorkNotice);
}

/// Whose row it is.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FoundTarget;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// assert_ne!(FoundTarget::Rule(SelectedRuleOrdinal(0)), FoundTarget::Rule(SelectedRuleOrdinal(1)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundTarget {
    /// A finding of the rule.
    Rule(SelectedRuleOrdinal),
    /// A row of a cross-file rule's frontier, by its place in the rule's plan frontier.
    Frontier(SelectedRuleOrdinal, InputNumber),
}

/// The cells of one found row, in the emit's column order, lent for the call.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FoundCells;
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert_eq!(FoundCells(&[CellWord(3)]).0.len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FoundCells<'a>(pub &'a [CellWord]);

/// What each column of a found row holds, in column order: a `Symbol` column holds a word of the file, which the host copies to
/// owned text; every other kind is a number.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::ColumnKinds;
/// use ryo_relations::domain::relations::types::{CellKind, Relation};
///
/// assert_eq!(ColumnKinds(vec![CellKind::Number, CellKind::Symbol]).0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnKinds(pub Vec<CellKind>);

/// The columns of one accepted rule's rows, as its lowering emits them: of its findings, and of each entry of its plan's
/// frontier, by the entry's place.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::RuleColumns;
///
/// assert!(RuleColumns::default().frontier.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleColumns {
    /// The columns of the rule's findings.
    pub emit: ColumnKinds,
    /// The columns of each frontier entry.
    pub frontier: Vec<ColumnKinds>,
}

/// The frontier rows every file handed over, lent back by the host in file order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FrontierCells, FrontierSource};
/// use ryo_values::domain::plan_parts::types::InputNumber;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// struct NoFile;
/// impl FrontierSource for NoFile {
///     fn rows(&self, _rule: SelectedRuleOrdinal, _input: InputNumber, _each: &mut dyn FnMut(FrontierCells<'_>)) {}
/// }
/// let mut rows = 0;
/// NoFile.rows(SelectedRuleOrdinal(0), InputNumber(0), &mut |_| rows += 1);
/// assert_eq!(rows, 0);
/// ```
pub trait FrontierSource {
    /// Calls `each` with every row of a rule's frontier entry, in file order then hand-over order.
    fn rows(
        &self,
        rule: SelectedRuleOrdinal,
        input: InputNumber,
        each: &mut dyn FnMut(FrontierCells<'_>),
    );
}

/// One frontier row read back: owned text and numbers, never a word of a file.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{FrontierCell, FrontierCells};
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(FrontierCells(&[FrontierCell::Text(NameText("os"))]).0.len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FrontierCells<'a>(pub &'a [FrontierCell<'a>]);

/// One cell of a frontier row.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::FrontierCell;
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert_eq!(FrontierCell::Number(CellWord(4)), FrontierCell::Number(CellWord(4)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierCell<'a> {
    /// A text the file's row held.
    Text(NameText<'a>),
    /// A number, span bound or flag, as its stored bits.
    Number(CellWord),
}

/// What a scan did, for the run's measures.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{InstructionCount, ScanCounters};
///
/// assert_eq!(ScanCounters::default().steps, InstructionCount(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanCounters {
    /// Instructions executed.
    pub steps: InstructionCount,
    /// Anchor candidates entered.
    pub candidates: InstructionCount,
    /// Rows written into made tables.
    pub made_rows: InstructionCount,
    /// The rows the fills' stages read and gave.
    pub fill_rows: InstructionCount,
    /// The rows the programs' iterations gave.
    pub visited: InstructionCount,
}

/// A count of engine events.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::InstructionCount;
///
/// assert!(InstructionCount(1) > InstructionCount(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstructionCount(pub u64);

/// What the engine says of a rule's work in a file, and what did most of it: counts of rows, never a time, so every run and
/// every machine agrees. The engine holds no text and no statement's name: the host finds the statement `step` was written as
/// in the rule's compile answer, and says `heaviest` in its own words.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{HeaviestWork, InstructionCount, WorkNotice, WorkVerdict};
/// use ryo_values::domain::plan_parts::types::StepId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let notice = WorkNotice {
///     rule: SelectedRuleOrdinal(3),
///     verdict: WorkVerdict::Over,
///     work: InstructionCount(1_000_000),
///     allowance: InstructionCount(128_000),
///     step: StepId(2),
///     heaviest: HeaviestWork::Product { left: InstructionCount(1_000), right: InstructionCount(1_000) },
/// };
/// assert!(notice.work > notice.allowance);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkNotice {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// What happened.
    pub verdict: WorkVerdict,
    /// The rows the rule's work visited and its fills read and gave in the file.
    pub work: InstructionCount,
    /// What its allowance was: the constant times the rows the file held for it.
    pub allowance: InstructionCount,
    /// The step of the rule's own plan whose rows the unit that did the most counted work in the file makes: of every unit
    /// charged to the rule there (its own program, each definition pattern, each made table), the one whose own counted work
    /// (the rows its iterations gave, or the rows its fill read and gave) is the greatest; the first in the programs' unit
    /// order when two are equal.
    pub step: StepId,
    /// What that unit is, with the counts that say how much it did.
    pub heaviest: HeaviestWork,
}

/// What a work notice says. Neither stops the rule: its rows are complete.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::WorkVerdict;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_ne!(WorkVerdict::Over, WorkVerdict::Oversized(Relation::SyntaxError));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkVerdict {
    /// The rule finished and its work passed its allowance.
    Over,
    /// A relation the rule read wrote more rows than its declared size allows.
    Oversized(Relation),
}

/// The unit that did the most counted work of a rule in a file: one kind for each way a unit makes rows (a program, and each
/// fill of a made table, a fill of rows told apart by whether it holds a product stage), each with the row counts of that
/// file that make it concrete. A table of written rows is made once for the run and does no work in a file: it is never the
/// answer.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{HeaviestWork, InstructionCount};
///
/// let product = HeaviestWork::Product { left: InstructionCount(1_000), right: InstructionCount(1_000) };
/// assert_ne!(product, HeaviestWork::Rows { read: InstructionCount(2_000), made: InstructionCount(1_000_000) });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaviestWork {
    /// The rule's own program: navigation from its candidates.
    Program {
        /// The candidates it entered.
        candidates: InstructionCount,
        /// The rows its iterations gave.
        visited: InstructionCount,
    },
    /// A definition asked by key, in call form: its program run once for each distinct key.
    Calls {
        /// The distinct keys it was asked.
        keys: InstructionCount,
        /// The rows its iterations gave, over every key.
        visited: InstructionCount,
    },
    /// A table of rows whose fill holds a product stage: a join the plan states with no key. Of several product stages, the
    /// one that gave the most rows.
    Product {
        /// The rows that reached the stage.
        left: InstructionCount,
        /// The rows of the source each of them met.
        right: InstructionCount,
    },
    /// A table of rows whose every stage reads by a key: the keys' rows are many.
    Rows {
        /// The rows its stages read.
        read: InstructionCount,
        /// The rows it wrote.
        made: InstructionCount,
    },
    /// A group table: a count, a digest, the ends or the places of each group.
    Group {
        /// The rows of its input.
        read: InstructionCount,
        /// The rows it wrote.
        made: InstructionCount,
    },
    /// A sorted table.
    Sorted {
        /// The rows it sorted.
        rows: InstructionCount,
    },
    /// An ordered match: each left row with the right rows of its group that its order admits.
    OrderedPairs {
        /// The rows of the left input.
        left: InstructionCount,
        /// The rows of the right input.
        right: InstructionCount,
        /// The pairs it wrote.
        made: InstructionCount,
    },
    /// A graph table: fixed points, an ordered walk, a worklist or cycles over edges.
    Graph {
        /// The rows of its inputs.
        read: InstructionCount,
        /// The rows it wrote.
        made: InstructionCount,
    },
}
