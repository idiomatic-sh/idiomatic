//! What the operation schemas accept and refuse, and that rendering is what parsing reads, byte for byte.

use super::{parse, render};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::violations::types::{Id, Key, Standing};

/// An operation's text.
type Input = String;
/// What it holds, after rendering it again and reading that back: the accepted records, each with its reasons, then
/// the removed keys and the perf records and keys.
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

/// An operation accepting the record of `fields` for `why`.
fn group(fields: &str, why: &str) -> String {
    format!(
        "version = 1\nparents = []\n\n[[accepted]]\nreasons = [{why}]\nrecords = [\n  {{ {fields} }},\n]\n"
    )
}

/// An operation setting the perf record of `fields` for `why`.
fn perf(fields: &str, why: &str) -> String {
    format!(
        "version = 1\nparents = []\n\n[[perf]]\nreasons = [{why}]\nrecords = [\n  {{ {fields} }},\n]\n"
    )
}

const KEY: &str = "tool = \"t\", rule = \"r\", path = \"a.rs\", level = \"warning\", fingerprint = \"00000000000000ff\"";
const FIELDS: &str = "tool = \"t\", rule = \"r\", path = \"a.rs\", level = \"warning\", fingerprint = \"00000000000000ff\", count = 1, message = \"m\", since = \"2026-10-19\"";
const BEST: &str =
    "tool = \"ryo-bench\", benchmark = \"check\", measure = \"wall-ms\", best = 831.0";

/// The fields of the product README's example row that starts with `start`: the README shows what the lock holds,
/// so its row must be one the lock reads and writes back byte for byte.
fn readme(start: &str) -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md");
    let text = std::fs::read_to_string(path).expect("the product README");
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(start))
        .and_then(|rest| rest.strip_suffix(" },"))
        .map(|fields| format!("{}{fields}", start.trim_start_matches("{ ")))
        .expect("the README's example row")
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_readmes_record_row_is_one_the_lock_reads_and_writes"),
        word: Word::Succeeds,
        input: || group(&readme("{ tool = \"ruff\""), "\"Existing backlog\""),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "accepted reported Existing backlog",
        )),
    },
    Case {
        name: CaseName("the_readmes_perf_row_is_one_the_lock_reads_and_writes"),
        word: Word::Succeeds,
        input: || perf(&readme("{ tool = \"bench\""), "\"Median of 3 rounds\""),
        expect: Expect::Value(std::borrow::Cow::Borrowed("perf Median of 3 rounds")),
    },
    Case {
        name: CaseName("a_record_reads_and_renders_back_to_itself_its_reason_once_for_its_group"),
        word: Word::Succeeds,
        input: || group(FIELDS, "\"why\""),
        expect: Expect::Value(std::borrow::Cow::Borrowed("accepted reported why")),
    },
    Case {
        name: CaseName("a_suppressed_record_carries_no_reason"),
        word: Word::Succeeds,
        input: || format!("version = 1\nparents = []\n\nsuppressed = [\n  {{ {FIELDS} }},\n]\n"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("accepted suppressed ")),
    },
    Case {
        name: CaseName("a_removed_record_is_its_key"),
        word: Word::Succeeds,
        input: || format!("version = 1\nparents = []\n\nremoved = [\n  {{ {KEY} }},\n]\n"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("removed reported")),
    },
    Case {
        name: CaseName("a_removed_suppressed_record_is_its_key_in_its_own_list"),
        word: Word::Succeeds,
        input: || {
            format!("version = 1\nparents = []\n\nsuppressed-removed = [\n  {{ {KEY} }},\n]\n")
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("removed suppressed")),
    },
    Case {
        name: CaseName("a_snapshot_names_what_it_covers"),
        word: Word::Succeeds,
        input: || {
            let digest = "a".repeat(64);
            format!("version = 1\nparents = []\ncovers = [\"{digest}\"]\n")
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("covers 1")),
    },
    Case {
        name: CaseName("a_perf_record_reads_and_renders_back_to_itself"),
        word: Word::Succeeds,
        input: || perf(BEST, "\"seeded\""),
        expect: Expect::Value(std::borrow::Cow::Borrowed("perf seeded")),
    },
    Case {
        name: CaseName("a_removed_perf_key_reads_and_renders_back_to_itself"),
        word: Word::Succeeds,
        input: || {
            "version = 1\nparents = []\n\nperf-removed = [\n  { tool = \"t\", benchmark = \"b\", measure = \"m\" },\n]\n"
                .to_owned()
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("perf removed")),
    },
    Case {
        name: CaseName("a_negative_best_is_refused"),
        word: Word::Refuses,
        input: || perf(&BEST.replace("831.0", "-1.0"), "\"seeded\""),
        expect: Expect::Refusal(|actual| selects(actual, "`best` is not a number")),
    },
    Case {
        name: CaseName("a_perf_group_without_a_reason_is_refused"),
        word: Word::Refuses,
        input: || perf(BEST, ""),
        expect: Expect::Refusal(|actual| selects(actual, "no attributable reason")),
    },
    Case {
        name: CaseName("a_reason_on_a_perf_row_is_refused"),
        word: Word::Refuses,
        input: || perf(&format!("{BEST}, reasons = [\"why\"]"), "\"seeded\""),
        expect: Expect::Refusal(|actual| selects(actual, "unknown field")),
    },
    Case {
        name: CaseName("a_group_without_a_reason_is_refused"),
        word: Word::Refuses,
        input: || group(FIELDS, ""),
        expect: Expect::Refusal(|actual| {
            selects(actual, "a group of records has no attributable reason")
        }),
    },
    Case {
        name: CaseName("a_reason_on_a_record_row_is_refused"),
        word: Word::Refuses,
        input: || group(&format!("{FIELDS}, reasons = [\"why\"]"), "\"why\""),
        expect: Expect::Refusal(|actual| selects(actual, "unknown field")),
    },
    Case {
        name: CaseName("a_removed_key_with_a_message_is_refused"),
        word: Word::Refuses,
        input: || {
            format!(
                "version = 1\nparents = []\n\nremoved = [\n  {{ {KEY}, message = \"m\" }},\n]\n"
            )
        },
        expect: Expect::Refusal(|actual| selects(actual, "unknown field")),
    },
    Case {
        name: CaseName("a_path_not_in_normal_form_is_refused"),
        word: Word::Refuses,
        input: || group(&FIELDS.replace("\"a.rs\"", "\"./a.rs\""), "\"why\""),
        expect: Expect::Refusal(|actual| {
            selects(actual, "path \"./a.rs\" is not in normalized form")
        }),
    },
    Case {
        name: CaseName("an_absolute_path_is_refused"),
        word: Word::Refuses,
        input: || group(&FIELDS.replace("\"a.rs\"", "\"/a.rs\""), "\"why\""),
        expect: Expect::Refusal(|actual| selects(actual, "path \"/a.rs\" is absolute")),
    },
    Case {
        name: CaseName("a_since_that_is_no_day_is_refused"),
        word: Word::Refuses,
        input: || group(&FIELDS.replace("2026-10-19", "yesterday"), "\"why\""),
        expect: Expect::Refusal(|actual| {
            selects(actual, "since \"yesterday\" is not a day, YYYY-MM-DD")
        }),
    },
    Case {
        name: CaseName("an_operation_with_no_parents_key_is_refused"),
        word: Word::Refuses,
        input: || group(FIELDS, "\"why\"").replacen("parents = []\n", "", 1),
        expect: Expect::Refusal(|actual| selects(actual, "missing field `parents`")),
    },
    Case {
        name: CaseName("a_group_of_no_records_is_refused"),
        word: Word::Refuses,
        input: || {
            "version = 1\nparents = []\n\n[[accepted]]\nreasons = [\"why\"]\nrecords = []\n"
                .to_owned()
        },
        expect: Expect::Refusal(|actual| selects(actual, "a group of records holds no record")),
    },
    Case {
        name: CaseName("an_unknown_level_is_refused"),
        word: Word::Refuses,
        input: || group(&FIELDS.replace("\"warning\"", "\"fatal\""), "\"why\""),
        expect: Expect::Refusal(|actual| selects(actual, "unknown level \"fatal\"")),
    },
    Case {
        name: CaseName("an_operation_with_no_version_is_refused"),
        word: Word::Refuses,
        input: || group(FIELDS, "\"why\"").replacen("version = 1\n", "", 1),
        expect: Expect::Refusal(|actual| selects(actual, "missing field `version`")),
    },
    Case {
        name: CaseName("another_version_is_refused"),
        word: Word::Refuses,
        input: || group(FIELDS, "\"why\"").replacen("version = 1", "version = 2", 1),
        expect: Expect::Refusal(|actual| selects(actual, "format version 2 is not 1")),
    },
    Case {
        name: CaseName("text_that_is_no_toml_is_refused"),
        word: Word::Refuses,
        input: || "version = ".to_owned(),
        expect: Expect::Refusal(|actual| selects(actual, "string values must be quoted")),
    },
];

