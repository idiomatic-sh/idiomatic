use std::fs;
mod support;
use support::{NativeConsumer, assert_success, write};

enum Expected {
    Accepted,
    Disallowed,
    Unproven,
    OverrideRejected,
}

#[test]
fn compiler_policy_checks_defining_interfaces_across_crates() {
    let consumer = NativeConsumer::new("policy");
    let cases = [
        (
            "automatic implementations include primitives",
            include_str!("fixtures/generic_receiver_auto.rs"),
            Expected::Unproven,
        ),
        (
            "mixed nominal and primitive owners",
            include_str!("fixtures/generic_receiver_mixed.rs"),
            Expected::Unproven,
        ),
        (
            "a different parameter owns the local bound",
            include_str!("fixtures/generic_receiver_other_parameter.rs"),
            Expected::Unproven,
        ),
        (
            "borrowed generic receiver",
            include_str!("fixtures/generic_receiver_borrowed.rs"),
            Expected::Accepted,
        ),
        (
            "borrowed primitive bound",
            include_str!("fixtures/generic_receiver_borrowed_primitive.rs"),
            Expected::Unproven,
        ),
        (
            "collection bound",
            include_str!("fixtures/generic_receiver_collection.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver",
            include_str!("fixtures/generic_receiver.rs"),
            Expected::Accepted,
        ),
        (
            "generic_receiver_foreign",
            include_str!("fixtures/generic_receiver_foreign.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver_parameterized",
            include_str!("fixtures/generic_receiver_parameterized.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver_unsized",
            include_str!("fixtures/generic_receiver_unsized.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver_primitive",
            include_str!("fixtures/generic_receiver_primitive.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver_cycle",
            include_str!("fixtures/generic_receiver_cycle.rs"),
            Expected::Unproven,
        ),
        (
            "generic_receiver_nested",
            include_str!("fixtures/generic_receiver_nested.rs"),
            Expected::Accepted,
        ),
        (
            "generic_receiver_nested_primitive",
            include_str!("fixtures/generic_receiver_nested_primitive.rs"),
            Expected::Unproven,
        ),
        (
            "sealed_primitive_receiver",
            include_str!("fixtures/sealed_primitive_receiver.rs"),
            Expected::Disallowed,
        ),
        (
            "optional_receiver",
            include_str!("fixtures/optional_receiver.rs"),
            Expected::Disallowed,
        ),
        (
            "generic_receiver_projection",
            include_str!("fixtures/generic_receiver_projection.rs"),
            Expected::Unproven,
        ),
        (
            "nominal",
            include_str!("fixtures/nominal.rs"),
            Expected::Accepted,
        ),
        (
            "borrowed",
            include_str!("fixtures/borrowed.rs"),
            Expected::Accepted,
        ),
        (
            "generic storage",
            include_str!("fixtures/generic_storage.rs"),
            Expected::Accepted,
        ),
        (
            "generic declaration",
            include_str!("fixtures/generic_declaration.rs"),
            Expected::Accepted,
        ),
        (
            "generic implementation",
            include_str!("fixtures/generic_implementation.rs"),
            Expected::Accepted,
        ),
        (
            "generic implementation owner",
            include_str!("fixtures/generic_implementation_owner.rs"),
            Expected::Unproven,
        ),
        (
            "unselected",
            include_str!("fixtures/unselected.rs"),
            Expected::Accepted,
        ),
        (
            "foreign alias",
            include_str!("fixtures/foreign_alias.rs"),
            Expected::Disallowed,
        ),
        (
            "projection",
            include_str!("fixtures/projection.rs"),
            Expected::Disallowed,
        ),
        (
            "nested result",
            include_str!("fixtures/nested_result.rs"),
            Expected::Disallowed,
        ),
        (
            "string alias",
            include_str!("fixtures/string_alias.rs"),
            Expected::Disallowed,
        ),
        (
            "vector alias",
            include_str!("fixtures/vector_alias.rs"),
            Expected::Disallowed,
        ),
        (
            "associated binding",
            include_str!("fixtures/associated_binding.rs"),
            Expected::Disallowed,
        ),
        (
            "primitive receiver",
            include_str!("fixtures/primitive_receiver.rs"),
            Expected::Disallowed,
        ),
        (
            "dynamic binding",
            include_str!("fixtures/dynamic_binding.rs"),
            Expected::Disallowed,
        ),
        (
            "generic boundary",
            include_str!("fixtures/generic_boundary.rs"),
            Expected::Unproven,
        ),
        (
            "local override",
            include_str!("fixtures/local_override.rs"),
            Expected::OverrideRejected,
        ),
        (
            "input",
            include_str!("fixtures/input.rs"),
            Expected::Disallowed,
        ),
        (
            "an outbound port's input",
            include_str!("fixtures/port_input.rs"),
            Expected::Disallowed,
        ),
        (
            "a use case's input",
            include_str!("fixtures/use_case_input.rs"),
            Expected::Disallowed,
        ),
        (
            "tuple_input",
            include_str!("fixtures/tuple_input.rs"),
            Expected::Disallowed,
        ),
        (
            "slice_input",
            include_str!("fixtures/slice_input.rs"),
            Expected::Disallowed,
        ),
        (
            "array_input",
            include_str!("fixtures/array_input.rs"),
            Expected::Disallowed,
        ),
        (
            "callback_input",
            include_str!("fixtures/callback_input.rs"),
            Expected::Disallowed,
        ),
        (
            "constant",
            include_str!("fixtures/constant.rs"),
            Expected::Disallowed,
        ),
        (
            "opaque",
            include_str!("fixtures/opaque.rs"),
            Expected::Unproven,
        ),
    ];
    for (name, source, expected) in cases {
        let output = consumer.check(source);
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let codes: Vec<_> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|message| {
                message["message"]["code"]["code"]
                    .as_str()
                    .map(str::to_owned)
            })
            .collect();
        match expected {
            Expected::Accepted => assert!(output.status.success(), "{name}: {diagnostics}"),
            Expected::Disallowed => assert!(
                !output.status.success()
                    && codes.iter().any(|code| code == "nominal_boundary_type"),
                "{name}: {diagnostics}"
            ),
            Expected::Unproven => assert!(
                !output.status.success()
                    && codes.iter().any(|code| code == "unproven_boundary_type")
                    && !codes.iter().any(|code| code == "nominal_boundary_type"),
                "{name}: {diagnostics}"
            ),
            Expected::OverrideRejected => assert!(
                !output.status.success() && codes.iter().any(|code| code == "E0453"),
                "{name}: {diagnostics}"
            ),
        }
        eprintln!("{name}: expected native outcome observed");
    }

    write(
        consumer.root.path(),
        "contract/src/lib.rs",
        include_str!("fixtures/generic_receiver_contract.rs"),
    );
    assert_success(consumer.check_tests(include_str!("fixtures/generic_receiver_downstream.rs")));
    write(
        consumer.root.path(),
        "consumer/src/lib.rs",
        "impl contract::LocalValue for u32 {}",
    );
    let foreign_primitive = consumer.run();
    assert!(!foreign_primitive.status.success());
    assert!(String::from_utf8_lossy(&foreign_primitive.stdout).contains("E0117"));
    write(
        consumer.root.path(),
        "contract/src/lib.rs",
        include_str!("fixtures/contract.rs"),
    );

    // The policy is the lint's own: a clippy.toml that disallows nothing, or only other
    // types, neither weakens nor extends it.
    for configuration in [
        "disallowed-types=[]\n",
        "disallowed-types=['foreign::Count']\n",
    ] {
        write(consumer.root.path(), "clippy.toml", configuration);
        let aliased = consumer.check(include_str!("fixtures/string_alias.rs"));
        assert!(
            !aliased.status.success()
                && String::from_utf8_lossy(&aliased.stdout).contains("nominal_boundary_type"),
            "clippy.toml changed the nominal policy: {configuration}"
        );
        assert_success(consumer.check(include_str!("fixtures/nominal.rs")));
    }
    fs::remove_file(consumer.root.path().join("clippy.toml")).expect("remove clippy.toml");
    assert_success(consumer.check(include_str!("fixtures/nominal.rs")));
    let member = consumer.root.path().join("consumer/Cargo.toml");
    let member_source = fs::read_to_string(&member).expect("member manifest");
    for lint in ["nominal_boundary_type", "unproven_boundary_type"] {
        write(
            consumer.root.path(),
            "consumer/Cargo.toml",
            &format!("{member_source}\n[lints.rust.{lint}]\nlevel='allow'\n"),
        );
        let weakened = consumer.run();
        assert!(!weakened.status.success(), "Cargo cannot relax {lint}");
        assert!(
            String::from_utf8_lossy(&weakened.stdout)
                .contains("requires unweakened native lint levels")
        );
    }
    write(consumer.root.path(), "consumer/Cargo.toml", &member_source);
    let cargo_config = fs::read_to_string(consumer.root.path().join(".cargo/config.toml"))
        .expect("native consumer configuration");
    for level in ["warn", "allow"] {
        write(
            consumer.root.path(),
            ".cargo/config.toml",
            &format!("{cargo_config}\nrustflags=['--cap-lints={level}']\n"),
        );
        let capped = consumer.run();
        assert!(
            !capped.status.success(),
            "lint cap cannot disable nominal policy"
        );
        assert!(
            String::from_utf8_lossy(&capped.stdout)
                .contains("requires unweakened native lint levels")
        );
    }
    write(consumer.root.path(), ".cargo/config.toml", &cargo_config);
    let manifest =
        fs::read_to_string(consumer.root.path().join("Cargo.toml")).expect("configured manifest");
    write(
        consumer.root.path(),
        "Cargo.toml",
        "[workspace]\nmembers=['foreign','contract','consumer']\nresolver='3'\n",
    );
    let missing = consumer.run();
    assert!(
        !missing.status.success(),
        "missing selected library must fail: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
    write(consumer.root.path(), "Cargo.toml", &manifest);
    assert_success(consumer.run());
}

/// Every representation type of the lint's policy is rejected on its own boundary: one
/// finding per method of the fixture, each exposing one type.
#[test]
fn each_representation_is_rejected() {
    let consumer = NativeConsumer::new("representations");
    let source = include_str!("fixtures/representations.rs");
    let methods = source
        .lines()
        .filter(|line| line.trim_start().starts_with("fn "))
        .count();
    let output = consumer.check(source);
    let lines: std::collections::BTreeSet<u64> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["message"]["code"]["code"] == "nominal_boundary_type")
        .filter_map(|message| message["message"]["spans"][0]["line_start"].as_u64())
        .collect();
    assert_eq!(
        methods, 27,
        "the fixture exposes each of the policy's 27 types"
    );
    assert_eq!(
        lines.len(),
        methods,
        "a representation type was accepted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
