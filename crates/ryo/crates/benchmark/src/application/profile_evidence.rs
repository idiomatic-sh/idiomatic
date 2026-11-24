//! Summarize Ryo's engine-owned counters beside the standard tool output.
use serde::Deserialize;
use std::{
    collections::{BTreeMap, btree_map::Entry},
    io::{self, BufRead, Write},
};

const PHASES: [(&str, &str); 7] = [
    ("prep", "Preparation"),
    ("admission", "Admission"),
    ("read_parse", "Read and parse"),
    ("first", "Screening"),
    ("second", "File execution"),
    ("project", "Cross-file"),
    ("assembly", "Delivery"),
];

/// Write the CPU ledger the ordinary engine reports after the last SDK call.
///
/// # Errors
/// The observer's record is absent or truncated, or the output cannot be written.
pub fn phases(log: &str, mut output: impl Write) -> io::Result<()> {
    let fields = phase_fields(log)?;
    writeln!(output, "## Phase clocks and engine counts\n")?;
    writeln!(output, "| Part | Ryo CPU-s |\n|---|---:|")?;
    let mut total = 0.;
    for (key, title) in PHASES {
        let time = phase_time(&fields, key)?;
        total += time;
        writeln!(output, "| {title} | {time:.6} |")?;
    }
    writeln!(output, "| Total Ryo | {total:.6} |\n")?;
    writeln!(output, "| Counter | Count |\n|---|---:|")?;
    for key in [
        "calls",
        "requests",
        "failed_calls",
        "distinct_pairs",
        "skipped",
        "evaluated",
    ] {
        if let Some(value) = fields.get(key) {
            writeln!(output, "| {key} | {value} |")?;
        }
    }
    Ok(())
}

fn phase_fields(log: &str) -> io::Result<BTreeMap<&str, &str>> {
    let record = log
        .lines()
        .find_map(|line| line.strip_prefix("CHECK_SESSION live "))
        .or_else(|| {
            log.lines()
                .find_map(|line| line.strip_prefix("CHECK_SESSION "))
        })
        .ok_or_else(|| invalid("missing Ryo phase ledger"))?;
    Ok(record
        .split_ascii_whitespace()
        .filter_map(|word| word.split_once('='))
        .collect())
}

fn phase_time(fields: &BTreeMap<&str, &str>, key: &str) -> io::Result<f64> {
    fields
        .get(key)
        .and_then(|word| word.parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.)
        .ok_or_else(|| invalid(format!("invalid Ryo phase clock {key}")))
}

pub(crate) fn phase_delta(base: &str, candidate: &str, mut output: impl Write) -> io::Result<()> {
    let first = phase_fields(base)?;
    let second = phase_fields(candidate)?;
    writeln!(output, "\n| Part | First CPU-s | Second CPU-s | Change s |")?;
    writeln!(output, "|---|---:|---:|---:|")?;
    for (key, title) in PHASES {
        let old = phase_time(&first, key)?;
        let new = phase_time(&second, key)?;
        writeln!(
            output,
            "| {title} | {old:.6} | {new:.6} | {:+.6} |",
            new - old
        )?;
    }
    Ok(())
}

/// A row's logical operator runs, input and output row totals, and self CPU ns.
type Counts = [u64; 4];

