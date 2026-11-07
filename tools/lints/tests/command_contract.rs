//! Command proof classification uses the supplied call, not spelling or downstream values.
mod support;
use support::{NativeConsumer, assert_success, cargo};

const UNIVERSAL: &str = r#"
    #[test] fn check_preserves_supplied_service_contract() { assert_eq!(2 + 2, 4); }
"#;
const FAILURE: &str = r#"
    #[test] fn check_fails_when_the_service_fails() { assert_eq!(Err::<u8, _>(7), Err(7)); }
"#;

fn source(body: &str, proofs: &str) -> String {
    format!(
        r#"
        pub mod commands {{ pub mod fixture {{ pub mod check {{
            pub mod command {{
                pub fn run() {{}}
                {body}
            }}
            #[cfg(test)] mod command_test {{ {proofs} }}
        }} }} }}
    "#
    )
}

fn rejected(output: std::process::Output, expected: &str) {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "unexpected pass: {text}");
    assert!(
        text.contains("command_service_contract"),
        "wrong diagnostic: {text}"
    );
    assert!(text.contains(expected), "wrong contract refusal: {text}");
}

#[test]
fn supplied_call_output_drives_the_error_obligation() {
    let consumer = NativeConsumer::new("command-contract");
    let cases = [
        (
            "Fn",
            "pub fn respond(service: impl Fn() -> Result<u8, ()>) { let _ = service(); }",
        ),
        (
            "Fn pointer alias",
            "type Service = fn() -> Result<u8, ()>; pub fn respond(service: Service) { let alias = service; let _ = alias(); }",
        ),
        (
            "cloned Fn pointer",
            "type Service = fn() -> Result<u8, ()>; pub fn respond(service: Service) { let alias = service.clone(); let _ = alias(); }",
        ),
        (
            "dyn Fn",
            "pub fn respond(service: &dyn Fn() -> Result<u8, ()>) { let _ = service(); }",
        ),
        (
            "field Fn",
            "pub struct Parts<F> { pub service: F } pub fn respond(parts: Parts<impl Fn() -> Result<u8, ()>>) { let alias = &parts.service; let _ = alias(); }",
        ),
        (
            "Interface",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { fn read(&self) -> Result<super::Answer, super::Failure>; } } use interface::ServiceInterface; pub fn respond(service: &impl ServiceInterface) { let _ = service.read(); }",
        ),
        (
            "field Interface alias",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { fn read(&self) -> Result<super::Answer, super::Failure>; } } use interface::ServiceInterface; pub struct Parts<S> { pub service: S } pub fn respond(parts: &Parts<impl ServiceInterface>) { let alias = &parts.service; let _ = alias.read(); }",
        ),
        (
            "UFCS",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { fn read(&self) -> Result<super::Answer, super::Failure>; } } use interface::ServiceInterface; pub fn respond(service: &impl ServiceInterface) { let _ = ServiceInterface::read(service); }",
        ),
        (
            "associated Interface method alias",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { fn read(&self) -> Result<super::Answer, super::Failure>; } } use interface::ServiceInterface; pub fn respond<S: ServiceInterface>(service: &S) { let read = S::read; let _ = read(service); }",
        ),
        (
            "associated Interface function pointer alias chain",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { fn read(&self) -> Result<super::Answer, super::Failure>; } } use interface::ServiceInterface; pub struct Parts<S> { pub service: S } pub fn respond<S: ServiceInterface>(parts: &Parts<S>) { let read: fn(&S) -> Result<Answer, Failure> = S::read; let alias = read; let _ = alias(&parts.service); }",
        ),
        (
            "Result alias",
            "type Answer<T> = Result<T, ()>; pub fn respond(service: impl Fn() -> Answer<u8>) { let _ = service(); }",
        ),
        (
            "generic Result content",
            "pub fn respond<T>(service: impl Fn() -> Result<T, ()>) { let _ = service(); }",
        ),
        (
            "normalized associated Result output",
            "pub struct Answer; pub struct Failure; pub mod interface { pub trait ServiceInterface { type Output; fn read(&self) -> Self::Output; } } use interface::ServiceInterface; pub fn respond(service: &impl ServiceInterface<Output = Result<Answer, Failure>>) { let _ = service.read(); }",
        ),
    ];
    for (name, body) in cases {
        let complete = source(body, &format!("{UNIVERSAL}{FAILURE}"));
        assert_success(consumer.check_tests(&complete));
        rejected(
            consumer.check_tests(&source(body, UNIVERSAL)),
            "check_fails_when_the_service_fails",
        );
        println!("proved supplied Result contract: {name}");
    }
}

