//! The test support of the fact fillers: a source read twice, by the model's fillers over the test view and by today's readers
//! over the language's own parse, each row shown the same way (a node as `kind@start:end`, a word as its text, a span as
//! `start:end`), so the two are compared as texts whatever each numbers its nodes and words with.

use crate::fixtures::model_rows::{Gathered, kind_shown, shown_row};
use crate::fixtures::reading::SHOWN;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;
use crate::outbound::syntax::fixtures::model_view::{FixtureView, ModelFixture, viewed};
use crate::settings::PythonParserSettings;
use ryo::languages::contract::{Collect, read};
use std::sync::Arc;

use ryo::languages::{
    self, AbsenceCause, Bits, ByteSlice, CatalogRelation, Elements, FieldRequest, FileId,
    FileWords, Fill, NoteRequest, ParseRequest, Position, Rows, SourcePath, Symbol, Symbols, facts,
};

/// What one side wrote for a source: its rows as texts, sorted, and the causes it gave for a relation read only in part.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Told {
    /// The rows, one text each, sorted.
    pub rows: Vec<String>,
    /// The causes, in the order given.
    pub causes: Vec<String>,
    /// Why the relation is not read at all, when it is not.
    pub absent: Option<AbsenceCause>,
}

/// How the fillers' rows of a source stand against today's readers' rows of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Compared {
    /// How many rows today's readers wrote.
    pub rows: usize,
    /// The rows only today's readers wrote (`-`) and the rows only the fillers wrote (`+`).
    pub differing: Vec<String>,
    /// The causes the fillers gave.
    pub causes: Vec<String>,
}

/// What a filler writes for `source`, read through the test view.
pub(crate) fn filled<R: CatalogRelation>(
    source: &str,
    fill: impl Fn(&mut FixtureView<'_>, &mut FileWords, &mut dyn Rows<R::Row>) -> Fill,
) -> Told {
    let mut fixture = ModelFixture::default();
    let mut told = Told::default();
    viewed(&mut fixture, source, FileId(0), &mut |view| {
        let base = facts::base_words(&[]);
        let mut words = facts::file_words(&base);
        let mut gathered = Gathered::<R> {
            rows: Vec::new(),
            causes: Vec::new(),
        };
        told.absent = match fill(view, &mut words, &mut gathered) {
            Fill::Written => None,
            Fill::Absent(cause) => Some(cause),
        };
        told.rows = gathered
            .rows
            .iter()
            .map(|cells| {
                crate::fixtures::model_rows::shown_filled::<R>(cells.as_ref(), view.table, &words)
            })
            .collect();
        told.rows.sort();
        told.causes = gathered.causes;
    });
    told
}

/// What today's reader of the relation writes for `source`, read with the language's own parse.
pub(crate) fn read_today<R: CatalogRelation>(source: &'static str) -> Told {
    let (mut elements, mut symbols, mut out) =
        (Elements::default(), Symbols::default(), Collect::default());
    let parse = ParseContract::new(
        PythonParserSettings::default(),
        Arc::new(ruff_python_parser(PythonParserSettings::default())),
        Arc::new(postcard()),
    );
    read(
        &parse,
        ParseRequest {
            path: SourcePath(SHOWN),
            file: FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
        &[R::relation()],
        &mut out,
    )
    .expect("the source is read");
    let node = |at: Position| {
        format!(
            "{}@{}:{}",
            kind_shown(languages::kind_at(&elements, at)),
            languages::start_at(&elements, at).0,
            languages::end_at(&elements, at).0
        )
    };
    let word = |symbol: Symbol| {
        languages::text_of(&symbols, ByteSlice(source.as_bytes()), symbol)
            .map_or_else(String::new, |text| text.0.to_owned())
    };
    let mut rows: Vec<String> = out
        .rows()
        .iter()
        .map(|(_, cells)| {
            let bits: Vec<Bits> = cells.iter().map(|cell| Bits(*cell)).collect();
            shown_row::<R>(&bits, &node, &word)
        })
        .collect();
    rows.sort();
    Told {
        rows,
        causes: out
            .partials
            .iter()
            .map(|(_, cause, _)| cause.clone())
            .collect(),
        absent: None,
    }
}

/// The fillers' rows of `source` against today's readers' rows of it.
pub(crate) fn compared<R: CatalogRelation>(
    source: &'static str,
    fill: impl Fn(&mut FixtureView<'_>, &mut FileWords, &mut dyn Rows<R::Row>) -> Fill,
) -> Compared {
    let ours = filled::<R>(source, fill);
    let theirs = read_today::<R>(source);
    let mut differing: Vec<String> = Vec::new();
    for row in &theirs.rows {
        if !ours.rows.contains(row) {
            differing.push(format!("- {row}"));
        }
    }
    for row in &ours.rows {
        if !theirs.rows.contains(row) {
            differing.push(format!("+ {row}"));
        }
    }
    if differing.is_empty() && ours.rows.len() != theirs.rows.len() {
        differing.push(format!(
            "{} rows against {}",
            ours.rows.len(),
            theirs.rows.len()
        ));
    }
    Compared {
        rows: theirs.rows.len(),
        differing,
        causes: ours.causes,
    }
}

/// Every construct that binds, reads, deletes or scopes a name, once.
pub(crate) const CONSTRUCTS: &str = include_str!("../../../tests/fixtures/model/constructs.py");
