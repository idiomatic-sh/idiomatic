//! Controlled stand-in for the engine port. It accepts the lowering fixture's one tree rule, and a compiled rule of the same shape
//! (one step that keeps columns of a relation read in place from the node table: the element, its file, its span, a token's
//! text; of every element, or of the elements whose kind is a word of the language's), and answers every other plan among the
//! refused while it loads the rest; it writes
//! the rules it accepted, with the columns of their rows, after the engine's own pack header. For each rule it runs it finds a
//! row for each element the rule names of a scanned file: the fixture rule's, per word, is (start, end, word) with a frontier
//! row (word); a compiled rule's is the columns it keeps. Before it finds a row it reads `syntax-error`, and
//! faults when the language filled it only in part. It counts the rule sets it lowered and the packs it read back, and the rules
//! it runs are those of the rule set it loaded last: the engine's programs are sealed, so a fake cannot put them there. It keeps
//! the statistics its last lowering was given; a fake told to notice gives its notice of its first rule in the file told; and
//! its ledger has one row for each rule it runs, the first with the rows every scan found.

use super::rules::{Anchor, FixtureRule, Held, lowered, named_rules, written};
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::counting::types::{LedgerRow, LedgerUnit, WorkClass, WorkLedger};
use ryo_engine::domain::findings::types::{
    ColumnKinds, FoundCells, FoundRows, FoundTarget, FrontierSource, HeaviestWork,
    InstructionCount, RuleColumns, ScanCounters, WorkNotice, WorkVerdict,
};
use ryo_engine::domain::lowering::types::RefusedRule;
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{EncodedPack, PackBytes, PackDigest};
use ryo_engine::domain::programs::service::scratch_for;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase as _;
use ryo_engine::inbound::use_cases::load::types::{LoadSource, LoadedRules};
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::domain::facts::service::{base_words, clear_words, file_words, intern};
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FileFacts, FileWords, FilledColumns, ProjectFacts, Wanted,
};
use ryo_language::domain::fixed_names::types::CheckedNames;
use ryo_language::domain::node_table::service::{element_count, kind_of, node_count, span_of};
use ryo_language::inbound::use_cases::walk::types::FileVisit;
use ryo_relations::domain::relations::service::{node_id, to_bits};
use ryo_relations::domain::relations::types::{
    CellKind, Count, FileId, Offset, Position, Relation, Span, Value,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::{InputNumber, StepId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

/// The fake engine.
#[derive(Debug, Default)]
pub struct EngineFake {
    /// How many rule sets it was asked to lower from plans.
    pub lowered: AtomicUsize,
    /// How many packs it was asked to read back.
    pub unpacked: AtomicUsize,
    /// The statistics its last lowering was given, as bytes.
    pub statistics: Mutex<Vec<u8>>,
    /// How many states that measure it was asked for.
    pub measuring: AtomicUsize,
    /// The file it notices in, and the notice it gives there of the first rule it runs; none: it notices nothing.
    notice: Option<(FileId, WorkNotice)>,
    /// The rows its scans found, together.
    found: AtomicU64,
    /// The rules of the rule set it loaded last.
    last: Mutex<Vec<FixtureRule>>,
}

impl EngineFake {
    /// A fake that notices: in the file `file`, it gives `notice` of the first rule it runs, and finds that file's rows all
    /// the same.
    #[must_use]
    pub fn noticing(file: FileId, notice: WorkNotice) -> Self {
        Self {
            notice: Some((file, notice)),
            ..Self::default()
        }
    }

    /// Remembers the rules of the rule set just loaded.
    fn loaded(&self, rules: Vec<FixtureRule>) -> Programs {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = rules;
        Programs::default()
    }

    /// The rules it runs now.
    fn rules(&self) -> Vec<FixtureRule> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The notice of the port's fixtures: a rule whose work, a million rows, passed its allowance, most of it in a join with no
/// key of a thousand rows with a thousand, at the third step of its plan.
#[must_use]
pub const fn overrun() -> WorkNotice {
    WorkNotice {
        rule: SelectedRuleOrdinal(0),
        verdict: WorkVerdict::Over,
        work: InstructionCount(1_000_000),
        allowance: InstructionCount(128_000),
        step: StepId(2),
        heaviest: HeaviestWork::Product {
            left: InstructionCount(1000),
            right: InstructionCount(1000),
        },
    }
}

/// The kind vocabulary of the port's fixtures: no kind, of no language.
#[must_use]
pub fn no_kinds() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName::default(),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    }
}

/// Counts the rows an adapter hands over.
#[derive(Clone, Copy, Debug)]
pub struct Counted(pub Count);

impl FoundRows for Counted {
    fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
        self.0 = Count(self.0.0.saturating_add(1));
    }

    fn noticed(&mut self, _notice: WorkNotice) {}
}

