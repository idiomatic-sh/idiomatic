//! The public operation over a real bench of one rule.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::verify_rule_profiles::{
    interface::VerifyRuleProfilesUseCase as _, use_case::VerifyRuleProfilesUseCaseImpl,
};
use ryo_benchmark::outbound::suite::types::ArtifactText;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::Path;

const PROFILE: &str = "tool = \"lint\"\nversion = \"lint 1.0.0\"\nsha256 = [\"0000000000000000000000000000000000000000000000000000000000000000\"]\ndefaults = [\"A1\"]\nparser-configuration = \"target=1\"\n\n[rule-applicability]\nA1 = \"always\"\n";
const CASES: &str = "[[cases]]\nname = \"negative\"\ncomparison = \"full\"\nfiles = { \"a.txt\" = \"x\" }\nexpected = []\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("file");
}

/// The directories the verification is given for a bench of `A1`, with a profile of `A1`'s own when `own`.
fn given(own: bool) -> Vec<bool> {
    let root = tempfile::tempdir().expect("fixture");
    let bench = root.path().join("bench");
    write(&bench.join("benchmark.toml"), PROFILE);
    if own {
        write(&bench.join("profiles/A1/benchmark.toml"), PROFILE);
    }
    write(&bench.join("rules/A1/cases.toml"), CASES);
    write(&bench.join("rules/A1/query.ryoql"), "emit nothing;\n");
    let suite = ryo_benchmark::outbound::suite::factory::local();
    let inventory = ryo_benchmark::outbound::inventory::factory::local();
    let mut directories = Vec::new();
    let _: CaseCount = VerifyRuleProfilesUseCaseImpl::new(&suite, &inventory)
        .verify_rule_profiles(
            &bench,
            ShardPartition::All,
            ArtifactText("accept-"),
            &mut |directory| {
                directories.push(directory == bench);
                Ok(CaseCount(1))
            },
        )
        .expect("the bench verifies");
    directories
}

#[then("verify_rule_profiles gives each rule its own staged profile")]
fn staged(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(given(true), vec![false]);
}

#[then("verify_rule_profiles gives a bench without rule profiles to the verification once")]
fn whole(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert_eq!(given(false), vec![true]);
}
