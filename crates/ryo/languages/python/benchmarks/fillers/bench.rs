//! The row-equality and cost tool of python's walk: over the files of a pinned request, the rows the walk's fillers write against
//! the rows today's reading writes (`equal`), and the cost of the walk alone (`floor`) or with its fillers (`cost`) on the worker
//! threads asked for. A tool, not a test: it reads a corpus, and its timings are taken by the meter that runs it.
//!
//! `fillers equal|cost|floor <request.json> <root> [relations] [threads]`: `relations` is a comma list of relation names
//! (`string-literal`), `all` for every relation python's walk fills; `threads` defaults to four.

#![expect(
    clippy::expect_used,
    reason = "a tool that cannot read its own input has nothing to measure: the panic is the failure"
)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the tool's answer is its report on stdout and its progress on stderr"
)]

use ryo::languages::{self, FactAnswer, FilledColumns, NodeTable, Relation, Walk, facts};
use ryo_python::fixtures::today::{Read, read, read_keys, read_placements, read_rows};
use ryo_python::fixtures::walking::{
    Placement, walked, walked_keys, walked_placements, walked_rows,
};
use ryo_python::settings::{PythonParserSettings, PythonTargetVersion};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The relations python's walk fills with a `fills!` item of its own.
const FILLED: &[Relation] = &[
    Relation::BytesLiteral,
    Relation::CallArgument,
    Relation::Comment,
    Relation::CompilerFeature,
    Relation::Decorates,
    Relation::Function,
    Relation::Identifier,
    Relation::IndentRegion,
    Relation::Inherits,
    Relation::InterpolatedStringLiteral,
    Relation::LanguageTarget,
    Relation::LiteralProperties,
    Relation::NumericLiteral,
    Relation::Parameter,
    Relation::SourceLocation,
    Relation::StaticImport,
    Relation::StringLiteral,
    Relation::TokenLink,
];

/// What one relation's comparison found over the files.
#[derive(Clone, Debug, Default)]
struct Tally {
    /// The rows today's reading wrote.
    today: usize,
    /// The rows the walk's filler wrote.
    walked: usize,
    /// The files whose rows differ.
    differing: usize,
    /// The files the fill did not answer in full for.
    unanswered: usize,
    /// The first differences, shown.
    examples: Vec<String>,
}

/// The relations named by a comma list; every filled relation for `all`.
fn relations(named: &str) -> Vec<Relation> {
    if named == "all" {
        return FILLED.to_vec();
    }
    named
        .split(',')
        .map(|name| {
            let wanted = name.replace('-', "");
            languages::all()
                .iter()
                .copied()
                .find(|relation| format!("{relation:?}").eq_ignore_ascii_case(&wanted))
                .unwrap_or_else(|| panic!("no relation is named `{name}`"))
        })
        .collect()
}

/// The files of the pinned request, relative to its root.
fn files(request: &str) -> Vec<String> {
    let text = std::fs::read_to_string(request).expect("the request is read");
    let json: serde_json::Value = serde_json::from_str(&text).expect("the request is JSON");
    json.get("files")
        .and_then(serde_json::Value::as_array)
        .expect("the request lists its files")
        .iter()
        .filter_map(|file| file.as_str().map(str::to_owned))
        .collect()
}

/// Runs `work` over the indexes of `count` files on `threads` threads, each with its own state.
fn on_threads<S: Send>(
    threads: usize,
    count: usize,
    state: impl Fn() -> S + Sync,
    work: impl Fn(&mut S, usize) + Sync,
) -> Vec<S> {
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut own = state();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        if index >= count {
                            break own;
                        }
                        work(&mut own, index);
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("the worker ends"))
            .collect()
    })
}

/// The cost run: every file walked, and every relation of `wanted` filled, on `threads` threads. Prints the rows written.
fn cost<W: Walk>(walk: &W, root: &str, listed: &[String], wanted: &[Relation], threads: usize) {
    let states = on_threads(
        threads,
        listed.len(),
        || {
            (
                NodeTable::default(),
                W::Scratch::default(),
                facts::file_words(&facts::base_words(&[])),
                FilledColumns::default(),
                0_usize,
                0_usize,
            )
        },
        |(table, scratch, words, columns, rows, elements), index| {
            let path = &listed[index];
            let bytes = std::fs::read(format!("{root}/{path}")).expect("the file is read");
            facts::clear_words(words);
            walked(walk, path, &bytes, wanted, table, scratch, &mut |file| {
                *elements += languages::node_table::element_count(file.table).0;
                for relation in wanted {
                    let answer = file.facts.fill(*relation, words, columns);
                    if !matches!(answer, FactAnswer::Refused(_)) {
                        *rows += columns.0.first().map_or(0, |column| column.0.len());
                    }
                }
            })
            .expect("the file is walked");
        },
    );
    let rows: usize = states.iter().map(|state| state.4).sum();
    let elements: usize = states.iter().map(|state| state.5).sum();
    println!(
        "{{\"files\":{},\"relations\":{},\"elements\":{elements},\"rows\":{rows},\"threads\":{threads}}}",
        listed.len(),
        wanted.len()
    );
}

/// The first placements one tree has and the other lacks.
fn lacking(side: &str, have: &[Placement], other: &[Placement]) -> Vec<String> {
    have.iter()
        .filter(|each| other.binary_search(each).is_err())
        .take(2)
        .map(|each| format!("only {side}: {each:?}"))
        .collect()
}

