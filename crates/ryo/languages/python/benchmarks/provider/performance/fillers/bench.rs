//! python's `fillers` workload: what each of the walk's fillers costs for each element of a file, at one size and at four times
//! it, over generated sources. A relation is filled whole; a relation the catalog declares a key of is also filled by every key
//! that holds a row, one fill for each. The walk itself is not timed.
//!
//! Run with `--bmf`, the bench prints one Bencher Metric Format document on stdout and nothing else: for each relation the
//! benchmark `python/fillers/<Relation>` with the measures `ns-per-element` (at `SIZE` elements), `ns-per-element-4n` (at four
//! times as many) and `growth-percent` (the second over the first: 100 is a filler whose time grows with the file and no
//! faster); for each declared key the benchmark `python/fillers/<Relation>/by-<Column>` with `ns-per-key`, `ns-per-key-4n` and
//! `growth-percent`. Asymptote holds each measure to its best; the bench asserts no budget. A filler that writes no row for any
//! shape, or a key no fill of which writes one, has nothing to measure and fails the bench.

use ryo::languages::{
    self, CellWord, Column, Count, FactAnswer, FillExtent, FilledColumns, NodeTable, Relation,
    Walk, Wanted,
};
use ryo_python::fixtures::grown::{GrownShape, SHAPES, grown_source};
use ryo_python::fixtures::walking::{declared_keys, walk_words, walked};

/// How many elements the smaller generated source has.
const SIZE: usize = 2_000;

/// How many times each fill is timed: the fastest is the one the rest of the machine slowed least.
const ROUNDS: usize = 9;

/// The argument that asks for the measures as a Bencher Metric Format document.
pub(crate) const BMF_FLAG: &str = "--bmf";

/// What one fill of a source cost: its time, the elements of the source, and what it wrote.
#[derive(Clone, Copy, Debug)]
struct Cost {
    /// The fastest of the rounds.
    time: std::time::Duration,
    /// The elements of the source.
    elements: usize,
    /// The rows written, or the keys filled.
    units: usize,
}

/// The cells of one column of filled rows, each once.
fn distinct(columns: &FilledColumns, place: usize) -> Vec<CellWord> {
    let mut cells = columns
        .0
        .get(place)
        .map(|column| column.0.clone())
        .unwrap_or_default();
    cells.sort_unstable();
    cells.dedup();
    cells
}

/// The whole fill of `relation` for `source`, timed alone after a walk of its own, `ROUNDS` times: the fastest, with the rows
/// written. None when the language does not answer the relation in full for the source, or writes no row.
fn whole<W: Walk>(walk: &W, source: &str, relation: Relation) -> Option<Cost> {
    let (mut table, mut scratch) = (NodeTable::default(), W::Scratch::default());
    let (mut words, mut columns) = (
        walk_words(walk).expect("each fixed name of the walk is stated once"),
        FilledColumns::default(),
    );
    let mut best: Option<Cost> = None;
    for _round in 0..ROUNDS {
        languages::facts::clear_words(&mut words);
        columns.0.clear();
        let mut found = None;
        walked(
            walk,
            "grown.py",
            source.as_bytes(),
            &[relation],
            &mut table,
            &mut scratch,
            &mut |file| {
                let started = std::time::Instant::now();
                let answer = file
                    .facts
                    .fill(relation, Wanted::Whole, &mut words, &mut columns);
                let time = started.elapsed();
                let rows = columns.0.first().map_or(0, |column| column.0.len());
                if answer == FactAnswer::Answered(FillExtent::Asked) && rows > 0 {
                    let elements = languages::node_table::element_count(file.table).0;
                    found = Some(Cost {
                        time,
                        elements,
                        units: rows,
                    });
                }
            },
        )
        .expect("the generated source is walked");
        let cost = found?;
        best = Some(best.map_or(cost, |held| if cost.time < held.time { cost } else { held }));
    }
    best
}