/// A visit that hands the lent file to a closure.
#[derive(Debug)]
pub struct Visited<F>(pub F);

impl<F: for<'f> FnMut(FileFacts<'f>)> FileVisit for Visited<F> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// The visit that hands every lent file to `visit`.
pub fn visiting<F: for<'f> FnMut(FileFacts<'f>)>(visit: F) -> Visited<F> {
    Visited(visit)
}

/// Counts one more call.
fn counted(calls: &AtomicUsize) {
    let _before = calls.fetch_add(1, Ordering::Relaxed);
}

impl EnginePort for EngineFake {
    fn load(
        &self,
        source: LoadSource<'_>,
        kinds: &KindVocabulary,
    ) -> Result<LoadedRules, LoadError> {
        match source {
            LoadSource::Plans(rules, statistics) => {
                counted(&self.lowered);
                *self
                    .statistics
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = statistics.0.to_vec();
                let (mut accepted, mut refused) = (Vec::new(), Vec::new());
                for rule in rules.0 {
                    match lowered(rule, kinds) {
                        Ok(lowered) => accepted.push(lowered),
                        Err(refusal) => refused.push(RefusedRule {
                            rule: rule.rule,
                            refusal,
                        }),
                    }
                }
                Ok(LoadedRules {
                    programs: self.loaded(accepted),
                    refused,
                    ..LoadedRules::default()
                })
            }
            LoadSource::Pack(bytes, digest) => {
                counted(&self.unpacked);
                // The engine's own header first, read by the engine: its version and digest refusals are the engine's.
                let header = encoded(&Programs::default(), digest).0.len();
                let (head, tail) = bytes.0.split_at_checked(header).unwrap_or((bytes.0, &[]));
                let _header =
                    LoadUseCaseImpl.load(LoadSource::Pack(PackBytes(head), digest), kinds)?;
                Ok(LoadedRules {
                    programs: self.loaded(named_rules(tail)?),
                    ..LoadedRules::default()
                })
            }
        }
    }

    fn accepted(&self, _programs: &Programs) -> Vec<SelectedRuleOrdinal> {
        self.rules().iter().map(|rule| rule.rule).collect()
    }

    fn columns(&self, _programs: &Programs) -> Vec<RuleColumns> {
        let kind = |held: &Held| match held {
            Held::Start | Held::End => CellKind::Number,
            Held::Word => CellKind::Symbol,
            Held::Node => CellKind::Node,
            Held::File => CellKind::File,
            Held::Span => CellKind::Span,
        };
        self.rules()
            .iter()
            .map(|rule| RuleColumns {
                emit: ColumnKinds(rule.cells.iter().map(kind).collect()),
                frontier: if rule.frontier {
                    vec![ColumnKinds(vec![CellKind::Symbol])]
                } else {
                    Vec::new()
                },
            })
            .collect()
    }

    fn encoded(&self, programs: &Programs, digest: PackDigest) -> EncodedPack {
        let mut pack = encoded(programs, digest);
        for rule in self.rules() {
            for word in written(&rule) {
                pack.0.extend_from_slice(&word.to_le_bytes());
            }
        }
        pack
    }

    fn state_for(&self, programs: &Programs) -> ScanScratch {
        scratch_for(programs)
    }

    fn measuring_state_for(&self, programs: &Programs) -> ScanScratch {
        counted(&self.measuring);
        scratch_for(programs)
    }

