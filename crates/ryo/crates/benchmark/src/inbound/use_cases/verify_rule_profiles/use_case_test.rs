//! A rule whose settings profile differs from the bench's is verified under its own profile, for any linter.
use super::{interface::VerifyRuleProfilesUseCase as _, use_case::VerifyRuleProfilesUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::types::ArtifactText;
use benchmark_contracts::domain::requests::types::ShardPartition;
use std::borrow::Cow;
use std::path::Path;

/// What the fixture bench of one rule, `A1`, authors and how it is verified.
#[derive(Clone, Copy)]
enum Scenario {
    /// A listed-disk bench whose rule has an isolated profile of its own.
    OwnProfile,
    /// An isolated bench whose rule has no profile of its own.
    WholeBench,
    /// The first bench, its cases prepared under the bench's own profile, as the verifiers did before.
    BenchProfile,
    /// The first bench, verified as one shard.
    Sharded,
}
type Input = Scenario;
/// The number of cases prepared.
type Value = usize;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a rule with its own context mode verifies its cases under its own profile"),
        word: Word::Succeeds,
        input: || Scenario::OwnProfile,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a bench without rule profiles is verified whole"),
        word: Word::Succeeds,
        input: || Scenario::WholeBench,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the bench's context mode refuses the cases of a rule with its own mode"),
        word: Word::Refuses,
        input: || Scenario::BenchProfile,
        expect: Expect::Error(Cow::Borrowed(
            "case context must agree with explicit profile context mode",
        )),
    },
    Case {
        name: CaseName("a shard cannot partition cases across per-rule profiles"),
        word: Word::Refuses,
        input: || Scenario::Sharded,
        expect: Expect::Error(Cow::Borrowed(
            "a shard cannot partition cases across the per-rule settings profiles; verify the whole bench",
        )),
    },
];

/// The profile of a generic linter fixture with one rule, `A1`.
const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"A1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\n";

/// A positive and a negative case, neither declaring a context.
const CASES_TOML: &str = "[[cases]]\nname = \"positive\"\ncomparison = \"full\"\nfiles = { \"a.txt\" = \"x\" }\nexpected = [{ rule = \"A1\", file = \"a.txt\", start = 0, end = 0 }]\n[[cases]]\nname = \"negative\"\ncomparison = \"full\"\nfiles = { \"a.txt\" = \"x\" }\nexpected = []\n";

/// Write one file, making its directory.
fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("file");
}

/// The bench of `A1`: listed-disk with an isolated profile of `A1`'s own when `own`, else isolated itself.
fn bench(root: &Path, own: bool) -> std::path::PathBuf {
    let bench = root.join("bench");
    write(
        &bench.join("benchmark.toml"),
        &format!(
            "project-context-mode = \"{}\"\n{PROFILE}",
            if own { "listed-disk" } else { "isolated" }
        ),
    );
    if own {
        write(
            &bench.join("profiles/A1/benchmark.toml"),
            &format!("project-context-mode = \"isolated\"\n{PROFILE}"),
        );
    }
    write(&bench.join("rules/A1/cases.toml"), CASES_TOML);
    write(&bench.join("rules/A1/query.ryoql"), "emit nothing;\n");
    bench
}

/// The number of cases prepared under the profile that sits beside `rules`, as the verifiers prepare them.
fn prepared(
    rules: &Path,
) -> Result<
    benchmark_contracts::domain::workloads::types::case_count::value::CaseCount,
    benchmark_contracts::domain::reports::errors::ReportsError,
> {
    let scratch = tempfile::tempdir().expect("scratch");
    crate::application::case_preparation::service::cases(
        &crate::outbound::suite::factory::local(),
        &crate::outbound::context::factory::local(),
        rules,
        scratch.path(),
    )
    .map(|cases| {
        benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(cases.len())
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("fixture");
    let bench = bench(root.path(), !matches!(scenario, Scenario::WholeBench));
    let suite = crate::outbound::suite::factory::local();
    let inventory = crate::outbound::inventory::factory::local();
    let shard = match scenario {
        Scenario::Sharded => ShardPartition::Shard(
            benchmark_contracts::domain::requests::types::case_shard::value::CaseShard::new(
                benchmark_contracts::domain::requests::types::case_shard::value::ShardIndex(0),
                benchmark_contracts::domain::requests::types::case_shard::value::ShardCount(1),
            )
            .expect("one shard of one"),
        ),
        Scenario::OwnProfile | Scenario::WholeBench | Scenario::BenchProfile => ShardPartition::All,
    };
    let answer = match scenario {
        Scenario::BenchProfile => prepared(&bench.join("rules")),
        Scenario::OwnProfile | Scenario::WholeBench | Scenario::Sharded => {
            VerifyRuleProfilesUseCaseImpl::new(&suite, &inventory).verify_rule_profiles(
                &bench,
                shard,
                ArtifactText("verify-each-"),
                &mut |directory| prepared(&directory.join("rules")),
            )
        }
    };
    answer
        .map(|count| count.0)
        .map_err(|error| error.to_string().into())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
