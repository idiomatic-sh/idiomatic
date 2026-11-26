//! What the replay's suites that hold ryo's manifest rows to the tools of the other ecosystems (Go, npm, pnpm, Bun, uv) share beside
//! `tools`: a tool run with a home of its own, the fixture projects they read, and the JSON they say.

use std::path::Path;

/// What `program` prints when it runs `arguments` in `directory`, as `tool_result` runs it with a `HOME` of its own beside `variables`,
/// so that nothing of the machine's configuration or caches is read.
///
/// # Panics
///
/// When the tool is not there or fails: a reference that cannot answer has nothing to compare.
#[cfg(any(feature = "go", feature = "typescript", feature = "python"))]
pub(crate) fn tool_output(
    program: &str,
    arguments: &[&str],
    directory: &Path,
    variables: &[(&str, &str)],
) -> String {
    let own = tempfile::tempdir().expect("a directory");
    let home = own.path().to_string_lossy().into_owned();
    let mut given = vec![("HOME", home.as_str())];
    given.extend_from_slice(variables);
    super::tools::tool_result(program, arguments, directory, &given)
        .unwrap_or_else(|said| panic!("`{program} {}` fails: {said}", arguments.join(" ")))
}

/// The manifest named `name` in the directory `directory` of the repository, the root's own when it is empty.
#[cfg(any(feature = "typescript", feature = "python"))]
pub(crate) fn manifest_in(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_owned()
    } else {
        format!("{directory}/{name}")
    }
}

/// The fixture project of the cli's tests named `name`, as the tools see it: its directory with every link resolved.
#[cfg(any(feature = "typescript", feature = "python"))]
pub(crate) fn fixture(name: &str) -> std::path::PathBuf {
    std::fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/manifests")
            .join(name),
    )
    .expect("the fixture project is there")
}

/// The entries of the JSON object `value` holds at `key`, none when it is absent.
#[cfg(any(feature = "typescript", feature = "python"))]
pub(crate) fn entries<'value>(
    value: &'value serde_json::Value,
    key: &str,
) -> Vec<(&'value String, &'value serde_json::Value)> {
    value
        .get(key)
        .and_then(serde_json::Value::as_object)
        .map(|object| object.iter().collect())
        .unwrap_or_default()
}

/// Each language this build links that reads `package.json`, by its name.
#[cfg(feature = "typescript")]
pub(crate) fn package_json_languages() -> Vec<(
    &'static str,
    ryo_sdk::languages::Language,
)> {
    vec![
        #[cfg(feature = "typescript")]
        (
            "typescript",
            ryo_typescript::linked(),
        ),
    ]
}
