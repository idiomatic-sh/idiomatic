//! What `update` promises its caller, one row of `CASES` for each case it has, over a workspace on disk.

use super::command::{UpdateArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use crate::resources::workspace::check::fixtures::{dry_run, edge, workspace};

/// The command line, after `update`.
type Input = (Vec<&'static str>, bool);
/// What the answer says, as JSON text.
type Value = bool;
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: UpdateArguments,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_persisted_upgrade_prints_its_changelog"),
        word: Word::Succeeds,
        input: || (vec!["marker", "--to", "0.2.0"], false),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("check_fix_repairs_a_failed_update_lock_write"),
        word: Word::Succeeds,
        input: || (vec!["marker", "--to", "0.2.0"], true),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_entry_follows_its_latest_version"),
        word: Word::Succeeds,
        input: || (vec!["-n"], false),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_version_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || (vec!["--to", "one", "-n"], false),
        expect: Expect::error(Outcome::Failure),
    },
];

fn subject((line, repair): Input) -> Result<Value, Failure> {
    let directory = workspace();
    // `-n` is the command line's, taken before the command reaches its arguments.
    let dry = line.contains(&"-n");
    let line = line.into_iter().filter(|argument| *argument != "-n");
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("update").chain(line))
        .map_err(|_error| Outcome::Error)?;
    let edge = if dry {
        dry_run(edge(&directory))
    } else {
        edge(&directory)
    };
    // The lock pins `marker` at 0.1.0, the one release there is, before 0.2.0 is offered.
    for args in [vec!["tm", "validate"], vec!["tm", "check", "--fix"]] {
        let locked = crate::resources::run_from(
            args.into_iter().map(std::ffi::OsString::from).collect(),
            &edge.start.0,
            "test",
        );
        assert_ne!(locked, Outcome::Error, "the lock is written");
    }
    if !dry {
        let release = directory.path().join("p/rules/marker/0.2.0");
        std::fs::create_dir_all(&release).expect("new release");
        let _: u64 = std::fs::copy(
            directory.path().join("p/rules/marker/0.1.0/rule.toml"),
            release.join("rule.toml"),
        )
        .expect("rule");
        std::fs::write(
            release.join("CHANGELOG.md"),
            "# marker\n## 0.2.0\nThe new behavior.\n## 0.1.0\nThe old behavior.\n",
        )
        .expect("notes");
    }
    if repair {
        let lock = directory.path().join("tastemaker.lock");
        std::fs::remove_file(&lock).expect("the lock check wrote");
        std::fs::create_dir_all(&lock).expect("refused lock path");
        assert!(matches!(
            answer(parsed.arguments, &edge),
            Err(Outcome::Error)
        ));
        std::fs::remove_dir(&lock).expect("remove the test's empty lock directory");
        for args in [vec!["tm", "validate"], vec!["tm", "check", "--fix"]] {
            let result = crate::resources::run_from(
                args.into_iter().map(std::ffi::OsString::from).collect(),
                &edge.start.0,
                "test",
            );
            assert_ne!(result, Outcome::Error, "the recovery command runs");
        }
        let lock = std::fs::read_to_string(lock).expect("lock repaired");
        return Ok(lock.contains("marker") && lock.contains("0.2.0"));
    }
    let answer = answer(parsed.arguments, &edge)?;
    if !dry {
        let text = answer.to_string();
        let json = serde_json::from_str::<serde_json::Value>(
            &crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?,
        )
        .map_err(|_error| Outcome::Error)?;
        return Ok(text.contains("The new behavior.")
            && !text.contains("The old behavior.")
            && json["moves"][0]["changes"]
                .as_str()
                .is_some_and(|notes| notes.contains("The new behavior.")));
    }
    let said = crate::output::json_text(&answer).map_err(|_error| Outcome::Error)?;
    Ok(said.contains("\"moves\":[]"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
