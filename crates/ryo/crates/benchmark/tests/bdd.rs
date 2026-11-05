//! Executable acceptance scenarios colocated with the shared benchmark operations.
#[path = "../src/inbound/use_cases/audit/acceptance.rs"]
mod audit;

#[path = "../src/inbound/use_cases/compare/acceptance.rs"]
mod compare;

#[path = "../src/inbound/use_cases/measure/acceptance.rs"]
mod measure;

#[path = "../src/inbound/use_cases/run_ryo/acceptance.rs"]
mod run_ryo;

#[path = "../src/inbound/use_cases/certify/acceptance.rs"]
mod certify;

#[path = "../src/inbound/use_cases/collect/acceptance.rs"]
mod collect;

#[path = "../src/inbound/use_cases/verify_inventory/acceptance.rs"]
mod verify_inventory;

#[path = "../src/inbound/use_cases/publish/acceptance.rs"]
mod publish;

#[path = "../src/inbound/use_cases/prepare_cases/acceptance.rs"]
mod prepare_cases;

#[path = "../src/inbound/use_cases/stage_suite/acceptance.rs"]
mod stage_suite;

#[path = "../src/inbound/use_cases/verify_cases/acceptance.rs"]
mod verify_cases;

#[path = "../src/inbound/use_cases/verify_reference_cases/acceptance.rs"]
mod verify_reference_cases;

#[path = "../src/inbound/use_cases/validate_case_receipts/acceptance.rs"]
mod validate_case_receipts;

/// Stateless world: each acceptance step constructs independently owned boundaries.
#[derive(Debug, Default, cucumber::World)]
struct Behavior;

/// The colocated acceptance features of this crate, sorted.
fn features()
-> ::std::result::Result<::std::vec::Vec<::std::path::PathBuf>, ::cargo_metadata::Error> {
    let tests = ::cargo_metadata::MetadataCommand::new()
        .manifest_path(::std::concat!(
            ::std::env!("CARGO_MANIFEST_DIR"),
            "/Cargo.toml"
        ))
        .no_deps()
        .other_options(::std::vec!["--offline".into(), "--locked".into()])
        .exec()?
        .workspace_root
        .join(::std::file!())
        .with_file_name("");
    let inbound = tests.with_file_name("src").join("inbound");
    let mut features = ::std::vec::Vec::new();
    for namespace in ["use_cases", "contracts"] {
        let owners = inbound.join(namespace);
        if !owners.is_dir() {
            continue;
        }
        for owner in ::std::fs::read_dir(owners)? {
            let owner = owner?.path();
            if owner.is_dir() {
                features.push(owner.join("acceptance.feature"));
            }
        }
    }
    features.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "feature")
    });
    features.sort();
    Ok(features)
}

/// Fails unless every feature parses, every scenario and outline carries exactly one case word,
/// and the features state a success and a refusal.
fn check(features: &[::std::path::PathBuf]) -> ::std::vec::Vec<usize> {
    ::std::assert!(!features.is_empty(), "no acceptance features discovered");
    let parsed = features
        .iter()
        .map(|feature| {
            ::cucumber::gherkin::Feature::parse_path(
                feature,
                ::cucumber::gherkin::GherkinEnv::default(),
            )
            .map_err(::std::boxed::Box::new)
        })
        .collect::<::std::result::Result<::std::vec::Vec<_>, _>>();
    ::std::assert!(parsed.is_ok(), "every feature parses");
    let scenarios = parsed
        .iter()
        .flatten()
        .flat_map(|feature| {
            feature
                .scenarios
                .iter()
                .chain(feature.rules.iter().flat_map(|rule| rule.scenarios.iter()))
        })
        .collect::<::std::vec::Vec<_>>();
    ::std::assert!(
        scenarios.iter().all(|scenario| !scenario.steps.is_empty()),
        "every scenario has executable steps"
    );
    ::std::assert!(
        scenarios.iter().all(|scenario| {
            (!matches!(
                scenario.keyword.as_str(),
                "Scenario Outline" | "Scenario Template"
            ) || !scenario.examples.is_empty())
                && scenario.examples.iter().all(|examples| {
                    examples
                        .table
                        .as_ref()
                        .is_some_and(|table| table.rows.len() > 1)
                })
        }),
        "every outline and Examples table has a data row"
    );
    let words = ["succeeds", "refuses", "fails", "rejects"];
    let one_word_each = scenarios.iter().all(|scenario| {
        scenario
            .tags
            .iter()
            .filter(|tag| words.contains(&tag.as_str()))
            .count()
            == 1
    });
    ::std::assert!(
        one_word_each,
        "every scenario and outline carries exactly one of @succeeds, @refuses, @fails, @rejects"
    );
    for word in ["succeeds", "refuses"] {
        let stated = scenarios
            .iter()
            .any(|scenario| scenario.tags.iter().any(|tag| tag == word));
        ::std::assert!(stated, "the features have a @{word} scenario");
    }
    parsed
        .iter()
        .flatten()
        .map(|feature| {
            let expanded = ::cucumber::feature::Ext::expand_examples(feature.clone());
            ::std::assert!(expanded.is_ok(), "every outline expands");
            expanded
                .iter()
                .map(|expanded_feature| {
                    let background = expanded_feature
                        .background
                        .as_ref()
                        .map_or(0, |steps| steps.steps.len());
                    let direct: usize = expanded_feature
                        .scenarios
                        .iter()
                        .map(|scenario| scenario.steps.len() + background)
                        .sum();
                    let rules: usize = expanded_feature
                        .rules
                        .iter()
                        .map(|rule| {
                            let rule_background = background
                                + rule
                                    .background
                                    .as_ref()
                                    .map_or(0, |steps| steps.steps.len());
                            rule.scenarios
                                .iter()
                                .map(|scenario| scenario.steps.len() + rule_background)
                                .sum::<usize>()
                        })
                        .sum();
                    ::std::assert!(direct + rules > 0, "every feature has executable scenarios");
                    direct + rules
                })
                .sum()
        })
        .collect()
}

/// Runs one feature, failing on a skipped step, on a failed step and when no step passed.
async fn run(feature: ::std::path::PathBuf, expected_steps: usize) {
    let result = <Behavior as ::cucumber::World>::cucumber()
        .with_default_cli()
        .fail_on_skipped()
        .run(feature)
        .await;
    let failed = ::cucumber::writer::Stats::<Behavior>::execution_has_failed(&result);
    let some_passed = ::cucumber::writer::Stats::<Behavior>::passed_steps(&result) != 0;
    ::std::assert!(!failed, "a step failed or was skipped");
    ::std::assert!(some_passed, "no step passed");
    ::std::assert_eq!(
        ::cucumber::writer::Stats::<Behavior>::passed_steps(&result),
        expected_steps,
        "every discovered scenario and example must execute"
    );
}

#[::tokio::main(flavor = "current_thread")]
async fn main() -> ::std::result::Result<(), ::cargo_metadata::Error> {
    let features = features()?;
    let expected = check(&features);
    for (feature, expected_steps) in features.into_iter().zip(expected) {
        run(feature, expected_steps).await;
    }
    Ok(())
}
