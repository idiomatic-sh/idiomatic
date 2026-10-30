//! The resolved construction boundary, independent from spelling-only Tastemaker fixtures.
mod support;
use support::{NativeConsumer, assert_success, cargo, write};

const SERVICES: &str = r#"
#[derive(Clone, Default)] pub struct Answer(pub u8);
pub enum Kind { Rule, Tuple(u8) }
pub struct Unit;
pub trait ServiceInterface { fn answer(&self) -> Answer; }
pub mod adapters {
    #[derive(Default)] pub struct Native;
    impl super::ServiceInterface for Native { fn answer(&self) -> super::Answer { super::Answer(7) } }
    impl Native { pub fn new() -> Self { Self } }
}
pub mod factory { pub fn selected() -> impl super::ServiceInterface { super::adapters::Native } }
pub use adapters::Native as Reexported;
pub type Alias = adapters::Native;
pub mod use_cases { pub mod check { pub mod use_case {
    pub struct Checker;
    impl Checker { pub fn new() -> Self { Self } }
    impl crate::ServiceInterface for Checker { fn answer(&self) -> crate::Answer { crate::Answer(9) } }
}}}
pub fn hidden() -> impl ServiceInterface { factory::selected() }
"#;

#[test]
fn resolved_construction_rejects_alias_default_macro_and_intermediate_bypasses() {
    let consumer = NativeConsumer::new("construction");
    write(consumer.root.path(), "foreign/src/lib.rs", SERVICES);
    let cases = [
        (
            "direct factory",
            "fn compose() -> impl foreign::ServiceInterface { foreign::factory::selected() }",
            false,
        ),
        (
            "import alias",
            "use foreign::factory::selected as renamed; fn compose() -> impl foreign::ServiceInterface { renamed() }",
            false,
        ),
        (
            "function reference",
            "fn compose() { let callback = foreign::factory::selected; let _ = callback(); }",
            false,
        ),
        (
            "reexport",
            "fn compose() -> impl foreign::ServiceInterface { foreign::Reexported::new() }",
            false,
        ),
        (
            "type alias",
            "fn compose() -> foreign::Alias { foreign::Alias::new() }",
            false,
        ),
        (
            "inferred Default",
            "fn compose() -> foreign::Alias { Default::default() }",
            false,
        ),
        (
            "inferred Default function reference",
            "fn compose() -> foreign::Alias { let selected: fn() -> foreign::Alias = Default::default; selected() }",
            false,
        ),
        (
            "macro expansion",
            "macro_rules! choose { () => { foreign::factory::selected() } } fn compose() -> impl foreign::ServiceInterface { choose!() }",
            false,
        ),
        (
            "intermediate service",
            "fn compose() -> impl foreign::ServiceInterface { foreign::use_cases::check::use_case::Checker::new() }",
            false,
        ),
        (
            "opaque helper",
            "fn compose() -> impl foreign::ServiceInterface { foreign::hidden() }",
            false,
        ),
        (
            "opaque helper reference",
            "fn compose() { let selected = foreign::hidden; let _ = selected(); }",
            false,
        ),
        (
            "builder bypass",
            "fn build_native() -> impl foreign::ServiceInterface { foreign::factory::selected() } fn compose() -> impl foreign::ServiceInterface { build_native() }",
            false,
        ),
        (
            "nested helper",
            "fn build() { fn hidden() { let _ = foreign::factory::selected(); } hidden(); }",
            false,
        ),
        (
            "same-named provider trait",
            "trait ProviderFamily { fn construct(); } struct Fake; impl ProviderFamily for Fake { fn construct() { let _ = foreign::factory::selected(); } }",
            false,
        ),
        (
            "production closure",
            "fn build() -> impl foreign::ServiceInterface { compose(|| foreign::factory::selected()) } fn compose<S: foreign::ServiceInterface>(factory: impl FnOnce() -> S) -> S { factory() }",
            true,
        ),
        (
            "parts callback fields",
            "struct Parts<F> { factory: F, build: u8 } fn compose<S: foreign::ServiceInterface>(parts: Parts<impl FnOnce(u8) -> S>) -> S { let factory = parts.factory; factory(parts.build) }",
            true,
        ),
        (
            "pure value default",
            "fn compose() -> foreign::Answer { foreign::Answer::default() }",
            true,
        ),
        (
            "pure unit enum value",
            "fn compose() -> foreign::Kind { foreign::Kind::Rule }",
            true,
        ),
        (
            "pure unit struct value",
            "fn compose() -> foreign::Unit { foreign::Unit }",
            true,
        ),
        (
            "pure tuple enum constructor",
            "fn compose() -> foreign::Kind { foreign::Kind::Tuple(3) }",
            true,
        ),
        (
            "unit service constructor",
            "fn compose() -> impl foreign::ServiceInterface { foreign::adapters::Native }",
            false,
        ),
        (
            "reexported unit service constructor",
            "fn compose() -> foreign::Alias { foreign::Reexported }",
            false,
        ),
        (
            "supplied service",
            "fn compose<S: foreign::ServiceInterface>(service: S) -> S { service }",
            true,
        ),
        (
            "checked composition helper",
            "fn helper<S: foreign::ServiceInterface>(service: S) -> impl foreign::ServiceInterface { service } fn compose<S: foreign::ServiceInterface>(service: S) -> impl foreign::ServiceInterface { helper(service) }",
            true,
        ),
        (
            "concrete selection in checked helper",
            "fn helper<S: foreign::ServiceInterface>(_: S) -> impl foreign::ServiceInterface { foreign::factory::selected() } fn compose<S: foreign::ServiceInterface>(service: S) -> impl foreign::ServiceInterface { helper(service) }",
            false,
        ),
        (
            "wrapped service selection",
            "fn compose() -> Box<dyn foreign::ServiceInterface> { Box::new(foreign::Alias::default()) }",
            false,
        ),
        (
            "collection service selection",
            "fn compose() -> Vec<Box<dyn foreign::ServiceInterface>> { vec![Box::new(foreign::Alias::default())] }",
            false,
        ),
        (
            "local lint weakening",
            "#[allow(composition_construction)] fn compose() {}",
            false,
        ),
    ];
    for (name, body, pass) in cases {
        let source =
            format!("#![allow(dead_code)]\nmod commands {{ mod composition {{ {body} }} }}\n");
        let output = consumer.check(&source);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.success(), pass, "{name}: {stderr}\n{stdout}");
        if !pass {
            assert!(
                stdout.contains("composition_construction")
                    || stderr.contains("composition_construction"),
                "{name}: wrong refusal: {stderr}\n{stdout}"
            );
        }
    }
}

