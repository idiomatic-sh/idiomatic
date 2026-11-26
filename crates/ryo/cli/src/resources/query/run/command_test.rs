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
    let budget = super::schemas::budget(parsed.arguments.memory_budget);
    let edge = edge_of(parsed.root.as_deref(), parsed.output.format());
    let request = super::schemas::request(
        edge.start.0.to_string(),
        parsed.arguments.text,
        parsed.arguments.limit,
    )
    .map_err(|_refused| Outcome::Failure)?;
    let printed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let format = parsed.output.format();
    let out = Box::new(Shared(std::sync::Arc::clone(&printed)));
    let ran = answered(request, budget, &edge, out).map_err(|_failed| Outcome::Error);
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

/// Runs `line` through the old engine (`through_executor` false) or the executor, and gives what the command would show.
fn printed(line: &[&'static str], through_executor: bool) -> Printed {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line.iter().copied()))
        .map_err(|refused| refused.to_string())?;
    let budget = super::schemas::budget(parsed.arguments.memory_budget);
    let format = parsed.output.format();
    let edge = edge_of(parsed.root.as_deref(), format);
    let texts = parsed.arguments.text.clone();
    let limit = parsed.arguments.limit;
    let root = edge.start.0.to_string();
    let request = super::schemas::request(root.clone(), parsed.arguments.text, limit)
        .map_err(|refused| refused.to_string())?;
    let rules = request.rules.clone();
    let bytes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let out = Box::new(Shared(std::sync::Arc::clone(&bytes)));
    let ended = crate::output::written(
        format,
        out,
        |rows| {
            if through_executor {
                super::command::answer_executed(&texts, limit, &root, budget, &edge, rows)
            } else {
                super::command::answer(request, budget, &edge, rows)
            }
        },
        super::schemas::unwritten,
    )
    .map_err(|error| super::schemas::failure(error, &rules).to_string());
    ended?;
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

/// The executor answers every line as the old engine does: the same rows, in the same order, under the same limit, and the same error
/// text when it refuses.
#[test]
fn the_executor_answers_as_the_old_engine_does() {
    let mut differing = Vec::new();
    for line in lines_compared() {
        let old = steady(printed(&line, false));
        let new = steady(printed(&line, true));
        if old != new {
            differing.push(format!("{line:?}\n  old: {old:?}\n  new: {new:?}"));
        }
    }
    assert!(differing.is_empty(), "{} of the lines differ:\n{}", differing.len(), differing.join("\n"));
}
