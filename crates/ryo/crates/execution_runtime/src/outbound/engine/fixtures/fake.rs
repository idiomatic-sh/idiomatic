//! Controlled stand-in for the engine port. It accepts the lowering fixture's one tree rule and refuses every other plan, writes the
//! rules it accepted after the engine's own pack header, and for each rule it runs finds, per word of a scanned file, a row
//! (start, end, word) and a frontier row (word). Before it finds a row it reads `syntax-error`, and faults when the language
//! filled it only in part. It counts the rule sets it lowered and the packs it read back, and the rules it runs are those of the
//! rule set it loaded last: the engine's programs are sealed, so a fake cannot put them there.

use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::findings::types::{
    ColumnKinds, FoundCells, FoundRows, FoundTarget, FrontierSource, InstructionCount, RuleColumns,
    ScanCounters,
};
use ryo_engine::domain::lowering::fixtures::named;
use ryo_engine::domain::lowering::types::{PlanNode, PlanShape};
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{EncodedPack, PackBytes, PackDigest};
use ryo_engine::domain::programs::service::scratch_for;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::domain::scratch::types::ScanScratch;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase as _;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_engine::inbound::use_cases::project::errors::ProjectPassError;
use ryo_engine::inbound::use_cases::scan::errors::ScanError;
use ryo_language::domain::facts::service::{base_words, clear_words, file_words, intern};
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FileFacts, FileWords, FilledColumns, ProjectFacts,
};
use ryo_language::domain::node_table::service::{element_count, node_count, span_of};
use ryo_language::inbound::use_cases::walk::types::FileVisit;
use ryo_relations::domain::relations::types::{CellKind, Count, Position, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

/// The fake engine.
#[derive(Debug, Default)]
pub struct EngineFake {
    /// How many rule sets it was asked to lower from plans.
    pub lowered: AtomicUsize,
    /// How many packs it was asked to read back.
    pub unpacked: AtomicUsize,
    /// The rules of the rule set it loaded last.
    last: Mutex<Vec<SelectedRuleOrdinal>>,
}

impl EngineFake {
    /// Remembers the rules of the rule set just loaded.
    fn loaded(&self, rules: Vec<SelectedRuleOrdinal>) -> Programs {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = rules;
        Programs::default()
    }
}

/// The kind vocabulary of the port's fixtures: no kind, of no language.
#[must_use]
pub fn no_kinds() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName::default(),
        kinds: Vec::new(),
    }
}

/// Counts the rows an adapter hands over.
#[derive(Clone, Copy, Debug)]
pub struct Counted(pub Count);

impl FoundRows for Counted {
    fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
        self.0 = Count(self.0.0.saturating_add(1));
    }
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

/// The rules a fake's pack names after the engine's header: each ordinal as four little-endian bytes.
fn named_rules(tail: &[u8]) -> Result<Vec<SelectedRuleOrdinal>, LoadError> {
    let (rules, rest) = tail.as_chunks::<{ size_of::<u32>() }>();
    if rest.is_empty() {
        Ok(rules
            .iter()
            .map(|rule| SelectedRuleOrdinal(u32::from_le_bytes(*rule)))
            .collect())
    } else {
        Err(LoadError::PackCorrupt)
    }
}

impl EnginePort for EngineFake {
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<Programs, LoadError> {
        match source {
            LoadSource::Plans(rules) => {
                counted(&self.lowered);
                let (tree, heavy) = (named("one tree rule"), named("program over budget"));
                for rule in rules.0 {
                    if *rule.plan == heavy {
                        return Err(LoadError::Budget { rule: rule.rule });
                    }
                    if *rule.plan != tree {
                        return Err(LoadError::Unlowerable {
                            rule: rule.rule,
                            shape: PlanShape::Step(PlanNode(0)),
                        });
                    }
                }
                Ok(self.loaded(rules.0.iter().map(|rule| rule.rule).collect()))
            }
            LoadSource::Pack(bytes, digest) => {
                counted(&self.unpacked);
                // The engine's own header first, read by the engine: its version and digest refusals are the engine's.
                let header = encoded(&Programs::default(), digest).0.len();
                let (head, tail) = bytes.0.split_at_checked(header).unwrap_or((bytes.0, &[]));
                let _header =
                    LoadUseCaseImpl.load(LoadSource::Pack(PackBytes(head), digest), kinds)?;
                Ok(self.loaded(named_rules(tail)?))
            }
        }
    }

    fn accepted(&self, _programs: &Programs) -> Vec<SelectedRuleOrdinal> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn columns(&self, programs: &Programs) -> Vec<RuleColumns> {
        let word = || ColumnKinds(vec![CellKind::Symbol]);
        let found = RuleColumns {
            emit: ColumnKinds(vec![CellKind::Number, CellKind::Number, CellKind::Symbol]),
            frontier: vec![word()],
        };
        vec![found; self.accepted(programs).len()]
    }

    fn encoded(&self, programs: &Programs, digest: PackDigest) -> EncodedPack {
        let mut pack = encoded(programs, digest);
        for rule in self.accepted(programs) {
            pack.0.extend_from_slice(&rule.0.to_le_bytes());
        }
        pack
    }

    fn state_for(&self, programs: &Programs) -> ScanScratch {
        scratch_for(programs)
    }

    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        _scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError> {
        let accepted = self.accepted(programs);
        let Some(first) = accepted.first() else {
            return Ok(ScanCounters::default());
        };
        let mut words = file_words(&base_words(&[]));
        let read = Relation::SyntaxError;
        if let FactAnswer::Partial(partial) =
            file.facts
                .fill(read, &mut words, &mut FilledColumns(Vec::new()))
        {
            return Err(ScanError::Partial {
                rule: *first,
                relation: read,
                cause: PartialCause(partial.cause.0),
                missing: partial.missing,
            });
        }
        // The file's words start over: the first word of every file has the same code.
        clear_words(&mut words);
        let (nodes, elements) = (node_count(file.table).0, element_count(file.table).0);
        let mut candidates = InstructionCount(0);
        for rule in &accepted {
            for token in nodes..elements {
                let span = span_of(
                    file.table,
                    Position(u32::try_from(token).unwrap_or(u32::MAX)),
                );
                let range = usize::from(Position(span.start.0))..usize::from(Position(span.end.0));
                let text = file
                    .source
                    .0
                    .get(range)
                    .map_or_else(Default::default, |bytes| {
                        std::str::from_utf8(bytes).unwrap_or_default()
                    });
                let word = intern(&mut words, NameText(text));
                let (start, end) = (
                    CellWord(u64::from(span.start.0)),
                    CellWord(u64::from(span.end.0)),
                );
                found.found(
                    FoundTarget::Rule(*rule),
                    FoundCells(&[start, end, word]),
                    &words,
                );
                found.found(
                    FoundTarget::Frontier(*rule, InputNumber(0)),
                    FoundCells(&[word]),
                    &words,
                );
                candidates = InstructionCount(candidates.0.saturating_add(1));
            }
        }
        Ok(ScanCounters {
            steps: candidates,
            candidates,
            made_rows: InstructionCount(0),
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
