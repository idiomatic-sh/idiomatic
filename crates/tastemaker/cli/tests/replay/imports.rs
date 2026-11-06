//! Built CLI import policy distinguishes restricted visibility from actual dependencies.
use crate::fixes::{FOUND, tm};

#[test]
fn visibility_scopes_do_not_authorize_real_imports_types_or_calls() {
    let scratch = tempfile::tempdir().unwrap();
    let project = scratch.path();
    for (file, text) in [
        (
            "Cargo.toml",
            "[package]\nname='visibility_contract'\nversion='0.1.0'\nedition='2024'\n",
        ),
        ("src/lib.rs", "mod owner;"),
        (
            "src/owner.rs",
            "pub mod inner; pub struct Owner; pub fn work() {}",
        ),
        (
            "tastemaker.toml",
            "workspace = { name = 'visibility', dependencies = [{ package = 'p', path = 'package' }] }\ntemplates = [{ template = 'p/guard', file = 'src/owner/{module}.rs', imports = { default = 'deny' } }]\n",
        ),
        ("package/package.toml", "name = 'p'\n"),
        (
            "package/templates/guard/0.1.0/template.toml",
            "description = 'A scoped file.'\nwhy = 'Its real dependencies require permission.'\ncontent = '// scope contract'\n",
        ),
    ] {
        let destination = project.join(file);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(destination, text).unwrap();
    }
    let mut failures = Vec::new();
    for (name, module, text, expected) in [
        (
            "owner visibility",
            "inner",
            "pub(in crate::owner) struct Item;",
            0,
        ),
        ("ancestor visibility", "inner", "pub(super) struct Item;", 0),
        ("self visibility", "inner", "pub(in self) struct Item;", 0),
        (
            "same-path import",
            "inner",
            "pub(in crate::owner) struct Item;\nuse crate::owner::Owner;",
            1,
        ),
        (
            "same-path type",
            "inner",
            "pub(in crate::owner) struct Item;\npub struct Holder(pub crate::owner::Owner);",
            1,
        ),
        (
            "same-path call",
            "inner",
            "pub(in crate::owner) struct Item;\nfn run() { crate::owner::work(); }",
            1,
        ),
        (
            "renamed import",
            "inner",
            "pub(in crate::owner) struct Item;\nuse crate::owner::Owner as Alias;",
            1,
        ),
        (
            "new-module call",
            "extra",
            "pub(in crate::owner) struct Item;\nfn run() { crate::owner::work(); }",
            1,
        ),
    ] {
        std::fs::write(
            project.join("src/owner.rs"),
            format!("pub mod {module}; pub struct Owner; pub fn work() {{}}"),
        )
        .unwrap();
        for stale in ["inner", "extra"] {
            let stale = project.join(format!("src/owner/{stale}.rs"));
            if stale.exists() {
                std::fs::remove_file(stale).unwrap();
            }
        }
        std::fs::write(project.join(format!("src/owner/{module}.rs")), text).unwrap();
        let output = tm(&["check", "--fix", "--format", "json"], project);
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{name}: {}", String::from_utf8_lossy(&output.stderr)));
        let findings = report["findings"].as_array().unwrap();
        if findings.len() != expected
            || findings
                .iter()
                .any(|finding| finding["rule"] != "p/guard#imports")
            || output.status.code() != Some(if expected == 0 { 0 } else { FOUND })
        {
            failures.push(format!(
                "{name}: status {:?}, findings {findings:?}",
                output.status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
