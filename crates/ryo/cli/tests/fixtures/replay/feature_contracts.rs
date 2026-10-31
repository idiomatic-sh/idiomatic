//! Real Python corpus witnesses for shared facts and operators introduced after the Rust recordings.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use super::base::{Pending, Recording, rule_of};

/// These queries assert lexical invariants; only the explicitly named provenance witness emits rows.
const QUERIES: &[(&str, &str)] = &[
    (
        "identifier-invariants",
        include_str!("../feature_contracts/identifiers/query.ryoql"),
    ),
    (
        "indentation-invariants",
        include_str!("../feature_contracts/indentation/query.ryoql"),
    ),
    (
        "origin-invariants",
        include_str!("../feature_contracts/origins/query.ryoql"),
    ),
    (
        "known-origin-witness",
        include_str!("../feature_contracts/origins/witness.ryoql"),
    ),
];

/// Check invariants and require positive known-origin witnesses before returning measured feature feeds.
fn measure() -> Result<BTreeMap<String, u64>, String> {
    let rules = QUERIES
        .iter()
        .map(|(name, query)| {
            rule_of(&Recording {
                name: (*name).to_owned(),
                query: (*query).to_owned(),
                parameters: Vec::new(),
                // The pinned project's executable Python tools and package, independently tokenized:
                // 43 files and 1,218 INDENT tokens, excluding intentionally invalid parser fixtures.
                include: vec!["scripts/**/*.py".to_owned(), "python/**/*.py".to_owned()],
                exclude: Vec::new(),
                pending: Pending::Missing,
            })
        })
        .collect();
    super::base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let ran = super::base::lint_report_with_languages(
        &super::base::snapshot::ruff(),
        rules,
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
        Vec::new(),
        false,
        vec![ryo_core::outbound::language::types::LanguageHandles::from(
            ryo_python::linked(),
        )],
    );
    super::base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    let (rows, report) = ran?;
    if !report.unknowns.is_empty() {
        return Err(format!(
            "Python feature contracts have {} undecided inputs",
            report.unknowns.len()
        ));
    }
    if rows.iter().any(|row| {
        row.first()
            .is_none_or(|rule| rule != "known-origin-witness")
    }) {
        return Err(
            "a Python identifier, source interval or qualified-origin invariant failed".to_owned(),
        );
    }
    if rows.len() < 1_000 {
        return Err(format!(
            "Python feature contracts have only {} known-origin witnesses",
            rows.len()
        ));
    }
    Ok(report
        .features
        .iter()
        .map(|entry| (super::answers::feature_name(entry.feature), entry.rows.0))
        .collect())
}

/// Reuse the real corpus result across concurrent replay assertions.
pub(crate) fn measured() -> &'static Result<BTreeMap<String, u64>, String> {
    static MEASURED: OnceLock<Result<BTreeMap<String, u64>, String>> = OnceLock::new();
    MEASURED.get_or_init(measure)
}
