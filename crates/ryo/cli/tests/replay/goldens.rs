//! What the goldens must hold for the replay to compare against them: a golden without a count for a live rule (F1), a golden
//! `#unknown` count that is not a count (F4), an unfed feature with no reason, and a golden rule no recording holds. Each fails.

use super::answers::{golden_count, golden_rules, golden_strays, unfed_in, unknowns_in};
use std::path::Path;

/// A table of TOML text.
fn table(text: &str) -> toml::Table {
    text.parse().expect("a table")
}

#[test]
fn a_golden_without_a_rules_table_fails() {
    let without = table("digest = \"0\"\n");
    assert!(
        std::panic::catch_unwind(|| golden_rules(&without, Path::new("findings.toml")).len())
            .is_err(),
        "a golden without counts matches every rule"
    );
}

#[test]
fn a_live_rule_without_a_whole_golden_count_fails() {
    let rules = table(
        "\"idiomatic.arch/two\" = \"two\"\n\"idiomatic.arch/less\" = -1\n\"idiomatic.arch/kept\" = 3\n",
    );
    assert!(
        golden_count(&rules, "absent", false).is_err(),
        "a live rule with no count is skipped"
    );
    assert!(
        golden_count(&rules, "two", false).is_err(),
        "a count that is not a number reads as 0"
    );
    assert!(
        golden_count(&rules, "less", false).is_err(),
        "a negative count is compared"
    );
    assert!(
        golden_count(&rules, "kept", true).is_err(),
        "an undecided rule without an `#unknown` count passes"
    );
    assert_eq!(golden_count(&rules, "kept", false), Ok(Some(3)));
}

#[test]
fn a_golden_unknown_count_that_is_not_a_count_fails() {
    for text in [
        "\"idiomatic.arch/r#unknown\" = \"many\"\n",
        "\"idiomatic.arch/r#unknown\" = -2\n",
        "\"other/r#unknown\" = 2\n",
    ] {
        let rules = table(text);
        assert!(
            std::panic::catch_unwind(|| unknowns_in(&rules)).is_err(),
            "`{text}` is dropped"
        );
    }
    assert_eq!(
        unknowns_in(&table("\"idiomatic.arch/r#unknown\" = 2\n")).get("r"),
        Some(&2)
    );
}

#[test]
fn an_unfed_feature_without_a_reason_fails() {
    let path = Path::new("unfed.toml");
    let without = table("[feature]\n[feature.\"relation:Function\"]\nwhy = \"\"\n");
    assert!(
        std::panic::catch_unwind(|| unfed_in(&without, path)).is_err(),
        "an unfed feature with no reason passes"
    );
    let none = table("other = 1\n");
    assert!(
        std::panic::catch_unwind(|| unfed_in(&none, path)).is_err(),
        "unfed.toml without a feature table passes"
    );
}

#[test]
fn a_golden_rule_that_is_not_recorded_fails() {
    let rules = table(
        "\"idiomatic.arch/kept\" = 1\n\"idiomatic.arch/kept#unknown\" = 2\n\"idiomatic.arch/renamed\" = 3\n\"idiomatic.arch/gone#unknown\" = 4\n\"other/kept\" = 5\n",
    );
    let strays = golden_strays(&rules, &["kept"]);
    assert_eq!(
        strays.len(),
        3,
        "a golden rule no recording holds passes: {strays:?}"
    );
    assert!(golden_strays(&table("\"idiomatic.arch/kept\" = 1\n"), &["kept"]).is_empty());
}
