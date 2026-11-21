//! The test support of the `file_scan` use case: its contract, a rule as a compile leaves it, and the use case composed over the
//! engine port's fake and the fixture language that walks files of words.
use super::types::{FileScanSetting, FileToScan, Overruns, ScanEngine, ScanLanguage, ScanMeasure};
use super::use_case::FileScanUseCaseImpl;
use ryo_execution_runtime::outbound::engine::fixtures::fake::{EngineFake, overrun};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;
use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service::{column_named, columns};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::descriptions::types::{
    FieldWord, FixedName, KindVocabularies, KindVocabulary, KindWord, LanguageName,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Heading, Input, LiteralText, Operand, PlanComparison,
    QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};
use std::sync::Arc;

pub mod contract;

/// What the fixture language calls the kind of a word.
const WORD: &str = "word";

/// The place of a column of the tree's relation among its columns.
fn column(name: &str) -> ColumnIndex {
    let relation = Relation::SyntaxTree;
    let place = column_named(relation, NameRef(name))
        .and_then(|named| columns(relation).iter().position(|each| *each == named));
    ColumnIndex(u32::try_from(place.unwrap_or(usize::MAX)).unwrap_or(u32::MAX))
}

/// A rule as a compile leaves it: one step keeping the columns `names` of the tree's relation for the elements whose kind is
/// `word`, its subject the second of them and its key the first.
#[must_use]
pub fn rule(names: &[&str]) -> Plan {
    let words = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column("kind")),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(WORD.into()))),
        origin: ComparisonOrigin::Unrecorded,
    };
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: vec![words],
                kept: names.iter().map(|name| column(name)).collect(),
            },
            scope: StepScope::PerFile,
        }],
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(1),
            key: vec![ColumnIndex(0)],
            headings: names.iter().map(|name| Heading((*name).into())).collect(),
        },
        frontier: Vec::new(),
    }
}

/// What a fixture edge composed: which engines run the files, whether it composed an engine, and whether its language links its
/// walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// The old engine alone, with the fake composed and the fixture language's walk linked.
    Old,
    /// The new engine alone, with the fake composed and the fixture language's walk linked.
    New,
    /// Both engines, with the fake composed and the fixture language's walk linked.
    Both,
    /// The new engine alone, at an edge that composed no engine.
    NoEngine,
    /// The new engine alone, with the fake composed and the fixture language linked without its walk.
    NoWalk,
    /// The new engine alone, as a user's run: the fake notices that its rule passes its allowance in the file numbered 1.
    Overrun,
    /// The same, as one of our own gates: the scan is set to fail on an overrun.
    Gate,
    /// The new engine alone, in a run that measures.
    Measuring,
}

/// The use case as a fixture edge composes it, the fixture language linked as `words`, with the fake it composed.
#[must_use]
pub fn composed(edge: Edge) -> (FileScanUseCaseImpl, Arc<EngineFake>) {
    let fake = Arc::new(match edge {
        Edge::Overrun | Edge::Gate => EngineFake::noticing(FileId(1), overrun()),
        Edge::Old | Edge::New | Edge::Both | Edge::NoEngine | Edge::NoWalk | Edge::Measuring => {
            EngineFake::default()
        }
    });
    // The names of the fixture language, as its walk states them.
    let names = WalkEngine::vocabulary(&Walking);
    let words: Arc<dyn WalkEngine> = match edge {
        Edge::NoWalk => Arc::new(WalkUseCaseImpl::new()),
        Edge::Old
        | Edge::New
        | Edge::Both
        | Edge::NoEngine
        | Edge::Overrun
        | Edge::Gate
        | Edge::Measuring => Arc::new(Walking),
    };
    let scan = FileScanUseCaseImpl::new(FileScanSetting {
        engine: match edge {
            Edge::NoEngine => ScanEngine::NotComposed,
            Edge::Old
            | Edge::New
            | Edge::Both
            | Edge::NoWalk
            | Edge::Overrun
            | Edge::Gate
            | Edge::Measuring => ScanEngine::Composed(Arc::<EngineFake>::clone(&fake)),
        },
        digest: Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
        walks: vec![words],
        vocabularies: KindVocabularies(vec![KindVocabulary {
            language: LanguageName("words".into()),
            kinds: names
                .kinds
                .0
                .iter()
                .map(|name| KindWord(name.0.into()))
                .collect(),
            fields: names
                .fields
                .0
                .iter()
                .map(|name| FieldWord(name.0.into()))
                .collect(),
            fixed: names
                .fixed
                .0
                .iter()
                .map(|name| FixedName(name.0.into()))
                .collect(),
        }]),
        language: ScanLanguage::EveryLinked,
        mode: match edge {
            Edge::Old => EngineMode::Old,
            Edge::Both => EngineMode::Both,
            Edge::New
            | Edge::NoEngine
            | Edge::NoWalk
            | Edge::Overrun
            | Edge::Gate
            | Edge::Measuring => EngineMode::New,
        },
        overruns: match edge {
            Edge::Gate => Overruns::Fail,
            Edge::Old
            | Edge::New
            | Edge::Both
            | Edge::NoEngine
            | Edge::NoWalk
            | Edge::Overrun
            | Edge::Measuring => Overruns::Report,
        },
        measure: match edge {
            Edge::Measuring => ScanMeasure::On,
            Edge::Old
            | Edge::New
            | Edge::Both
            | Edge::NoEngine
            | Edge::NoWalk
            | Edge::Overrun
            | Edge::Gate => ScanMeasure::Off,
        },
    });
    (scan, fake)
}

/// The file numbered `number` of a fixture run: its path and its bytes.
#[must_use]
pub fn file<'a>(number: u32, path: &'a str, source: &'a [u8]) -> FileToScan<'a> {
    FileToScan {
        path: SourcePath(path),
        file: FileId(number),
        source: ByteSlice(source),
        executable: Default::default(),
    }
}
