//! What `check` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{CheckArguments, answer};
use super::fixtures::{CONTRACTED, dry_run, edge, files, proven, sources, workspace};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::{Ending as _, Outcome};
use crate::output::Format;

/// The command line, after `check`.
type Input = Vec<&'static str>;
/// The rules found, and how the command ends.
type Value = (Vec<String>, Outcome);
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: CheckArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_comment_of_one_line_suppresses_findings_whatever_its_delimiters"),
        word: Word::Succeeds,
        input: || vec!["--fix", "comment-contract"],
        expect: Expect::Value((Vec::new(), Outcome::Failure)),
    },
    Case {
        name: CaseName("a_template_is_compared_over_the_shared_relations_whatever_its_language"),
        word: Word::Succeeds,
        input: || vec!["--fix", "template-contract"],
        expect: Expect::Value((Vec::new(), Outcome::Failure)),
    },
    Case {
        name: CaseName("a_symbolic_link_no_entry_places_is_a_closed_finding_and_is_never_read"),
        word: Word::Succeeds,
        input: || vec!["--fix", "symbolic-link"],
        expect: Expect::Value((Vec::new(), Outcome::Failure)),
    },
    Case {
        name: CaseName("the_workspace_s_findings_fail_the_check"),
        word: Word::Succeeds,
        input: || vec!["--fix"],
        expect: Expect::Value((Vec::new(), Outcome::Failure)),
    },
    Case {
        name: CaseName("a_dry_run_with_fix_writes_nothing_not_even_the_lock_or_the_store"),
        word: Word::Succeeds,
        input: || vec!["-n", "--fix"],
        expect: Expect::Value((Vec::new(), Outcome::Failure)),
    },
    Case {
        name: CaseName("the_lint_store_holds_every_file_read_and_one_edit_keeps_the_rest"),
        word: Word::Succeeds,
        input: || vec!["lint-store"],
        expect: Expect::Value((Vec::new(), Outcome::Success)),
    },
    Case {
        name: CaseName("one_edit_reads_one_source_file"),
        word: Word::Succeeds,
        input: || vec!["one-edit"],
        expect: Expect::Value((Vec::new(), Outcome::Success)),
    },
    Case {
        name: CaseName("an_entry_that_selects_nothing_applies_its_rule_to_nothing"),
        word: Word::Succeeds,
        input: || vec!["selects-nothing"],
        expect: Expect::Value((Vec::new(), Outcome::Success)),
    },
    Case {
        name: CaseName("a_lock_with_a_bad_version_is_written_again_by_fix"),
        word: Word::Succeeds,
        input: || vec!["bad-lock"],
        expect: Expect::Value((Vec::new(), Outcome::Success)),
    },
    Case {
        name: CaseName("a_dry_run_on_a_cold_store_proves_in_memory_and_writes_nothing"),
        word: Word::Succeeds,
        input: || vec!["cold-dry-run"],
        expect: Expect::Value((Vec::new(), Outcome::Success)),
    },
    Case {
        name: CaseName("a_path_that_holds_no_file_is_refused"),
        word: Word::Refuses,
        input: || vec!["--fix", "nowhere"],
        expect: Expect::error(Outcome::Error),
    },
    Case {
        name: CaseName("a_missing_lock_without_fix_is_refused"),
        word: Word::Refuses,
        input: Vec::new,
        expect: Expect::error(Outcome::Error),
    },
];

/// What the fixture's check finds.
const FOUND: [&str; 3] = ["p/marker", "p/part#closed", "p/part/errors#count"];

