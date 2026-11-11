//! Pure settlement preserves suppression identity and ordering; a placed manifest's variables are its own.
use std::collections::{BTreeMap, BTreeSet};

use super::service::{candidates, fingerprinted, mismatch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::Compared;
use crate::domain::contracts::types::{
    Contract, DataRow, RenderedContent, RowKey, RowNode, RowPlace, Syntax, SyntaxKind,
    TemplateIdentity, WrittenValue,
};
use crate::domain::definitions::types::Exactness;
use crate::domain::files::types::BorrowedText;
use crate::domain::findings::service::{error, identified};
use crate::domain::findings::types::{Fingerprint, PreparedIdentity, Reason, Suppression};
use crate::domain::paths::types::RepoPath;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// What a settlement of one standing and one suppressed finding keeps.
    Settled,
    /// How many findings an exact template's file that adds a value to its `variables` has, as a manifest and not.
    Variables,
}

type Input = Question;
/// What the row said.
#[derive(Debug, PartialEq, Eq)]
enum Value {
    /// The standing and accepted findings, and the accepted one's fingerprint.
    Settled(usize, usize, Fingerprint),
    /// The findings of a `tastemaker.toml` and of another file.
    Reported(usize, usize),
}
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("suppressed_and_standing_findings_keep_their_identities"),
        word: Word::Succeeds,
        input: || Question::Settled,
        expect: Expect::Value(Value::Settled(1, 1, Fingerprint(7))),
    },
    Case {
        name: CaseName("a_manifest_adds_values_to_its_variables_and_another_file_may_not"),
        word: Word::Succeeds,
        input: || Question::Variables,
        expect: Expect::Value(Value::Reported(0, 1)),
    },
];

/// The rows of `variables = { libraries = [...] }` with `libraries`, as a data language gives them.
fn variables(libraries: &[&str]) -> Vec<DataRow> {
    let row = |syntax, (id, owner): (&str, &str), (key, kind, text): (&str, &str, &str)| DataRow {
        syntax,
        id: RowNode(id.into()),
        owner: RowNode(owner.into()),
        key: RowKey(key.into()),
        kind: SyntaxKind(kind.into()),
        text: WrittenValue(text.into()),
        span: RowPlace::Unplaced,
    };
    let mut rows = vec![
        row(Syntax::Entries, ("#v", "#0"), ("variables", "mapping", "")),
        row(Syntax::Mappings, ("#vm", "#v"), ("0", "standard", "")),
        row(
            Syntax::Entries,
            ("#vl", "#vm"),
            ("libraries", "sequence", ""),
        ),
    ];
    for (place, library) in libraries.iter().enumerate() {
        rows.push(row(
            Syntax::Elements,
            (&format!("#vl{place}"), "#vl"),
            (&place.to_string(), "string", &format!("\"{library}\"")),
        ));
    }
    rows
}

/// The findings of an exact template's file at `target` that adds a library to its template's.
fn reported(target: &str) -> usize {
    let rendered = RepoPath(format!("rendered/{target}").into());
    let target = RepoPath(target.into());
    let data = BTreeMap::from([
        (rendered.clone(), variables(&["a"])),
        (target.clone(), variables(&["a", "b"])),
    ]);
    let none = BTreeSet::new();
    let contract = Contract {
        target,
        rendered,
        text: RenderedContent("".into()),
        identity: TemplateIdentity("demo".into()),
        exact: Exactness::Exact,
    };
    let compared = Compared {
        data: &data,
        unread: &none,
        dropped: &none,
    };
    mismatch(&contract, &compared, BorrowedText("")).len()
}

fn subject(question: Input) -> Value {
    match question {
        Question::Settled => settled(),
        Question::Variables => {
            Value::Reported(reported("demo/tastemaker.toml"), reported("demo/own.toml"))
        }
    }
}

/// What settling one standing finding and one suppressed one keeps.
fn settled() -> Value {
    let first = error(
        crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
            "r".into(),
        )),
        RepoPath("a.txt".into()),
        "first",
    );
    let second = error(
        crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
            "s".into(),
        )),
        RepoPath("b.txt".into()),
        "second",
    );
    let all = candidates(
        vec![second],
        vec![Suppression {
            finding: first,
            reason: Reason("valid".into()),
        }],
    );
    let identities = identified(all.iter().map(|candidate| PreparedIdentity {
        finding: &candidate.finding,
        line: candidate.line.clone(),
    }));
    let result = fingerprinted(all, identities, &|_| Fingerprint(7));
    Value::Settled(
        result.standing.len(),
        result.accepted.len(),
        result.accepted[0].finding.fingerprint,
    )
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
