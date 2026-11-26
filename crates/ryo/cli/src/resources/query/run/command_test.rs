//! What `query run` promises its caller, one row of `CASES` for each case it has.

use super::command::{RunArguments, answered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;

/// What a row gives the command: the command line, one argument at a time.
type Input = Vec<&'static str>;
/// How many lines the command prints when it succeeds, and how many of them name `notes.txt`, the file of the fixtures no language reads.
type Value = (usize, usize);
/// How the command ends when it does not succeed.
type Failure = Outcome;

const SAMPLE: &str = "tests/fixtures/sample";
const EMPTY: &str = "tests/fixtures/empty";
const MANIFEST: &str = "tests/fixtures/manifest";
/// The file of the fixtures that no language reads.
const STRAY: &str = "notes.txt";
const FUNCTIONS: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// Both functions of the sample, each matching one alternative of an `or`.
const EITHER: &str = "let functions = select * from \"function\" where name = \"first\" or name = \"second\"; emit functions subject \"file\" key(\"span\");";
/// One function matching both alternatives of an `or`: it is printed once.
const BOTH: &str = "let functions = select * from \"function\" where name = \"first\" or (name = \"first\" and file = file); emit functions subject \"file\" key(\"span\");";
/// A `where` on a column of the joined side that the select does not keep.
const JOINED_SIDE: &str = "let functions = select l.\"file\" as \"f\", l.\"span\" as \"s\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"name\" = \"second\"; emit functions subject f key(s);";
/// A `where` on a column the joined side does not have.
const JOINED_UNKNOWN: &str = "let functions = select l.\"file\" as \"f\", l.\"span\" as \"s\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"nothing\" = \"second\"; emit functions subject f key(s);";

/// A stream the test reads back.
struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other("poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// Where the command line looks for the project, which the command tree reads into the edge.
    #[arg(long)]
    root: Option<String>,
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: RunArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
}

/// The rows of this command's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prints_rows"),
        word: Word::Succeeds,
        input: || vec!["--root", SAMPLE, "--text", FUNCTIONS],
        expect: Expect::Value((3, 0)),
    },
    Case {
        name: CaseName("prints_one_json_document"),
        word: Word::Succeeds,
        input: || vec!["--root", SAMPLE, "--json", "--text", FUNCTIONS],
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("prints_no_json_when_the_process_holds_more_memory_than_the_budget"),
        word: Word::Fails,
        input: || {
            vec![
                "--root",
                SAMPLE,
                "--json",
                "--memory-budget",
                "100000",
                "--text",
                FUNCTIONS,
            ]
        },
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("prints_the_columns_and_the_unread_file_when_no_row_matches"),
        word: Word::Succeeds,
        input: || vec!["--root", EMPTY, "--text", FUNCTIONS],
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName(
            "the_file_listed_unread_is_the_stray_one_and_not_the_manifest_a_language_reads",
        ),
        word: Word::Succeeds,
        input: || vec!["--root", MANIFEST, "--text", FUNCTIONS],
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("an_or_prints_the_rows_of_either_alternative"),
        word: Word::Succeeds,
        input: || vec!["--root", SAMPLE, "--text", EITHER],
        expect: Expect::Value((3, 0)),
    },
    Case {
        name: CaseName("a_row_matching_both_alternatives_of_an_or_is_printed_once"),
        word: Word::Succeeds,
        input: || vec!["--root", SAMPLE, "--text", BOTH],
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("a_query_of_a_relation_the_project_fills_prints_its_rows"),
        word: Word::Succeeds,
        input: || vec!["--root", PACKAGES, "--text", DEPENDENCIES],
        expect: Expect::Value((7, 0)),
    },
    Case {
        name: CaseName("two_queries_print_the_rows_of_each_the_project_relations_included"),
        word: Word::Succeeds,
        input: || {
            vec![
                "--root",
                PACKAGES,
                "--text",
                FUNCTIONS,
                "--text",
                DEPENDENCIES,
            ]
        },
        expect: Expect::Value((11, 0)),
    },
    Case {
        name: CaseName("a_where_on_the_joined_side_keeps_the_rows_it_matches"),
        word: Word::Succeeds,
        input: || vec!["--root", SAMPLE, "--text", JOINED_SIDE],
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("a_where_on_a_column_the_joined_side_lacks_is_an_error"),
        word: Word::Refuses,
        input: || vec!["--root", SAMPLE, "--text", JOINED_UNKNOWN],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("a_wrong_query_is_an_error"),
        word: Word::Refuses,
        input: || vec!["--root", SAMPLE, "--text", MISSPELLED],
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("unknown_flag_is_refused"),
        word: Word::Refuses,
        input: || vec!["--bogus", "--text", FUNCTIONS],
        expect: Expect::Error(Outcome::Failure),
    },
    Case {
        name: CaseName("a_blank_text_is_refused"),
        word: Word::Refuses,
        input: || vec!["--text", ""],
        expect: Expect::Error(Outcome::Failure),
    },
    Case {
        name: CaseName("fails_when_the_process_holds_more_memory_than_the_budget"),
        word: Word::Fails,
        input: || {
            vec![
                "--root",
                SAMPLE,
                "--memory-budget",
                "100000",
                "--text",
                FUNCTIONS,
            ]
        },
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("fails_when_the_root_is_a_file"),
        word: Word::Fails,
        input: || {
            vec![
                "--root",
                "tests/fixtures/sample/lib.rs",
                "--text",
                FUNCTIONS,
            ]
        },
        expect: Expect::Error(Outcome::Error),
    },
    Case {
        name: CaseName("fails_when_the_root_is_not_there"),
        word: Word::Fails,
        input: || vec!["--root", "tests/fixtures/missing", "--text", FUNCTIONS],
        expect: Expect::Error(Outcome::Error),
    },
];