/// The equality run: for every file, the tree and the rows of `wanted` the walk writes against today's. Prints one line for each
/// relation and for the tree, and answers whether everything is equal.
fn equal<W: Walk + Sync>(
    walk: &W,
    settings: PythonParserSettings,
    root: &str,
    listed: &[String],
    wanted: &[Relation],
    threads: usize,
) -> bool {
    let linked = ryo_python::linked_with_settings(settings);
    let engine = linked.parse_engine();
    let tallies: Mutex<BTreeMap<String, Tally>> = Mutex::new(BTreeMap::new());
    drop(on_threads(
        threads,
        listed.len(),
        || (NodeTable::default(), W::Scratch::default(), Read::default()),
        |(table, scratch, today), index| {
            let path = &listed[index];
            let bytes = std::fs::read(format!("{root}/{path}")).expect("the file is read");
            read(engine.as_ref(), path, &bytes, wanted, today)
                .expect("today's reading reads the file");
            let today_keys = read_keys(&today.elements);
            let mut found: Vec<(String, Tally)> = Vec::new();
            walked(
                walk,
                path,
                &bytes,
                wanted,
                table,
                scratch,
                &mut |mut file| {
                    let keys = walked_keys(file.table);
                    let accepted = languages::node_table::acceptance_of(file.table)
                        == languages::SyntaxAcceptance::Accepted;
                    let (mut old, mut new) = (
                        read_placements(&today.elements, &today_keys),
                        walked_placements(file.table, &keys),
                    );
                    if !accepted {
                        // A recovered node's field is the parser's typed one in the walk and a positional one today.
                        for each in old.iter_mut().chain(new.iter_mut()) {
                            each.field = 0;
                        }
                        old.sort_unstable();
                        new.sort_unstable();
                    }
                    let mut tree = Tally {
                        today: old.len(),
                        walked: new.len(),
                        ..Tally::default()
                    };
                    if old != new {
                        tree.differing = 1;
                        tree.examples = lacking("today", &old, &new);
                        tree.examples.extend(lacking("walked", &new, &old));
                        tree.examples
                            .iter_mut()
                            .for_each(|line| line.insert_str(0, &format!("{path}: ")));
                    }
                    found.push(("(tree)".to_owned(), tree));
                    let mut words = facts::file_words(&facts::base_words(&[]));
                    for relation in wanted {
                        let before = read_rows(today, &bytes, &today_keys, *relation);
                        let (after, answer) = walked_rows(&mut file, &keys, *relation, &mut words);
                        let partial_today = today
                            .rows
                            .partials
                            .iter()
                            .any(|(each, _, _)| each == relation);
                        let answered = match answer {
                            FactAnswer::Answered => !partial_today,
                            FactAnswer::Partial(_) => partial_today,
                            FactAnswer::Refused(_) => false,
                        };
                        let mut tally = Tally {
                            today: before.len(),
                            walked: after.len(),
                            unanswered: usize::from(!answered),
                            ..Tally::default()
                        };
                        if before != after {
                            tally.differing = 1;
                            let only_today = before.iter().find(|row| !after.contains(row));
                            let only_walked = after.iter().find(|row| !before.contains(row));
                            tally.examples.push(format!(
                                "{path}: today {only_today:?}; walked {only_walked:?}"
                            ));
                        }
                        found.push((format!("{relation:?}"), tally));
                    }
                },
            )
            .expect("the file is walked");
            let mut all = tallies.lock().expect("the tallies are held");
            for (name, tally) in found {
                let total = all.entry(name).or_default();
                total.today += tally.today;
                total.walked += tally.walked;
                total.differing += tally.differing;
                total.unanswered += tally.unanswered;
                if total.examples.len() < 3 {
                    total.examples.extend(tally.examples);
                }
            }
        },
    ));
    let all = tallies.into_inner().expect("the tallies are whole");
    let mut whole = true;
    for (name, tally) in &all {
        let same = tally.differing == 0 && tally.unanswered == 0;
        whole &= same;
        println!(
            "{{\"relation\":\"{name}\",\"equal\":{same},\"rows_today\":{},\"rows_walked\":{},\"files_differing\":{},\"files_answered_differently\":{}}}",
            tally.today, tally.walked, tally.differing, tally.unanswered
        );
        for example in &tally.examples {
            eprintln!("{name}: {example}");
        }
    }
    whole
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [mode, request, root, rest @ ..] = arguments.as_slice() else {
        // `cargo test` builds and runs a bench with no arguments: there is nothing to do without a corpus.
        eprintln!("fillers equal|cost|floor <request.json> <root> [relations] [threads]");
        return;
    };
    let wanted = relations(rest.first().map_or("all", String::as_str));
    let threads = rest
        .get(1)
        .and_then(|count| count.parse().ok())
        .unwrap_or(4);
    let listed = files(request);
    let settings = PythonParserSettings::new(PythonTargetVersion::Py310);
    let walk = ryo_python::walk_with_settings(settings);
    match mode.as_str() {
        "equal" => {
            if !equal(&walk, settings, root, &listed, &wanted, threads) {
                std::process::exit(1);
            }
        }
        "cost" => cost(&walk, root, &listed, &wanted, threads),
        "floor" => cost(&walk, root, &listed, &[], threads),
        other => panic!("no mode is named `{other}`"),
    }
}