/// The fills of `relation` by every key of `column` that holds a row for `source`, one fill for each key, timed together after a
/// walk of their own, `ROUNDS` times: the fastest, with the keys filled. The first fill pays for what the file's fills by key
/// share. None when the whole fill writes no row, or a fill by key does not answer that it wrote what was asked.
fn by_key<W: Walk>(
    walk: &W,
    source: &str,
    relation: Relation,
    (column, place): (Column, usize),
) -> Option<Cost> {
    let (mut table, mut scratch) = (NodeTable::default(), W::Scratch::default());
    let (mut words, mut columns) = (
        walk_words(walk).expect("each fixed name of the walk is stated once"),
        FilledColumns::default(),
    );
    let mut best: Option<Cost> = None;
    for _round in 0..ROUNDS {
        languages::facts::clear_words(&mut words);
        columns.0.clear();
        let mut found = None;
        // The keys are read from a whole fill of a walk of its own, so the timed walk begins with nothing built.
        let mut keys = Vec::new();
        walked(
            walk,
            "grown.py",
            source.as_bytes(),
            &[relation],
            &mut table,
            &mut scratch,
            &mut |file| {
                let mut all = FilledColumns::default();
                let _answer = file
                    .facts
                    .fill(relation, Wanted::Whole, &mut words, &mut all);
                keys = distinct(&all, place);
            },
        )
        .expect("the generated source is walked");
        walked(
            walk,
            "grown.py",
            source.as_bytes(),
            &[relation],
            &mut table,
            &mut scratch,
            &mut |file| {
                let started = std::time::Instant::now();
                let asked = keys
                    .iter()
                    .filter(|key| {
                        let wanted = Wanted::Key(column, **key);
                        file.facts.fill(relation, wanted, &mut words, &mut columns)
                            == FactAnswer::Answered(FillExtent::Asked)
                    })
                    .count();
                let time = started.elapsed();
                let rows = columns.0.first().map_or(0, |cells| cells.0.len());
                if asked == keys.len() && rows >= keys.len() && !keys.is_empty() {
                    let elements = languages::node_table::element_count(file.table).0;
                    found = Some(Cost {
                        time,
                        elements,
                        units: keys.len(),
                    });
                }
            },
        )
        .expect("the generated source is walked");
        let cost = found?;
        best = Some(best.map_or(cost, |held| if cost.time < held.time { cost } else { held }));
    }
    best
}

/// One benchmark of the document: its name and its three measures.
#[path = "../../../json_output.rs"]
mod json_output;

#[derive(serde::Serialize)]
struct Measure { value: f64 }

fn benchmark(name: String, unit: &str, small: f64, large: f64) -> (String, std::collections::BTreeMap<String, Measure>) {
    let value = |number: f64| Measure { value: (number * 1000.0).round() / 1000.0 };
    let mut measures = std::collections::BTreeMap::new();
    drop(measures.insert(format!("ns-per-{unit}"), value(small)));
    drop(measures.insert(format!("ns-per-{unit}-4n"), value(large)));
    drop(measures.insert("growth-percent".to_owned(), value(100.0 * large / small)));
    (name, measures)
}

/// The nanoseconds a cost is for each of `count` things.
fn each(cost: Cost, count: usize) -> f64 {
    cost.time.as_secs_f64() * 1e9 / (count.max(1) as f64)
}

/// The two sources of the first shape `relation` is measured on at both sizes by `measure`, with the two costs.
fn on_a_shape(measure: &dyn Fn(&str) -> Option<Cost>) -> Option<(GrownShape, Cost, Cost)> {
    SHAPES.iter().find_map(|shape| {
        let small = measure(&grown_source(*shape, Count(SIZE)))?;
        let large = measure(&grown_source(*shape, Count(4 * SIZE)))?;
        Some((*shape, small, large))
    })
}

