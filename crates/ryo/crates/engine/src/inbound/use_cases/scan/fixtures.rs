//! The test support of the `scan` use case: a file of words walked by the language crate's fixture language and scanned with a
//! measuring state, keeping what the scan handed over (each row as text, each notice), what it asked the language to fill, and
//! what it counted of its own units.

use super::errors::ScanError;
use super::interface::ScanUseCase as _;
use super::use_case::ScanUseCaseImpl;
use crate::domain::code::types::UnitCode;
use crate::domain::code::types::UnitKey;
use crate::domain::counting::service::counted;
use crate::domain::findings::types::{
    FoundCells, FoundRows, FoundTarget, ScanCounters, WorkNotice,
};
use crate::domain::programs::service::scratch_measuring;
use crate::domain::programs::types::Programs;
use crate::domain::statistics::types::PointCounts;
use ryo_language::domain::facts::types::{
    FactAnswer, FactSource, FileFacts, FileWords, FilledColumns,
};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::node_table::service::builder;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::interface::WalkUseCase as _;
use ryo_language::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkRequest};
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

/// What a scan of a file answered and handed over, what it asked of the language, and what it counted.
#[derive(Debug)]
pub struct Scanned {
    /// The scan's answer.
    pub answer: Result<ScanCounters, ScanError>,
    /// The fills it asked, whole or by key.
    pub fills: Count,
    /// The fills it asked by one key. A fill says which once the fact contract's keyed fill is merged; until then none does.
    pub key_fills: Count,
    /// The rows it handed over, to a rule or to a frontier.
    pub found: Count,
    /// Each row it handed over, as text: whose it is and its cells.
    pub rows: Vec<String>,
    /// Each notice it gave.
    pub notices: Vec<WorkNotice>,
    /// How many times it made a table.
    pub tables_made: Count,
    /// How many times it evaluated a definition's pattern for a key.
    pub evaluations: Count,
    /// What it counted, for a sample's statistics.
    pub counts: PointCounts,
}

/// The language's facts, counting the relations the engine asks it to fill.
struct Counting<'a> {
    /// The language's own.
    inner: &'a mut dyn FactSource,
    /// How many fills were asked.
    fills: Count,
}

impl FactSource for Counting<'_> {
    fn fill(
        &mut self,
        relation: Relation,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer {
        self.fills = Count(self.fills.0.saturating_add(1));
        self.inner.fill(relation, words, out)
    }

    fn render(&mut self, request: RenderRequest) -> RenderOutcome {
        self.inner.render(request)
    }
}

/// Keeps what a scan hands over.
#[derive(Debug, Default)]
struct Found {
    /// Each row, as text.
    rows: Vec<String>,
    /// Each notice.
    notices: Vec<WorkNotice>,
}

impl FoundRows for Found {
    fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, _words: &FileWords) {
        let cells: Vec<u64> = row.0.iter().map(|cell| cell.0).collect();
        self.rows.push(format!("{target:?} {cells:?}"));
    }

    fn noticed(&mut self, notice: WorkNotice) {
        self.notices.push(notice);
    }
}

/// The visit that scans the lent file.
struct Scanning<'p> {
    /// The rule set.
    programs: &'p Programs,
    /// What the scan answered.
    seen: Option<Scanned>,
}

/// What a scan's units counted of the units whose key `kind` picks: the keys each was evaluated for, summed.
fn evaluated(programs: &Programs, counts: &PointCounts, kind: fn(&UnitCode) -> bool) -> Count {
    let units = programs.0.code.units.iter().zip(&counts.0.units);
    Count(
        units
            .filter(|(unit, _)| kind(unit))
            .map(|(_, counted)| counted.keys.0)
            .sum(),
    )
}

impl FileVisit for Scanning<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let mut counting = Counting {
            inner: file.facts,
            fills: Count(0),
        };
        let lent = FileFacts {
            table: file.table,
            source: file.source,
            file: file.file,
            facts: &mut counting,
        };
        let (mut found, mut scratch) = (Found::default(), scratch_measuring(self.programs));
        let answer = ScanUseCaseImpl.scan(self.programs, lent, &mut scratch, &mut found);
        let counts = counted(&scratch);
        self.seen = Some(Scanned {
            answer,
            fills: counting.fills,
            key_fills: Count(0),
            found: Count(found.rows.len()),
            rows: found.rows,
            notices: found.notices,
            tables_made: evaluated(self.programs, &counts, |unit| {
                matches!(unit.key, UnitKey::Table(..))
            }),
            evaluations: evaluated(self.programs, &counts, |unit| {
                matches!(unit.key, UnitKey::Pattern(..))
            }),
            counts,
        });
    }
}

/// Walks a file of `text` through the fixture language and scans it with `programs`.
///
/// # Errors
///
/// The fixture language's refusal of the file.
pub fn scanned(programs: &Programs, text: &str) -> Result<Option<Scanned>, WalkError> {
    let request = WalkRequest {
        path: SourcePath("scene.walk"),
        file: FileId(1),
        source: ByteSlice(text.as_bytes()),
    };
    let (mut table, mut scanning) = (
        NodeTable::default(),
        Scanning {
            programs,
            seen: None,
        },
    );
    Walking.walk(
        request,
        RelationNeeds(&[]),
        &mut builder(&mut table, Count(3)),
        &mut (),
        &mut scanning,
    )?;
    Ok(scanning.seen)
}

/// A file of `words` words of one text: one key group of that many rows.
#[must_use]
pub fn one_group(words: usize) -> String {
    vec!["a"; words].join(" ")
}

/// A file of `words` words, each of its own text: `w0 w1 ..`. What the growth harness scans.
#[must_use]
pub fn each_its_own(words: usize) -> String {
    (0..words)
        .map(|word| format!("w{word}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// How many rows a scene's rule finds in a file of `words` words of one group, where the scenario says "every row"; none for a
/// scene that does not say.
#[must_use]
pub fn every_row(scene: &str, words: usize) -> Option<Count> {
    // Each word beside each word after it: the pairs of the group, each once.
    (scene == "pairs within one group")
        .then(|| Count(words.saturating_mul(words.saturating_sub(1)) / 2))
}
