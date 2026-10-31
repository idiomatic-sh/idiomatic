//! Real Python corpus witnesses for shared facts and operators introduced after the Rust recordings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use super::base::{Pending, Recording, rule_of};

/// These queries assert invariants; only explicitly named provenance and shape witnesses emit rows.
const QUERIES: &[(&str, &str)] = &[
    (
        "expression-shape-invariants",
        include_str!("../feature_contracts/shapes/query.ryoql"),
    ),
    (
        "expression-shape-witness",
        include_str!("../feature_contracts/shapes/witness.ryoql"),
    ),
    (
        "semantic-projection-invariants",
        include_str!("../feature_contracts/semantic/query.ryoql"),
    ),
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

/// Real upstream annotation occurrences, with an independently counted root inventory.
const EMBEDDED_QUERIES: &[(&str, &str)] = &[
    (
        "embedded-root-invariants",
        include_str!("../feature_contracts/embedded/query.ryoql"),
    ),
    (
        "embedded-root-witness",
        include_str!("../feature_contracts/embedded/witness.ryoql"),
    ),
];

/// Check invariants and require positive known-origin witnesses before returning measured feature feeds.
fn measure() -> Result<BTreeMap<String, u64>, String> {
    // COUNTING is shared with the other replay helpers, whose writers also run alone.
    let _alone = crate::alone();
    super::base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let measured = (|| {
        let (mut feeds, _) = measure_corpus(
            &super::base::snapshot::ruff(),
            QUERIES,
            &["scripts/**/*.py", "python/**/*.py"],
            &["known-origin-witness", "expression-shape-witness"],
        )?;
        // One upstream generated visitor API, not repeated or synthesized locally.
        // CPython's AST independently counts 1,712 quoted annotation occurrences.
        let (embedded, roots) = measure_corpus(
            &super::base::snapshot::target("libcst-annotations"),
            EMBEDDED_QUERIES,
            &["libcst/_typed_visitor.py"],
            &["embedded-root-witness"],
        )?;
        if roots != 1_712 {
            return Err(format!("LibCST has {roots} embedded roots, expected 1712"));
        }
        for (feature, count) in embedded {
            let total = feeds.entry(feature).or_default();
            *total = total
                .checked_add(count)
                .ok_or("feature row count overflow")?;
        }
        Ok(feeds)
    })();
    super::base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    measured
}

/// Require a whole real corpus, independent invariants and distinct positive witnesses.
fn measure_corpus(
    root: &Path,
    queries: &[(&str, &str)],
    include: &[&str],
    witnesses: &[&str],
) -> Result<(BTreeMap<String, u64>, usize), String> {
    let rules = queries
        .iter()
        .map(|(name, query)| {
            rule_of(&Recording {
                name: (*name).to_owned(),
                query: (*query).to_owned(),
                parameters: Vec::new(),
                include: include.iter().map(|path| (*path).to_owned()).collect(),
                exclude: Vec::new(),
                pending: Pending::Missing,
            })
        })
        .collect();
    let ran = super::base::lint_report_with_languages(
        root,
        rules,
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
        Vec::new(),
        false,
        vec![ryo_core::outbound::language::types::LanguageHandles::from(
            ryo_python::linked(),
        )],
    );
    let (rows, report) = ran?;
    if !report.unread.is_empty() {
        return Err(format!(
            "Python feature contracts have {} unread selected files",
            report.unread.len()
        ));
    }
    if !report.unknowns.is_empty() {
        return Err(format!(
            "Python feature contracts have {} undecided inputs",
            report.unknowns.len()
        ));
    }
    if rows.iter().any(|row| {
        row.first()
            .is_none_or(|rule| !witnesses.contains(&rule.as_str()))
    }) {
        return Err(format!(
            "Python feature invariants failed beside {witnesses:?}: {rows:?}"
        ));
    }
    for witness in witnesses {
        let count = rows
            .iter()
            .filter(|row| row.first().is_some_and(|rule| rule.as_str() == *witness))
            .count();
        if count < 1_000 {
            return Err(format!(
                "Python feature contracts have only {count} {witness} rows"
            ));
        }
    }
    if rows.iter().collect::<BTreeSet<_>>().len() != rows.len() {
        return Err(format!("{witnesses:?} contains duplicate identities"));
    }
    let feeds = report
        .features
        .iter()
        .map(|entry| (super::answers::feature_name(entry.feature), entry.rows.0))
        .collect();
    Ok((feeds, rows.len()))
}

/// Reuse the real corpus result across concurrent replay assertions.
pub(crate) fn measured() -> &'static Result<BTreeMap<String, u64>, String> {
    static MEASURED: OnceLock<Result<BTreeMap<String, u64>, String>> = OnceLock::new();
    MEASURED.get_or_init(measure)
}
