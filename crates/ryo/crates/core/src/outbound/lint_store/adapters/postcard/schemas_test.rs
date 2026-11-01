//! What the schemas of this adapter accept and refuse, one row of `CASES` for each input.

use super::super::types::TableOfContents;
use super::{
    bucket_bytes, bucket_from, contents, contents_bytes, rest_bytes, rest_from, tables_bytes,
    tables_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::lint_store::fixtures::fake::stored;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::Cell;
use ryo_values::domain::runs::types::{RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{Stored, StoredFinding, StoredHanded, StoredWords};

/// What a row offers the schemas.
#[derive(Clone, Copy, Debug)]
enum Offered {
    /// Each section written and read back.
    TheSameBuild,
    /// A table of contents read back by another build.
    AnotherBuild,
    /// Tables cut short.
    CutShort,
    /// A thousand findings in one file: how many times its path is written.
    APathOfManyFindings,
}

type Input = Offered;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_section_reads_back_as_it_was_written"),
        word: Word::Succeeds,
        input: || Offered::TheSameBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("another_builds_table_of_contents_reads_as_nothing"),
        word: Word::Succeeds,
        input: || Offered::AnotherBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("tables_cut_short_read_as_nothing"),
        word: Word::Succeeds,
        input: || Offered::CutShort,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_text_is_written_once_however_many_rows_name_it"),
        word: Word::Succeeds,
        input: || Offered::APathOfManyFindings,
        expect: Expect::Value(true),
    },
];

/// The bytes of the small tables of `stored`.
fn written(stored: &Stored) -> Vec<u8> {
    tables_bytes(stored).expect("written")
}

/// `stored` without what its files handed: what the small tables hold.
fn tables(mut stored: Stored) -> Stored {
    stored.handed = StoredHanded::default();
    stored
}

/// Whether what the files of `stored` handed reads back from its sections: the rest, and the words of each bucket.
fn handed_reads_back(stored: &Stored) -> bool {
    let rest = rest_bytes(&stored.handed)
        .ok()
        .and_then(|bytes| rest_from(&bytes));
    let mut buckets: Vec<Bucket> = stored
        .handed
        .frontier
        .iter()
        .map(|each| each.bucket)
        .collect();
    buckets.sort_unstable();
    buckets.dedup();
    let mut words = Vec::new();
    for bucket in buckets {
        let rows: Vec<&StoredWords> = stored
            .handed
            .frontier
            .iter()
            .filter(|each| each.bucket == bucket)
            .collect();
        let Some(read) = bucket_bytes(&rows)
            .ok()
            .and_then(|bytes| bucket_from(bucket, &bytes))
        else {
            return false;
        };
        words.extend(read);
    }
    rest.is_some_and(|rest| {
        StoredHanded {
            frontier: words,
            ..rest
        } == stored.handed
    })
}

fn subject(offered: Input) -> Value {
    match offered {
        Offered::TheSameBuild => {
            let mut stored = stored(1);
            stored.skipped = vec![Skipped {
                rule: RuleId("syntax-only".into()),
                path: FilePath("src/1/rejected.py".into()),
                reason: SkipReason::SyntaxRejected,
            }];
            tables_of(&written(&stored)) == tables(stored.clone()) && handed_reads_back(&stored)
        }
        Offered::AnotherBuild => {
            let bytes = contents_bytes("b", &TableOfContents::default()).expect("written");
            contents("b", &bytes).is_some() && contents("c", &bytes).is_none()
        }
        Offered::CutShort => {
            let mut bytes = written(&stored(1));
            bytes.truncate(bytes.len() - 3);
            tables_of(&bytes) == Stored::default()
        }
        Offered::APathOfManyFindings => {
            let mut many = tables(stored(4));
            let path = FilePath("src/a/very/long/path/of/the/file.rs".into());
            let template = many.findings.first().cloned();
            many.findings = (0..1000)
                .filter_map(|_| template.clone())
                .map(|each| StoredFinding {
                    subject: Some(Cell::File(path.0.clone())).into(),
                    ..each
                })
                .collect();
            let bytes = written(&many);
            let needle = path.0.as_bytes();
            bytes
                .windows(needle.len())
                .filter(|each| *each == needle)
                .count()
                == 1
                && tables_of(&bytes) == many
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |offered| Ok(subject(offered)));
}
