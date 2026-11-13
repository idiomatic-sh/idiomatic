//! The scenarios of substituted invocations, profiles and measured sources over the registration fixture.
use super::super::registered::fixtures::*;
use serde_json::json;
use std::borrow::Cow;
use std::path::PathBuf;

/// How a foreign measured profile reaches certification.
#[derive(Clone, Copy)]
pub(in crate::inbound::use_cases::certify::use_case_test) enum Route {
    /// The public operation with the foreign profile as the measured one.
    Certify,
    /// The case inventory preparation alone, with the foreign rules.
    Prepare,
    /// The public operation with the foreign profile and its own rules.
    Rules,
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn substitution(
    change: &str,
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
    let mut f = Fixture::new();
    match change {
        "executable" => f.document["rules"][0]["command"]["executable"] = json!("foreign-worker"),
        "arguments" => {
            f.document["rules"][0]["command"]["arguments"] = json!(["different-operation"])
        }
        "directory" => f.document["rules"][0]["command"]["directory"] = json!("foreign-context"),
        "environment" => {
            f.document["rules"][0]["command"]["environment"] = json!({"CONFIG":"foreign"})
        }
        "context" => {
            f.document["rules"][0]["command"]["context"] =
                f.document["rules"][1]["command"]["context"].clone()
        }
        "extra-context-file" => write(
            &PathBuf::from(
                f.document["rules"][0]["command"]["context"]
                    .as_str()
                    .unwrap(),
            )
            .join("rogue-config.toml"),
            "changed configuration",
        ),
        "foreign-profile" => {
            let foreign = f._root.path().join("foreign");
            let bytes = std::fs::read_to_string(f.profile.join("benchmark.toml")).unwrap();
            write(
                &foreign.join("benchmark.toml"),
                &(bytes + "parser-configuration='different-parser'\n"),
            );
            f.document["authority"]["profile"] = json!(foreign);
            f.document["authority"]["profile_sha256"] =
                json!(digest(&foreign.join("benchmark.toml")));
        }
        _ => unreachable!(),
    }
    f.save();
    f.certify()
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, reason, 0))
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn foreign_profile(
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
    let f = Fixture::new();
    let foreign = f._root.path().join("measured-foreign");
    for relative in [
        "helper.ryoql",
        "second.ryoql",
        "rules/A/query.ryoql",
        "rules/A/cases.toml",
        "rules/B/query.ryoql",
        "rules/B/cases.toml",
    ] {
        write(
            &foreign.join(relative),
            &std::fs::read_to_string(f.profile.join(relative)).unwrap(),
        );
    }
    let bytes = std::fs::read_to_string(f.profile.join("benchmark.toml")).unwrap();
    write(
        &foreign.join("benchmark.toml"),
        &(bytes + "parser-configuration='different-parser'\n"),
    );
    // The selected query directory remains the registered original: authority must
    // still bind the independently supplied measured profile and its settings.
    f.certify_profile(&foreign)
        .map(|_| 0)
        .map_err(|error| refusal(&f, &error, reason, 0))
}

pub(in crate::inbound::use_cases::certify::use_case_test) fn identical(
    changed: &str,
    route: Route,
    reason: &'static str,
) -> Result<usize, Cow<'static, str>> {
    use crate::outbound::suite::interface::SuitePort as _;
    let f = Fixture::new();
    let foreign = f._root.path().join("measured-foreign");
    copy_measured_profile(&f, &foreign);
    assert_eq!(
        digest(&foreign.join("benchmark.toml")),
        digest(&f.profile.join("benchmark.toml"))
    );
    write(&foreign.join(changed), "substituted measured source bytes");
    match route {
        Route::Certify => f.certify_profile(&foreign).map(|_| 0),
        Route::Prepare => {
            let suite = crate::outbound::suite::factory::local();
            let tool = suite
                .profile(crate::outbound::suite::types::ArtifactPath(&f.profile))
                .unwrap();
            crate::inbound::use_cases::certify::service::prepare(
                &crate::outbound::inventory::factory::local(),
                &suite,
                &f.registration,
                &f.profile,
                &tool,
                &foreign.join("rules"),
                &f.reference_executable,
                &f.worker_executable,
                &ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage::Full,
            )
            .map(|context| context.paths.len())
        }
        Route::Rules => f
            .certify_profile_rules(&foreign, &foreign.join("rules"))
            .map(|_| 0),
    }
    .map_err(|error| refusal(&f, &error, reason, 0))
}
