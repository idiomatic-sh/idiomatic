//! Each rule held for the replay gives, over its own scenarios, the findings they state (`rule_fixtures::check`).

use crate::rule_fixtures;

#[test]
fn module_cycles_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("module-cycles");
}

#[test]
fn error_variant_never_constructed_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("error-variant-never-constructed");
}

#[test]
fn trait_with_one_impl_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("trait-with-one-impl");
}

#[test]
fn workspace_membership_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("workspace-membership");
}

#[test]
fn layer_order_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("layer-order");
}

#[test]
fn module_declares_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("module-declares");
}

#[test]
fn variants_benched_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("variants-benched");
}

#[test]
fn captured_workload_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("captured-workload");
}

#[test]
fn bdd_runner_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("bdd-runner");
}

#[test]
fn case_vocabulary_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("case-vocabulary");
}

#[test]
fn crate_dependency_depth_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("crate-dependency-depth");
}

#[test]
fn distribution_wiring_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("distribution-wiring");
}

#[test]
fn orphan_rust_files_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("orphan-rust-files");
}

#[test]
fn scattered_dispatch_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("scattered-dispatch");
}

#[test]
fn workspace_completeness_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("workspace-completeness");
}

#[test]
fn pin_revision_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("pin-revision");
}

#[test]
fn no_hardcoded_paths_finds_what_its_scenarios_state() {
    let _beside = crate::beside();
    rule_fixtures::check("no-hardcoded-paths");
}