#[test]
fn infallible_supplied_calls_do_not_inherit_argument_or_answer_errors() {
    let consumer = NativeConsumer::new("command-infallible");
    for body in [
        "pub fn respond(service: impl Fn() -> u8) { let _ = service(); }",
        "pub struct Answer; pub mod interface { pub trait ServiceInterface { fn read(&self) -> super::Answer; } } use interface::ServiceInterface; pub struct Parts<S> { pub service: S } pub fn respond(parts: &Parts<impl ServiceInterface>) { let _ = parts.service.read(); }",
        "pub struct Answer; pub mod interface { pub trait ServiceInterface { fn read(&self) -> super::Answer; } } use interface::ServiceInterface; pub fn respond<S: ServiceInterface>(service: &S) { let read = S::read; let alias = read; let _ = alias(service); }",
        "pub struct Answer; impl Answer { pub fn find(&self) -> Result<u8, ()> { Err(()) } } pub fn respond(service: impl Fn() -> Answer) { let _ = service().find(); }",
        "pub struct Arguments; impl Arguments { pub fn parse(&self) -> Result<u8, ()> { Err(()) } } pub fn respond(arguments: &Arguments, service: impl Fn() -> u8) { let _ = arguments.parse(); let _ = service(); }",
        "pub trait ValueInterface { fn parse(&self) -> Result<u8, ()>; } pub fn respond(arguments: &impl ValueInterface, service: impl Fn() -> u8) { let _ = arguments.parse(); let _ = service(); }",
        "pub struct Result; pub fn respond(service: impl Fn() -> Result) { let _ = service(); }",
        "pub struct Answer; pub mod interface { pub trait ServiceInterface { type Output; fn read(&self) -> Self::Output; } } use interface::ServiceInterface; pub fn respond(service: &impl ServiceInterface<Output = Answer>) { let _ = service.read(); }",
    ] {
        assert_success(consumer.check_tests(&source(body, UNIVERSAL)));
        rejected(
            consumer.check_tests(&source(body, "")),
            "check_preserves_supplied_service_contract",
        );
    }
}

#[test]
fn unresolved_and_inactive_proofs_cannot_grant_infallibility() {
    let consumer = NativeConsumer::new("command-unproven");
    for body in [
        "pub mod interface { pub trait ServiceInterface { type Output; fn read(&self) -> Self::Output; } } use interface::ServiceInterface; pub fn respond(service: &impl ServiceInterface) { let _ = service.read(); }",
        "pub fn respond<T>(service: impl Fn() -> T) { let _ = service(); }",
        "pub fn respond(service: fn() -> u8) { let mut alias = service; let _ = alias(); }",
        "pub fn respond(service: fn() -> Result<u8, ()>) { let (alias,) = (service,); let _ = alias(); }",
        "fn forward<F>(f: F) -> F { f } pub fn respond(service: fn() -> Result<u8, ()>) { let alias = forward(service); let _ = alias(); }",
    ] {
        println!("checking unresolved supplied call: {body}");
        rejected(
            consumer.check_tests(&source(body, &format!("{UNIVERSAL}{FAILURE}"))),
            "unresolved",
        );
    }
    let fallible = "pub fn respond(service: impl Fn() -> Result<u8, ()>) { let _ = service(); }";
    for invalid in [
        "fn check_fails_when_the_service_fails() {}",
        "#[test] #[ignore] fn check_fails_when_the_service_fails() {}",
        "#[test] #[should_panic] fn check_fails_when_the_service_fails() { panic!(\"fake error\"); }",
    ] {
        println!("checking inactive proof: {invalid}");
        rejected(
            consumer.check_tests(&source(fallible, &format!("{UNIVERSAL}{invalid}"))),
            "check_fails_when_the_service_fails",
        );
    }
    // The same source passes a production-only check but must fail the configured
    // all-target native invocation. A missing test-target flag is not a green gate.
    let missing = source(fallible, UNIVERSAL);
    assert_success(consumer.check(&missing));
    rejected(
        consumer.check_tests(&missing),
        "check_fails_when_the_service_fails",
    );
}

/// A consumer's cargo never builds into a target the caller's environment names: under a shared `CARGO_TARGET_DIR`
/// the consumers of tests run at once would share one build and see each other's results.
#[test]
fn a_consumer_build_never_inherits_the_callers_target() {
    let command = cargo(std::path::Path::new("."));
    let removed: Vec<String> = command
        .get_envs()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect();
    assert!(
        removed.iter().any(|name| name == "CARGO_TARGET_DIR")
            && removed.iter().any(|name| name == "CARGO_BUILD_TARGET_DIR"),
        "{removed:?}"
    );
}