#[derive(Default, Deserialize)]
struct TraceFields {
    node: Option<u64>,
    kind: Option<String>,
    runs: Option<u64>,
    rows_in: Option<u64>,
    rows_out: Option<u64>,
    cpu_nanos: Option<TraceNumber>,
    rules: Option<String>,
    phase: Option<String>,
    elapsed_ns: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TraceNumber {
    Integer(u64),
    Text(String),
}

#[derive(Deserialize)]
struct TraceEvent {
    target: String,
    fields: TraceFields,
}

/// Write the linter's physical-node counts and the phase emitted by its reader.
///
/// # Errors
/// Returns an error for missing, malformed or inconsistent engine records.
pub(crate) fn linter(ledger: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let mut kinds = BTreeMap::<String, Counts>::new();
    let mut rules = BTreeMap::<String, Counts>::new();
    let mut parse_ns = 0u64;
    let mut runs_reported = 0;
    for line in ledger.lines() {
        let row: TraceEvent = serde_json::from_str(&line?)
            .map_err(|error| invalid(format!("invalid engine trace: {error}")))?;
        let event = row.fields;
        if event.phase.as_deref() == Some("provider_parse") {
            parse_ns += event
                .elapsed_ns
                .ok_or_else(|| invalid("missing parse clock"))?;
        }
        if row.target != "ryo::node_counters" {
            continue;
        }
        let Some(_) = event.node else {
            runs_reported += 1;
            continue;
        };
        let counts = [
            event.runs.ok_or_else(|| invalid("missing node runs"))?,
            event
                .rows_in
                .ok_or_else(|| invalid("missing node inputs"))?,
            event
                .rows_out
                .ok_or_else(|| invalid("missing node outputs"))?,
            match event.cpu_nanos {
                Some(TraceNumber::Integer(ns)) => ns,
                Some(TraceNumber::Text(ns)) => number(&ns)?,
                None => return Err(invalid("missing node self CPU time")),
            },
        ];
        let kind = event.kind.ok_or_else(|| invalid("missing node kind"))?;
        add(kinds.entry(kind).or_default(), counts);
        let ids = event
            .rules
            .ok_or_else(|| invalid("missing node rule owners"))?;
        for id in ids.split("RuleId(\"").skip(1) {
            let label = id
                .split_once("\")")
                .ok_or_else(|| invalid("invalid node rule ID"))?;
            add(rules.entry(label.0.to_owned()).or_default(), counts);
        }
    }
    if runs_reported == 0 {
        return Err(invalid("missing native operator counters"));
    }
    writeln!(output, "## Phase clocks and engine counts\n")?;
    writeln!(output, "| Part | Observed wall ns |\n|---|---:|")?;
    writeln!(output, "| Provider parse | {parse_ns} |")?;
    writeln!(
        output,
        "\nShared nodes are counted once by kind and attributed to every owning rule.\n"
    )?;
    write_operators(
        &mut output,
        "Operators by kind",
        "Self CPU ns",
        kinds,
        usize::MAX,
    )?;
    write_operators(&mut output, "Top twenty rules", "Self CPU ns", rules, 20)
}

/// Reduce the observer's typed TSV into counts by operator and by query text.
///
/// # Errors
/// The ledger is absent or inconsistent, or the report cannot be written.
pub fn operators(ledger: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let mut queries = BTreeMap::new();
    let mut kinds = BTreeMap::<String, Counts>::new();
    let mut per_query = BTreeMap::<u64, Counts>::new();
    for line in ledger.lines() {
        let line = line?;
        let fields: Vec<_> = line.split('\t').collect();
        match fields.as_slice() {
            ["query", id, text] => match queries.entry(number(id)?) {
                Entry::Vacant(entry) => {
                    entry.insert(unhex(text)?);
                }
                Entry::Occupied(entry) if entry.get() == &unhex(text)? => {}
                Entry::Occupied(_) => return Err(invalid("conflicting query text for one ID")),
            },
            ["operator", id, _, kind, runs, input, produced, cpu] => {
                let values = [
                    number(runs)?,
                    number(input)?,
                    number(produced)?,
                    number(cpu)?,
                ];
                add(kinds.entry((*kind).to_owned()).or_default(), values);
                add(per_query.entry(number(id)?).or_default(), values);
            }
            _ => return Err(invalid("malformed operator ledger row")),
        }
    }
    if kinds.is_empty() {
        return Err(invalid("empty operator ledger"));
    }
    let mut texts = BTreeMap::new();
    for (id, counts) in per_query {
        let query = queries
            .remove(&id)
            .ok_or_else(|| invalid("operator without query text"))?;
        add(texts.entry(query).or_default(), counts);
    }
    write_operators(
        &mut output,
        "Operators by kind",
        "Self CPU ns",
        kinds,
        usize::MAX,
    )?;
    write_operators(
        &mut output,
        "Top twenty query texts",
        "Self CPU ns",
        texts,
        20,
    )
}

fn write_operators(
    output: &mut impl Write,
    title: &str,
    clock: &str,
    data: BTreeMap<String, Counts>,
    count: usize,
) -> io::Result<()> {
    let mut rows: Vec<_> = data.into_iter().collect();
    rows.sort_by(|a, b| b.1[3].cmp(&a.1[3]).then_with(|| a.0.cmp(&b.0)));
    writeln!(output, "\n## {title}\n")?;
    writeln!(
        output,
        "| Operator or query | Runs | Rows in | Rows out | {clock} |"
    )?;
    writeln!(output, "|---|---:|---:|---:|---:|")?;
    for (name, [runs, input, produced, ns]) in rows.into_iter().take(count) {
        let name = name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace('|', "\\|");
        writeln!(output, "| {name} | {runs} | {input} | {produced} | {ns} |")?;
    }
    Ok(())
}

fn add(target: &mut Counts, values: Counts) {
    for (target, value) in target.iter_mut().zip(values) {
        *target += value;
    }
}

fn number(value: &str) -> io::Result<u64> {
    value.parse().map_err(|_| invalid("invalid operator count"))
}

fn unhex(value: &str) -> io::Result<String> {
    let chunks = value.as_bytes().chunks_exact(2);
    if !chunks.remainder().is_empty() {
        return Err(invalid("odd query text escape"));
    }
    let text = chunks
        .map(|pair| {
            let pair =
                std::str::from_utf8(pair).map_err(|_| invalid("non-ASCII query text escape"))?;
            u8::from_str_radix(pair, 16).map_err(|_| invalid("invalid query text escape"))
        })
        .collect::<io::Result<Vec<_>>>()?;
    String::from_utf8(text).map_err(|_| invalid("query text is not UTF-8"))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
