//! Real Python corpus witnesses for shared facts and operators introduced after the Rust recordings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use super::base::{Pending, Recording, rule_of};

/// These queries assert invariants; only explicitly named provenance and shape witnesses emit rows.
const QUERIES: &[(&str, &str)] = &[
    (
        "numeric-order-invariants",
        include_str!("../feature_contracts/numeric/query.ryoql"),
    ),
    (
        "integer-successor-invariants",
        include_str!("../feature_contracts/numeric/successor-invariants.ryoql"),
    ),
    (
        "integer-successor-witness",
        include_str!("../feature_contracts/numeric/witness.ryoql"),
    ),
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

/// Actual optional facts and generic operators; positive rows retain source identities.
const CATALOG_QUERIES: &[(&str, &str)] = &[
    (
        "numeric-literal-witness",
        include_str!("../feature_contracts/numeric/literal-witness.ryoql"),
    ),
    (
        "metadata-fact-invariants",
        include_str!("../feature_contracts/catalog/metadata-invariants.ryoql"),
    ),
    (
        "function-fact-invariants",
        include_str!("../feature_contracts/catalog/function-invariants.ryoql"),
    ),
    (
        "binding-event-witness",
        include_str!("../feature_contracts/catalog/binding-event.ryoql"),
    ),
    (
        "binding-read-witness",
        include_str!("../feature_contracts/catalog/binding-read.ryoql"),
    ),
    (
        "binding-shadow-witness",
        include_str!("../feature_contracts/catalog/binding-shadow.ryoql"),
    ),
    (
        "binding-use-witness",
        include_str!("../feature_contracts/catalog/binding-use.ryoql"),
    ),
    (
        "brace-format-field-witness",
        include_str!("../feature_contracts/catalog/brace-format-field.ryoql"),
    ),
    (
        "brace-format-parse-witness",
        include_str!("../feature_contracts/catalog/brace-format-parse.ryoql"),
    ),
    (
        "compiler-feature-witness",
        include_str!("../feature_contracts/catalog/compiler-feature.ryoql"),
    ),
    (
        "constant-key-witness",
        include_str!("../feature_contracts/catalog/constant-key.ryoql"),
    ),
    (
        "expression-type-witness",
        include_str!("../feature_contracts/catalog/expression-type.ryoql"),
    ),
    (
        "format-field-witness",
        include_str!("../feature_contracts/catalog/format-field.ryoql"),
    ),
    (
        "format-parse-witness",
        include_str!("../feature_contracts/catalog/format-parse.ryoql"),
    ),
    (
        "function-terminal-witness",
        include_str!("../feature_contracts/catalog/function-terminal.ryoql"),
    ),
    (
        "implicit-symbol-witness",
        include_str!("../feature_contracts/catalog/implicit-symbol.ryoql"),
    ),
    (
        "format-specification-witness",
        include_str!("../feature_contracts/catalog/format-specification.ryoql"),
    ),
    (
        "bytes-literal-witness",
        include_str!("../feature_contracts/catalog/bytes-literal.ryoql"),
    ),
    (
        "interpolated-string-literal-witness",
        include_str!("../feature_contracts/catalog/interpolated-string-literal.ryoql"),
    ),
    (
        "language-target-witness",
        include_str!("../feature_contracts/catalog/language-target.ryoql"),
    ),
    (
        "literal-expression-parse-witness",
        include_str!("../feature_contracts/catalog/literal-expression-parse.ryoql"),
    ),
    (
        "literal-properties-witness",
        include_str!("../feature_contracts/catalog/literal-properties.ryoql"),
    ),
    (
        "module-identity-witness",
        include_str!("../feature_contracts/catalog/module-identity.ryoql"),
    ),
    (
        "raw-format-field-witness",
        include_str!("../feature_contracts/catalog/raw-format-field.ryoql"),
    ),
    (
        "raw-format-segment-witness",
        include_str!("../feature_contracts/catalog/raw-format-segment.ryoql"),
    ),
    (
        "binding-fact-invariants",
        include_str!("../feature_contracts/catalog/binding-invariants.ryoql"),
    ),
    (
        "literal-fact-invariants",
        include_str!("../feature_contracts/catalog/literal-invariants.ryoql"),
    ),
    (
        "format-fact-invariants",
        include_str!("../feature_contracts/catalog/format-invariants.ryoql"),
    ),
    (
        "ordered-dfs-witness",
        include_str!("../feature_contracts/operators/ordered-dfs-witness.ryoql"),
    ),
    (
        "ordered-dfs-invariants",
        include_str!("../feature_contracts/operators/ordered-dfs-invariants.ryoql"),
    ),
    (
        "span-between-witness",
        include_str!("../feature_contracts/operators/span-between-witness.ryoql"),
    ),
];

/// Package ancestry witnesses must come from the real upstream package tree.
const CATALOG_CORPUS_WITNESSES: &[&str] = &["module-identity-witness", "numeric-literal-witness"];

/// Every newly exercised relation must have a nonvacuous source witness.
const CATALOG_WITNESSES: &[&str] = &[
    "binding-event-witness",
    "binding-read-witness",
    "binding-shadow-witness",
    "binding-use-witness",
    "brace-format-field-witness",
    "brace-format-parse-witness",
    "compiler-feature-witness",
    "constant-key-witness",
    "expression-type-witness",
    "format-field-witness",
    "format-parse-witness",
    "function-terminal-witness",
    "implicit-symbol-witness",
    "interpolated-string-literal-witness",
    "bytes-literal-witness",
    "format-specification-witness",
    "language-target-witness",
    "literal-expression-parse-witness",
    "literal-properties-witness",
    "module-identity-witness",
    "numeric-literal-witness",
    "raw-format-field-witness",
    "raw-format-segment-witness",
    "ordered-dfs-witness",
    "span-between-witness",
];

/// Sparse relations need a larger real package corpus; each query runs once per corpus.
const CATALOG_STRESS_WITNESSES: &[&str] = &[
    "binding-shadow-witness",
    "brace-format-field-witness",
    "compiler-feature-witness",
    "format-field-witness",
    "function-terminal-witness",
    "interpolated-string-literal-witness",
    "language-target-witness",
    "module-identity-witness",
    "numeric-literal-witness",
    "raw-format-field-witness",
];

/// Integrity checks accompany the sparse relation witnesses on the larger corpus.
const CATALOG_STRESS_INVARIANTS: &[&str] = &[
    "metadata-fact-invariants",
    "function-fact-invariants",
    "format-fact-invariants",
    "binding-fact-invariants",
];

/// Django's production sources add independently authored percent and brace formatting.
const CATALOG_FORMAT_STRESS_WITNESSES: &[&str] = &[
    "brace-format-field-witness",
    "format-field-witness",
    "raw-format-field-witness",
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
            &[
                "known-origin-witness",
                "expression-shape-witness",
                "integer-successor-witness",
            ],
            1_000,
        )?;
        // One upstream generated visitor API, not repeated or synthesized locally.
        // CPython's AST independently counts 1,712 quoted annotation occurrences.
        let (embedded, roots) = measure_corpus(
            &super::base::snapshot::target("libcst-annotations"),
            EMBEDDED_QUERIES,
            &["libcst/_typed_visitor.py"],
            &["embedded-root-witness"],
            1_000,
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
        let (catalog, _) = measure_corpus(
            &super::base::snapshot::ruff(),
            CATALOG_QUERIES,
            &["scripts/**/*.py", "python/**/*.py"],
            CATALOG_CORPUS_WITNESSES,
            1,
        )?;
        merge(&mut feeds, catalog)?;
        // Standalone relation fixtures have no package ancestry.
        let fixture_witnesses: Vec<_> = CATALOG_WITNESSES
            .iter()
            .copied()
            .filter(|witness| !CATALOG_CORPUS_WITNESSES.contains(witness))
            .collect();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../languages/python/tests/fixtures/relations");
        let (fixture_feeds, _) = measure_corpus(
            &fixtures,
            CATALOG_QUERIES,
            &[
                "binding_event/input*.py",
                "binding_read/input*.py",
                "binding_shadow/input*.py",
                "binding_use/input*.py",
                "brace_format_field/input*.py",
                "brace_format_parse/input*.py",
                "compiler_feature/input*.py",
                "constant_key/input*.py",
                "expression_type/input*.py",
                "format_field/input*.py",
                "format_parse/input*.py",
                "function_terminal/input*.py",
                "implicit_symbol/input*.py",
                "interpolated_string_literal/input*.py",
                "bytes_literal/input*.py",
                "format_specification/input*.py",
                "language_target/input*.py",
                "literal_expression_parse/input*.py",
                "literal_properties/input*.py",
                "raw_format_field/input*.py",
                "raw_format_segment/input*.py",
            ],
            &fixture_witnesses,
            1,
        )?;
        merge(&mut feeds, fixture_feeds)?;
        let stress_queries: Vec<_> = CATALOG_QUERIES
            .iter()
            .copied()
            .filter(|(name, _)| {
                CATALOG_STRESS_WITNESSES.contains(name) || CATALOG_STRESS_INVARIANTS.contains(name)
            })
            .collect();
        let (stress_feeds, _) = measure_corpus(
            &super::base::snapshot::target("catalog-pandas"),
            &stress_queries,
            &["pandas/**/*.py"],
            CATALOG_STRESS_WITNESSES,
            1,
        )?;
        merge(&mut feeds, stress_feeds)?;
        let format_queries: Vec<_> = CATALOG_QUERIES
            .iter()
            .copied()
            .filter(|(name, _)| {
                CATALOG_FORMAT_STRESS_WITNESSES.contains(name) || *name == "format-fact-invariants"
            })
            .collect();
        let (format_feeds, _) = measure_corpus(
            &super::base::snapshot::target("catalog-django"),
            &format_queries,
            &["django/**/*.py"],
            CATALOG_FORMAT_STRESS_WITNESSES,
            1,
        )?;
        merge(&mut feeds, format_feeds)?;
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
    minimum_witnesses: usize,
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
            "Python feature contracts have {} unread selected files; first: {:?}",
            report.unread.len(),
            report.unread.first()
        ));
    }
    if !report.unknowns.is_empty() {
        return Err(format!(
            "Python feature contracts have {} undecided inputs; first: {:?}",
            report.unknowns.len(),
            report.unknowns.first()
        ));
    }
    if rows.iter().any(|row| {
        row.first().is_none_or(|rule| {
            !witnesses.contains(&rule.as_str()) && !CATALOG_WITNESSES.contains(&rule.as_str())
        })
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
        if count < minimum_witnesses {
            return Err(format!(
                "Python feature contracts have only {count} {witness} rows"
            ));
        }
    }
    verify_composed_spans(&rows)?;
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

/// Merge observed engine feed counts; never multiply sparse source rows to meet the stress threshold.
fn merge(into: &mut BTreeMap<String, u64>, measured: BTreeMap<String, u64>) -> Result<(), String> {
    for (feature, count) in measured {
        let total = into.entry(feature).or_default();
        *total = total
            .checked_add(count)
            .ok_or("feature row count overflow")?;
    }
    Ok(())
}

/// The replay renders span cells as byte_start..byte_end; compare every actual composed span
/// with independently selected child-start and parent-end source locations, including delimiters.
fn verify_composed_spans(rows: &[Vec<String>]) -> Result<(), String> {
    for row in rows {
        if row
            .first()
            .is_none_or(|rule| rule != "span-between-witness")
        {
            continue;
        }
        let [_, _, key, _, _] = row.as_slice() else {
            return Err("malformed span witness row".into());
        };
        let mut parts = key.split(',');
        let Some(((_id, begin), (finish, span))) = parts
            .next()
            .zip(parts.next())
            .zip(parts.next().zip(parts.next()))
        else {
            return Err("malformed span witness key".into());
        };
        if parts.next().is_some() {
            return Err("malformed span witness key".into());
        }
        let begin: u32 = begin.parse().map_err(|_| "invalid span start")?;
        let finish: u32 = finish.parse().map_err(|_| "invalid span end")?;
        let Some((actual_begin, actual_finish)) = span.split_once("..") else {
            return Err("malformed composed span".into());
        };
        let actual_begin: u32 = actual_begin.parse().map_err(|_| "invalid composed start")?;
        let actual_finish: u32 = actual_finish.parse().map_err(|_| "invalid composed end")?;
        if begin >= finish || actual_begin != begin || actual_finish != finish {
            return Err(format!("span-between endpoints differ: {key}"));
        }
    }
    Ok(())
}

/// These are rendered-cell unit examples, not source rows added to the measured feed counts.
#[test]
fn composed_span_witness_checks_both_boundaries_and_nonempty_extent() {
    let row = |key: &str| {
        vec![
            "span-between-witness".to_owned(),
            "input.py".to_owned(),
            key.to_owned(),
            "1".to_owned(),
            "1".to_owned(),
        ]
    };
    assert!(verify_composed_spans(&[row("id,2,9,2..9")]).is_ok());
    for key in [
        "id,2,9,1..9",
        "id,2,9,2..8",
        "id,9,2,9..2",
        "id,2,2,2..2",
        "id,2,9,missing",
        "id,2,9,2..9,extra",
    ] {
        assert!(verify_composed_spans(&[row(key)]).is_err(), "{key}");
    }
}