fn selects(actual: &str, expected: &str) -> crate::cases::Selection {
    if actual.contains(expected) {
        crate::cases::Selection::Matches
    } else {
        crate::cases::Selection::Differs
    }
}

/// An id that is `x` for every key, as the rows above carry.
fn id_of(_key: &Key) -> Id {
    Id("x".into())
}

fn subject(text: Input) -> Result<Value, Failure> {
    let text: Box<str> = text.into();
    let operation = parse(&text, &id_of).map_err(|error| error.to_string())?;
    let again = render(&operation);
    let read = parse(&again, &id_of).map_err(|error| error.to_string())?;
    if *again != *text || read != operation {
        return Err(format!("rendered differently:\n{again}"));
    }
    let standing = |standing: Standing| match standing {
        Standing::Reported => "reported",
        Standing::Suppressed => "suppressed",
    };
    let mut said: Vec<String> = Vec::new();
    for record in &read.accepted {
        let reasons: Vec<&str> = record.reasons.iter().map(|reason| &*reason.0).collect();
        said.push(format!(
            "accepted {} {}",
            standing(record.key.standing),
            reasons.join(",")
        ));
    }
    for removal in &read.removed {
        said.push(format!("removed {}", standing(removal.key.standing)));
    }
    if !read.covers.is_empty() {
        said.push(format!("covers {}", read.covers.len()));
    }
    for record in &read.perf {
        let reasons: Vec<&str> = record.reasons.iter().map(|reason| &*reason.0).collect();
        said.push(format!("perf {}", reasons.join(",")));
    }
    if !read.perf_removed.is_empty() {
        said.push("perf removed".to_owned());
    }
    Ok(std::borrow::Cow::Owned(said.join("; ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