#[test]
fn only_command_and_entry_compositions_are_roots() {
    let consumer = NativeConsumer::new("construction-roots");
    write(consumer.root.path(), "foreign/src/lib.rs", SERVICES);
    let selection =
        "fn compose() -> impl foreign::ServiceInterface { foreign::factory::selected() }";
    for (name, source, pass) in [
        (
            "crate composition",
            format!("#![allow(dead_code)]\nmod composition {{ {selection} }}\n"),
            true,
        ),
        (
            "hex composition",
            format!(
                "#![allow(dead_code)]\nmod timings {{ mod heap {{ mod composition {{ {selection} }} }} }}\n"
            ),
            true,
        ),
        (
            "command composition",
            format!(
                "#![allow(dead_code)]\nmod commands {{ mod check {{ mod composition {{ {selection} }} }} }}\n"
            ),
            false,
        ),
        (
            "entry composition",
            format!(
                "#![allow(dead_code)]\nmod entries {{ mod check {{ mod composition {{ {selection} }} }} }}\n"
            ),
            false,
        ),
    ] {
        let output = consumer.check(&source);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.success(), pass, "{name}: {stderr}");
    }
}

#[test]
fn provider_bootstrap_requires_the_resolved_registration_trait() {
    let consumer = NativeConsumer::new("construction-provider");
    write(consumer.root.path(), "foreign/src/lib.rs", SERVICES);
    write(
        consumer.root.path(),
        "contract/Cargo.toml",
        "[package]\nname='ryo_language'\nversion='0.1.0'\nedition='2024'\n",
    );
    write(
        consumer.root.path(),
        "contract/src/lib.rs",
        "pub mod registration { pub mod types { pub trait ProviderFamily { fn construct(); } } }",
    );
    write(
        consumer.root.path(),
        "consumer/Cargo.toml",
        "[package]\nname='consumer'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nforeign={path='../foreign'}\nryo_language={path='../contract'}\n",
    );
    // Rename the package before compiling, keeping the local consumer lock exact.
    assert_success(
        cargo(consumer.root.path())
            .args(["generate-lockfile", "--offline"])
            .output()
            .unwrap(),
    );
    let source = "mod commands { mod composition { struct Bridged; impl ryo_language::registration::types::ProviderFamily for Bridged { fn construct() { build_bridged(); } } fn build_bridged() { let _ = foreign::factory::selected(); } } }";
    assert_success(consumer.check_tests(source));
    let bypass = source.to_owned()
        + "\nmod other { pub trait ProviderFamily { fn construct(); } }\nmod entries { mod composition { struct Bridged; impl crate::other::ProviderFamily for Bridged { fn construct() { let _ = foreign::factory::selected(); } } } }";
    let output = consumer.check(&bypass);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("composition_construction")
            || String::from_utf8_lossy(&output.stderr).contains("composition_construction")
    );
}