    fn ledger(&self, _programs: &Programs, _workers: &[&ScanScratch]) -> WorkLedger {
        let mut found =
            Count(usize::try_from(self.found.load(Ordering::Relaxed)).unwrap_or(usize::MAX));
        let row = |rule: &FixtureRule| LedgerRow {
            unit: LedgerUnit::Rule(rule.rule),
            class: WorkClass::Navigation,
            owners: vec![rule.rule],
            rows: std::mem::take(&mut found),
            steps: Count(0),
            nanos: Count(0),
            idle_nanos: Count(0),
        };
        WorkLedger(self.rules().iter().map(row).collect())
    }

    fn scan(
        &self,
        _programs: &Programs,
        file: FileFacts<'_>,
        _scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError> {
        let rules = self.rules();
        let Some(first) = rules.first() else {
            return Ok(ScanCounters::default());
        };
        let mut words = file_words(&base_words(CheckedNames::default(), &[]));
        let read = Relation::SyntaxError;
        if let FactAnswer::Partial(partial) = file.facts.fill(
            read,
            Wanted::Whole,
            &mut words,
            &mut FilledColumns(Vec::new()),
        ) {
            return Err(ScanError::Partial {
                rule: first.rule,
                relation: read,
                cause: PartialCause(partial.cause.0),
                missing: partial.missing,
            });
        }
        // The file's words start over: the first word of every file has the same code.
        clear_words(&mut words);
        let (nodes, elements) = (node_count(file.table).0, element_count(file.table).0);
        let mut candidates = InstructionCount(0);
        for rule in &rules {
            let anchored = match rule.anchor {
                Anchor::Tokens => nodes..elements,
                Anchor::Every | Anchor::Kind(_) => 0..elements,
            };
            for element in anchored {
                let position = Position(u32::try_from(element).unwrap_or(u32::MAX));
                if matches!(rule.anchor, Anchor::Kind(kind) if kind_of(file.table, position) != kind)
                {
                    continue;
                }
                let span = span_of(file.table, position);
                let range = usize::from(Position(span.start.0))..usize::from(Position(span.end.0));
                // A node has no text of its own: its row's word is the empty one.
                let text = file
                    .source
                    .0
                    .get(range)
                    .filter(|_| element >= nodes)
                    .map_or_else(Default::default, |bytes| {
                        std::str::from_utf8(bytes).unwrap_or_default()
                    });
                let word = intern(&mut words, NameText(text));
                let whole = Span {
                    start: Offset(span.start.0),
                    end: Offset(span.end.0),
                };
                let cells: Vec<CellWord> = rule
                    .cells
                    .iter()
                    .map(|held| match held {
                        Held::Start => CellWord(u64::from(span.start.0)),
                        Held::End => CellWord(u64::from(span.end.0)),
                        Held::Word => word,
                        Held::Node => {
                            CellWord(to_bits(Value::Node(node_id(file.file, position))).0)
                        }
                        Held::File => CellWord(to_bits(Value::File(file.file)).0),
                        Held::Span => CellWord(to_bits(Value::Span(whole)).0),
                    })
                    .collect();
                found.found(FoundTarget::Rule(rule.rule), FoundCells(&cells), &words);
                if rule.frontier {
                    found.found(
                        FoundTarget::Frontier(rule.rule, InputNumber(0)),
                        FoundCells(&[word]),
                        &words,
                    );
                }
                candidates = InstructionCount(candidates.0.saturating_add(1));
            }
        }
        let _before = self.found.fetch_add(candidates.0, Ordering::Relaxed);
        if let Some((_, notice)) = self.notice.filter(|(noticed, _)| *noticed == file.file) {
            found.noticed(WorkNotice {
                rule: first.rule,
                ..notice
            });
        }
        Ok(ScanCounters {
            steps: candidates,
            candidates,
            made_rows: InstructionCount(0),
            fill_rows: InstructionCount(0),
            visited: candidates,
        })
    }

    fn project(
        &self,
        _programs: &Programs,
        _frontier: &dyn FrontierSource,
        _project: ProjectFacts<'_>,
        _scratch: &mut ScanScratch,
        _found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError> {
        Ok(ScanCounters::default())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