/// Prints the measures of every filler as one Bencher Metric Format document on stdout.
///
/// # Panics
///
/// When a relation the walk fills writes no row for any shape, or a declared key of it has no fill that writes one: there is
/// nothing to measure.
#[expect(
    clippy::print_stdout,
    reason = "with `--bmf` the document on stdout is what the asymptote tool reads"
)]
pub(crate) fn measured() {
    let walk = ryo_python::walk();
    let probe = grown_source(GrownShape::Statements, Count(SIZE));
    let mut document = std::collections::BTreeMap::new();
    for relation in languages::all() {
        let mut refused = false;
        let (mut table, mut scratch) = (NodeTable::default(), Default::default());
        walked(
            &walk,
            "grown.py",
            probe.as_bytes(),
            &[],
            &mut table,
            &mut scratch,
            &mut |file| {
                let answer = file.facts.fill(
                    *relation,
                    Wanted::Whole,
                    &mut walk_words(&walk).expect("each fixed name of the walk is stated once"),
                    &mut FilledColumns::default(),
                );
                refused = matches!(answer, FactAnswer::Refused(_));
            },
        )
        .expect("the generated source is walked");
        if refused {
            continue;
        }
        let (_shape, small, large) = on_a_shape(&|source| whole(&walk, source, *relation))
            .unwrap_or_else(|| panic!("no shape gives {relation:?} a row at both sizes"));
        let (name, measures) = benchmark(
            format!("python/fillers/{relation:?}"),
            "element",
            each(small, small.elements),
            each(large, large.elements),
        );
        drop(document.insert(name, measures));
        for (column, place, _kind) in declared_keys(*relation) {
            let (_shape, small, large) =
                on_a_shape(&|source| by_key(&walk, source, *relation, (column, place)))
                    .unwrap_or_else(|| {
                        panic!("no shape gives {column:?} a key with a row at both sizes")
                    });
            let (name, measures) = benchmark(
                format!("python/fillers/{relation:?}/by-{column:?}"),
                "key",
                each(small, small.units),
                each(large, large.units),
            );
            drop(document.insert(name, measures));
        }
    }
    assert!(
        !document.is_empty(),
        "the walk fills no relation: there is nothing to measure"
    );
    println!("{}", String::from_utf8(json_output::bytes(&document).expect("measurements")).expect("UTF-8"));
}

/// Walks a generated source of `elements` elements and fills every relation the walk answers, whole, then every declared key
/// that holds a row: N and 4N.
#[divan::bench(args = [SIZE, 4 * SIZE])]
fn fillers_whole_and_by_key(bencher: divan::Bencher<'_, '_>, elements: usize) {
    let walk = ryo_python::walk();
    let source = grown_source(GrownShape::Statements, Count(elements));
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut words = walk_words(&walk).expect("each fixed name of the walk is stated once");
    bencher.bench_local(|| {
        languages::facts::clear_words(&mut words);
        let mut rows = 0_usize;
        walked(
            &walk,
            "grown.py",
            source.as_bytes(),
            languages::all(),
            &mut table,
            &mut scratch,
            &mut |file| {
                for relation in languages::all() {
                    let mut whole = FilledColumns::default();
                    if matches!(
                        file.facts
                            .fill(*relation, Wanted::Whole, &mut words, &mut whole),
                        FactAnswer::Refused(_)
                    ) {
                        continue;
                    }
                    rows += whole.0.first().map_or(0, |column| column.0.len());
                    for (column, place, _kind) in declared_keys(*relation) {
                        let mut one = FilledColumns::default();
                        for key in distinct(&whole, place) {
                            let _answer = file.facts.fill(
                                *relation,
                                Wanted::Key(column, key),
                                &mut words,
                                &mut one,
                            );
                        }
                        rows += one.0.first().map_or(0, |cells| cells.0.len());
                    }
                }
            },
        )
        .expect("the generated source is walked");
        assert!(
            rows > elements,
            "the fillers wrote {rows} rows for {elements} elements: nothing was measured"
        );
        rows
    });
}