/// The edge the command tree builds for a line that names `root`, or none.
fn edge_of(root: Option<&str>, format: crate::output::Format) -> crate::edge::Edge {
    let here = ryo_language::inbound::use_cases::project::types::RootPath(".".into());
    let start = root.map_or_else(
        || here.clone(),
        |root| ryo_language::inbound::use_cases::project::types::RootPath(root.into()),
    );
    crate::resources::edge_at(
        start,
        here,
        ryo_values::domain::runs::types::MachineWorkers(1),
        format,
    )
}

/// Runs `query run` on one command line and returns what it printed or the outcome it ended with.
fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .map_err(|_refused| Outcome::Failure)?;
    let edge = edge_of(parsed.root.as_deref(), parsed.output.format());
    let _checked = super::schemas::request(
        edge.start.0.to_string(),
        &parsed.arguments.text,
        parsed.arguments.limit,
    )
    .map_err(|_refused| Outcome::Failure)?;
    let printed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let format = parsed.output.format();
    let out = Box::new(Shared(std::sync::Arc::clone(&printed)));
    let ran = answered(&parsed.arguments.text, parsed.arguments.limit, parsed.arguments.memory_budget, &edge, out).map_err(|_failed| Outcome::Error);
    if matches!(ran, Err(Outcome::Error)) && format == crate::output::Format::Json {
        // A run that stops leaves no part of a document behind.
        assert!(
            printed.lock().map_or(true, |bytes| bytes.is_empty()),
            "a failed run printed a prefix"
        );
    }
    ran?;
    let text = printed
        .lock()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    Ok((
        text.lines().count(),
        text.lines().filter(|said| said.contains(STRAY)).count(),
    ))
}

/// The `succeeds` rows: the command line is accepted and the printed answer is the expected one.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The `refuses` rows: the command line is refused with the expected outcome.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The `fails` rows: a service the command calls fails and the command ends with the expected outcome.
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// The Cargo workspace of the Rust language's manifest fixtures: three packages, one depending on the other two.
const PACKAGES: &str = "../languages/rust/tests/fixtures/manifests/packages";
/// The dependencies the manifests declare: a relation the project fills, not a file.
const DEPENDENCIES: &str =
    "let d = select * from \"package-dependency\"; emit d subject source key(source);";

/// What one path printed or how it ended: the bytes of the answer, or the text of the error the command reports.
type Printed = Result<String, String>;

