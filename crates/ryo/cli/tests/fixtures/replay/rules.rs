//! The rule a recorded name stands for, and its query: a spec rule of the Tastemaker package, else a replay rule the cli tests own
//! (`tests/fixtures/replay-rules/<rule>/rule.toml`, an engine query no Tastemaker rule ships), else the highest version of a
//! Tastemaker rule (`tastemaker/rules/<rule>`) or a benchmark rule (`tastemaker/benchmarks/rules/<rule>`). The live rule is the
//! highest version directory, compared as three numbers; a lock may pin a lower one (`crates/ryo/tastemaker.lock` pins
//! pub-item-unreferenced-elsewhere 0.1.0), which the replay does not read.

use std::path::{Path, PathBuf};

/// The rules a spec of the Tastemaker package holds, by the directory under the package.
const SPEC_RULES: [(&str, &str); 5] = [
    (
        "captured-workload",
        "specs/crate/0.13.0/rules/captured-workload",
    ),
    ("bdd-runner", "specs/crate/0.13.0/rules/bdd-runner"),
    (
        "pin-revision",
        "specs/distribution/0.3.0/rules/pin-revision",
    ),
    (
        "no-hardcoded-paths",
        "specs/idiomatic-rust/0.1.0/rules/no-hardcoded-paths",
    ),
    (
        "variants-benched",
        "specs/idiomatic-rust/0.1.0/rules/variants-benched",
    ),
];

/// Where rule definitions are looked up: the Tastemaker package (its specs, rules and benchmarks/rules) and the replay rules.
pub(crate) struct Roots {
    /// The Tastemaker package: `specs/`, `rules/` and `benchmarks/rules/` under it.
    pub package: PathBuf,
    /// The replay rules, one directory holding a `rule.toml` for each.
    pub replay: PathBuf,
}

impl Roots {
    /// `tastemaker/` and `tests/fixtures/replay-rules/` of this repository.
    pub(crate) fn here() -> Self {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        Self {
            package: manifest.join("../../../tastemaker"),
            replay: manifest.join("tests/fixtures/replay-rules"),
        }
    }
}

/// The directory of `rule`'s rule.toml under `roots`: a spec rule; else `replay/<rule>`; else the highest version of
/// `package/rules/<rule>` or `package/benchmarks/rules/<rule>`. Panics when the name is none of these, when a replay rule is also a
/// spec, Tastemaker or benchmark rule, or when a rule is both a Tastemaker and a benchmark rule.
pub(crate) fn resolve(rule: &str, roots: &Roots) -> PathBuf {
    let spec = SPEC_RULES
        .iter()
        .find(|(name, _)| *name == rule)
        .map(|(_, under)| roots.package.join(under));
    let replay = roots.replay.join(rule);
    let packaged: Vec<PathBuf> = [
        roots.package.join("rules").join(rule),
        roots.package.join("benchmarks/rules").join(rule),
    ]
    .into_iter()
    .filter(|versions| versions.is_dir())
    .collect();
    assert!(
        packaged.len() < 2,
        "`{rule}` is both a Tastemaker rule and a benchmark rule: {packaged:?}"
    );
    assert!(
        !replay.is_dir() || (spec.is_none() && packaged.is_empty()),
        "the replay rule `{rule}` ({}) is named like a spec, Tastemaker or benchmark rule",
        replay.display()
    );
    if let Some(spec) = spec {
        return spec;
    }
    if replay.is_dir() {
        return replay;
    }
    match packaged.first() {
        Some(versions) => latest_version(versions),
        None => panic!("`{rule}` is no spec, replay, Tastemaker or benchmark rule"),
    }
}

/// The directory of `rule`'s rule.toml in this repository: `resolve(rule, &Roots::here())`.
pub(crate) fn directory(rule: &str) -> PathBuf {
    resolve(rule, &Roots::here())
}

/// The `query` of `rule`'s rule.toml. Panics when it holds none.
pub(crate) fn query(rule: &str) -> String {
    let path = directory(rule).join("rule.toml");
    let read = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let table: toml::Table = read
        .parse()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    table
        .get("query")
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{}: no `query`", path.display()))
        .to_owned()
}

/// The highest-numbered `major.minor.patch` directory under `versions`, compared as three numbers. Panics on any other name.
pub(crate) fn latest_version(versions: &Path) -> PathBuf {
    let mut numbered: Vec<([u64; 3], PathBuf)> = std::fs::read_dir(versions)
        .expect("the versions of the rule")
        .map(|entry| {
            let path = entry.expect("a version of the rule").path();
            let number = path
                .file_name()
                .and_then(|name| version_number(&name.to_string_lossy()))
                .unwrap_or_else(|| panic!("{}: not a `major.minor.patch` version", path.display()));
            (number, path)
        })
        .collect();
    numbered.sort();
    numbered
        .pop()
        .map(|(_, path)| path)
        .unwrap_or_else(|| panic!("{}: no version of the rule", versions.display()))
}

/// The three numbers of a `major.minor.patch` name, when it is one.
fn version_number(name: &str) -> Option<[u64; 3]> {
    let mut parts = name.split('.').map(str::parse::<u64>);
    let number = [
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ];
    parts.next().is_none().then_some(number)
}
