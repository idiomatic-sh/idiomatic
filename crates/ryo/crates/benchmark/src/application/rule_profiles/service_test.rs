//! A rule whose settings profile differs from the bench's is verified under its own profile, for any linter.
use super::service::verify_each;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;
use std::path::{Path, PathBuf};

/// The profile of a generic linter fixture with one rule, `A1`.
const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"A1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\n";

/// A positive and a negative case, neither declaring a context.
const CASES: &str = "[[cases]]\nname = \"positive\"\ncomparison = \"full\"\nfiles = { \"a.txt\" = \"x\" }\nexpected = [{ rule = \"A1\", file = \"a.txt\", start = 0, end = 0 }]\n[[cases]]\nname = \"negative\"\ncomparison = \"full\"\nfiles = { \"a.txt\" = \"x\" }\nexpected = []\n";

/// Write one file, making its directory.
fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("file");
}

/// A bench of `A1`: listed-disk with a profile of `A1`'s own that is isolated when `own`, else isolated itself.
fn bench(root: &Path, own: bool) -> PathBuf {
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
    write(&bench.join("rules/A1/cases.toml"), CASES);
    write(&bench.join("rules/A1/query.ryoql"), "emit nothing;\n");
    bench
}

/// The number of cases prepared under the profile that sits beside `rules`, as the verifiers prepare them.
fn prepared(rules: &Path) -> Result<usize, ReportsError> {
    let scratch = tempfile::tempdir().expect("scratch");
    crate::application::case_preparation::service::cases(
        &crate::outbound::suite::factory::local(),
        &crate::outbound::context::factory::local(),
        rules,
        scratch.path(),
    )
    .map(|cases| cases.len())
}

/// `verify_each` over the fixture bench, preparing the cases of each directory it is given: its answer, and whether each
/// directory was the bench itself.
fn run(own: bool, shard: ShardPartition) -> (Result<usize, String>, Vec<bool>) {
    let root = tempfile::tempdir().expect("fixture");
    let bench = bench(root.path(), own);
    let mut given = Vec::new();
    let answer = verify_each(
        &crate::outbound::suite::factory::local(),
        &crate::outbound::inventory::factory::local(),
        &bench,
        shard,
        "verify-each-",
        &mut |directory| {
            given.push(directory == bench);
            prepared(&directory.join("rules"))
        },
    );
    (answer.map_err(|error| error.to_string()), given)
}

/// The bench's listed-disk mode refuses the isolated cases of `A1`; staged under its own profile, `A1` verifies.
#[test]
fn a_rule_with_its_own_context_mode_verifies_its_cases_under_its_own_profile() {
    let root = tempfile::tempdir().expect("fixture");
    let bench = bench(root.path(), true);
    assert_eq!(
        prepared(&bench.join("rules")).map_err(|error| error.to_string()),
        Err("case context must agree with explicit profile context mode".to_owned()),
        "the bench's mode is not the rule's"
    );
    assert_eq!(run(true, ShardPartition::All), (Ok(2), vec![false]));
}

/// A bench with no profile of a rule's own is verified whole, as it always was.
#[test]
fn a_bench_without_rule_profiles_is_verified_whole() {
    assert_eq!(run(false, ShardPartition::All), (Ok(2), vec![true]));
}