/// Runs `line` through the SDK's `query run` (`through_executor`) or the old engine, and gives what the command would show.
fn printed(line: &[&'static str], through_executor: bool) -> Printed {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line.iter().copied()))
        .map_err(|refused| refused.to_string())?;
    let format = parsed.output.format();
    let edge = edge_of(parsed.root.as_deref(), format);
    let limit = parsed.arguments.limit;
    let root = edge.start.0.to_string();
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let out = Box::new(Shared(std::sync::Arc::clone(&bytes)));
    if through_executor {
        let _checked = super::schemas::request(root, &parsed.arguments.text, limit).map_err(|refused| refused.to_string())?;
        answered(&parsed.arguments.text, limit, parsed.arguments.memory_budget, &edge, out).map_err(|error| error.to_string())?;
    } else {
        let budget = super::schemas::budget(parsed.arguments.memory_budget);
        let request = super::schemas::old_request(root, parsed.arguments.text, limit)
            .map_err(|refused| refused.to_string())?;
        let rules = request.rules.clone();
        crate::output::written(
            format,
            out,
            |rows| super::command::answer(request, budget, &edge, rows),
            super::schemas::old_unwritten,
        )
        .map_err(|error| super::schemas::old_failure(error, &rules).to_string())?;
    }
    let text = bytes
        .lock()
        .map(|held| String::from_utf8_lossy(&held).into_owned())
        .unwrap_or_default();
    Ok(text)
}

/// The error of a run that stops for the memory budget names the bytes the process held, which no two runs share: they read as one.
fn steady(printed: Printed) -> Printed {
    printed.map_err(|error| match error.strip_prefix("the run holds ") {
        Some(rest) => format!(
            "the run holds N{}",
            rest.trim_start_matches(|c: char| c.is_ascii_digit())
        ),
        None => error,
    })
}

/// Two functions of one file, whose rows share the file the key names: a finding has one row for a rule, subject and key.
const SHARED_KEY: &str = "let functions = select \"file\", \"span\" from \"function\"; emit functions subject \"file\" key(\"file\");";

/// The rows shape on its own: every row is kept, though two share their rule, subject and key; they leave in the order the old
/// engine gave them; and a limit counts the rows past it without writing them.
#[test]
fn the_rows_shape_keeps_every_row_and_counts_past_the_limit() {
    let all = printed(&["--root", SAMPLE, "--text", SHARED_KEY], true).expect("the query runs");
    let old = printed(&["--root", SAMPLE, "--text", SHARED_KEY], false).expect("the query runs");
    assert_eq!(all.lines().count(), 3, "a heading and two rows, though the key is shared: {all}; the old engine: {old}");
    assert_eq!(all, old, "the same rows in the same order as the old engine");
    let cut = printed(&["--root", SAMPLE, "--text", SHARED_KEY, "--limit", "1"], true).expect("the query runs");
    assert_eq!(cut.lines().count(), 3, "a heading, one row and the count: {cut}");
    assert!(cut.contains("2 rows; showing 1"), "every row is counted: {cut}");
}

/// Every line of the command's own cases, with the other forms of the answer: text, JSON and a limit.
fn lines_compared() -> Vec<Vec<&'static str>> {
    let mut lines: Vec<Vec<&'static str>> = Vec::new();
    for case in CASES {
        let line = (case.input)();
        if line.contains(&"--bogus") || line.contains(&"") {
            continue;
        }
        lines.push(line.clone());
        let mut json = line.clone();
        json.push("--json");
        lines.push(json);
        let mut limited = line;
        limited.extend(["--limit", "1"]);
        lines.push(limited);
    }
    lines
}

/// Where the answers of the old engine are kept, one file for each line of `lines_compared`, and the index of their digests.
const FROZEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/resources/query/run/command_test/frozen");

/// What a path printed, as a frozen file holds it: whether it ended well, then the bytes it printed or the error it reported.
fn recorded(printed: &Printed) -> String {
    match printed {
        Ok(text) => format!("ok\n{text}"),
        Err(error) => format!("err\n{error}"),
    }
}

/// The digest of a frozen file, as the index holds it.
fn digest_of(text: &str) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The answer of every line is the one the old engine gave, which the files beside this test hold, and, until the old engine is deleted, the
/// one the old engine gives now: the same rows, in the same order, under the same limit, and the same error text when it refuses.
#[test]
fn the_executor_answers_as_the_old_engine_does() {
    let mut differing = Vec::new();
    let index = std::fs::read_to_string(format!("{FROZEN}/index.txt")).expect("the index of the frozen answers");
    let lines = lines_compared();
    assert_eq!(index.lines().count(), lines.len(), "one frozen answer for each line");
    for (at, line) in lines.iter().enumerate() {
        let frozen = std::fs::read_to_string(format!("{FROZEN}/{at:02}.txt")).expect("a frozen answer");
        let expected = index.lines().nth(at).and_then(|entry| entry.split('\t').nth(1)).expect("an index entry");
        assert_eq!(digest_of(&frozen), expected, "the frozen answer {at:02} is as it was recorded");
        let old = steady(printed(line, false));
        let new = steady(printed(line, true));
        if recorded(&old) != frozen || recorded(&new) != frozen {
            differing.push(format!("{at:02} {line:?}\n  frozen: {frozen:?}\n  old: {old:?}\n  new: {new:?}"));
        }
    }
    assert!(differing.is_empty(), "{} of the lines differ:\n{}", differing.len(), differing.join("\n"));
}
