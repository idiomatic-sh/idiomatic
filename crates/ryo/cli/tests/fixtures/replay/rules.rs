//! The rule a recorded name stands for, and its query: its one definition, a rule of a spec of the Tastemaker package (at the spec's
//! highest version), a replay rule the cli tests own (`tests/fixtures/replay-rules/<rule>/rule.toml`, an engine query no Tastemaker
//! package defines), or the highest version of a Tastemaker rule (`tastemaker/rules/<rule>`) or a benchmark rule
//! (`tastemaker/benchmarks/rules/<rule>`). The live rule is the
//! highest version directory, compared as three numbers; a lock may pin a lower one (`crates/ryo/tastemaker.lock` pins
//! pub-item-unreferenced-elsewhere 0.1.0), which the replay does not read.

use std::path::{Path, PathBuf};

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

/// The directory of `rule`'s rule.toml under `roots`: its one definition, which is a rule of the highest version of a spec
/// (`package/specs/<spec>/<highest>/rules/<rule>`), a replay rule (`replay/<rule>`), or the highest version of a Tastemaker rule
/// (`package/rules/<rule>`) or a benchmark rule (`package/benchmarks/rules/<rule>`). Panics when the name has no definition, or
/// more than one: a replay rule named like a spec, Tastemaker or benchmark rule, a rule two specs hold, or any other pair.
pub(crate) fn resolve(rule: &str, roots: &Roots) -> PathBuf {
    let specs = roots.package.join("specs");
    let mut defined: Vec<PathBuf> = if specs.is_dir() {
        std::fs::read_dir(&specs)
            .expect("the specs of the package")
            .map(|entry| entry.expect("a spec of the package").path())
            .map(|spec| latest_version(&spec).join("rules").join(rule))
            .filter(|definition| definition.is_dir())
            .collect()
    } else {
        Vec::new()
    };
    defined.extend(
        [
            roots.replay.join(rule),
            roots.package.join("rules").join(rule),
            roots.package.join("benchmarks/rules").join(rule),
        ]
        .into_iter()
        .filter(|definition| definition.is_dir()),
    );
    defined.sort();
    match defined.as_slice() {
        [one]
            if one.starts_with(&roots.package.join("rules"))
                || one.starts_with(&roots.package.join("benchmarks/rules")) =>
        {
            latest_version(one)
        }
        [one] => one.clone(),
        [] => panic!("`{rule}` is no spec, replay, Tastemaker or benchmark rule"),
        many => panic!(
            "`{rule}` has more than one definition, so a replay rule or a spec could shadow another: {many:?}"
        ),
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