#[test]
fn registered_macro_bootstrap_allows_only_its_direct_local_builder_call() {
    let consumer = NativeConsumer::new("construction-macro");
    write(consumer.root.path(), "foreign/src/lib.rs", SERVICES);
    write(
        consumer.root.path(),
        "consumer/Cargo.toml",
        "[package]\nname='consumer'\nversion='0.1.0'\nedition='2024'\n\
         [lib]\nproc-macro=true\n[dependencies]\nforeign={path='../foreign'}\n",
    );
    assert_success(
        cargo(consumer.root.path())
            .args(["generate-lockfile", "--offline"])
            .output()
            .unwrap(),
    );
    let cases = [
        ("direct bootstrap", "let _ = build();", "", true),
        (
            "factory beside bootstrap",
            "let _ = build(); let _ = foreign::factory::selected();",
            "",
            false,
        ),
        (
            "hidden helper",
            "helper();",
            "fn helper() { let _ = foreign::factory::selected(); }",
            false,
        ),
        (
            "compose cannot bootstrap",
            "compose();",
            "fn compose() { let _ = build(); }",
            false,
        ),
        (
            "indirect builder reference",
            "let factory = build; let _ = factory();",
            "",
            false,
        ),
        (
            "closure cannot bootstrap",
            "let factory = || build(); let _ = factory();",
            "",
            false,
        ),
        (
            "ordinary lookalike",
            "let _ = build();",
            "fn lookalike() { let _ = build(); }",
            false,
        ),
        (
            "different module builder",
            "let _ = nested::build();",
            "mod nested { pub fn build() -> impl foreign::ServiceInterface { foreign::factory::selected() } }",
            false,
        ),
    ];
    for (name, body, helpers, pass) in cases {
        let source = macro_consumer("#[proc_macro] pub", "input", body, helpers);
        assert_macro_result(&consumer, &source, name, pass);
    }
    for (name, registration, argument) in [
        ("not registered", "", "input"),
        (
            "not direct forwarding",
            "#[proc_macro] pub",
            "input.clone()",
        ),
    ] {
        let source = macro_consumer(registration, argument, "let _ = build();", "");
        assert_macro_result(&consumer, &source, name, false);
    }
    let derive = macro_consumer(
        "#[proc_macro_derive(Example)] pub",
        "input",
        "let _ = build();",
        "",
    );
    assert_macro_result(&consumer, &derive, "derive registration", true);
    let attribute = "#[proc_macro_attribute] pub fn edge(attribute: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream { composition::edge(attribute, input) } mod composition { fn build() { let _ = foreign::factory::selected(); } pub fn edge(_: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream { build(); input } }";
    assert_macro_result(&consumer, attribute, "attribute registration", true);
    let reordered = attribute.replace(
        "composition::edge(attribute, input)",
        "composition::edge(input, attribute)",
    );
    assert_macro_result(&consumer, &reordered, "reordered forwarding", false);
}

fn macro_consumer(registration: &str, argument: &str, body: &str, helpers: &str) -> String {
    format!(
        "#![allow(dead_code)]\n{registration} fn edge(input: proc_macro::TokenStream) -> proc_macro::TokenStream {{ composition::edge({argument}) }}\n\
         mod composition {{ fn build() -> impl foreign::ServiceInterface {{ foreign::factory::selected() }}\n\
         pub fn edge(input: proc_macro::TokenStream) -> proc_macro::TokenStream {{ {body} input }} {helpers} }}"
    )
}

fn assert_macro_result(consumer: &NativeConsumer, source: &str, name: &str, pass: bool) {
    let output = consumer.check(source);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.success(), pass, "{name}: {stdout}\n{stderr}");
    if !pass {
        assert!(
            stdout.contains("composition_construction")
                || stderr.contains("composition_construction"),
            "{name}: wrong refusal: {stdout}\n{stderr}"
        );
    }
}