/// What the check said of the symbolic link beyond its one `#closed` finding: nothing, as a link is never read.
fn said_of_the_link(found: &serde_json::Value) -> Vec<String> {
    let said = found["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["path"].as_str() == Some("src/a/link.toml"))
        .filter_map(|row| row["rule"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    if said == ["p/part#closed"] {
        Vec::new()
    } else {
        said
    }
}

/// Holds the comment contract's findings and suppressions to the files that write each kind of directive.
fn suppressed_as_written(found: &serde_json::Value) {
    for (field, expected) in [
        ("findings", vec!["src/nested.rs"]),
        (
            "suppressed",
            vec![
                "src/block.rs",
                "src/doc.rs",
                "src/inline.rs",
                "src/ordinary.rs",
            ],
        ),
    ] {
        let mut paths = found[field]
            .as_array()
            .expect("finding rows")
            .iter()
            .map(|row| row["path"].as_str().expect("path"))
            .collect::<Vec<_>>();
        paths.sort_unstable();
        assert_eq!(paths, expected, "{field}");
    }
}

/// Runs `check` with `arguments` over `directory`, and reads its answer as JSON.
fn found(directory: &tempfile::TempDir, arguments: &[&str]) -> Result<serde_json::Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(arguments.iter().copied())
        .map_err(|_error| Outcome::Error)?;
    let answer = answer(parsed.arguments, &edge(directory), Format::Json)?;
    serde_json::from_str(&crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?)
        .map_err(|_error| Outcome::Error)
}

/// The paths of the findings of rule `p/marker` in `found`, sorted.
fn marked(found: &serde_json::Value) -> Vec<String> {
    let mut paths = found["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["rule"].as_str() == Some("p/marker"))
        .filter_map(|finding| finding["path"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

/// What a probe found wrong, if anything, and the ending of the command: a probe row passes when it found nothing.
fn probed(wrong: Vec<String>) -> Value {
    (wrong, Outcome::Success)
}

/// The lint store `check` keeps holds every file its lint call read, though a second lint call judges the counts after
/// it: that call keeps nothing, so one edit leaves the store of every other file as it was.
fn lint_store() -> Result<Value, Failure> {
    let directory = workspace();
    proven(&directory)?;
    let store = directory.path().join(".tastemaker_cache/ryo/lint.store");
    let read = ["src/a/errors.toml", "src/b/notes.toml"];
    let holds = || {
        std::fs::read(&store).is_ok_and(|kept| {
            read.iter().all(|path| {
                kept.windows(path.len())
                    .any(|window| window == path.as_bytes())
            })
        })
    };
    let mut wrong = Vec::new();
    drop(found(&directory, &["check", "--fix"])?);
    if !holds() {
        wrong.push("a cold check does not keep every file it read".to_owned());
    }
    std::fs::write(directory.path().join("src/b/notes.toml"), "b = [1]\n")
        .map_err(|_error| Outcome::Error)?;
    drop(found(&directory, &["check", "--fix"])?);
    if !holds() {
        wrong.push("a check after one edit does not keep the files it did not re-read".to_owned());
    }
    Ok(probed(wrong))
}

/// Moves the file's modification time to `time`, so a change to its bytes leaves no sign in its metadata.
fn stamp(file: &std::path::Path, time: std::time::SystemTime) -> Result<(), Failure> {
    std::fs::File::options()
        .write(true)
        .open(file)
        .and_then(|open| open.set_modified(time))
        .map_err(|_error| Outcome::Error)
}

/// One edit re-reads exactly one source file. Every file but the edited one is changed behind the store's back with its
/// size and modification time kept, so a run that read it again would find the marker the change plants: the check's
/// primary lint call carries parameters, files beside the corpus and the spec's rules that join a hosted relation, and
/// still reads only the file that moved.
fn one_edit() -> Result<Value, Failure> {
    let directory = workspace();
    proven(&directory)?;
    let sources = 12;
    let source = |number: usize| directory.path().join(format!("docs/{number}.toml"));
    let modified = |number: usize| {
        std::fs::metadata(source(number))
            .and_then(|meta| meta.modified())
            .map_err(|_error| Outcome::Error)
    };
    std::fs::create_dir_all(directory.path().join("docs")).map_err(|_error| Outcome::Error)?;
    for number in 0..sources {
        std::fs::write(source(number), "# MARKEX\nk = 1\n").map_err(|_error| Outcome::Error)?;
    }
    let mut wrong = Vec::new();
    if marked(&found(&directory, &["check", "--fix"])?) != ["src/a/errors.toml"] {
        wrong.push("a cold check does not read every source and find the one marker".to_owned());
    }
    for number in 1..sources {
        let kept = modified(number)?;
        std::fs::write(source(number), "# MARKER\nk = 1\n").map_err(|_error| Outcome::Error)?;
        stamp(&source(number), kept)?;
    }
    let edited = modified(0)?;
    std::fs::write(source(0), "# MARKER\nk = 1\n").map_err(|_error| Outcome::Error)?;
    stamp(&source(0), edited + std::time::Duration::from_secs(5))?;
    if marked(&found(&directory, &["check", "--fix"])?) != ["docs/0.toml", "src/a/errors.toml"] {
        wrong.push("a check after one edit does not read the edited file and no other".to_owned());
    }
    Ok(probed(wrong))
}

/// A rule entry whose `include` selects nothing applies its rule to nothing, as one whose glob matches no file does: the
/// marker in `src/a/errors.toml` is not found.
fn selects_nothing() -> Result<Value, Failure> {
    let mut wrong = Vec::new();
    for include in [
        "[]",
        "[\"no/such/**\"]",
        "[{ part = \"zzz\" }]",
        "[{ placed-by = \"nonesuch\" }]",
    ] {
        let directory = workspace();
        let manifest = directory.path().join("tastemaker.toml");
        let text = std::fs::read_to_string(&manifest).map_err(|_error| Outcome::Error)?;
        let selecting = text.replace(
            "rules = [{ rule = \"p/marker\", include = [\"**\"] }]",
            &format!("rules = [{{ rule = \"p/marker\", include = {include} }}]"),
        );
        if text == selecting {
            return Err(Outcome::Error);
        }
        std::fs::write(&manifest, selecting).map_err(|_error| Outcome::Error)?;
        proven(&directory)?;
        if !marked(&found(&directory, &["check", "--fix"])?).is_empty() {
            wrong.push(format!("`include = {include}` finds the marker"));
        }
    }
    Ok(probed(wrong))
}

/// A lock that does not begin `version = 1` is refused, and the `tm check --fix` its error names writes it again.
fn bad_lock() -> Result<Value, Failure> {
    let directory = workspace();
    proven(&directory)?;
    let check = |arguments: &[&str]| found(&directory, arguments).map(|_answer| ());
    check(&["check", "--fix"])?;
    let lock = directory.path().join("tastemaker.lock");
    let mut wrong = Vec::new();
    for bad in ["version = 99\n", "dependencies = []\n"] {
        std::fs::write(&lock, bad).map_err(|_error| Outcome::Error)?;
        if check(&["check"]) != Err(Outcome::Error) {
            wrong.push(format!("`{bad}` is not refused without --fix"));
        }
        check(&["check", "--fix"])?;
        let written = std::fs::read_to_string(&lock).map_err(|_error| Outcome::Error)?;
        if !written.contains("\nversion = 1\n") {
            wrong.push(format!("--fix wrote `{written}`"));
        }
    }
    Ok(probed(wrong))
}

/// `-n` on a cold store proves the package as it checks, in memory: it does not end 2 for want of a proof, and it writes
/// no store.
fn cold_dry_run() -> Result<Value, Failure> {
    let directory = workspace();
    let parsed = <Line as clap::Parser>::try_parse_from(["check", "--fix"])
        .map_err(|_error| Outcome::Error)?;
    let mut wrong = Vec::new();
    let answered =
        answer(parsed.arguments, &dry_run(edge(&directory)), Format::Json).map(|_answer| ());
    if answered != Ok(()) {
        wrong.push("a cold -n check does not answer".to_owned());
    }
    if directory.path().join(".tastemaker_cache").exists() {
        wrong.push("a dry run writes a store".to_owned());
    }
    Ok(probed(wrong))
}

fn subject(line: Input) -> Result<Value, Failure> {
    match line.as_slice() {
        ["lint-store"] => return lint_store(),
        ["one-edit"] => return one_edit(),
        ["selects-nothing"] => return selects_nothing(),
        ["bad-lock"] => return bad_lock(),
        ["cold-dry-run"] => return cold_dry_run(),
        _ => {}
    }
    let directory = workspace();
    let (comments, contracted) = sources(directory.path(), &line);
    proven(&directory)?;
    // `-n` is the command line's: the whole line parses with it before `check`, and the command gets it on its edge.
    let dry = line.contains(&"-n");
    let linked = line.contains(&"symbolic-link");
    let rest: Vec<&str> = line
        .into_iter()
        .filter(|argument| {
            *argument != "-n"
                && *argument != "comment-contract"
                && *argument != "template-contract"
                && *argument != "symbolic-link"
        })
        .collect();
    if dry {
        drop(
            crate::resources::command()
                .try_get_matches_from(
                    ["tm", "-n", "check"]
                        .into_iter()
                        .chain(rest.iter().copied()),
                )
                .map_err(|_error| Outcome::Error)?,
        );
    }
    let before = files(directory.path()).map_err(|_error| Outcome::Error)?;
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("check").chain(rest))
        .map_err(|_error| Outcome::Error)?;
    let edge = if dry {
        dry_run(edge(&directory))
    } else {
        edge(&directory)
    };
    let answer = answer(parsed.arguments, &edge, Format::Json)?;
    let outcome = answer.ending();
    if dry {
        let after = files(directory.path()).map_err(|_error| Outcome::Error)?;
        let written = after
            .iter()
            .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
            .map(|(path, _)| format!("wrote {}", path.display()))
            .collect::<Vec<_>>();
        if !written.is_empty() {
            return Ok((written, outcome));
        }
    }
    let found = serde_json::from_str::<serde_json::Value>(
        &crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?,
    )
    .map_err(|_error| Outcome::Error)?;
    if linked {
        return Ok((said_of_the_link(&found), outcome));
    }
    if comments {
        suppressed_as_written(&found);
        return Ok((Vec::new(), outcome));
    }
    let mut rules = found["findings"]
        .as_array()
        .map(|findings| {
            findings
                .iter()
                .filter_map(|finding| finding["rule"].as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rules.sort();
    let expected: &[&str] = if contracted { &CONTRACTED } else { &FOUND };
    Ok((if rules == expected { Vec::new() } else { rules }, outcome))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
