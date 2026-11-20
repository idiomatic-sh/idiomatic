//! The test support of the `scan` use case: a file of words walked by the language crate's fixture language and scanned, counting
//! the relations the scan asks the language to fill and the rows it hands over.

use super::errors::ScanError;
use super::interface::ScanUseCase as _;
use super::use_case::ScanUseCaseImpl;
use crate::domain::findings::types::{FoundCells, FoundRows, FoundTarget, ScanCounters};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::Programs;
use ryo_language::domain::facts::types::{
    FactAnswer, FactSource, FileFacts, FileWords, FilledColumns, Wanted,
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

/// What a scan of a file answered, and how many relations it asked the language to fill.
#[derive(Debug)]
pub struct Scanned {
    /// The scan's answer.
    pub answer: Result<ScanCounters, ScanError>,
    /// The fills it asked.
    pub fills: Count,
    /// The rows it handed over, to a rule or to a frontier.
    pub found: Count,
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
        wanted: Wanted,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer {
        self.fills = Count(self.fills.0.saturating_add(1));
        self.inner.fill(relation, wanted, words, out)
    }

    fn render(&mut self, request: RenderRequest) -> RenderOutcome {
        self.inner.render(request)
    }
}

/// Counts the rows a scan hands over.
#[derive(Clone, Copy, Debug, Default)]
struct Found(Count);

impl FoundRows for Found {
    fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
        self.0 = Count(self.0.0.saturating_add(1));
    }
}

/// The visit that scans the lent file.
struct Scanning<'p> {
    /// The rule set.
    programs: &'p Programs,
    /// What the scan answered.
    seen: Option<Scanned>,
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
        let mut found = Found::default();
        let answer = ScanUseCaseImpl.scan(
            self.programs,
            lent,
            &mut scratch_for(self.programs),
            &mut found,
        );
        self.seen = Some(Scanned {
            answer,
            fills: counting.fills,
            found: found.0,
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
